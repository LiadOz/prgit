use crate::commands::process::P4Command;
use crate::commands::types::FileType;
use crate::error::P4Error;
use crate::p4::P4;
use crate::{ChangeSpec, ChangeType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) enum PendingOp {
    Add { path: String, file_type: FileType },
    Edit { path: String, file_type: FileType },
    Delete { path: String },
    Move { from: String, to: String, file_type: Option<FileType> },
}

pub struct ChangelistBuilder<'p> {
    pub(crate) p4: &'p P4,
    pub changelist: usize,
    pub root: PathBuf,
    pending: Vec<PendingOp>,
    immediate: bool,
}

impl<'p> ChangelistBuilder<'p> {
    pub fn new(p4: &'p P4, root: PathBuf, description: &str) -> Result<Self, P4Error> {
        let changelist = p4
            .change()
            .set(&ChangeSpec::new(ChangeType::New).description(description))
            .run()?
            .single()?;
        Ok(Self::with_changelist(p4, root, changelist))
    }

    pub fn with_changelist(p4: &'p P4, root: PathBuf, changelist: usize) -> Self {
        Self {
            p4,
            changelist,
            root,
            pending: Vec::new(),
            immediate: false,
        }
    }

    pub fn immediate(mut self) -> Self {
        self.immediate = true;
        self
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    pub fn determine_file_type(path: &Path) -> Result<FileType, P4Error> {
        use std::os::unix::fs::PermissionsExt;
        let meta = path.symlink_metadata().map_err(|e| {
            P4Error::UnexpectedError(format!("Cannot detect file type for {}: {}", path.display(), e))
        })?;
        if meta.file_type().is_symlink() {
            Ok(FileType::symlink())
        } else if meta.permissions().mode() & 0o111 != 0 {
            Ok(FileType::text().executable())
        } else {
            Ok(FileType::text())
        }
    }

    pub fn add(&mut self, path: &str) -> Result<&mut Self, P4Error> {
        let full_path = self.resolve_path(path);
        let file_type = Self::determine_file_type(&full_path)?;
        self.add_with_type(path, file_type)
    }

    pub fn add_with_type(&mut self, path: &str, file_type: FileType) -> Result<&mut Self, P4Error> {
        let full_path = self.resolve_path(path);
        let path_str = full_path.to_string_lossy();
        if self.immediate {
            self.p4.add(&[path_str.as_ref()])
                .changelist(self.changelist)
                .file_type(file_type)
                .run()?;
        } else {
            self.pending.push(PendingOp::Add {
                path: path_str.into_owned(),
                file_type,
            });
        }
        Ok(self)
    }

    pub fn edit(&mut self, path: &str) -> Result<&mut Self, P4Error> {
        let full_path = self.resolve_path(path);
        let file_type = Self::determine_file_type(&full_path)?;
        self.edit_with_type(path, file_type)
    }

    pub fn edit_with_type(&mut self, path: &str, file_type: FileType) -> Result<&mut Self, P4Error> {
        let full_path = self.resolve_path(path);
        let path_str = full_path.to_string_lossy();
        if self.immediate {
            self.p4.edit(&[path_str.as_ref()])
                .changelist(self.changelist)
                .file_type(file_type)
                .run()?;
        } else {
            self.pending.push(PendingOp::Edit {
                path: path_str.into_owned(),
                file_type,
            });
        }
        Ok(self)
    }

    pub fn delete(&mut self, path: &str) -> Result<&mut Self, P4Error> {
        let full_path = self.resolve_path(path);
        let path_str = full_path.to_string_lossy();
        if self.immediate {
            self.p4.delete(&[path_str.as_ref()])
                .changelist(self.changelist)
                .run()?;
        } else {
            self.pending.push(PendingOp::Delete {
                path: path_str.into_owned(),
            });
        }
        Ok(self)
    }

    pub fn move_file(&mut self, from: &str, to: &str) -> Result<&mut Self, P4Error> {
        let from_path = self.resolve_path(from);
        let file_type = Self::determine_file_type(&from_path)?;
        self.move_file_with_type(from, to, file_type)
    }

    pub fn move_file_with_type(&mut self, from: &str, to: &str, file_type: FileType) -> Result<&mut Self, P4Error> {
        let from_path = self.resolve_path(from);
        let to_path = self.resolve_path(to);
        let from_str = from_path.to_string_lossy();
        let to_str = to_path.to_string_lossy();
        if self.immediate {
            self.p4.edit(&[from_str.as_ref()])
                .changelist(self.changelist)
                .run()?;
            self.p4.move_file(from_str.as_ref(), to_str.as_ref())
                .changelist(self.changelist)
                .file_type(file_type)
                .run()?;
        } else {
            self.pending.push(PendingOp::Move {
                from: from_str.into_owned(),
                to: to_str.into_owned(),
                file_type: Some(file_type),
            });
        }
        Ok(self)
    }

    pub fn flush(&mut self) -> Result<(), P4Error> {
        if self.pending.is_empty() {
            return Ok(());
        }

        let mut edits: HashMap<FileType, Vec<String>> = HashMap::new();
        let mut adds: HashMap<FileType, Vec<String>> = HashMap::new();
        let mut deletes: Vec<String> = Vec::new();
        let mut moves: Vec<(String, String, Option<FileType>)> = Vec::new();

        for op in self.pending.drain(..) {
            match op {
                PendingOp::Edit { path, file_type } => {
                    edits.entry(file_type).or_default().push(path);
                }
                PendingOp::Add { path, file_type } => {
                    adds.entry(file_type).or_default().push(path);
                }
                PendingOp::Delete { path } => {
                    deletes.push(path);
                }
                PendingOp::Move { from, to, file_type } => {
                    moves.push((from, to, file_type));
                }
            }
        }

        for (ft, paths) in &edits {
            let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            self.p4.edit(&refs).changelist(self.changelist).file_type(ft.clone()).run()?;
        }

        for (from, to, file_type) in moves {
            if !edits.values().any(|paths| paths.contains(&from)) {
                self.p4.edit(&[&from]).changelist(self.changelist).run()?;
            }
            let mut cmd = self.p4.move_file(&from, &to).changelist(self.changelist);
            if let Some(ft) = file_type {
                cmd = cmd.file_type(ft);
            }
            cmd.run()?;
        }

        for (ft, paths) in &adds {
            let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            self.p4.add(&refs).changelist(self.changelist).file_type(ft.clone()).run()?;
        }

        if !deletes.is_empty() {
            let refs: Vec<&str> = deletes.iter().map(|s| s.as_str()).collect();
            self.p4.delete(&refs).changelist(self.changelist).run()?;
        }

        Ok(())
    }

    pub fn submit(mut self) -> Result<crate::commands::submit::SubmitResult, P4Error> {
        self.flush()?;
        self.p4.submit(self.changelist).run()?.single()
    }
}
