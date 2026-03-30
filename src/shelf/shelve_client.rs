use p4rs::{ChangeSpec, ChangeType, ChangelistBuilder, P4Command, P4Error, P4};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShelveDescriptionMode {
    /// Always update the CL description from the current branch tip commit (default).
    #[default]
    Update,
    /// Keep the description from the first shelve, never update on reshelve.
    KeepOriginal,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum FileAction {
    Add,
    Edit,
    Delete,
}

pub struct FileChange<'a> {
    pub path: &'a str,
    pub action: FileAction,
}

/// Apply only the git-representable attributes (base type + executable) from `git_type`
/// onto the depot type, preserving all P4-specific modifiers.
fn apply_git_type_to_depot(depot_type: &p4rs::FileType, git_type: &p4rs::FileType) -> p4rs::FileType {
    // If the base type changed (e.g. text → symlink), use the new base type without old modifiers
    if depot_type.base != git_type.base {
        return git_type.clone();
    }
    // Same base type: preserve depot modifiers, only toggle executable
    let mut result = depot_type.clone();
    result.executable = git_type.executable;
    result
}

pub struct ShelveClient {
    p4: P4,
    client_name: String,
    client_root: PathBuf,
}

impl ShelveClient {
    pub fn client_name(&self) -> &str {
        &self.client_name
    }

    pub fn new(p4: P4, client_name: &str, client_root: PathBuf) -> Result<Self, P4Error> {
        let p4 = p4.client_name(client_name);
        Self::cleanup_workspace(&p4, &client_root)?;
        Ok(Self {
            p4,
            client_name: client_name.to_string(),
            client_root,
        })
    }

    fn cleanup_workspace(p4: &P4, client_root: &Path) -> Result<(), P4Error> {
        let _ = p4.revert(&["//..."]).run();
        let _ = p4.sync(&["//...#none"]).run();
        let _ = Self::clear_directory_contents(client_root);
        Ok(())
    }

