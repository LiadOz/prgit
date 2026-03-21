use git2::{FileMode, Repository};
use p4rs::testkit::{TestClient, SERVER};
use p4rs::{ChangeSpec, ChangeType, FileAction, FileType, P4Command, SubmitResult};
use prgit::mirror::{HashMapMirrorData, IntegrateStrategy, Mirror, MirrorData};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;
use test_log::test;

#[test]
fn test_single_file_add() {
    let env = MirrorTestEnv::new();
    env.commit_file("file.txt", "content", "Add file");
    env.mirror().run().expect("Mirror failed");
    assert_eq!(env.git_commit_count(), 1, "Expected 1 git commit");
    env.assert_p4_matches_git();
}

#[test]
fn test_single_file_edit() {
    let env = MirrorTestEnv::new();
    env.commit_file("file.txt", "initial", "Add file");
    env.edit_file("file.txt", "updated", "Edit file");
    env.mirror().run().expect("Mirror failed");
    env.assert_p4_matches_git();
    assert_eq!(env.git_commit_count(), 2);
}

#[test]
fn test_single_file_delete() {
    let env = MirrorTestEnv::new();
    env.commit_file("file.txt", "content", "Add file");
    env.delete_file("file.txt", "Delete file");
    env.mirror().run().expect("Mirror failed");
    env.assert_p4_matches_git();
    assert_eq!(env.git_commit_count(), 2);
}

#[test]
fn test_multiple_files_one_change() {
    let env = MirrorTestEnv::new();
    env.p4_client
        .changelist("Add files")
        .add_file("a.txt", b"a")
        .add_file("b.txt", b"b")
        .submit()
        .expect("submit failed");

    env.mirror().run().expect("Mirror failed");
    env.assert_p4_matches_git();
    assert_eq!(env.git_commit_count(), 1);
}

#[test]
fn test_sequential_changes() {
    let env = MirrorTestEnv::new();
    env.commit_file("a.txt", "a", "First");
    env.commit_file("b.txt", "b", "Second");
    env.commit_file("c.txt", "c", "Third");
    env.mirror().run().expect("Mirror failed");
    env.assert_p4_matches_git();
    assert_eq!(env.git_commit_count(), 3);
}

#[test]
fn test_empty_initial_repo() {
    let env = MirrorTestEnv::new();
    env.mirror().run().expect("Mirror failed");
    assert_eq!(env.git_commit_count(), 0);
}

#[test]
fn test_subdirectories() {
    let env = MirrorTestEnv::new();
    env.commit_file("dir/subdir/file.txt", "nested", "Add nested file");
    env.mirror().run().expect("Mirror failed");
    env.assert_p4_matches_git();
    assert_eq!(env.git_commit_count(), 1);
}

#[test]
fn test_merge_with_branch_parent() {
    let env = MirrorTestEnv::new();
    let base = env.commit_file("base.txt", "base", "Base commit");
    env.mirror().run().expect("Mirror failed");

    env.create_git_branch("feature");

    let original_cl = env
        .p4_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Merge commit"))
        .run()
        .expect("Failed to create change")
        .single()
        .unwrap();
    let file_path = env.p4_client.client_root().join("merged.txt");
    fs::write(&file_path, "merged").expect("Failed to write");
    env.p4_client
        .p4
        .add(&[file_path.to_str().unwrap()])
        .changelist(original_cl)
        .run()
        .expect("Failed to add");
    env.p4_client
        .p4
        .submit(original_cl)
        .run()
        .expect("Failed to submit");

    let mut data = env.default_data();
    data.set_last_sync_change(base.submitted_change);
    data.set_branch_mapping(original_cl, "refs/heads/feature".to_string());

    env.mirror_with_data(data).run().expect("Mirror failed");
    assert_eq!(env.git_head_parent_count(), 2);
}

#[test]
fn test_merge_missing_branch_skipped() {
    let env = MirrorTestEnv::new();
    let base = env.commit_file("base.txt", "base", "Base commit");
    env.mirror().run().expect("Mirror failed");

    let original_cl = env
        .p4_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Merge commit"))
        .run()
        .expect("Failed to create change")
        .single()
        .unwrap();
    let file_path = env.p4_client.client_root().join("merged.txt");
    fs::write(&file_path, "merged").expect("Failed to write");
    env.p4_client
        .p4
        .add(&[file_path.to_str().unwrap()])
        .changelist(original_cl)
        .run()
        .expect("Failed to add");
    env.p4_client
        .p4
        .submit(original_cl)
        .run()
        .expect("Failed to submit");

    let mut data = env.default_data();
    data.set_last_sync_change(base.submitted_change);
    data.set_branch_mapping(original_cl, "refs/heads/nonexistent".to_string());

    env.mirror_with_data(data).run().expect("Mirror failed");
    assert_eq!(env.git_head_parent_count(), 1);
    env.assert_p4_matches_git();
}

