use git2::{FileMode, Repository};
use p4rs::{BaseFileType, ChangeData, FileAction, P4Command, P4Error, P4};

use super::commit_builder::CommitBuilder;
use super::error::MirrorError;
use super::mirror_data::{IntegrateStrategy, MirrorData};

pub struct Mirror {
    p4: P4,
    repo: Repository,
    mirror_data: MirrorData,
}

impl Mirror {
    pub fn new(p4: P4, repo: Repository, mirror_data: MirrorData) -> Self {
        Self {
            p4,
            repo,
            mirror_data,
        }
    }

    pub fn run(&mut self) -> Result<(), MirrorError> {
        loop {
            let changes = self.fetch_changes()?;
            if changes.is_empty() {
                break;
            }
            for change in changes {
                self.process_change(change)?;
            }
        }
        Ok(())
    }

    pub fn last_sync_change(&self) -> usize {
        self.mirror_data.last_sync_change()
    }

    fn fetch_changes(&mut self) -> Result<Vec<ChangeData>, P4Error> {
        let path = format!("//{}/...", self.mirror_data.p4_client);
        let paths: &[&str] = &[path.as_str()];
        let mut cmd = self
            .p4
            .changes(paths)
            .since_changelist(self.mirror_data.last_sync_change() + 1)
            .reverse();
        if let Some(max) = self.mirror_data.max_changes_query {
            cmd = cmd.max_changes(max);
        }
        Ok(cmd.run()?)
    }

    fn process_change(&mut self, change: ChangeData) -> Result<(), MirrorError> {
        let ctx = self.fetch_change_context(&change)?;
        log::debug!("Attempting to create commit for change {change:?}");
        self.create_commit(&change, &ctx)?;
        self.mirror_data.set_last_sync_change(change.change);
        Ok(())
    }

    fn fetch_change_context(&mut self, change: &ChangeData) -> Result<ChangeContext, MirrorError> {
        let email = self.resolve_user_email(&change.user)?;
        let temp_dir = tempfile::tempdir().map_err(|e| {
            MirrorError::MirrorFailed(format!("Failed to create temporary directory: {}", e))
        })?;
        let client_path = format!("//{}/...", self.mirror_data.p4_client);
        let file_spec = format!("{}@={}", client_path, change.change);

        let file_data = self
            .p4
            .print()
            .to_file(
                &[file_spec.as_str()],
                format!("{}/...", temp_dir.path().display()).as_str(),
            )
            .run()?;

        let where_result = self.p4.where_cmd(&[&client_path]).run()?;
        let depot_base = where_result
            .first()
            .and_then(|w| w.depot_file.strip_suffix("..."))
            .ok_or_else(|| MirrorError::MirrorFailed("Failed to get client base path".to_string()))?
            .to_string();

        Ok(ChangeContext {
            email,
            file_data,
            depot_base,
            temp_dir,
        })
    }

    fn resolve_user_email(&mut self, user: &str) -> Result<String, MirrorError> {
        if let Some(e) = self.mirror_data.get_user_email(user) {
            return Ok(e.clone());
        }
        let user_info = self.p4.user().get(user).run()?;
        self.mirror_data
            .set_user_email(user, user_info.email.clone());
        Ok(user_info.email)
    }

    fn create_commit(&self, change: &ChangeData, ctx: &ChangeContext) -> Result<(), MirrorError> {
        let mut builder = CommitBuilder::from_head(&self.repo)?;
        if let Some(branch) = self
            .mirror_data
            .get_related_branch(change.old_change.unwrap_or(change.change))
        {
            match self.mirror_data.integrate_strategy {
                IntegrateStrategy::MergeOurs => {
                    let result = builder.add_parent_from_ref(branch);
                    if result.is_err() {
                        log::warn!(
                            "Failed to add branch {branch} as a parent {result:?}. skipping..."
                        );
                    }
                    log::debug!("Added branch {branch} as a parent for change {change:?}");
                }
            }
        }

        for file in &ctx.file_data {
            let path_in_repo = file
                .depot_file
                .strip_prefix(&ctx.depot_base)
                .ok_or_else(|| {
                    MirrorError::MirrorFailed(format!(
                        "Failed to get path in repository for file {}: {}",
                        file.depot_file, ctx.depot_base
                    ))
                })?;
            let path_in_temp = ctx.temp_dir.path().join(path_in_repo);
            let mode = Self::file_mode(&file.file_type);

            match file.action {
                FileAction::Add | FileAction::Edit | FileAction::MoveAdd => {
                    builder.upsert(path_in_repo, &path_in_temp, mode)?;
                }
                FileAction::Delete | FileAction::MoveDelete => {
                    builder.remove(path_in_repo);
                }
                FileAction::Branch | FileAction::Integrate => {}
            }
        }

        let commit_hash = builder.commit(&change.user, &ctx.email, change.time, &change.desc)?;
        log::debug!("Committed change {change:?} with hash {commit_hash}");
        Ok(())
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
    depot_base: String,
    temp_dir: tempfile::TempDir,
}
