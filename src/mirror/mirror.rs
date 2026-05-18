use std::time::Instant;

use git2::{FileMode, Repository};
use p4rs::{BaseFileType, ChangeData, FileAction, P4Command, P4Error, WhereResult, P4};

use super::commit_builder::{CommitBuilder, CommitMetadata};
use super::error::MirrorError;
use super::mirror_data::{IntegrateStrategy, MirrorData};

/// A single line of a P4 client view, normalized: prefixes have the
/// trailing `...` stripped so a depot-side path can be matched and
/// rewritten to its client-relative location.
struct ViewMapping {
    depot_prefix: String,
    client_subdir: String,
}

fn parse_view_mappings(where_results: &[WhereResult], client_name: &str) -> Vec<ViewMapping> {
    let client_root = format!("//{}/", client_name);
    where_results
        .iter()
        .filter_map(|w| {
            if w.depot_file.starts_with('-') {
                return None;
            }
            let depot_prefix = w.depot_file.strip_suffix("...")?.to_string();
            let client_path = w.client_file.strip_suffix("...")?;
            let client_subdir = client_path.strip_prefix(&client_root)?.to_string();
            Some(ViewMapping {
                depot_prefix,
                client_subdir,
            })
        })
        .collect()
}

fn resolve_path_in_repo(mappings: &[ViewMapping], depot_file: &str) -> Option<String> {
    mappings
        .iter()
        .filter(|m| depot_file.starts_with(&m.depot_prefix))
        .max_by_key(|m| m.depot_prefix.len())
        .map(|m| format!("{}{}", m.client_subdir, &depot_file[m.depot_prefix.len()..]))
}

/// Info about a single mirrored change, returned to the caller for observability.
pub struct MirrorChangeInfo {
    pub p4_change: usize,
    pub commit_hash: String,
    pub user: String,
    pub file_count: usize,
    pub duration_ms: u64,
    pub merge_parent: Option<String>,
    pub merge_strategy: Option<String>,
    pub skipped_files: Vec<String>,
}

pub struct Mirror<M: MirrorData> {
    p4: P4,
    repo: Repository,
    mirror_data: M,
}

impl<M: MirrorData> Mirror<M> {
    pub fn new(p4: P4, repo: Repository, mirror_data: M) -> Self {
        Self {
            p4,
            repo,
            mirror_data,
        }
    }

    pub fn run(&mut self) -> Result<Vec<MirrorChangeInfo>, MirrorError> {
        let mut infos = Vec::new();
        loop {
            let changes = self.fetch_changes()?;
            if changes.is_empty() {
                break;
            }
            for change in changes {
                infos.push(self.process_change(change)?);
            }
        }
        Ok(infos)
    }

    pub fn last_sync_change(&self) -> usize {
        self.mirror_data.last_sync_change()
    }

    fn fetch_changes(&mut self) -> Result<Vec<ChangeData>, P4Error> {
        let path = format!("//{}/...", self.mirror_data.p4_client());
        let paths: &[&str] = &[path.as_str()];
        let mut cmd = self
            .p4
            .changes(paths)
            .long()
            .since_changelist(self.mirror_data.last_sync_change() + 1)
            .reverse();
        if let Some(max) = self.mirror_data.max_changes_query() {
            cmd = cmd.max_changes(max);
        }
        Ok(cmd.run()?.results)
    }

    fn process_change(&mut self, change: ChangeData) -> Result<MirrorChangeInfo, MirrorError> {
        let start = Instant::now();
        let ctx = self.fetch_change_context(&change)?;
        let file_count = ctx.file_data.len();
        log::debug!("Attempting to create commit for change {change:?}");

        // Detect merge parent before creating commit
        let related_branch = self
            .mirror_data
            .get_related_branch(change.old_change.unwrap_or(change.change));

        let (commit_hash, skipped_files) =
            self.create_commit(&change, &ctx).map_err(|e| {
                MirrorError::MirrorFailed(format!(
                    "Failed to commit change {} (user={}, client={}, files={}, desc={:?}): {}",
                    change.change, change.user, change.client, file_count, change.desc, e
                ))
            })?;
        self.mirror_data
            .map_commit_to_change(&commit_hash, change.change);
        self.mirror_data.set_last_sync_change(change.change);

        let (merge_parent, merge_strategy) = match &related_branch {
            Some(branch) => (Some(branch.clone()), Some("merge_ours".to_string())),
            None => (None, None),
        };

        Ok(MirrorChangeInfo {
            p4_change: change.change,
            commit_hash,
            user: change.user,
            file_count,
            duration_ms: start.elapsed().as_millis() as u64,
            merge_parent,
            merge_strategy,
            skipped_files,
        })
    }

