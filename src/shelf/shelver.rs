use std::path::PathBuf;
use p4rs::{P4, P4Error, P4Command, ChangeSpec, ChangeType, FileType};

#[derive(PartialEq, Eq)]
pub enum FileAction {
    Add,
    Edit,
    Delete,
}

pub struct FileChange {
    pub path: PathBuf,
    pub action: FileAction,
    pub executable: bool,
}

pub struct ShelveClient {
    p4: P4,
    client_root: PathBuf,
}

impl ShelveClient {
    pub fn new(p4: P4, client_name: &str, client_root: PathBuf) -> Result<Self, P4Error> {
        p4.revert(&["/...   "]).run()?;
        Ok(Self { p4: p4.client_name(client_name), client_root })
    }

    pub fn sync(&self, base_change: usize, files: &[&str]) -> Result<(), P4Error> {
        let versioned: Vec<String> = files.iter().map(|f| format!("{f}@{base_change}")).collect();
        self.p4.sync(&versioned.iter().map(|s| s.as_str()).collect::<Vec<_>>()).run()?;
        Ok(())
    }

    pub fn apply_changes(&self, changes: &[FileChange]) -> Result<(), P4Error> {
        let (mut adds, mut adds_x, mut edits, mut edits_x, mut deletes) = 
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());

        for change in changes {
            let dest = self.client_root.join(&change.path);
            if change.action != FileAction::Delete {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&change.path, &dest)?;
            }
            let dest_str = dest.to_string_lossy().into_owned();
            match (&change.action, change.executable) {
                (FileAction::Add, false) => adds.push(dest_str),
                (FileAction::Add, true) => adds_x.push(dest_str),
                (FileAction::Edit, false) => edits.push(dest_str),
                (FileAction::Edit, true) => edits_x.push(dest_str),
                (FileAction::Delete, _) => deletes.push(dest_str),
            }
        }

        if !adds.is_empty() {
            self.p4.add(&adds.iter().map(|s| s.as_str()).collect::<Vec<_>>()).run()?;
        }
        if !adds_x.is_empty() {
            self.p4.add(&adds_x.iter().map(|s| s.as_str()).collect::<Vec<_>>()).file_type(FileType::text().executable()).run()?;
        }
        if !edits.is_empty() {
            self.p4.edit(&edits.iter().map(|s| s.as_str()).collect::<Vec<_>>()).run()?;
        }
        if !edits_x.is_empty() {
            self.p4.edit(&edits_x.iter().map(|s| s.as_str()).collect::<Vec<_>>()).file_type(FileType::text().executable()).run()?;
        }
        if !deletes.is_empty() {
            self.p4.delete(&deletes.iter().map(|s| s.as_str()).collect::<Vec<_>>()).run()?;
        }
        Ok(())
    }

    pub fn shelve(&self, description: &str, original_change: Option<usize>) -> Result<usize, P4Error> {
        let change_spec = match original_change {
            Some(cl) => ChangeSpec::new(ChangeType::Number(cl)),
            None => ChangeSpec::new(ChangeType::New),
        }.description(description);
        let cl = self.p4.change().set(&change_spec).run()?;
        self.p4.shelve().set(cl).run()?;
        Ok(cl)
    }
}

impl Drop for ShelveClient {
    fn drop(&mut self) {
        self.p4.revert(&["//..."]).run().ok();
    }
}

//pub struct Shelver {
//    client: ShelvingClient<Synced>,
//    repo: Repository,
//    base_change: usize,
//    commit_hash: String,
//    base_commit_hash: String,
//}



// impl Shelver {
//     pub fn new(client: P4, repo: Repository, base_change: usize, commit_hash: String, base_commit_hash: String) -> Self {
//         Self {
//             client,
//             repo,
//             base_change,
//             commit_hash,
//             base_commit_hash,
//         }
//     }
// }
