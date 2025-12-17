use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use crate::perforce::p4::P4;
use crate::perforce::error::P4Error;
use crate::perforce::commands::command::P4Command;
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
        let mut args = vec!["changes"];
        if self.include_integrated {
            args.push("-i");
        }
        if self.long {
            args.push("-l");
        }
        let since_changelist_str;
        if let Some(since_changelist) = &self.since_changelist {
            since_changelist_str = since_changelist.to_string();
            args.extend(["-e", &since_changelist_str]);
        }
        let max_changes_str;
        if let Some(max_changes) = &self.max_changes {
            max_changes_str = max_changes.to_string();
            args.extend(["-m", &max_changes_str]);
        }
        let status_str;
        if let Some(status) = &self.status {
            status_str = status.to_string();
            args.extend(["-s", &status_str]);
        }
        if let Some(user) = &self.user {
            args.extend(["-u", &user]);
        }
        args.extend(self.files.iter().map(|file| file));
        let json = self.p4.run_multi_line(&args)?;
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
    pub path: String,
    pub status: ChangeStatus,
    pub time: String,
    pub user: String,
}