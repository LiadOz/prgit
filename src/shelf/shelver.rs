use std::path::Path;

use git2::{Delta, DiffOptions, Repository};
use thiserror::Error;

use crate::cabinet::PrgitClient;

use super::client_pool::{ClientLease, ClientPool, ClientPoolError};
use super::shelve_client::{FileAction, FileChange, ShelveClient};

pub struct Shelver<'a> {
    prgit_client: &'a PrgitClient<'a>,
    pool: ClientPool<'a>,
    repo: Repository,
}

impl<'a> Shelver<'a> {
    pub fn new(prgit_client: &'a PrgitClient<'a>) -> Result<Self, ShelverError> {
        let pool = ClientPool::new(prgit_client)?;
        let repo = Repository::open(&prgit_client.git_config.repo_path)?;
        Ok(Self { prgit_client, pool, repo })
    }

    pub fn shelve(&self, branch: &str) -> Result<usize, ShelverError> {
        let branch_ref = self.repo.find_branch(branch, git2::BranchType::Local)?;
        let target_commit = branch_ref.get().peel_to_commit()?;
        let target_oid = target_commit.id();

        let (base_oid, base_change) = self.find_merge_base_and_change(target_oid)?;
        let base_commit = self.repo.find_commit(base_oid)?;
        let target_commit = self.repo.find_commit(target_oid)?;

        let changes = self.compute_changes(&base_commit, &target_commit)?;
        if changes.is_empty() {
            return Err(ShelverError::NoChanges);
        }

        let description = target_commit.message().unwrap_or("Shelved from git").to_string();
        let existing_shelve = self.prgit_client.get_shelved_change_for_branch(branch);

        let lease = self.pool.acquire()?;
        let shelve_cl = self.execute_shelve(&lease, base_change, &target_commit, &changes, &description, existing_shelve)?;

        self.prgit_client.set_shelved_change_for_branch(branch, shelve_cl);

        Ok(shelve_cl)
    }

    fn find_merge_base_and_change(&self, target_oid: git2::Oid) -> Result<(git2::Oid, usize), ShelverError> {
        let synced_branch = &self.prgit_client.git_config.synced_branch;
        let synced_ref = self.repo.find_branch(synced_branch, git2::BranchType::Local)?;
        let synced_oid = synced_ref.get().peel_to_commit()?.id();

        let base_oid = self.repo.merge_base(synced_oid, target_oid)?;
        let commit_hash = base_oid.to_string();

        let base_change = self.prgit_client
            .get_change_for_commit(&commit_hash)
            .ok_or(ShelverError::NoBaseCommit)?;

        Ok((base_oid, base_change))
    }

    fn compute_changes(
        &self,
        base: &git2::Commit,
        target: &git2::Commit,
    ) -> Result<Vec<ChangedFile>, ShelverError> {
        let base_tree = base.tree()?;
        let target_tree = target.tree()?;

        let mut opts = DiffOptions::new();
        let diff = self.repo.diff_tree_to_tree(Some(&base_tree), Some(&target_tree), Some(&mut opts))?;

        let mut changes = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                let action = match delta.status() {
                    Delta::Added => FileAction::Add,
                    Delta::Deleted => FileAction::Delete,
                    Delta::Modified | Delta::Renamed | Delta::Copied | Delta::Typechange => FileAction::Edit,
                    _ => return true,
                };
                let path = delta.new_file().path()
                    .or_else(|| delta.old_file().path())
                    .map(|p| p.to_string_lossy().into_owned());
                if let Some(path) = path {
                    changes.push(ChangedFile { path, action });
                }
                true
            },
            None,
            None,
            None,
        )?;

        Ok(changes)
    }

    fn execute_shelve(
        &self,
        lease: &ClientLease,
        base_change: usize,
        target: &git2::Commit,
        changes: &[ChangedFile],
        description: &str,
        existing_shelve: Option<usize>,
    ) -> Result<usize, ShelverError> {
        let file_changes: Vec<FileChange> = changes
            .iter()
            .map(|c| FileChange {
                path: &c.path,
                action: match c.action {
                    FileAction::Add => FileAction::Add,
                    FileAction::Edit => FileAction::Edit,
                    FileAction::Delete => FileAction::Delete,
                },
            })
            .collect();

        let work_dir = self.extract_files_to_temp(target, changes)?;

        let shelve_client = ShelveClient::new(
            lease.p4().clone(),
            &lease.client_name,
            lease.client_root().clone(),
        )?;

        let cl = shelve_client.run(
            base_change,
            work_dir.path(),
            &file_changes,
            description,
            existing_shelve,
        )?;

        Ok(cl)
    }

    fn extract_files_to_temp(&self, target: &git2::Commit, changes: &[ChangedFile]) -> Result<tempfile::TempDir, ShelverError> {
        let temp_dir = tempfile::TempDir::new()?;
        let tree = target.tree()?;

        for change in changes {
            if change.action == FileAction::Delete {
                continue;
            }
            let entry = tree.get_path(Path::new(&change.path))?;
            let blob = self.repo.find_blob(entry.id())?;
            let dest_path = temp_dir.path().join(&change.path);

            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest_path, blob.content())?;
        }

        Ok(temp_dir)
    }
}

