use crate::commands::process::{CmdType, P4Command};
use crate::error::P4Error;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct DeleteCommand<'p, 'f> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
    changelist: Option<usize>,
    #[setters(bool)]
    preview: bool,
    #[setters(bool)]
    keep: bool,
    #[setters(bool)]
    virtual_delete: bool,
}

impl<'p, 'f> DeleteCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self {
            p4,
            files,
            changelist: None,
            preview: false,
            keep: false,
            virtual_delete: false,
        }
    }
}

impl<'p, 'f> P4Command for DeleteCommand<'p, 'f> {
    type Response = Vec<DeleteResult>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("delete", CmdType::Query);
        process
            .opt("-c", &self.changelist)
            .flag(self.preview, "-n")
            .flag(self.keep, "-k")
            .flag(self.virtual_delete, "-v")
            .args(self.files);
        let json = self.p4.run_multi_line(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub depot_file: String,
    pub client_file: String,
    pub work_rev: String,
    pub action: String,
    #[serde(rename = "type")]
    pub file_type: String,
}
