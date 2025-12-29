use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::{deserialize_unix_timestamp, ChangeStatus};
use crate::error::P4Error;
use crate::p4::P4;
use chrono::{DateTime, Utc};
use derive_setters::Setters;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

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
    client: Option<String>,
    #[setters(bool)]
    reverse: bool,
}

impl<'p, 'f> ChangesCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self {
            p4,
            files,
            include_integrated: false,
            long: false,
            since_changelist: None,
            max_changes: None,
            status: None,
            user: None,
            client: None,
            reverse: false,
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
            .flag(self.reverse, "-r")
            .opt("-e", &self.since_changelist)
            .opt("-m", &self.max_changes)
            .opt("-s", &self.status)
            .opt("-u", &self.user)
            .opt("-c", &self.client)
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
    #[serde(deserialize_with = "deserialize_unix_timestamp")]
    pub time: DateTime<Utc>,
    pub user: String,
    pub old_change: Option<usize>,
}
