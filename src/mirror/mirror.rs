use git2::Repository;
use p4rs::{ChangeData, P4, P4Command, P4Error};

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
            .since_changelist(self.mirror_data.last_sync_change());
        if let Some(max) = self.mirror_data.max_changes_query {
            cmd = cmd.max_changes(max);
        }
        Ok(cmd.run()?)
    }

    fn process_change(&mut self, change: ChangeData) -> Result<(), MirrorError> {
        let email = self.mirror_data.get_user_email(&change.user);
        if email.is_none() {
            let user_info = self.p4.user().get(change.user).run()?;
            let email = user_info.email;
            self.mirror_data.set_user_email(&change.user, email);
        }
        let mut index = self.repo.index()?;
        let temp_dir = tempfile::tempdir().unwrap();  // TODO: handle error
        let print_result = self.p4.print()
            .to_file(
            &[format!("//{}/...@={}", self.mirror_data.p4_client, change.change).as_str()],
            format!("{}/...", temp_dir.path().display()).as_str()
            )
            .run()?;


        Ok(())
    }
}