#[test]
fn test_executable_file() {
    let env = MirrorTestEnv::new();
    env.p4_client
        .changelist("Add executable")
        .add_file_with_opts(
            "script.sh",
            b"#!/bin/bash\necho hello",
            Some(FileType::text().executable()),
        )
        .submit()
        .expect("submit failed");

    env.mirror().run().expect("Mirror failed");
    let git_files = env.get_git_files();
    assert!(git_files.contains_key("script.sh"));
}

#[test]
fn test_binary_file() {
    let env = MirrorTestEnv::new();
    let binary_content: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
    env.p4_client
        .changelist("Add binary")
        .add_file_with_opts("data.bin", &binary_content, Some(FileType::binary()))
        .submit()
        .expect("submit failed");

    env.mirror().run().expect("Mirror failed");
    let git_files = env.get_git_files();
    assert_eq!(git_files.get("data.bin"), Some(&binary_content));
}

#[test]
fn test_special_characters_in_path() {
    let env = MirrorTestEnv::new();
    env.commit_file("file with spaces.txt", "content", "Spaces in name");
    env.mirror().run().expect("Mirror failed");
    env.assert_p4_matches_git();
}

#[test]
fn test_many_sequential_changes() {
    let env = MirrorTestEnv::new();
    for i in 0..50 {
        env.commit_file(
            &format!("file_{}.txt", i),
            &format!("content {}", i),
            &format!("Add file {}", i),
        );
    }
    env.mirror().run().expect("Mirror failed");
    env.assert_p4_matches_git();
    assert_eq!(env.git_commit_count(), 50);
}

#[test]
fn test_resume_partial_sync() {
    let env = MirrorTestEnv::new();
    let cl1 = env.commit_file("a.txt", "a", "First");
    let _cl2 = env.commit_file("b.txt", "b", "Second");
    let _cl3 = env.commit_file("c.txt", "c", "Third");

    let mut data = env.default_data();
    data.set_last_sync_change(cl1.submitted_change);
    env.mirror_with_data(data).run().expect("Mirror failed");

    assert_eq!(env.git_commit_count(), 2);
}

#[test]
fn test_max_changes_batching() {
    let env = MirrorTestEnv::new();
    for i in 0..10 {
        env.commit_file(
            &format!("file_{}.txt", i),
            &format!("content {}", i),
            &format!("Add {}", i),
        );
    }

    let data = HashMapMirrorData::new(
        env.p4_client.client_name.clone(),
        IntegrateStrategy::MergeOurs,
        Some(3),
    );
    env.mirror_with_data(data).run().expect("First batch");
    assert_eq!(env.git_commit_count(), 10);
    env.assert_p4_matches_git();
}

#[test]
fn test_symlink_file() {
    use std::os::unix::fs::symlink;

    let env = MirrorTestEnv::new();

    // Create a symlink whose target does NOT exist in this change.
    // This matches real-world P4 usage where symlinks point to relative paths.
    let link_path = env.p4_client.client_root().join("config.txt");
    symlink("shared_config.txt", &link_path).unwrap();

    let cl = env
        .p4_client
        .p4
        .change()
        .set(&ChangeSpec::new(ChangeType::New).description("Add symlink"))
        .run()
        .unwrap()
        .single()
        .unwrap();

    env.p4_client
        .p4
        .add(&[link_path.to_str().unwrap()])
        .changelist(cl)
        .file_type(FileType::symlink())
        .run()
        .unwrap();
    env.p4_client.p4.submit(cl).run().unwrap();

    env.mirror().run().expect("Mirror should handle symlinks");
    assert_eq!(env.git_commit_count(), 1);

    // In git, a symlink entry has Link mode and blob contains the target path
    let repo = env.git_repo_non_bare();
    let head = repo.head().unwrap();
    let commit = head.peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    let entry = tree
        .get_name("config.txt")
        .expect("config.txt should exist in tree");
    assert_eq!(entry.filemode(), i32::from(FileMode::Link));
    let blob = repo.find_blob(entry.id()).unwrap();
    assert_eq!(
        std::str::from_utf8(blob.content()).unwrap(),
        "shared_config.txt",
        "Symlink blob should contain the target path, not dereferenced content"
    );
}

struct MirrorTestEnv {
    p4_client: TestClient,
    git_dir: TempDir,
}

impl MirrorTestEnv {
    fn new() -> Self {
        let p4_client = SERVER.test_client();
        let git_dir = TempDir::new().expect("Failed to create git temp dir");
        Repository::init_bare(git_dir.path()).expect("Failed to init git repo");
        Self { p4_client, git_dir }
    }

    fn mirror(&self) -> Mirror<HashMapMirrorData> {
        let data = HashMapMirrorData::new(
            self.p4_client.client_name.clone(),
            IntegrateStrategy::MergeOurs,
            None,
        );
        Mirror::new(self.p4_client.p4.clone(), self.git_repo_non_bare(), data)
    }

