use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::FileType;
use crate::error::P4Error;
use crate::output::P4Output;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

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
    type Response = DeleteResult;

    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let mut process = self.p4.build_cmd("delete", CmdType::Query);
        process
            .opt("-c", &self.changelist)
            .flag(self.preview, "-n")
            .flag(self.keep, "-k")
            .flag(self.virtual_delete, "-v")
            .args(self.files);
        self.p4.run_parsed(process, true)
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub depot_file: String,
    pub client_file: String,
    pub work_rev: String,
    pub action: String,
    #[serde(rename = "type")]
    #[serde_as(as = "DisplayFromStr")]
    pub file_type: FileType,
}