    fn fetch_change_context(&mut self, change: &ChangeData) -> Result<ChangeContext, MirrorError> {
        let email = self
            .resolve_user_email(&change.user)
            .unwrap_or("unknown".to_string());
        let temp_dir = tempfile::tempdir().map_err(|e| {
            MirrorError::MirrorFailed(format!("Failed to create temporary directory: {}", e))
        })?;
        let client_path = format!("//{}/...", self.mirror_data.p4_client());
        let file_spec = format!("{}@={}", client_path, change.change);

        let file_data = self
            .p4
            .print()
            .to_file(
                &[file_spec.as_str()],
                format!("{}/...", temp_dir.path().display()).as_str(),
            )
            .run()?
            .results;

        let where_result = self.p4.where_cmd(&[&client_path]).run()?;
        let view_mappings =
            parse_view_mappings(&where_result.results, self.mirror_data.p4_client());
        if view_mappings.is_empty() {
            return Err(MirrorError::MirrorFailed(
                "Client view has no usable mappings".to_string(),
            ));
        }

        Ok(ChangeContext {
            email,
            file_data,
            view_mappings,
            temp_dir,
        })
    }

    fn resolve_user_email(&mut self, user: &str) -> Result<String, MirrorError> {
        if let Some(e) = self.mirror_data.get_user_email(user) {
            return Ok(e);
        }
        let user_info = self.p4.user().get(user).run()?.single()?;
        self.mirror_data.set_user_email(user, &user_info.email);
        Ok(user_info.email)
    }

    fn create_commit(
        &self,
        change: &ChangeData,
        ctx: &ChangeContext,
    ) -> Result<(String, Vec<String>), MirrorError> {
        let mut builder = CommitBuilder::from_head(&self.repo)?;
        if let Some(branch) = self
            .mirror_data
            .get_related_branch(change.old_change.unwrap_or(change.change))
        {
            match self.mirror_data.integrate_strategy() {
                IntegrateStrategy::MergeOurs => {
                    let result = builder.add_parent_from_ref(&branch);
                    if result.is_err() {
                        log::warn!(
                            "Failed to add branch {branch} as a parent {result:?}. skipping..."
                        );
                    }
                    log::debug!("Added branch {branch} as a parent for change {change:?}");
                }
            }
        }

        let mut skipped_files = Vec::new();
        for file in &ctx.file_data {
            let path_in_repo =
                resolve_path_in_repo(&ctx.view_mappings, &file.depot_file).ok_or_else(|| {
                    MirrorError::MirrorFailed(format!(
                        "Failed to get path in repository for file {}: no matching client view mapping",
                        file.depot_file
                    ))
                })?;

            if std::path::Path::new(&path_in_repo)
                .components()
                .any(|c| c.as_os_str() == ".git")
            {
                log::warn!(
                    "Skipping file with .git path component: {}",
                    file.depot_file
                );
                skipped_files.push(file.depot_file.clone());
                continue;
            }

            let path_in_temp = ctx.temp_dir.path().join(&path_in_repo);
            let mode = Self::file_mode(&file.file_type);

            match file.action {
                FileAction::Add
                | FileAction::Edit
                | FileAction::MoveAdd
                | FileAction::Branch
                | FileAction::Integrate => {
                    builder.upsert(&path_in_repo, &path_in_temp, mode)?;
                }
                FileAction::Delete | FileAction::MoveDelete => {
                    builder.remove(&path_in_repo);
                }
            }
        }

        let metadata = CommitMetadata {
            change: change.change,
            old_change: change.old_change,
            client: change.client.clone(),
        };
        let commit_hash = builder.commit(
            &change.user,
            &ctx.email,
            change.time,
            &change.desc,
            &metadata,
        )?;
        log::debug!("Committed change {change:?} with hash {commit_hash}");
        Ok((commit_hash.to_string(), skipped_files))
    }

    fn file_mode(file_type: &p4rs::FileType) -> FileMode {
        match file_type.base {
            BaseFileType::Symlink => FileMode::Link,
            BaseFileType::Text if file_type.executable => FileMode::BlobExecutable,
            _ => FileMode::Blob,
        }
    }
}

struct ChangeContext {
    email: String,
    file_data: Vec<p4rs::PrintFileInfo>,
    view_mappings: Vec<ViewMapping>,
    temp_dir: tempfile::TempDir,
}
