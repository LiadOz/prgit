use serde::Deserialize;
use crate::perforce::p4::P4;
use crate::perforce::commands::command::{make_command, P4Command, P4CommandBase};

make_command!(InfoCommand<'p, 'a>, "info",
    [test: &'a str],
    short: bool,
);

impl P4Command for InfoCommand<'_, '_> {
    type Response = InfoResponse;
    fn args(&self) -> Vec<&str> {
        let mut args = vec![];
        if let Some(short) = self.short {
            if short {
                args.push("-s");
            }
        }
        args.push(self.test);
        args
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