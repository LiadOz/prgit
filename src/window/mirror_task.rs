use git2::Repository;

use crate::cabinet::Database;
use crate::mirror::Mirror;

use super::ServerConfig;

pub fn spawn_all(config: &ServerConfig) {
    let db_path = config.data_dir.join("prgit.db");
    let db_path_str = db_path.to_string_lossy().to_string();
    for repo_config in &config.repos {
        let db_path = db_path_str.clone();
        let p4client = repo_config.p4client.clone();
        let interval_secs = repo_config.mirror_interval_secs;
        let repo_config = repo_config.clone();
        let data_dir = config.data_dir.clone();
        let task_client = p4client.clone();
        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(interval_secs);
            loop {
                let db_path = db_path.clone();
                let rc = repo_config.clone();
                let dd = data_dir.clone();
                let result = tokio::task::spawn_blocking(move || -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
                    let db = Database::open(&db_path)?;
                    let client = db
                        .client_by_name(&rc.p4client)?
                        .ok_or_else(|| format!("Client '{}' not found", rc.p4client))?;
                    let bare_path = rc.bare_repo_path(&dd);
                    let repo = Repository::open_bare(&bare_path)?;
                    let p4 = client.p4();
                    let mut mirror = Mirror::new(p4, repo, client);
                    mirror.run()?;
                    Ok(mirror.last_sync_change())
                })
                .await;

                match result {
                    Ok(Ok(change)) => {
                        log::debug!("Mirror '{task_client}': synced to change {change}")
                    }
                    Ok(Err(e)) => log::error!("Mirror '{task_client}' failed: {e}"),
                    Err(e) => log::error!("Mirror '{task_client}' task panicked: {e}"),
                }

                tokio::time::sleep(interval).await;
            }
        });
        log::info!("Started mirror task for '{p4client}' (every {interval_secs}s)");
    }
}
