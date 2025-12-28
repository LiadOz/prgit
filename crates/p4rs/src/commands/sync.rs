use crate::commands::process::{CmdType, P4Command};
use crate::error::P4Error;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct SyncCommand<'p, 'f> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
    max_files: Option<usize>,
    #[setters(bool)]
    force: bool,
    #[setters(bool)]
    preview: bool,
}

impl<'p, 'f> SyncCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self {
            p4,
            files,
            max_files: None,
            force: false,
            preview: false,
        }
    }
}

impl<'p, 'f> P4Command for SyncCommand<'p, 'f> {
    type Response = Vec<SyncResult>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("sync", CmdType::Query);
        process
            .opt("-m", &self.max_files)
            .flag(self.force, "-f")
            .flag(self.preview, "-n")
            .args(self.files);
        let json = self.p4.run_multi_line(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncAction {
    Added,
    Updated,
    Deleted,
    Refreshed,
}

impl std::fmt::Display for SyncAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SyncAction::Added => "added",
            SyncAction::Updated => "updated",
            SyncAction::Deleted => "deleted",
            SyncAction::Refreshed => "refreshed",
        };
        f.write_str(s)
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub depot_file: String,
    pub client_file: String,
    #[serde_as(as = "DisplayFromStr")]
    pub rev: usize,
    pub action: SyncAction,
    #[serde_as(as = "DisplayFromStr")]
    pub file_size: usize,
}

