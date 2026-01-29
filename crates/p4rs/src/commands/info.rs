use crate::commands::process::{CmdType, P4Command};
use crate::error::P4Error;
use crate::output::P4Output;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;

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
        Self { p4, short: false }
    }
}

impl<'p> P4Command for InfoCommand<'p> {
    type Response = InfoResponse;

    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let mut process = self.p4.build_cmd("info", CmdType::Query);
        process.flag(self.short, "-s");
        self.p4.run_parsed(process, false)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    #[serde(rename = "ServerID")]
    pub server_id: Option<String>,
    pub client_name: String,
    pub client_root: Option<String>,
    pub client_cwd: String,
    pub client_host: String,
    pub server_version: String,
    pub user_name: String,
}
