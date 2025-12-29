use chrono::{DateTime, Utc};
use git2::build::TreeUpdateBuilder;
use git2::{FileMode, Oid, Repository, Signature, Time};
use std::path::Path;

use super::error::MirrorError;

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
        let blob = self.repo.blob_path(file_path)?;
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
        Ok(oid)
    }

    fn build_tree(&mut self) -> Result<Oid, MirrorError> {
        if let Some(&parent_oid) = self.parents.first() {
            let parent = self.repo.find_commit(parent_oid)?;
            let tree = parent.tree()?;
            Ok(self.tree_builder.create_updated(self.repo, &tree)?)
        } else {
            let empty = self.repo.treebuilder(None)?;
            Ok(empty.write()?)
        }
    }
}
