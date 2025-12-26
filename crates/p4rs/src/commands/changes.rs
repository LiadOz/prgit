use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use crate::p4::P4;
use crate::error::P4Error;
use crate::commands::process::{P4Command, CmdType};
use derive_setters::Setters;
use crate::commands::types::ChangeStatus;

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct ChangesCommand<'p, 'f> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
    #[setters(bool)]
    include_integrated: bool,
    #[setters(bool)]
    long: bool,
    since_changelist: Option<usize>,
    max_changes: Option<usize>,
    status: Option<ChangeStatus>,
    user: Option<String>,
}

impl<'p, 'f> ChangesCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self {
            p4: p4,
            files: files,
            include_integrated: false,
            long: false,
            since_changelist: None,
            max_changes: None,
            status: None,
            user: None,
        }
    }
}

impl<'p, 'f> P4Command for ChangesCommand<'p, 'f> {
    type Response = Vec<ChangeData>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("changes", CmdType::Query);
        process
            .flag(self.include_integrated, "-i")
            .flag(self.long, "-l")
            .opt("-e", &self.since_changelist)
            .opt("-m", &self.max_changes)
            .opt("-s", &self.status)
            .opt("-u", &self.user)
            .args(self.files);
        let json = self.p4.run_multi_line(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChangeData {
    #[serde_as(as = "DisplayFromStr")]
    pub change: usize,
    pub change_type: String,
    pub client: String,
    pub desc: String,
    pub path: Option<String>,
    pub status: ChangeStatus,
    pub time: String,
    pub user: String,
}
