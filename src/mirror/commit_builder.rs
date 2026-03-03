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
    parents: Vec<Oid>,
}

impl<'r> CommitBuilder<'r> {
    pub fn new(repo: &'r Repository) -> Self {
        Self {
            repo,
            tree_builder: TreeUpdateBuilder::new(),
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
        self.tree_builder.remove(path);
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
        Ok(self.tree_builder.create_updated(self.repo, &base_tree)?)
    }
}
