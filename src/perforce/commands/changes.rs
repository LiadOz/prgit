use serde::Deserialize;
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
    since_changelist: Option<String>,
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
    type Response = ChangesResponse;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut args = vec!["changes"];
        if self.include_integrated {
            args.push("-i");
        }
        if self.long {
            args.push("-l");
        }
        if let Some(since_changelist) = &self.since_changelist {
            args.extend(["-s", since_changelist]);
        }
        if let Some(max_changes) = &self.max_changes {
            args.extend(["-m", &max_changes.to_string().as_str()]);
        }
        if let Some(status) = &self.status {
            args.extend(["-s", &status.to_string().as_str()]);
        }
        if let Some(user) = &self.user {
            args.extend(["-u", &user]);
        }
        args.extend(self.files.iter().map(|file| file));
        let json = self.p4.run(&args)?;
        let response: Self::Response = serde_json::from_value(json)?;
        Ok(response)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChangesResponse {
    pub change: String,
    pub change_type: String,
    pub client: String,
    pub desc: String,
    pub path: String,
    pub status: ChangeStatus,
    pub time: String,
    pub user: String,
}