struct ChangedFile {
    path: String,
    action: FileAction,
}

#[derive(Error, Debug)]
pub enum ShelverError {
    #[error("No shelve config found")]
    NoShelveConfig,
    #[error("No base commit found that maps to a P4 change")]
    NoBaseCommit,
    #[error("No changes to shelve")]
    NoChanges,
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("P4 error: {0}")]
    P4(#[from] p4rs::P4Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Client pool error: {0}")]
    Pool(#[from] ClientPoolError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cabinet::Database;
    use crate::mirror::IntegrateStrategy;
    use git2::{Repository, Signature};
    use p4rs::testkit::{TestClient, SERVER};
    use p4rs::P4Command;
    use std::time::Duration;

    struct TestEnv {
        p4_client: TestClient,
        db: Database,
        git_repo: Repository,
        _git_dir: tempfile::TempDir,
        clients_root: std::path::PathBuf,
    }

    fn setup_test_env() -> TestEnv {
        let p4_client = SERVER.test_client();
        let db = Database::open(":memory:").unwrap();
        let git_dir = tempfile::TempDir::new().unwrap();
        let git_repo = Repository::init(git_dir.path()).unwrap();
        let clients_root = p4_client.client_root().join("shelve_clients");
        std::fs::create_dir_all(&clients_root).unwrap();

        TestEnv {
            p4_client,
            db,
            git_repo,
            _git_dir: git_dir,
            clients_root,
        }
    }

    fn create_git_commit(repo: &Repository, files: &[(&str, &[u8])], message: &str) -> git2::Oid {
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();

        for (path, content) in files {
            let full_path = repo.workdir().unwrap().join(path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full_path, content).unwrap();
            index.add_path(Path::new(path)).unwrap();
        }

        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
    }

    fn setup_prgit_client<'a>(env: &'a TestEnv) -> PrgitClient<'a> {
        let client_id = env.db
            .create_prgit_client(
                &env.p4_client.client_name,
                "p4",
                &format!("localhost:{}", SERVER.port),
                "",
            )
            .unwrap();
        env.db.create_prgit_repo(
            client_id,
            env.git_repo.workdir().unwrap().to_str().unwrap(),
            "master",
            IntegrateStrategy::MergeOurs,
            None,
        ).unwrap();
        env.db.create_shelve_config(
            client_id,
            3,
            Duration::from_secs(300),
            env.clients_root.to_str().unwrap(),
        ).unwrap();
        env.db.client(client_id).unwrap().unwrap()
    }

    fn create_feature_commit(repo: &Repository, base_oid: git2::Oid, files: &[(&str, &[u8])], message: &str) -> git2::Oid {
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();

        for (path, content) in files {
            let full_path = repo.workdir().unwrap().join(path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full_path, content).unwrap();
            index.add_path(Path::new(path)).unwrap();
        }

        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let base_commit = repo.find_commit(base_oid).unwrap();
        repo.commit(None, &sig, &sig, message, &tree, &[&base_commit]).unwrap()
    }

    #[test]
    fn test_shelve_added_file() {
        let env = setup_test_env();

        let base_change = env.p4_client.changelist("Initial")
            .add_file("existing.txt", b"existing content")
            .submit().submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("existing.txt", b"existing content")],
            "Initial commit",
        );

        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("new_file.txt", b"new content")],
            "Add new file",
        );
        env.git_repo.branch("feature", &env.git_repo.find_commit(feature_oid).unwrap(), false).unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let shelve_cl = shelver.shelve("feature").unwrap();

        let described = env.p4_client.p4.describe(&[shelve_cl]).run().unwrap().single().unwrap();
        assert_eq!(described.description.trim(), "Add new file");
    }

    #[test]
    fn test_shelve_edited_file() {
        let env = setup_test_env();

        let base_change = env.p4_client.changelist("Initial")
            .add_file("file.txt", b"original content")
            .submit().submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("file.txt", b"original content")],
            "Initial commit",
        );

        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("file.txt", b"modified content")],
            "Edit file",
        );
        env.git_repo.branch("feature", &env.git_repo.find_commit(feature_oid).unwrap(), false).unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let shelve_cl = shelver.shelve("feature").unwrap();

        let described = env.p4_client.p4.describe(&[shelve_cl]).run().unwrap().single().unwrap();
        assert_eq!(described.description.trim(), "Edit file");
    }

    #[test]
    fn test_shelve_stores_mapping() {
        let env = setup_test_env();

        let base_change = env.p4_client.changelist("Initial")
            .add_file("file.txt", b"content")
            .submit().submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("file.txt", b"content")],
            "Initial",
        );

        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("new.txt", b"new")],
            "Add file",
        );
        env.git_repo.branch("feature", &env.git_repo.find_commit(feature_oid).unwrap(), false).unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let shelve_cl = shelver.shelve("feature").unwrap();

        assert_eq!(prgit_client.get_shelved_change_for_branch("feature"), Some(shelve_cl));
    }

    #[test]
    fn test_shelve_no_changes_error() {
        let env = setup_test_env();

        let base_change = env.p4_client.changelist("Initial")
            .add_file("file.txt", b"content")
            .submit().submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("file.txt", b"content")],
            "Initial",
        );
        env.git_repo.branch("feature", &env.git_repo.find_commit(base_oid).unwrap(), false).unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let result = shelver.shelve("feature");

        assert!(matches!(result, Err(ShelverError::NoChanges)));
    }

    #[test]
    fn test_shelve_no_base_commit_error() {
        let env = setup_test_env();

        env.p4_client.changelist("Initial")
            .add_file("file.txt", b"content")
            .submit();

        create_git_commit(
            &env.git_repo,
            &[("file.txt", b"content")],
            "Initial",
        );

        create_git_commit(
            &env.git_repo,
            &[("new.txt", b"new")],
            "Add file",
        );
        env.git_repo.branch("feature", &env.git_repo.head().unwrap().peel_to_commit().unwrap(), false).unwrap();

        let prgit_client = setup_prgit_client(&env);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let result = shelver.shelve("feature");

        assert!(matches!(result, Err(ShelverError::NoBaseCommit)));
    }

    #[test]
    fn test_reshelve_uses_existing_changelist() {
        let env = setup_test_env();

        let base_change = env.p4_client.changelist("Initial")
            .add_file("file.txt", b"content")
            .submit().submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("file.txt", b"content")],
            "Initial",
        );

        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("new.txt", b"first version")],
            "First shelve",
        );
        env.git_repo.branch("feature", &env.git_repo.find_commit(feature_oid).unwrap(), false).unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let first_cl = shelver.shelve("feature").unwrap();

        let feature_oid2 = create_feature_commit(
            &env.git_repo,
            feature_oid,
            &[("new.txt", b"second version")],
            "Second shelve",
        );
        env.git_repo.branch("feature", &env.git_repo.find_commit(feature_oid2).unwrap(), true).unwrap();

        let second_cl = shelver.shelve("feature").unwrap();

        assert_eq!(first_cl, second_cl);
    }

    #[test]
    fn test_shelve_moved_file() {
        let env = setup_test_env();

        let base_change = env.p4_client.changelist("Initial")
            .add_file("old_name.txt", b"file content")
            .submit().submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("old_name.txt", b"file content")],
            "Initial commit",
        );

        let sig = Signature::now("Test", "test@test.com").unwrap();
        let mut index = env.git_repo.index().unwrap();
        
        let old_path = env.git_repo.workdir().unwrap().join("old_name.txt");
        let new_path = env.git_repo.workdir().unwrap().join("new_name.txt");
        std::fs::rename(&old_path, &new_path).unwrap();
        
        index.remove_path(Path::new("old_name.txt")).unwrap();
        index.add_path(Path::new("new_name.txt")).unwrap();
        index.write().unwrap();
        
        let tree_id = index.write_tree().unwrap();
        let tree = env.git_repo.find_tree(tree_id).unwrap();
        let base_commit = env.git_repo.find_commit(base_oid).unwrap();
        let feature_oid = env.git_repo.commit(None, &sig, &sig, "Move file", &tree, &[&base_commit]).unwrap();
        
        env.git_repo.branch("feature", &env.git_repo.find_commit(feature_oid).unwrap(), false).unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let shelve_cl = shelver.shelve("feature").unwrap();

        let described = env.p4_client.p4.describe(&[shelve_cl]).shelved().run().unwrap().single().unwrap();
        let files: Vec<&str> = described.files.iter()
            .map(|f| f.depot_file.split('/').last().unwrap())
            .collect();
        
        assert!(files.contains(&"old_name.txt"), "Should delete old file");
        assert!(files.contains(&"new_name.txt"), "Should add new file");
    }
}
