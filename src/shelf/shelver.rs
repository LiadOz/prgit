use std::path::Path;

use git2::{Delta, DiffOptions, Repository};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use p4rs::P4;
use thiserror::Error;

use crate::cabinet::PrgitClient;

use super::client_pool::{get_shelve_client, ShelveClientError};
use super::shelve_client::{FileAction, FileChange, ShelveClient, ShelveDescriptionMode};

const PRGITIGNORE_PATH: &str = ".prgitignore";

pub struct ShelveResult {
    pub changelist: usize,
    pub client_name: String,
    pub is_reshelve: bool,
    pub file_count: usize,
    pub commits_in_branch: usize,
}

pub struct Shelver<'a> {
    prgit_client: &'a PrgitClient<'a>,
    repo: Repository,
}

impl<'a> Shelver<'a> {
    pub fn new(prgit_client: &'a PrgitClient<'a>) -> Result<Self, ShelverError> {
        let repo = Repository::open(&prgit_client.git_config.repo_path)?;
        Ok(Self { prgit_client, repo })
    }

    pub fn shelve(
        &self,
        branch: &str,
        user_p4: &P4,
        shelver_user: &str,
        description_mode: ShelveDescriptionMode,
    ) -> Result<ShelveResult, ShelverError> {
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

        let file_count = changes.len();
        let commits_in_branch = self.count_commits(base_oid, target_oid);

        let description = target_commit
            .message()
            .unwrap_or("Shelved from git")
            .to_string();
        let existing_shelve = self.prgit_client.get_shelved_change_for_branch(branch);
        let is_reshelve = existing_shelve.is_some();

        let handle = get_shelve_client(self.prgit_client, user_p4)?;
        let client_name = handle.shelve_client.client_name().to_string();
        let shelve_cl = self.execute_shelve(
            &handle.shelve_client,
            base_change,
            &target_commit,
            &changes,
            &description,
            existing_shelve,
            description_mode,
        )?;

        self.prgit_client
            .set_shelved_change_for_branch(branch, shelve_cl, shelver_user);

        Ok(ShelveResult {
            changelist: shelve_cl,
            client_name,
            is_reshelve,
            file_count,
            commits_in_branch,
        })
    }

    fn count_commits(&self, base_oid: git2::Oid, target_oid: git2::Oid) -> usize {
        let mut revwalk = match self.repo.revwalk() {
            Ok(rw) => rw,
            Err(_) => return 0,
        };
        if revwalk.push(target_oid).is_err() || revwalk.hide(base_oid).is_err() {
            return 0;
        }
        revwalk.count()
    }

    fn find_merge_base_and_change(
        &self,
        target_oid: git2::Oid,
    ) -> Result<(git2::Oid, usize), ShelverError> {
        let synced_branch = &self.prgit_client.git_config.synced_branch;
        let synced_ref = self
            .repo
            .find_branch(synced_branch, git2::BranchType::Local)?;
        let synced_oid = synced_ref.get().peel_to_commit()?.id();

        let base_oid = self.repo.merge_base(synced_oid, target_oid)?;
        let commit_hash = base_oid.to_string();

        let base_change = self
            .prgit_client
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
        let diff =
            self.repo
                .diff_tree_to_tree(Some(&base_tree), Some(&target_tree), Some(&mut opts))?;

        let mut changes = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                let action = match delta.status() {
                    Delta::Added => FileAction::Add,
                    Delta::Deleted => FileAction::Delete,
                    Delta::Modified | Delta::Renamed | Delta::Copied | Delta::Typechange => {
                        FileAction::Edit
                    }
                    _ => return true,
                };
                let path = delta
                    .new_file()
                    .path()
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

        let ignore = load_prgitignore(&self.repo, target)?;
        if let Some(gi) = ignore.as_ref() {
            changes.retain(|c| !is_ignored_add(gi, c));
        }

        Ok(changes)
    }

    fn execute_shelve(
        &self,
        shelve_client: &ShelveClient,
        base_change: usize,
        target: &git2::Commit,
        changes: &[ChangedFile],
        description: &str,
        existing_shelve: Option<usize>,
        description_mode: ShelveDescriptionMode,
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

        let cl = shelve_client.run(
            base_change,
            work_dir.path(),
            &file_changes,
            description,
            existing_shelve,
            description_mode,
        )?;

        Ok(cl)
    }

