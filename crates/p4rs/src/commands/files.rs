use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::FileType;
use crate::error::P4Error;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct FilesCommand<'p, 'f> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
}

impl<'p, 'f> FilesCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self { p4, files }
    }
}

impl<'p, 'f> P4Command for FilesCommand<'p, 'f> {
    type Response = Vec<FileInfo>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("files", CmdType::Query);
        process.args(self.files);
        let json = self.p4.run_multi_line(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub depot_file: String,
    #[serde_as(as = "DisplayFromStr")]
    pub rev: usize,
    #[serde(rename = "type")]
    #[serde_as(as = "DisplayFromStr")]
    pub file_type: FileType,
    #[serde_as(as = "DisplayFromStr")]
    pub change: usize,
    pub action: String,
}
