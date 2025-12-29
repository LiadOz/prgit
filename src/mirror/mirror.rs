use git2::{Repository, Signature, FileMode, Time};
use git2::build::TreeUpdateBuilder;
use p4rs::{ChangeData, P4, P4Command, P4Error, FileAction, BaseFileType, PrintFileInfo};

use super::error::MirrorError;
use super::mirror_data::MirrorData;

pub struct Mirror {
    p4: P4,
    repo: Repository,
    mirror_data: MirrorData,
}

impl Mirror {
    pub fn new(p4: P4, repo: Repository, mirror_data: MirrorData) -> Self {
        Self { p4, repo, mirror_data }
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

    fn fetch_changes(&mut self) -> Result<Vec<ChangeData>, P4Error> {
        let path = format!("//{}/...", self.mirror_data.p4_client);
        let paths: &[&str] = &[path.as_str()];
        let mut cmd = self.p4.changes(paths)
            .since_changelist(self.mirror_data.last_sync_change())
            .reverse();
        if let Some(max) = self.mirror_data.max_changes_query {
            cmd = cmd.max_changes(max);
        }
        Ok(cmd.run()?)
    }

    fn process_change(&mut self, change: ChangeData) -> Result<(), MirrorError> {
        let email = match self.mirror_data.get_user_email(&change.user) {
            Some(e) => e.clone(),
            None => {
                let user_info = self.p4.user().get(&change.user).run()?;
                self.mirror_data.set_user_email(&change.user, user_info.email.clone());
                user_info.email
            }
        };
        let temp_dir = tempfile::tempdir().unwrap();  // TODO: handle error
        let print_result = self.p4.print()
            .to_file(
            &[format!("//{}/...@={}", self.mirror_data.p4_client, change.change).as_str()],
            format!("{}/...", temp_dir.path().display()).as_str()
            )
            .run()?;

        let file_spec = format!("//{}/...@={}", self.mirror_data.p4_client, change.change);
        let where_result = self.p4.where_cmd(&[&file_spec]).run()?;
        let client_base = where_result[0].depot_file.strip_suffix("...").unwrap();
        let mut tree_builder = TreeUpdateBuilder::new();
        for file_data in print_result {
            let path_in_repo = file_data.depot_file.strip_prefix(client_base).unwrap();
            let path_in_temp_dir = temp_dir.path().join(path_in_repo);
            let file_mode = match file_data.file_type.base {
                BaseFileType::Symlink => FileMode::Link,  // TODO: we need to test this sometime
                BaseFileType::Text => {
                    if file_data.file_type.executable {
                        FileMode::BlobExecutable
                    } else {
                        FileMode::Blob
                    }
                }
                _ => FileMode::Blob,
            };

            match file_data.action {
                FileAction::Add | FileAction::Edit | FileAction::MoveAdd => {
                    let blob = self.repo.blob_path(&path_in_temp_dir)?;
                    tree_builder.upsert(path_in_repo, blob, file_mode);
                }
                FileAction::Delete | FileAction::MoveDelete => {
                    tree_builder.remove(path_in_repo);
                }
                FileAction::Branch | FileAction::Integrate => continue,
            }
        }

        let signature = Signature::new(&change.user, &email, &Time::new(change.time.timestamp(), 0)).unwrap();
        let parent_commit = self.repo.head().ok()  // TODO refactor
            .and_then(|h| h.target())
            .and_then(|oid| self.repo.find_commit(oid).ok()); // there is always a parent commit in our case will be easier in refactor, actually maybe not see later
        

        let tree_id = if let Some(parent_commit) = parent_commit {
            tree_builder.create_updated(&self.repo, &parent_commit.tree()?)?
        } else {
            let empty_tree = self.repo.treebuilder(None)?;
            empty_tree.write()?
        }
        let tree = self.repo.find_tree(tree_id)?;
        let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();
        if let Some(branch) = self.mirror_data.get_related_branch(change.change) {
            let branch_commit = self.repo.find_commit(self.repo.find_reference(branch.as_str())?.target().unwrap()).unwrap();
            parents.push(&branch_commit);
        }

        self.repo.commit(Some("HEAD"), &signature, &signature, &change.desc, &tree, &parents)?;

        Ok(())
    }
}