    fn extract_files_to_temp(
        &self,
        target: &git2::Commit,
        changes: &[ChangedFile],
    ) -> Result<tempfile::TempDir, ShelverError> {
        use std::os::unix::fs::PermissionsExt;
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
            // Preserve executable bit from git tree entry
            if entry.filemode() == i32::from(git2::FileMode::BlobExecutable) {
                std::fs::set_permissions(&dest_path, std::fs::Permissions::from_mode(0o755))?;
            }
        }

        Ok(temp_dir)
    }
}

struct ChangedFile {
    path: String,
    action: FileAction,
}

fn load_prgitignore(
    repo: &Repository,
    target: &git2::Commit,
) -> Result<Option<Gitignore>, ShelverError> {
    let tree = target.tree()?;
    let entry = match tree.get_path(Path::new(PRGITIGNORE_PATH)) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    let blob = repo.find_blob(entry.id())?;
    let content = std::str::from_utf8(blob.content())
        .map_err(|e| ShelverError::InvalidPrgitignore(e.to_string()))?;
    let mut builder = GitignoreBuilder::new("");
    for (lineno, line) in content.lines().enumerate() {
        builder
            .add_line(None, line)
            .map_err(|e| ShelverError::InvalidPrgitignore(format!("line {}: {}", lineno + 1, e)))?;
    }
    let gi = builder
        .build()
        .map_err(|e| ShelverError::InvalidPrgitignore(e.to_string()))?;
    Ok(Some(gi))
}

fn is_ignored_add(gi: &Gitignore, change: &ChangedFile) -> bool {
    if change.action != FileAction::Add {
        return false;
    }
    if change.path == PRGITIGNORE_PATH {
        return false;
    }
    let matched = gi.matched_path_or_any_parents(Path::new(&change.path), false);
    if matched.is_ignore() {
        log::info!("prgitignore: skipping new file {}", change.path);
        true
    } else {
        false
    }
}

