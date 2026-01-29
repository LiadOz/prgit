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
pub struct ReopenCommand<'p, 'f> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
    changelist: Option<usize>,
    file_type: Option<FileType>,
}

impl<'p, 'f> ReopenCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self {
            p4,
            files,
            changelist: None,
            file_type: None,
        }
    }
}

impl<'p, 'f> P4Command for ReopenCommand<'p, 'f> {
    type Response = ReopenResult;

    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let mut process = self.p4.build_cmd("reopen", CmdType::Query);
        process
            .opt("-c", &self.changelist)
            .opt("-t", &self.file_type)
            .args(self.files);
        self.p4.run_parsed(process, true)
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReopenResult {
    pub depot_file: String,
    pub work_rev: String,
    #[serde(default)]
    pub change: Option<String>,
    #[serde(default, rename = "type")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub file_type: Option<FileType>,
}
