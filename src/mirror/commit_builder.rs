use chrono::{DateTime, Utc};
use git2::build::TreeUpdateBuilder;
use git2::{FileMode, Oid, Repository, Signature, Time};
use std::path::Path;

use super::error::MirrorError;

pub struct CommitMetadata {
    pub change: usize,
    pub old_change: Option<usize>,
    pub client: String,
}

pub struct CommitBuilder<'r> {
    repo: &'r Repository,
    tree_builder: TreeUpdateBuilder,
    pending_removes: Vec<String>,
    parents: Vec<Oid>,
}

impl<'r> CommitBuilder<'r> {
    pub fn new(repo: &'r Repository) -> Self {
        Self {
            repo,
            tree_builder: TreeUpdateBuilder::new(),
            pending_removes: Vec::new(),
            parents: Vec::new(),
        }
    }

    pub fn from_head(repo: &'r Repository) -> Result<Self, MirrorError> {
        let mut builder = Self::new(repo);
        if let Some(oid) = repo.head().ok().and_then(|h| h.target()) {
            builder.parents.push(oid);
        }
        Ok(builder)
    }

    pub fn add_parent_from_ref(&mut self, ref_name: &str) -> Result<(), MirrorError> {
        let reference = self.repo.find_reference(ref_name)?;
        let oid = reference.target().ok_or_else(|| {
            MirrorError::MirrorFailed(format!("Reference {} has no target", ref_name))
        })?;
        self.parents.push(oid);
        Ok(())
    }

    pub fn upsert(
        &mut self,
        path: &str,
        file_path: &Path,
        mode: FileMode,
    ) -> Result<(), MirrorError> {
        let blob = if mode == FileMode::Link {
            // p4 print -o creates actual symlinks on disk. Read the link target
            // and store it as blob content (git symlinks are blobs containing the target path).
            let target = std::fs::read_link(file_path).map_err(|e| {
                MirrorError::MirrorFailed(format!(
                    "Failed to read symlink at {}: {}",
                    file_path.display(),
                    e
                ))
            })?;
            self.repo.blob(target.as_os_str().as_encoded_bytes())?
        } else {
            self.repo.blob_path(file_path)?
        };
        self.tree_builder.upsert(path, blob, mode);
        Ok(())
    }

    pub fn remove(&mut self, path: &str) {
        self.pending_removes.push(path.to_string());
    }

    pub fn commit(
        mut self,
        author: &str,
        email: &str,
        time: DateTime<Utc>,
        message: &str,
        metadata: &CommitMetadata,
    ) -> Result<Oid, MirrorError> {
        let signature = Signature::new(author, email, &Time::new(time.timestamp(), 0))?;
        let tree_id = self.build_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let parent_commits: Vec<_> = self
            .parents
            .iter()
            .filter_map(|oid| self.repo.find_commit(*oid).ok())
            .collect();
        let parents: Vec<_> = parent_commits.iter().collect();

        let oid = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?;

        let note_content = Self::format_note(metadata);
        self.repo.note(&signature, &signature, Some("refs/notes/p4"), oid, &note_content, false)?;

        Ok(oid)
    }

    fn format_note(metadata: &CommitMetadata) -> String {
        let mut note = format!("P4-Change: {}\nP4-Client: {}", metadata.change, metadata.client);
        if let Some(old) = metadata.old_change {
            note.push_str(&format!("\nP4-OldChange: {}", old));
        }
        note
    }

    fn build_tree(&mut self) -> Result<Oid, MirrorError> {
        let base_tree = match self.parents.first() {
            Some(&parent_oid) => self.repo.find_commit(parent_oid)?.tree()?,
            None => {
                let empty_oid = self.repo.treebuilder(None)?.write()?;
                self.repo.find_tree(empty_oid)?
            }
        };
        for path in &self.pending_removes {
            if base_tree.get_path(Path::new(path)).is_ok() {
                self.tree_builder.remove(path);
            } else {
                log::warn!("Skipping remove of '{path}': not in tree");
            }
        }
        Ok(self.tree_builder.create_updated(self.repo, &base_tree)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn setup_repo_with_file(dir: &TempDir) -> Repository {
        let repo = Repository::init_bare(dir.path()).unwrap();
        let blob_oid = repo.blob(b"hello").unwrap();
        let tree_oid = {
            let mut tb = repo.treebuilder(None).unwrap();
            tb.insert("existing.txt", blob_oid, FileMode::Blob.into())
                .unwrap();
            tb.write().unwrap()
        };
        {
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = Signature::now("test", "test@test.com").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }
        repo
    }

    fn dummy_metadata() -> CommitMetadata {
        CommitMetadata {
            change: 1,
            old_change: None,
            client: "test".to_string(),
        }
    }

    #[test]
    fn remove_existing_file_produces_correct_tree() {
        let dir = TempDir::new().unwrap();
        let repo = setup_repo_with_file(&dir);

        let mut builder = CommitBuilder::from_head(&repo).unwrap();
        builder.remove("existing.txt");
        let oid = builder
            .commit("test", "test@test.com", Utc::now(), "delete", &dummy_metadata())
            .unwrap();

        let commit = repo.find_commit(oid).unwrap();
        let tree = commit.tree().unwrap();
        assert!(
            tree.get_name("existing.txt").is_none(),
            "File should be removed from tree"
        );
    }

    #[test]
    fn remove_nonexistent_file_succeeds() {
        let dir = TempDir::new().unwrap();
        let repo = setup_repo_with_file(&dir);

        let mut builder = CommitBuilder::from_head(&repo).unwrap();
        builder.remove("no_such_file.txt");
        let result = builder.commit("test", "test@test.com", Utc::now(), "noop delete", &dummy_metadata());

        assert!(result.is_ok(), "Removing non-existent file should not error");
    }

    #[test]
    fn double_remove_succeeds() {
        let dir = TempDir::new().unwrap();
        let repo = setup_repo_with_file(&dir);

        // First commit: remove the file
        let mut builder = CommitBuilder::from_head(&repo).unwrap();
        builder.remove("existing.txt");
        builder
            .commit("test", "test@test.com", Utc::now(), "delete", &dummy_metadata())
            .unwrap();

        // Second commit: remove the same file again (already gone)
        let mut builder2 = CommitBuilder::from_head(&repo).unwrap();
        builder2.remove("existing.txt");
        let result = builder2.commit("test", "test@test.com", Utc::now(), "double delete", &dummy_metadata());

        assert!(result.is_ok(), "Double-remove should not error");
    }
}
