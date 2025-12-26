use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use derive_setters::Setters;
use crate::p4::P4;
use crate::error::P4Error;
use crate::commands::process::{P4Command, CmdType};
use crate::commands::change::ChangeType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpenAction {
    Edit,
    Add,
    Delete,
    Branch,
    Integrate,
    MoveAdd,
    MoveDelete,
}

impl std::fmt::Display for OpenAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenAction::Edit => write!(f, "edit"),
            OpenAction::Add => write!(f, "add"),
            OpenAction::Delete => write!(f, "delete"),
            OpenAction::Branch => write!(f, "branch"),
            OpenAction::Integrate => write!(f, "integrate"),
            OpenAction::MoveAdd => write!(f, "move/add"),
            OpenAction::MoveDelete => write!(f, "move/delete"),
        }
    }
}

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct OpenedCommand<'p, 'f> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
    changelist: Option<usize>,
    client: Option<String>,
    user: Option<String>,
    #[setters(bool)]
    all_users: bool,
    #[setters(bool)]
    short: bool,
}

impl<'p, 'f> OpenedCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self { p4, files, changelist: None, client: None, user: None, all_users: false, short: false }
    }
}

impl<'p, 'f> P4Command for OpenedCommand<'p, 'f> {
    type Response = Vec<OpenedFile>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("opened", CmdType::Query);
        process
            .opt("-c", &self.changelist)
            .opt("-C", &self.client)
            .opt("-u", &self.user)
            .flag(self.all_users, "-a")
            .flag(self.short, "-s")
            .args(self.files);
        let json = self.p4.run_multi_line(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedFile {
    pub depot_file: String,
    pub client_file: String,
    #[serde_as(as = "DisplayFromStr")]
    pub rev: usize,
    #[serde_as(as = "DisplayFromStr")]
    pub have_rev: usize,
    pub action: OpenAction,
    #[serde_as(as = "DisplayFromStr")]
    pub change: ChangeType,
    #[serde(rename = "type")]
    pub file_type: String,
    pub user: String,
    pub client: String,
}
