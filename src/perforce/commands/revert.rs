use serde::Deserialize;
use derive_setters::Setters;
use crate::perforce::p4::P4;
use crate::perforce::error::P4Error;
use crate::perforce::commands::command::{P4Command, CmdType};

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct RevertCommand<'p, 'f> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
    changelist: Option<usize>,
    #[setters(bool)]
    keep: bool,
    #[setters(bool)]
    preview: bool,
    #[setters(bool)]
    unchanged: bool,
}

impl<'p, 'f> RevertCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self { p4, files, changelist: None, keep: false, preview: false, unchanged: false }
    }
}

impl<'p, 'f> P4Command for RevertCommand<'p, 'f> {
    type Response = Vec<RevertResult>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("revert", CmdType::Query);
        process
            .opt("-c", &self.changelist)
            .flag(self.keep, "-k")
            .flag(self.preview, "-n")
            .flag(self.unchanged, "-a")
            .args(self.files);
        let json = self.p4.run_multi_line(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertResult {
    pub depot_file: String,
    pub client_file: String,
    #[serde(default)]
    pub have_rev: Option<String>,
    #[serde(default)]
    pub old_action: Option<String>,
}