#[derive(Error, Debug)]
pub enum ShelverError {
    #[error("No shelve config found")]
    NoShelveConfig,
    #[error("No base commit found that maps to a P4 change")]
    NoBaseCommit,
    #[error("No changes to shelve")]
    NoChanges,
    #[error("Invalid .prgitignore: {0}")]
    InvalidPrgitignore(String),
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("P4 error: {0}")]
    P4(#[from] p4rs::P4Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Shelve client error: {0}")]
    ShelveClient(#[from] ShelveClientError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cabinet::Database;
    use crate::mirror::IntegrateStrategy;
    use git2::{Repository, Signature};
    use p4rs::testkit::{TestClient, SERVER};
    use p4rs::P4Command;

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

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    fn setup_prgit_client<'a>(env: &'a TestEnv) -> PrgitClient<'a> {
        let client_id = env
            .db
            .create_prgit_client(
                &env.p4_client.client_name,
                "p4",
                &format!("localhost:{}", SERVER.port),
                "",
            )
            .unwrap();
        env.db
            .create_prgit_repo(
                client_id,
                env.git_repo.workdir().unwrap().to_str().unwrap(),
                "master",
                IntegrateStrategy::MergeOurs,
                None,
            )
            .unwrap();
        env.db
            .create_shelve_config(client_id, env.clients_root.to_str().unwrap())
            .unwrap();
        env.db.client(client_id).unwrap().unwrap()
    }

    struct FileSpec<'a> {
        path: &'a str,
        content: &'a [u8],
        executable: bool,
    }

    fn create_feature_commit(
        repo: &Repository,
        base_oid: git2::Oid,
        files: &[(&str, &[u8])],
        message: &str,
    ) -> git2::Oid {
        let specs: Vec<FileSpec> = files
            .iter()
            .map(|(p, c)| FileSpec {
                path: p,
                content: c,
                executable: false,
            })
            .collect();
        create_feature_commit_with_modes(repo, base_oid, &specs, message)
    }

    fn create_feature_commit_with_modes(
        repo: &Repository,
        base_oid: git2::Oid,
        files: &[FileSpec],
        message: &str,
    ) -> git2::Oid {
        use std::os::unix::fs::PermissionsExt;
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();

        for spec in files {
            let full_path = repo.workdir().unwrap().join(spec.path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full_path, spec.content).unwrap();
            if spec.executable {
                std::fs::set_permissions(&full_path, std::fs::Permissions::from_mode(0o755))
                    .unwrap();
            }
            index.add_path(Path::new(spec.path)).unwrap();
        }

        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let base_commit = repo.find_commit(base_oid).unwrap();
        repo.commit(None, &sig, &sig, message, &tree, &[&base_commit])
            .unwrap()
    }

    #[test]
    fn test_shelve_added_file() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("existing.txt", b"existing content")
            .submit()
            .unwrap()
            .submitted_change;

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
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let shelve_cl = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap()
            .changelist;

        let described = env
            .p4_client
            .p4
            .describe(&[shelve_cl])
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(described.description.trim(), "Add new file");
    }

    #[test]
    fn test_shelve_edited_file() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("file.txt", b"original content")
            .submit()
            .unwrap()
            .submitted_change;

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
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let shelve_cl = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap()
            .changelist;

        let described = env
            .p4_client
            .p4
            .describe(&[shelve_cl])
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(described.description.trim(), "Edit file");
    }

    #[test]
    fn test_shelve_stores_mapping() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("file.txt", b"content")
            .submit()
            .unwrap()
            .submitted_change;

        let base_oid = create_git_commit(&env.git_repo, &[("file.txt", b"content")], "Initial");

        let feature_oid =
            create_feature_commit(&env.git_repo, base_oid, &[("new.txt", b"new")], "Add file");
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let shelve_cl = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap()
            .changelist;

        assert_eq!(
            prgit_client.get_shelved_change_for_branch("feature"),
            Some(shelve_cl)
        );
    }

    #[test]
    fn test_shelve_no_changes_error() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("file.txt", b"content")
            .submit()
            .unwrap()
            .submitted_change;

        let base_oid = create_git_commit(&env.git_repo, &[("file.txt", b"content")], "Initial");
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(base_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let result = shelver.shelve("feature", &env.p4_client.p4, "testuser", Default::default());

        assert!(matches!(result, Err(ShelverError::NoChanges)));
    }

    #[test]
    fn test_shelve_no_base_commit_error() {
        let env = setup_test_env();

        env.p4_client
            .changelist("Initial")
            .add_file("file.txt", b"content")
            .submit()
            .expect("submit initial");

        create_git_commit(&env.git_repo, &[("file.txt", b"content")], "Initial");

        create_git_commit(&env.git_repo, &[("new.txt", b"new")], "Add file");
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.head().unwrap().peel_to_commit().unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let result = shelver.shelve("feature", &env.p4_client.p4, "testuser", Default::default());

        assert!(matches!(result, Err(ShelverError::NoBaseCommit)));
    }

    #[test]
    fn test_reshelve_uses_existing_changelist() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("file.txt", b"content")
            .submit()
            .unwrap()
            .submitted_change;

        let base_oid = create_git_commit(&env.git_repo, &[("file.txt", b"content")], "Initial");

        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("new.txt", b"first version")],
            "First shelve",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let first_cl = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap()
            .changelist;

        let feature_oid2 = create_feature_commit(
            &env.git_repo,
            feature_oid,
            &[("new.txt", b"second version")],
            "Second shelve",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid2).unwrap(),
                true,
            )
            .unwrap();

        let second_cl = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap()
            .changelist;

        assert_eq!(first_cl, second_cl);
    }

    #[test]
    fn test_reshelve_removes_stale_files() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("existing.txt", b"content")
            .submit()
            .unwrap()
            .submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("existing.txt", b"content")],
            "Initial commit",
        );

        // First shelve: add both file_a.txt and file_b.txt
        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("file_a.txt", b"aaa"), ("file_b.txt", b"bbb")],
            "Add two files",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let first_result = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap();

        let described = env
            .p4_client
            .p4
            .describe(&[first_result.changelist])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(described.files.len(), 2, "First shelve should have 2 files");

        // Reshelve: branch now only has file_b.txt (file_a.txt removed)
        // Reset index to base tree so file_a.txt is not carried over
        {
            let base_tree = env.git_repo.find_commit(base_oid).unwrap().tree().unwrap();
            let mut index = env.git_repo.index().unwrap();
            index.read_tree(&base_tree).unwrap();
            index.write().unwrap();
        }
        let feature_oid2 = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("file_b.txt", b"bbb updated")],
            "Only file_b",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid2).unwrap(),
                true,
            )
            .unwrap();

        let second_result = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap();

        assert_eq!(first_result.changelist, second_result.changelist);

        let described = env
            .p4_client
            .p4
            .describe(&[second_result.changelist])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        let file_names: Vec<&str> = described
            .files
            .iter()
            .map(|f| f.depot_file.split('/').next_back().unwrap())
            .collect();
        assert_eq!(
            file_names.len(),
            1,
            "Reshelve should have only 1 file, got: {file_names:?}"
        );
        assert!(
            file_names.contains(&"file_b.txt"),
            "Should contain file_b.txt, got: {file_names:?}"
        );
        assert!(
            !file_names.contains(&"file_a.txt"),
            "Should NOT contain stale file_a.txt, got: {file_names:?}"
        );
    }

    #[test]
    fn test_reshelve_replaces_all_files() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("existing.txt", b"content")
            .submit()
            .unwrap()
            .submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("existing.txt", b"content")],
            "Initial commit",
        );

        // First shelve: add file_a.txt
        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("file_a.txt", b"aaa")],
            "Add file_a",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let first_result = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap();

        // Reshelve: completely different file set (file_a removed, file_b added)
        // Reset index to base tree so file_a.txt is not carried over
        {
            let base_tree = env.git_repo.find_commit(base_oid).unwrap().tree().unwrap();
            let mut index = env.git_repo.index().unwrap();
            index.read_tree(&base_tree).unwrap();
            index.write().unwrap();
        }
        let feature_oid2 = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("file_b.txt", b"bbb")],
            "Replace with file_b",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid2).unwrap(),
                true,
            )
            .unwrap();

        let second_result = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap();

        assert_eq!(first_result.changelist, second_result.changelist);

        let described = env
            .p4_client
            .p4
            .describe(&[second_result.changelist])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        let file_names: Vec<&str> = described
            .files
            .iter()
            .map(|f| f.depot_file.split('/').next_back().unwrap())
            .collect();
        assert_eq!(
            file_names.len(),
            1,
            "Should have exactly 1 file after replacing all, got: {file_names:?}"
        );
        assert!(
            file_names.contains(&"file_b.txt"),
            "Should contain file_b.txt, got: {file_names:?}"
        );
        assert!(
            !file_names.contains(&"file_a.txt"),
            "Should NOT contain old file_a.txt, got: {file_names:?}"
        );
    }

    #[test]
    fn test_shelve_moved_file() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("old_name.txt", b"file content")
            .submit()
            .unwrap()
            .submitted_change;

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
        let feature_oid = env
            .git_repo
            .commit(None, &sig, &sig, "Move file", &tree, &[&base_commit])
            .unwrap();

        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let shelve_cl = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap()
            .changelist;

        let described = env
            .p4_client
            .p4
            .describe(&[shelve_cl])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        let files: Vec<&str> = described
            .files
            .iter()
            .map(|f| {
                f.depot_file
                    .split('/')
                    .next_back()
                    .expect("split always has at least one element")
            })
            .collect();

        assert!(files.contains(&"old_name.txt"), "Should delete old file");
        assert!(files.contains(&"new_name.txt"), "Should add new file");
    }

    /// End-to-end: executable file (text+x in P4, 100755 in git) edited in git.
    /// Without the extract_files_to_temp fix, the executable bit is lost during
    /// extraction, causing determine_file_type to return 'text' and reopen to
    /// strip the +x modifier.
    #[test]
    fn test_shelve_preserves_executable_through_full_path() {
        use std::os::unix::fs::PermissionsExt;
        let env = setup_test_env();

        // Create executable file in P4
        let base_change = env
            .p4_client
            .changelist("Add executable script")
            .add_file_with_opts(
                "run.py",
                b"#!/usr/bin/env python\nprint('v1')",
                Some(p4rs::FileType::text().executable()),
            )
            .submit()
            .unwrap()
            .submitted_change;

        // Mirror to git with executable bit (100755)
        let workdir = env.git_repo.workdir().unwrap();
        let script_path = workdir.join("run.py");
        std::fs::write(&script_path, b"#!/usr/bin/env python\nprint('v1')").unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let base_oid = {
            let sig = Signature::now("Test", "test@test.com").unwrap();
            let mut index = env.git_repo.index().unwrap();
            index.add_path(Path::new("run.py")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = env.git_repo.find_tree(tree_id).unwrap();
            env.git_repo
                .commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
                .unwrap()
        };

        // Create feature branch: edit content, keep executable
        let feature_oid = create_feature_commit_with_modes(
            &env.git_repo,
            base_oid,
            &[FileSpec {
                path: "run.py",
                content: b"#!/usr/bin/env python\nprint('v2')",
                executable: true,
            }],
            "Edit script",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let result = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap();

        let described = env
            .p4_client
            .p4
            .describe(&[result.changelist])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(described.files.len(), 1);
        let shelved_type = &described.files[0].file_type;
        assert!(
            shelved_type.executable,
            "Should preserve executable through full shelve path, got: {shelved_type:?}"
        );
    }

    /// End-to-end test: P4 file with text+k, mirrored to git, edited in git,
    /// pushed back through Shelver. The shelved CL should preserve text+k.
    #[test]
    fn test_shelve_preserves_keyword_modifier_e2e() {
        let env = setup_test_env();

        // Create a file with text+k in P4
        let base_change = env
            .p4_client
            .changelist("Add keyword file")
            .add_file_with_opts(
                "version.h",
                b"// $Id$\nversion 1",
                Some(p4rs::FileType::text().keyword_expansion()),
            )
            .submit()
            .unwrap()
            .submitted_change;

        // Mirror to git (simulate: create same content in git repo on master)
        let base_oid = create_git_commit(
            &env.git_repo,
            &[("version.h", b"// $Id$\nversion 1")],
            "Initial commit",
        );

        // Create feature branch with edited content
        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[("version.h", b"// $Id$\nversion 2")],
            "Update version",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let result = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap();

        // Verify the shelved file preserves text+k
        let described = env
            .p4_client
            .p4
            .describe(&[result.changelist])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(described.files.len(), 1);
        let shelved_type = &described.files[0].file_type;
        assert!(
            shelved_type.keyword_expansion,
            "Should preserve keyword expansion (+k), got: {shelved_type:?}"
        );
        assert!(
            !shelved_type.compressed,
            "Should NOT gain compressed (+C), got: {shelved_type:?}"
        );
    }

    #[test]
    fn test_prgitignore_skips_matching_adds() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("existing.txt", b"existing content")
            .submit()
            .unwrap()
            .submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("existing.txt", b"existing content")],
            "Initial commit",
        );

        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[
                (".prgitignore", b"*.log\nlocal/\n"),
                ("keep.txt", b"keep me"),
                ("debug.log", b"ignore me"),
                ("local/notes.md", b"also ignored"),
            ],
            "Add files with .prgitignore",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let result = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap();

        let described = env
            .p4_client
            .p4
            .describe(&[result.changelist])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        let mut paths: Vec<String> = described
            .files
            .iter()
            .map(|f| f.depot_file.clone())
            .collect();
        paths.sort();
        let names: Vec<&str> = paths
            .iter()
            .map(|p| p.rsplit('/').next().unwrap())
            .collect();
        assert_eq!(names, vec![".prgitignore", "keep.txt"]);
    }

    #[test]
    fn test_prgitignore_does_not_filter_edits() {
        let env = setup_test_env();

        let base_change = env
            .p4_client
            .changelist("Initial")
            .add_file("tracked.log", b"original")
            .submit()
            .unwrap()
            .submitted_change;

        let base_oid = create_git_commit(
            &env.git_repo,
            &[("tracked.log", b"original")],
            "Initial commit",
        );

        let feature_oid = create_feature_commit(
            &env.git_repo,
            base_oid,
            &[(".prgitignore", b"*.log\n"), ("tracked.log", b"modified")],
            "Edit tracked .log file",
        );
        env.git_repo
            .branch(
                "feature",
                &env.git_repo.find_commit(feature_oid).unwrap(),
                false,
            )
            .unwrap();

        let prgit_client = setup_prgit_client(&env);
        prgit_client.map_commit_to_change(&base_oid.to_string(), base_change);

        let shelver = Shelver::new(&prgit_client).unwrap();
        let result = shelver
            .shelve("feature", &env.p4_client.p4, "testuser", Default::default())
            .unwrap();

        let described = env
            .p4_client
            .p4
            .describe(&[result.changelist])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        let mut names: Vec<&str> = described
            .files
            .iter()
            .map(|f| f.depot_file.rsplit('/').next().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, vec![".prgitignore", "tracked.log"]);
    }
}
