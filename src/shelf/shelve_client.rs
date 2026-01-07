use std::path::{Path, PathBuf};
use p4rs::{P4, P4Error, P4Command, ChangeSpec, ChangeType, FileType};

#[derive(PartialEq, Eq)]
pub enum FileAction {
    Add,
    Edit,
    Delete,
}

pub struct FileChange<'a> {
    pub path: &'a str,
    pub action: FileAction,
    pub executable: bool,
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
            if !self.p4.files(&[&depot_path]).run().is_ok() {
                continue
            }
            match self.p4.sync(&[&depot_path]).metadata_only().run() {
                Err(P4Error::CommandSpecificError(msg, _)) if msg.contains("file(s) up-to-date") => {},
                Ok(_) => {},
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn create_empty_change(&self, description: &str) -> Result<usize, P4Error> {
        let change_spec = ChangeSpec::new(ChangeType::New).description(description);
        self.p4.change().set(&change_spec).run()
    }

    fn apply_changes(&self, changelist: usize, base_dir: &Path, changes: &[FileChange]) -> Result<(), P4Error> {
        let (mut adds, mut adds_x, mut edits, mut edits_x, mut deletes) = 
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());

        for change in changes {
            let depot_path = format!("//{}/{}", self.client_name, change.path);
            match (&change.action, change.executable) {
                (FileAction::Add, false) => adds.push(depot_path),
                (FileAction::Add, true) => adds_x.push(depot_path),
                (FileAction::Edit, false) => edits.push(depot_path),
                (FileAction::Edit, true) => edits_x.push(depot_path),
                (FileAction::Delete, _) => deletes.push(depot_path),
            }
        }

        if !edits.is_empty() {
            let refs: Vec<&str> = edits.iter().map(|s| s.as_str()).collect();
            self.p4.edit(&refs).changelist(changelist).run()?;
        }
        if !edits_x.is_empty() {
            let refs: Vec<&str> = edits_x.iter().map(|s| s.as_str()).collect();
            self.p4.edit(&refs).changelist(changelist).file_type(FileType::text().executable()).run()?;
        }

        for change in changes {
            let dest = self.client_root.join(&change.path);
            if change.action != FileAction::Delete {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(base_dir.join(change.path), &dest)?;
            }
        }

        if !adds.is_empty() {
            let refs: Vec<&str> = adds.iter().map(|s| s.as_str()).collect();
            self.p4.add(&refs).changelist(changelist).run()?;
        }
        if !adds_x.is_empty() {
            let refs: Vec<&str> = adds_x.iter().map(|s| s.as_str()).collect();
            self.p4.add(&refs).changelist(changelist).file_type(FileType::text().executable()).run()?;
        }
        if !deletes.is_empty() {
            let refs: Vec<&str> = deletes.iter().map(|s| s.as_str()).collect();
            self.p4.delete(&refs).changelist(changelist).run()?;
        }
        Ok(())
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
            .submit().submitted_change
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

        let changes = [FileChange {
            path: "new.txt",
            action: FileAction::Add,
            executable: false,
        }];
        let cl = client.run(0, &tmp.path(), &changes, "Add new file", None).unwrap();
        let shelved = &tc.p4.describe(&[cl]).run().unwrap()[0];
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

        let changes = [FileChange {
            path: "file1.txt",
            action: FileAction::Edit,
            executable: false,
        }];
        let cl = client.run(base, tmp.path(), &changes, "Edit file", None).unwrap();
        
        let shelved = &tc.p4.describe(&[cl]).run().unwrap()[0];
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

        let changes = [FileChange {
            path: "file1.txt",
            action: FileAction::Delete,
            executable: false,
        }];
        let cl = client.run(base, tmp.path(), &changes, "Delete file", None).unwrap();
        
        let shelved = &tc.p4.describe(&[cl]).run().unwrap()[0];
        assert_eq!(shelved.description.trim(), "Delete file");
        
        tc.p4.shelve().delete(cl).run().unwrap();
    }

    #[test]
    fn test_shelve_executable_file() {
        let tc = SERVER.test_client();
        setup_test_files(&tc);

        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("script.sh"), b"#!/bin/bash\necho hello").unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        ).unwrap();

        let changes = [FileChange {
            path: "script.sh",
            action: FileAction::Add,
            executable: true,
        }];
        let cl = client.run(0, tmp.path(), &changes, "Add executable", None).unwrap();
        
        let shelved = &tc.p4.describe(&[cl]).run().unwrap()[0];
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
            FileChange { path: "new.txt", action: FileAction::Add, executable: false },
            FileChange { path: "file2.txt", action: FileAction::Edit, executable: false },
            FileChange { path: "file1.txt", action: FileAction::Delete, executable: false },
        ];
        let cl = client.run(base, tmp.path(), &changes, "Multiple changes", None).unwrap();
        
        let shelved = &tc.p4.describe(&[cl]).run().unwrap()[0];
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
            .submit().submitted_change;
        
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
        
        let changes = [FileChange {
            path: "evolving.txt",
            action: FileAction::Edit,
            executable: false,
        }];
        
        let cl = client.run(base2, tmp.path(), &changes, "Edit from base2", None)
            .expect("Failed to run shelve client");
        
        let shelved = &tc.p4.describe(&[cl]).shelved().run()
            .expect("Failed to describe shelved")[0];
        assert_eq!(shelved.files.len(), 1);
        assert_eq!(shelved.files[0].rev, Some(2));
        assert!(shelved.files[0].depot_file.ends_with("evolving.txt"));
        
        tc.p4.shelve().delete(cl).run().unwrap();
    }

}
