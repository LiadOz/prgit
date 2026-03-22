use std::time::Instant;

use chrono::Utc;
use git2::Repository;

use crate::cabinet::Database;
use crate::mirror::{Mirror, MirrorChangeInfo, MirrorData};

use super::observability::{EventEmitter, ObservabilityEvent};
use super::ServerConfig;

pub fn spawn_all(config: &ServerConfig, emitter: &EventEmitter) {
    let db_path = config.data_dir.join("prgit.db");
    let db_path_str = db_path.to_string_lossy().to_string();
    for repo_config in &config.repos {
        let db_path = db_path_str.clone();
        let p4client = repo_config.p4client.clone();
        let interval_secs = repo_config.mirror_interval_secs;
        let repo_config = repo_config.clone();
        let data_dir = config.data_dir.clone();
        let task_client = p4client.clone();
        let emitter = emitter.clone();
        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(interval_secs);
            let repo_name = format!("{}/{}", repo_config.group, repo_config.name);
            loop {
                let db_path = db_path.clone();
                let rc = repo_config.clone();
                let dd = data_dir.clone();
                let emitter_clone = emitter.clone();
                let repo_name_clone = repo_name.clone();

                // Emit cycle started
                let last_sync = get_last_sync(&db_path, &rc.p4client);
                emitter.try_emit(ObservabilityEvent::MirrorCycleStarted {
                    timestamp: Utc::now(),
                    repo: repo_name.clone(),
                    last_sync_change: last_sync,
                });

                let cycle_start = Instant::now();
                let result = tokio::task::spawn_blocking(move || -> Result<MirrorCycleResult, Box<dyn std::error::Error + Send + Sync>> {
                    let db = Database::open(&db_path)?;
                    let client = db
                        .client_by_name(&rc.p4client)?
                        .ok_or_else(|| format!("Client '{}' not found", rc.p4client))?;
                    let last_before = client.last_sync_change();
                    let bare_path = rc.bare_repo_path(&dd);
                    let repo = Repository::open_bare(&bare_path)?;
                    let p4 = client.p4();
                    let mut mirror = Mirror::new(p4, repo, client);
                    let change_infos = mirror.run()?;

                    // Look up shelve.merged info for changes that had merge parents
                    let mut merged = Vec::new();
                    let db2 = Database::open(&db_path)?;
                    for info in &change_infos {
                        if let Some(ref branch) = info.merge_parent {
                            let client2 = db2.client_by_name(&rc.p4client)?.ok_or("client not found")?;
                            if let Some(shelved_cl) = client2.get_shelved_change_for_branch(branch) {
                                let shelver_user = client2.get_shelver_for_change(shelved_cl)
                                    .unwrap_or_default();
                                merged.push(ShelveMergedInfo {
                                    branch: branch.clone(),
                                    shelved_cl,
                                    submitted_cl: info.p4_change,
                                    shelver_user,
                                });
                            }
                        }
                    }

                    let last_after = mirror.last_sync_change();
                    let changes_synced = last_after.saturating_sub(last_before);
                    Ok(MirrorCycleResult { last_change: last_after, changes_synced, change_infos, merged })
                })
                .await;

                let duration_ms = cycle_start.elapsed().as_millis() as u64;
                match result {
                    Ok(Ok(cycle)) => {
                        log::debug!(
                            "Mirror '{task_client}': synced to change {}",
                            cycle.last_change
                        );

                        // Emit per-change events
                        for info in &cycle.change_infos {
                            emitter_clone.try_emit(ObservabilityEvent::MirrorChangeCommitted {
                                timestamp: Utc::now(),
                                repo: repo_name_clone.clone(),
                                p4_change: info.p4_change,
                                commit_hash: info.commit_hash.clone(),
                                user: info.user.clone(),
                                file_count: info.file_count,
                                duration_ms: info.duration_ms,
                                merge_parent: info.merge_parent.clone(),
                                merge_strategy: info.merge_strategy.clone(),
                            });
                            for depot_path in &info.skipped_files {
                                emitter_clone.try_emit(ObservabilityEvent::MirrorFileSkipped {
                                    timestamp: Utc::now(),
                                    repo: repo_name_clone.clone(),
                                    p4_change: info.p4_change,
                                    depot_path: depot_path.clone(),
                                    reason: ".git path component".to_string(),
                                });
                            }
                        }

                        // Emit shelve.merged events
                        for m in &cycle.merged {
                            emitter_clone.try_emit(ObservabilityEvent::ShelveMerged {
                                timestamp: Utc::now(),
                                repo: repo_name_clone.clone(),
                                branch: m.branch.clone(),
                                shelved_cl: m.shelved_cl,
                                submitted_cl: m.submitted_cl,
                                shelver_user: m.shelver_user.clone(),
                            });
                        }

                        emitter_clone.try_emit(ObservabilityEvent::MirrorCycleCompleted {
                            timestamp: Utc::now(),
                            repo: repo_name_clone,
                            changes_synced: cycle.changes_synced,
                            new_last_sync: cycle.last_change,
                            duration_ms,
                        });
                    }
                    Ok(Err(e)) => {
                        log::error!("Mirror '{task_client}' failed: {e}");
                        emitter_clone.try_emit(ObservabilityEvent::MirrorCycleFailed {
                            timestamp: Utc::now(),
                            repo: repo_name_clone,
                            error: e.to_string(),
                            duration_ms,
                        });
                    }
                    Err(e) => {
                        log::error!("Mirror '{task_client}' task panicked: {e}");
                        emitter_clone.try_emit(ObservabilityEvent::MirrorCycleFailed {
                            timestamp: Utc::now(),
                            repo: repo_name_clone,
                            error: format!("Task panicked: {e}"),
                            duration_ms,
                        });
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });
        log::info!("Started mirror task for '{p4client}' (every {interval_secs}s)");
    }
}

struct MirrorCycleResult {
    last_change: usize,
    changes_synced: usize,
    change_infos: Vec<MirrorChangeInfo>,
    merged: Vec<ShelveMergedInfo>,
}

struct ShelveMergedInfo {
    branch: String,
    shelved_cl: usize,
    submitted_cl: usize,
    shelver_user: String,
}

fn get_last_sync(db_path: &str, p4client: &str) -> usize {
    let db = match Database::open(db_path) {
        Ok(db) => db,
        Err(_) => return 0,
    };
    match db.client_by_name(p4client) {
        Ok(Some(client)) => client.last_sync_change(),
        _ => 0,
    }
}
