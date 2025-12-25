use serde::Deserialize;
use crate::perforce::p4::P4;
use crate::perforce::error::P4Error;
use crate::perforce::commands::command::{P4Command, CmdType};
use derive_setters::Setters;

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct InfoCommand<'p> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(bool)]
    short: bool,
}

impl<'p> InfoCommand<'p> {
    pub fn new(p4: &'p P4) -> Self {
        Self {
            p4: p4,
            short: false,
        }
    }
}

impl<'p> P4Command for InfoCommand<'p> {
    type Response = InfoResponse;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("info", CmdType::Query);
        if self.short {
            process.cmd.args(["-s"]);
        }
        let json = self.p4.run(process)?;
        let response: Self::Response = serde_json::from_value(json)?;
        Ok(response)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    #[serde(rename = "ServerID")]
    pub server_id: String,
    pub client_name: String,
    pub client_root: Option<String>,
    pub client_cwd: String,
    pub client_host: String,
    pub server_version: String,
    pub user_name: String,
}