    fn clear_directory_contents(dir: &Path) -> Result<(), P4Error> {
        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.file_name() == Some(std::ffi::OsStr::new(".prgit.lock")) {
                    continue;
                }
                if path.is_dir() {
                    std::fs::remove_dir_all(&path)?;
                } else {
                    std::fs::remove_file(&path)?;
                }
            }
        }
        Ok(())
    }

    fn sync(&self, base_change: usize, files: &[&str]) -> Result<(), P4Error> {
        for file in files {
            let depot_path = format!("//{}/{}@{base_change}", self.client_name, file);
            if self.p4.files(&[&depot_path]).run().is_err() {
                continue;
            }
            match self.p4.sync(&[&depot_path]).run() {
                Err(P4Error::Command { ref errors, .. })
                    if errors.iter().any(|e| e.data.contains("file(s) up-to-date")) => {}
                Ok(_) => {}
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

    fn apply_changes(
        &self,
        changelist: usize,
        base_dir: &Path,
        changes: &[FileChange],
    ) -> Result<(), P4Error> {
        let mut builder =
            ChangelistBuilder::with_changelist(&self.p4, self.client_root.clone(), changelist);
        for change in changes {
            let src = base_dir.join(change.path);
            let dest = self.client_root.join(change.path);
            match change.action {
                FileAction::Add => {
                    Self::copy_file(&src, &dest)?;
                    builder.add(change.path)?;
                }
                FileAction::Edit => {
                    let git_type = ChangelistBuilder::determine_file_type(&src)?;
                    let full_path = self.client_root.join(change.path);
                    let full_path_str = full_path.to_string_lossy().to_string();
                    self.p4
                        .edit(&[full_path_str.as_ref()])
                        .changelist(changelist)
                        .run()?;

                    // Get the depot file type (preserves P4 modifiers like +C, +k, +l)
                    let depot_type = self
                        .p4
                        .opened(&[full_path_str.as_ref()])
                        .run()?
                        .into_iter()
                        .next()
                        .map(|f| f.file_type);

                    if let Some(depot_type) = depot_type {
                        // Apply only the bits git can represent onto the depot type
                        let effective_type = apply_git_type_to_depot(&depot_type, &git_type);
                        if effective_type != depot_type {
                            self.p4
                                .reopen(&[full_path_str.as_ref()])
                                .changelist(changelist)
                                .file_type(effective_type)
                                .run()?;
                        }
                    }
                    Self::copy_file(&src, &dest)?;
                }
                FileAction::Delete => {
                    builder.delete(change.path)?;
                }
            }
        }
        builder.flush()
    }

    fn update_change_description(&self, cl: usize, description: &str) -> Result<(), P4Error> {
        let change_spec = ChangeSpec::new(ChangeType::Number(cl)).description(description);
        self.p4.change().set(&change_spec).run()?;
        Ok(())
    }

    pub fn create_or_reuse_changelist(
        &self,
        description: &str,
        original_change: Option<usize>,
        description_mode: ShelveDescriptionMode,
    ) -> Result<usize, P4Error> {
        match original_change {
            Some(cl) => {
                if description_mode == ShelveDescriptionMode::Update {
                    self.update_change_description(cl, description)?;
                }
                Ok(cl)
            }
            None => self.create_empty_change(description),
        }
    }

    pub fn shelve_changelist(
        &self,
        cl: usize,
        base_change: usize,
        base_dir: &Path,
        changes: &[FileChange],
    ) -> Result<(), P4Error> {
        self.sync(
            base_change,
            &changes.iter().map(|c| c.path).collect::<Vec<&str>>(),
        )?;
        self.apply_changes(cl, base_dir, changes)?;
        self.p4.shelve().set(cl).replace().run()?;
        Self::cleanup_workspace(&self.p4, &self.client_root)?;
        Ok(())
    }

    pub fn run(
        &self,
        base_change: usize,
        base_dir: &Path,
        changes: &[FileChange],
        description: &str,
        original_change: Option<usize>,
        description_mode: ShelveDescriptionMode,
    ) -> Result<usize, P4Error> {
        let cl = self.create_or_reuse_changelist(description, original_change, description_mode)?;
        self.shelve_changelist(cl, base_change, base_dir, changes)?;
        Ok(cl)
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
        test_client
            .changelist("Setup files")
            .add_file("file1.txt", b"content 1")
            .add_file("file2.txt", b"content 2")
            .add_file("subdir/file3.txt", b"content 3")
            .submit()
            .unwrap()
            .submitted_change
    }

    fn cleanup_shelved_change(tc: &p4rs::testkit::TestClient, cl: usize) {
        tc.p4.shelve().delete(cl).run().unwrap();
        tc.p4.change().delete(cl).run().unwrap();
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
        )
        .unwrap();

        let changes = [FileChange {
            path: "new.txt",
            action: FileAction::Add,
        }];
        let cl = client
            .run(0, tmp.path(), &changes, "Add new file", None, Default::default())
            .unwrap();
        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Add new file");
        cleanup_shelved_change(&tc, cl);
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
        )
        .unwrap();

        let changes = [FileChange {
            path: "file1.txt",
            action: FileAction::Edit,
        }];
        let cl = client
            .run(base, tmp.path(), &changes, "Edit file", None, Default::default())
            .unwrap();

        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Edit file");
        cleanup_shelved_change(&tc, cl);
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
        )
        .unwrap();

        let changes = [FileChange {
            path: "file1.txt",
            action: FileAction::Delete,
        }];
        let cl = client
            .run(base, tmp.path(), &changes, "Delete file", None, Default::default())
            .unwrap();

        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Delete file");
        cleanup_shelved_change(&tc, cl);
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
        )
        .unwrap();

        let changes = [FileChange {
            path: "script.sh",
            action: FileAction::Add,
        }];
        let cl = client
            .run(0, tmp.path(), &changes, "Add executable", None, Default::default())
            .unwrap();

        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Add executable");
        cleanup_shelved_change(&tc, cl);
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
        )
        .unwrap();

        let changes = [
            FileChange {
                path: "new.txt",
                action: FileAction::Add,
            },
            FileChange {
                path: "file2.txt",
                action: FileAction::Edit,
            },
            FileChange {
                path: "file1.txt",
                action: FileAction::Delete,
            },
        ];
        let cl = client
            .run(base, tmp.path(), &changes, "Multiple changes", None, Default::default())
            .unwrap();

        let shelved = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(shelved.description.trim(), "Multiple changes");
        cleanup_shelved_change(&tc, cl);
    }

    #[test]
    fn test_drop_reverts_files() {
        let tc = SERVER.test_client();
        {
            let _client = ShelveClient::new(
                tc.p4.clone(),
                &tc.client_name,
                tc.client_root().to_path_buf(),
            )
            .unwrap();
        }
        let opened = tc.p4.opened(&["//..."]).run().unwrap();
        assert!(opened.is_empty());
    }

    #[test]
    fn test_shelve_correct_base_revision() {
        let tc = SERVER.test_client();

        tc.changelist("Rev 1")
            .add_file("evolving.txt", b"version 1")
            .submit()
            .expect("submit rev 1");

        let base2 = tc
            .changelist("Rev 2")
            .edit_file("evolving.txt", b"version 2")
            .submit()
            .expect("submit rev 2")
            .submitted_change;

        tc.changelist("Rev 3")
            .edit_file("evolving.txt", b"version 3")
            .submit()
            .expect("submit rev 3");

        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("evolving.txt"), b"shelved content").unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();

        let changes = [FileChange {
            path: "evolving.txt",
            action: FileAction::Edit,
        }];

        let cl = client
            .run(base2, tmp.path(), &changes, "Edit from base2", None, Default::default())
            .expect("Failed to run shelve client");

        let shelved = tc
            .p4
            .describe(&[cl])
            .shelved()
            .run()
            .expect("Failed to describe shelved")
            .single()
            .unwrap();
        assert_eq!(shelved.files.len(), 1);
        assert_eq!(shelved.files[0].rev, Some(2));
        assert!(shelved.files[0].depot_file.ends_with("evolving.txt"));
        cleanup_shelved_change(&tc, cl);
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
        )
        .unwrap();

        let changes = [FileChange {
            path: "link.txt",
            action: FileAction::Add,
        }];
        let cl = client
            .run(0, tmp.path(), &changes, "Add symlink", None, Default::default())
            .unwrap();

        let shelved = tc
            .p4
            .describe(&[cl])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(shelved.files.len(), 1);
        assert!(shelved.files[0].depot_file.ends_with("link.txt"));
        assert_eq!(shelved.files[0].file_type.base, p4rs::BaseFileType::Symlink);

        cleanup_shelved_change(&tc, cl);
    }

    #[test]
    fn test_shelve_edit_symlink() {
        use p4rs::FileType;
        use std::os::unix::fs::symlink;
        let tc = SERVER.test_client();

        fs::write(tc.client_root().join("original_target.txt"), b"original").unwrap();
        let link_path = tc.client_root().join("link.txt");
        symlink("original_target.txt", &link_path).unwrap();
        let base = tc
            .changelist("Setup symlink")
            .add_file_with_opts("link.txt", b"", Some(FileType::symlink()))
            .submit()
            .unwrap()
            .submitted_change;

        let tmp = TempDir::new().unwrap();
        symlink("new_target.txt", tmp.path().join("link.txt")).unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();

        let changes = [FileChange {
            path: "link.txt",
            action: FileAction::Edit,
        }];
        let cl = client
            .run(base, tmp.path(), &changes, "Edit symlink", None, Default::default())
            .unwrap();

        let shelved = tc
            .p4
            .describe(&[cl])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(shelved.files.len(), 1);
        assert!(shelved.files[0].depot_file.ends_with("link.txt"));
        assert_eq!(shelved.files[0].file_type.base, p4rs::BaseFileType::Symlink);

        cleanup_shelved_change(&tc, cl);
    }

    #[test]
    fn test_shelve_file_to_symlink() {
        use std::os::unix::fs::symlink;
        let tc = SERVER.test_client();

        let base = tc
            .changelist("Setup regular file")
            .add_file("config.txt", b"original content")
            .submit()
            .unwrap()
            .submitted_change;

        let tmp = TempDir::new().unwrap();
        symlink("shared_config.txt", tmp.path().join("config.txt")).unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();

        let changes = [FileChange {
            path: "config.txt",
            action: FileAction::Edit,
        }];
        let cl = client
            .run(base, tmp.path(), &changes, "Convert to symlink", None, Default::default())
            .unwrap();

        let shelved = tc
            .p4
            .describe(&[cl])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(shelved.files.len(), 1);
        assert!(shelved.files[0].depot_file.ends_with("config.txt"));
        assert_eq!(shelved.files[0].file_type.base, p4rs::BaseFileType::Symlink);

        cleanup_shelved_change(&tc, cl);
    }

    /// Reproduces the bug where editing a file with P4 modifiers (e.g. text+Cx)
    /// through prgit strips the modifiers, leaving only what git can represent.
    #[test]
    fn test_edit_preserves_p4_file_type_modifiers() {
        use std::os::unix::fs::PermissionsExt;
        let tc = SERVER.test_client();

        // Create a file with text+Cx (compressed + executable) type in P4
        let base = tc
            .changelist("Add script with modifiers")
            .add_file_with_opts(
                "run.sh",
                b"#!/bin/bash\necho hello",
                Some(p4rs::FileType::text().compressed().executable()),
            )
            .submit()
            .unwrap()
            .submitted_change;

        // Edit the file content (keep executable bit) and shelve
        let tmp = TempDir::new().unwrap();
        let new_script = tmp.path().join("run.sh");
        fs::write(&new_script, b"#!/bin/bash\necho modified").unwrap();
        fs::set_permissions(&new_script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();

        let changes = [FileChange {
            path: "run.sh",
            action: FileAction::Edit,
        }];
        let cl = client
            .run(base, tmp.path(), &changes, "Edit script", None, Default::default())
            .unwrap();

        // Check the shelved file type — should still have compressed + executable
        let shelved = tc
            .p4
            .describe(&[cl])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(shelved.files.len(), 1);
        let shelved_type = &shelved.files[0].file_type;
        assert!(
            shelved_type.executable,
            "Shelved file should preserve executable flag, got: {shelved_type:?}"
        );
        assert!(
            shelved_type.compressed,
            "Shelved file should preserve compressed flag, got: {shelved_type:?}"
        );
        assert_eq!(
            shelved_type.base,
            p4rs::BaseFileType::Text,
            "Shelved file should remain text type"
        );

        cleanup_shelved_change(&tc, cl);
    }

    /// When the executable bit changes on a file with P4 modifiers,
    /// the shelver should only toggle the executable flag and preserve
    /// all other modifiers (e.g. +C, +k).
    #[test]
    fn test_edit_adding_executable_preserves_other_modifiers() {
        use std::os::unix::fs::PermissionsExt;
        let tc = SERVER.test_client();

        // Create a file with text+C (compressed, non-executable) type in P4
        let base = tc
            .changelist("Add compressed file")
            .add_file_with_opts(
                "data.txt",
                b"some data",
                Some(p4rs::FileType::text().compressed()),
            )
            .submit()
            .unwrap()
            .submitted_change;

        // Edit and make it executable in git
        let tmp = TempDir::new().unwrap();
        let new_file = tmp.path().join("data.txt");
        fs::write(&new_file, b"modified data").unwrap();
        fs::set_permissions(&new_file, std::fs::Permissions::from_mode(0o755)).unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();

        let changes = [FileChange {
            path: "data.txt",
            action: FileAction::Edit,
        }];
        let cl = client
            .run(base, tmp.path(), &changes, "Make executable", None, Default::default())
            .unwrap();

        // Should be text+Cx — executable added, compressed preserved
        let shelved = tc
            .p4
            .describe(&[cl])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(shelved.files.len(), 1);
        let shelved_type = &shelved.files[0].file_type;
        assert!(
            shelved_type.executable,
            "Should have executable flag after chmod, got: {shelved_type:?}"
        );
        assert!(
            shelved_type.compressed,
            "Should preserve compressed flag, got: {shelved_type:?}"
        );

        cleanup_shelved_change(&tc, cl);
    }

    #[test]
    fn test_edit_removing_executable_preserves_other_modifiers() {
        let tc = SERVER.test_client();

        // Create a file with text+kx (keyword expansion + executable) in P4
        let base = tc
            .changelist("Add file with keyword+exec")
            .add_file_with_opts(
                "versioned.txt",
                b"$Id$\nsome content",
                Some(p4rs::FileType::text().keyword_expansion().executable()),
            )
            .submit()
            .unwrap()
            .submitted_change;

        // Edit and remove executable in git (no +x permission)
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("versioned.txt"), b"$Id$\nmodified content").unwrap();
        // Default permissions are non-executable (0o644)

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();

        let changes = [FileChange {
            path: "versioned.txt",
            action: FileAction::Edit,
        }];
        let cl = client
            .run(base, tmp.path(), &changes, "Remove executable", None, Default::default())
            .unwrap();

        // Should be text+k — executable removed, keyword expansion preserved
        let shelved = tc
            .p4
            .describe(&[cl])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(shelved.files.len(), 1);
        let shelved_type = &shelved.files[0].file_type;
        assert!(
            !shelved_type.executable,
            "Should not have executable flag, got: {shelved_type:?}"
        );
        assert!(
            shelved_type.keyword_expansion,
            "Should preserve keyword expansion flag, got: {shelved_type:?}"
        );

        cleanup_shelved_change(&tc, cl);
    }

    #[test]
    fn test_shelve_symlink_to_file() {
        use p4rs::FileType;
        use std::os::unix::fs::symlink;
        let tc = SERVER.test_client();

        fs::write(tc.client_root().join("shared_config.txt"), b"shared").unwrap();
        let link_path = tc.client_root().join("config.txt");
        symlink("shared_config.txt", &link_path).unwrap();
        let base = tc
            .changelist("Setup symlink")
            .add_file_with_opts("config.txt", b"", Some(FileType::symlink()))
            .submit()
            .unwrap()
            .submitted_change;

        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("config.txt"), b"inline content").unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();

        let changes = [FileChange {
            path: "config.txt",
            action: FileAction::Edit,
        }];
        let cl = client
            .run(base, tmp.path(), &changes, "Convert to regular file", None, Default::default())
            .unwrap();

        let shelved = tc
            .p4
            .describe(&[cl])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(shelved.files.len(), 1);
        assert!(shelved.files[0].depot_file.ends_with("config.txt"));
        assert_eq!(shelved.files[0].file_type.base, p4rs::BaseFileType::Text);

        cleanup_shelved_change(&tc, cl);
    }

    /// Test that reshelving (p4 shelve -r) preserves file type modifiers
    #[test]
    fn test_reshelve_preserves_file_type_modifiers() {
        let tc = SERVER.test_client();

        // Create a file with text+k in P4
        let base = tc
            .changelist("Add keyword file")
            .add_file_with_opts(
                "version.h",
                b"$Id$\nv1",
                Some(p4rs::FileType::text().keyword_expansion()),
            )
            .submit()
            .unwrap()
            .submitted_change;

        // First shelve
        let tmp1 = TempDir::new().unwrap();
        fs::write(tmp1.path().join("version.h"), b"$Id$\nv2").unwrap();
        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();
        let cl = client
            .run(base, tmp1.path(), &[FileChange { path: "version.h", action: FileAction::Edit }], "First edit", None, Default::default())
            .unwrap();

        // Verify first shelve preserves +k
        let shelved = tc.p4.describe(&[cl]).shelved().run().unwrap().single().unwrap();
        assert!(shelved.files[0].file_type.keyword_expansion, "First shelve should preserve +k");

        // Reshelve (same CL)
        let tmp2 = TempDir::new().unwrap();
        fs::write(tmp2.path().join("version.h"), b"$Id$\nv3").unwrap();
        let client2 = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();
        let cl2 = client2
            .run(base, tmp2.path(), &[FileChange { path: "version.h", action: FileAction::Edit }], "Second edit", Some(cl), Default::default())
            .unwrap();
        assert_eq!(cl, cl2, "Should reuse same CL");

        // Verify reshelve also preserves +k
        let shelved2 = tc.p4.describe(&[cl2]).shelved().run().unwrap().single().unwrap();
        let shelved_type = &shelved2.files[0].file_type;
        assert!(
            shelved_type.keyword_expansion,
            "Reshelve should preserve +k, got: {shelved_type:?}"
        );
        assert!(
            !shelved_type.compressed,
            "Reshelve should NOT gain +C, got: {shelved_type:?}"
        );

        cleanup_shelved_change(&tc, cl);
    }

    /// Reproduces reported bug: text+k file edited through prgit becomes text+C
    #[test]
    fn test_edit_preserves_keyword_expansion_modifier() {
        let tc = SERVER.test_client();

        // Create a file with text+k (keyword expansion) in P4
        let base = tc
            .changelist("Add file with keyword expansion")
            .add_file_with_opts(
                "version.h",
                b"// $Id$\nconst char* version = \"1.0\";",
                Some(p4rs::FileType::text().keyword_expansion()),
            )
            .submit()
            .unwrap()
            .submitted_change;

        // Edit content only, no permission changes
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("version.h"),
            b"// $Id$\nconst char* version = \"2.0\";",
        )
        .unwrap();

        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();

        let changes = [FileChange {
            path: "version.h",
            action: FileAction::Edit,
        }];
        let cl = client
            .run(base, tmp.path(), &changes, "Edit version", None, Default::default())
            .unwrap();

        let shelved = tc
            .p4
            .describe(&[cl])
            .shelved()
            .run()
            .unwrap()
            .single()
            .unwrap();
        assert_eq!(shelved.files.len(), 1);
        let shelved_type = &shelved.files[0].file_type;
        assert!(
            shelved_type.keyword_expansion,
            "Should preserve keyword expansion (+k), got: {shelved_type:?}"
        );
        assert!(
            !shelved_type.compressed,
            "Should NOT gain compressed (+C) flag, got: {shelved_type:?}"
        );
        assert_eq!(shelved_type.base, p4rs::BaseFileType::Text);

        cleanup_shelved_change(&tc, cl);
    }

    #[test]
    fn test_reshelve_updates_description_in_update_mode() {
        let tc = SERVER.test_client();
        let base = setup_test_files(&tc);

        let tmp1 = TempDir::new().unwrap();
        fs::write(tmp1.path().join("file1.txt"), b"v2").unwrap();
        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();
        let cl = client
            .run(
                base,
                tmp1.path(),
                &[FileChange { path: "file1.txt", action: FileAction::Edit }],
                "First description",
                None,
                ShelveDescriptionMode::Update,
            )
            .unwrap();

        let described = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(described.description.trim(), "First description");

        // Reshelve with Update mode — description should change
        let tmp2 = TempDir::new().unwrap();
        fs::write(tmp2.path().join("file1.txt"), b"v3").unwrap();
        let client2 = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();
        client2
            .run(
                base,
                tmp2.path(),
                &[FileChange { path: "file1.txt", action: FileAction::Edit }],
                "Updated description",
                Some(cl),
                ShelveDescriptionMode::Update,
            )
            .unwrap();

        let described2 = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(described2.description.trim(), "Updated description");

        cleanup_shelved_change(&tc, cl);
    }

    #[test]
    fn test_reshelve_keeps_description_in_keep_original_mode() {
        let tc = SERVER.test_client();
        let base = setup_test_files(&tc);

        let tmp1 = TempDir::new().unwrap();
        fs::write(tmp1.path().join("file1.txt"), b"v2").unwrap();
        let client = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();
        let cl = client
            .run(
                base,
                tmp1.path(),
                &[FileChange { path: "file1.txt", action: FileAction::Edit }],
                "Original description",
                None,
                ShelveDescriptionMode::KeepOriginal,
            )
            .unwrap();

        let described = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(described.description.trim(), "Original description");

        // Reshelve with KeepOriginal mode — description should NOT change
        let tmp2 = TempDir::new().unwrap();
        fs::write(tmp2.path().join("file1.txt"), b"v3").unwrap();
        let client2 = ShelveClient::new(
            tc.p4.clone(),
            &tc.client_name,
            tc.client_root().to_path_buf(),
        )
        .unwrap();
        client2
            .run(
                base,
                tmp2.path(),
                &[FileChange { path: "file1.txt", action: FileAction::Edit }],
                "This should be ignored",
                Some(cl),
                ShelveDescriptionMode::KeepOriginal,
            )
            .unwrap();

        let described2 = tc.p4.describe(&[cl]).run().unwrap().single().unwrap();
        assert_eq!(described2.description.trim(), "Original description");

        cleanup_shelved_change(&tc, cl);
    }
}
