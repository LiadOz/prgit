use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::FileType;
use crate::error::P4Error;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct AddCommand<'p, 'f> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
    changelist: Option<usize>,
    file_type: Option<FileType>,
    #[setters(bool)]
    preview: bool,
}

impl<'p, 'f> AddCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self {
            p4,
            files,
            changelist: None,
            file_type: None,
            preview: false,
        }
    }
}

impl<'p, 'f> P4Command for AddCommand<'p, 'f> {
    type Response = Vec<AddResult>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("add", CmdType::Query);
        process
            .opt("-c", &self.changelist)
            .opt("-t", &self.file_type)
            .flag(self.preview, "-n")
            .args(self.files);
        let json = self.p4.run_multi_line(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddResult {
    pub depot_file: String,
    pub client_file: String,
    pub work_rev: String,
    pub action: String,
    #[serde(rename = "type")]
    pub file_type: String,
}
