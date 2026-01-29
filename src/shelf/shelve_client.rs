use std::path::{Path, PathBuf};
use p4rs::{P4, P4Error, P4Command, ChangeSpec, ChangeType, ChangelistBuilder};

#[derive(PartialEq, Eq)]
pub enum FileAction {
    Add,
    Edit,
    Delete,
}

pub struct FileChange<'a> {
    pub path: &'a str,
    pub action: FileAction,
}

pub struct ShelveClient {
    p4: P4,
    client_name: String,
    client_root: PathBuf,
}

impl ShelveClient {
    pub fn new(p4: P4, client_name: &str, client_root: PathBuf) -> Result<Self, P4Error> {
        let p4 = p4.client_name(client_name);
        if !p4.opened(&["//..."]).run()?.is_empty() {
            p4.revert(&["//..."]).run()?;
        }
        Ok(Self { p4, client_name: client_name.to_string(), client_root })
    }

    fn sync(&self, base_change: usize, files: &[&str]) -> Result<(), P4Error> {
        for file in files {
            let depot_path = format!("//{}/{}@{base_change}", self.client_name, file);
            if self.p4.files(&[&depot_path]).run().is_err() {
                continue
            }
            match self.p4.sync(&[&depot_path]).run() {
                Err(P4Error::Command { ref errors, .. }) if errors.iter().any(|e| e.data.contains("file(s) up-to-date")) => {},
                Ok(_) => {},
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn create_empty_change(&self, description: &str) -> Result<usize, P4Error> {
        let change_spec = ChangeSpec::new(ChangeType::New).description(description);
        self.p4.change().set(&change_spec).run()?.single()
    }

    fn copy_file(src: &Path, dest: &Path) -> std::io::Result<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if src.symlink_metadata()?.file_type().is_symlink() {
            if dest.exists() || dest.is_symlink() {
                std::fs::remove_file(dest)?;
            }
            let target = std::fs::read_link(src)?;
            std::os::unix::fs::symlink(target, dest)?;
        } else {
            if dest.is_symlink() {
                std::fs::remove_file(dest)?;
            }
            std::fs::copy(src, dest)?;
        }
        Ok(())
    }

    fn apply_changes(&self, changelist: usize, base_dir: &Path, changes: &[FileChange]) -> Result<(), P4Error> {
        for change in changes {
            if change.action != FileAction::Delete {
                Self::copy_file(&base_dir.join(change.path), &self.client_root.join(change.path))?;
            }
        }

        let mut builder = ChangelistBuilder::with_changelist(&self.p4, self.client_root.clone(), changelist);
        for change in changes {
            match change.action {
                FileAction::Add => { builder.add(change.path)?; }
                FileAction::Edit => { builder.edit(change.path)?; }
                FileAction::Delete => { builder.delete(change.path)?; }
            }
        }
        builder.flush()
    }

    pub fn run(&self, base_change: usize, base_dir: &Path, changes: &[FileChange], description: &str, original_change: Option<usize>) -> Result<usize, P4Error> {
        self.sync(base_change, &changes.iter().map(|c| c.path).collect::<Vec<&str>>())?;
        let cl = match original_change {
            Some(cl) => cl,
            None => self.create_empty_change(description)?,
        };
        self.apply_changes(cl, base_dir, changes)?;
        self.p4.shelve().set(cl).replace().run()?;
        Ok(cl)
    }
}

impl Drop for ShelveClient {
    fn drop(&mut self) {
        if let Ok(opened) = self.p4.opened(&["//..."]).run() {
            if !opened.is_empty() {
                self.p4.revert(&["//..."]).run().ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p4rs::testkit::SERVER;
    use p4rs::P4Command;
    use std::fs;
    use tempfile::TempDir;
    use test_log::test;

    fn setup_test_files(test_client: &p4rs::testkit::TestClient) -> usize {
        test_client.changelist("Setup files")
            .add_file("file1.txt", b"content 1")
            .add_file("file2.txt", b"content 2")
            .add_file("subdir/file3.txt", b"content 3")
            .submit().unwrap().submitted_change
    }

    #[test]
    fn test_shelve_add_file() {
        let tc = SERVER.test_client();
        setup_test_files(&tc);

        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("new.txt"), b"new content").unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [FileChange { path: "new.txt", action: FileAction::Add }];
        let cl = client.run(0, &tmp.path(), &changes, "Add new file", None).unwrap();
        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Add new file");
        
        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_shelve_edit_file() {
        let tc = SERVER.test_client();
        let base = setup_test_files(&tc);

        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file1.txt"), b"modified content").unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [FileChange { path: "file1.txt", action: FileAction::Edit }];
        let cl = client.run(base, tmp.path(), &changes, "Edit file", None).unwrap();
        
        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Edit file");
        
        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_shelve_delete_file() {
        let tc = SERVER.test_client();
        let base = setup_test_files(&tc);

        let tmp = TempDir::new().unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [FileChange { path: "file1.txt", action: FileAction::Delete }];
        let cl = client.run(base, tmp.path(), &changes, "Delete file", None).unwrap();
        
        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Delete file");
        
        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_shelve_executable_file() {
        use std::os::unix::fs::PermissionsExt;
        let tc = SERVER.test_client();
        setup_test_files(&tc);

        let tmp = TempDir::new().unwrap();
        let script_path = tmp.path().join("script.sh");
        fs::write(&script_path, b"#!/bin/bash\necho hello").unwrap();
        fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [FileChange { path: "script.sh", action: FileAction::Add }];
        let cl = client.run(0, tmp.path(), &changes, "Add executable", None).unwrap();
        
        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Add executable");
        
        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_shelve_multiple_changes() {
        let tc = SERVER.test_client();
        let base = setup_test_files(&tc);

        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("new.txt"), b"brand new").unwrap();
        fs::write(tmp.path().join("file2.txt"), b"edited content").unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [
            FileChange { path: "new.txt", action: FileAction::Add },
            FileChange { path: "file2.txt", action: FileAction::Edit },
            FileChange { path: "file1.txt", action: FileAction::Delete },
        ];
        let cl = client.run(base, tmp.path(), &changes, "Multiple changes", None).unwrap();
        
        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Multiple changes");
        
        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_drop_reverts_files() {
        let tc = SERVER.test_client();
        {
            let _client = ShelveClient::new(
                tc.p4.clone(),
                &tc.client_name,
                tc.client_root().to_path_buf(),
            ).unwrap();
        }
        let opened = tc.p4.opened(&["//..."]).run().unwrap();
        assert!(opened.is_empty());
    }

    #[test]
    fn test_shelve_correct_base_revision() {
        let tc = SERVER.test_client();
        
        tc.changelist("Rev 1")
            .add_file("evolving.txt", b"version 1")
            .submit();
        
        let base2 = tc.changelist("Rev 2")
            .edit_file("evolving.txt", b"version 2")
            .submit().unwrap().submitted_change;
        
        tc.changelist("Rev 3")
            .edit_file("evolving.txt", b"version 3")
            .submit();
        
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("evolving.txt"), b"shelved content").unwrap();
        
        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();
        
        let changes = [FileChange { path: "evolving.txt", action: FileAction::Edit }];
        
        let cl = client.run(base2, tmp.path(), &changes, "Edit from base2", None)
            .expect("Failed to run shelve client");
        
        let shelved = tc.p4.describe(&[cl]).shelved().run()
            .expect("Failed to describe shelved").single().unwrap();
        assert_eq!(shelved.files.len(), 1);
        assert_eq!(shelved.files[0].rev, Some(2));
        assert!(shelved.files[0].depot_file.ends_with("evolving.txt"));
        
        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_shelve_add_symlink() {
        use std::os::unix::fs::symlink;
        let tc = SERVER.test_client();
        setup_test_files(&tc);

        let tmp = TempDir::new().unwrap();
        symlink("target.txt", tmp.path().join("link.txt")).unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [FileChange { path: "link.txt", action: FileAction::Add }];
        let cl = client.run(0, tmp.path(), &changes, "Add symlink", None).unwrap();

        let target = fs::read_link(tc.client_root().join("link.txt")).unwrap();
        assert_eq!(target.to_str().unwrap(), "target.txt");

        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_shelve_edit_symlink() {
        use std::os::unix::fs::symlink;
        use p4rs::FileType;
        let tc = SERVER.test_client();

        let link_path = tc.client_root().join("link.txt");
        symlink("original_target.txt", &link_path).unwrap();
        let base = tc.changelist("Setup symlink")
            .add_file_with_opts("link.txt", b"", Some(FileType::symlink()))
            .submit().unwrap().submitted_change;

        let tmp = TempDir::new().unwrap();
        symlink("new_target.txt", tmp.path().join("link.txt")).unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [FileChange { path: "link.txt", action: FileAction::Edit }];
        let cl = client.run(base, tmp.path(), &changes, "Edit symlink", None).unwrap();

        let target = fs::read_link(tc.client_root().join("link.txt")).unwrap();
        assert_eq!(target.to_str().unwrap(), "new_target.txt");

        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_shelve_file_to_symlink() {
        use std::os::unix::fs::symlink;
        let tc = SERVER.test_client();

        let base = tc.changelist("Setup regular file")
            .add_file("config.txt", b"original content")
            .submit().unwrap().submitted_change;

        let tmp = TempDir::new().unwrap();
        symlink("shared_config.txt", tmp.path().join("config.txt")).unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [FileChange { path: "config.txt", action: FileAction::Edit }];
        let cl = client.run(base, tmp.path(), &changes, "Convert to symlink", None).unwrap();

        let link_path = tc.client_root().join("config.txt");
        assert!(link_path.is_symlink());
        let target = fs::read_link(&link_path).unwrap();
        assert_eq!(target.to_str().unwrap(), "shared_config.txt");

        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_shelve_symlink_to_file() {
        use std::os::unix::fs::symlink;
        use p4rs::FileType;
        let tc = SERVER.test_client();

        let link_path = tc.client_root().join("config.txt");
        symlink("shared_config.txt", &link_path).unwrap();
        let base = tc.changelist("Setup symlink")
            .add_file_with_opts("config.txt", b"", Some(FileType::symlink()))
            .submit().unwrap().submitted_change;

        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("config.txt"), b"inline content").unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [FileChange { path: "config.txt", action: FileAction::Edit }];
        let cl = client.run(base, tmp.path(), &changes, "Convert to regular file", None).unwrap();

        let file_path = tc.client_root().join("config.txt");
        assert!(!file_path.is_symlink());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "inline content");

        tc.p4.shelve().delete(cl).run().unwrap();
    }

}
