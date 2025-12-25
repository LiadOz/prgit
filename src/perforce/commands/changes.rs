use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use crate::perforce::p4::P4;
use crate::perforce::error::P4Error;
use crate::perforce::commands::command::{P4Command, CmdType};
use derive_setters::Setters;
use crate::perforce::commands::types::ChangeStatus;

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
        if self.include_integrated {
            process.cmd.args(["-i"]);
        }
        if self.long {
            process.cmd.args(["-l"]);
        }
        if let Some(since_changelist) = &self.since_changelist {
            process.cmd.args(["-e", &since_changelist.to_string()]);
        }
        if let Some(max_changes) = &self.max_changes {
            process.cmd.args(["-m", &max_changes.to_string()]);
        }
        if let Some(status) = &self.status {
            process.cmd.args(["-s", &status.to_string()]);
        }
        if let Some(user) = &self.user {
            process.cmd.args(["-u", user]);
        }
        process.cmd.args(self.files.iter().map(|file| *file));
        let json = self.p4.run_multi_line(process)?;
        let response: Self::Response = serde_json::from_value(json)?;
        Ok(response)
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