    fn mirror_with_data(&self, data: HashMapMirrorData) -> Mirror<HashMapMirrorData> {
        Mirror::new(self.p4_client.p4.clone(), self.git_repo_non_bare(), data)
    }

    fn git_repo_non_bare(&self) -> Repository {
        Repository::open(self.git_dir.path()).expect("Failed to open git repo")
    }

    fn default_data(&self) -> HashMapMirrorData {
        HashMapMirrorData::new(
            self.p4_client.client_name.clone(),
            IntegrateStrategy::MergeOurs,
            None,
        )
    }

    fn commit_file(&self, path: &str, content: &str, desc: &str) -> SubmitResult {
        self.p4_client
            .changelist(desc)
            .add_file(path, content)
            .submit()
            .unwrap()
    }

    fn edit_file(&self, path: &str, content: &str, desc: &str) -> SubmitResult {
        self.p4_client
            .changelist(desc)
            .edit_file(path, content)
            .submit()
            .unwrap()
    }

    fn delete_file(&self, path: &str, desc: &str) -> SubmitResult {
        self.p4_client
            .changelist(desc)
            .delete_file(path)
            .submit()
            .unwrap()
    }

    fn assert_p4_matches_git(&self) {
        let p4_files = self.get_p4_files();
        let git_files = self.get_git_files();

        assert_eq!(
            p4_files.len(),
            git_files.len(),
            "File count mismatch: P4 has {} files, Git has {}",
            p4_files.len(),
            git_files.len()
        );

        for (path, p4_content) in &p4_files {
            let git_content = git_files
                .get(path)
                .unwrap_or_else(|| panic!("File {} exists in P4 but not in Git", path));
            assert_eq!(
                p4_content, git_content,
                "Content mismatch for file {}",
                path
            );
        }
    }

    fn get_p4_files(&self) -> HashMap<String, Vec<u8>> {
        let client_path = format!("//{}/...", self.p4_client.client_name);

        let where_result = self
            .p4_client
            .p4
            .where_cmd(&[&client_path])
            .run()
            .expect("Failed to get where");
        let depot_prefix = where_result
            .results
            .first()
            .and_then(|w| w.depot_file.strip_suffix("..."))
            .expect("Failed to get depot prefix")
            .to_string();

        let print_result = self.p4_client.p4.print().content(&[&client_path]).run();

        let mut files = HashMap::new();
        if let Ok(results) = print_result {
            for file in results.results {
                if file.info.action == FileAction::Delete {
                    continue;
                }
                if let Some(rel_path) = file.info.depot_file.strip_prefix(&depot_prefix) {
                    files.insert(rel_path.to_string(), file.data.into_bytes());
                }
            }
        }
        files
    }

    fn get_git_files(&self) -> HashMap<String, Vec<u8>> {
        let mut files = HashMap::new();
        let repo = self.git_repo_non_bare();
        if repo.head().is_err() {
            return files;
        }

        let clone_dir = TempDir::new().expect("Failed to create clone dir");
        Repository::clone(
            self.git_dir.path().to_str().expect("Invalid path"),
            clone_dir.path(),
        )
        .expect("Failed to clone bare repo");

        collect_files_recursive(clone_dir.path(), clone_dir.path(), &mut files);
        files
    }

    fn git_commit_count(&self) -> usize {
        let repo = self.git_repo_non_bare();
        let head = match repo.head() {
            Ok(h) => h,
            Err(_) => return 0,
        };
        let mut count = 0;
        let mut revwalk = repo.revwalk().expect("Failed to create revwalk");
        revwalk
            .push(head.target().expect("No head target"))
            .expect("Failed to push head");
        for _ in revwalk {
            count += 1;
        }
        count
    }

    fn git_head_parent_count(&self) -> usize {
        let repo = self.git_repo_non_bare();
        let head = match repo.head() {
            Ok(h) => h,
            Err(_) => return 0,
        };
        let commit = head.peel_to_commit().expect("Failed to peel to commit");
        commit.parent_count()
    }

    fn create_git_branch(&self, branch_name: &str) {
        let repo = self.git_repo_non_bare();
        let head = repo.head().expect("No HEAD");
        let commit = head.peel_to_commit().expect("Failed to peel to commit");
        repo.branch(branch_name, &commit, false)
            .expect("Failed to create branch");
    }
}

fn collect_files_recursive(
    base: &std::path::Path,
    dir: &std::path::Path,
    files: &mut HashMap<String, Vec<u8>>,
) {
    for entry in fs::read_dir(dir).expect("Failed to read dir") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.file_name().map(|n| n == ".git").unwrap_or(false) {
            continue;
        }
        if path.is_dir() {
            collect_files_recursive(base, &path, files);
        } else {
            let rel = path.strip_prefix(base).expect("Failed to strip prefix");
            let content = fs::read(&path).expect("Failed to read file");
            files.insert(rel.to_string_lossy().to_string(), content);
        }
    }
}
