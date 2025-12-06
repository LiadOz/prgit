use std::path::{Path, PathBuf};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::commands::ChangesCmd;

pub trait P4Command {
    type Response: DeserializeOwned;
    fn command_name() -> &'static str;
    fn args(&self) -> &[String];
}

#[derive(thiserror::Error, Debug)]
pub enum P4Error {
    #[error("Failed to execute command: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse JSON result: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Command Failed: {0}")]
    CommandFailed(String),
}

pub struct P4 {
    p4_path: PathBuf,
    port: Option<String>,
    user: Option<String>,
    password: Option<String>,
    client: Option<String>,
    retries: Option<usize>,
}

impl P4 {
    pub fn new() -> Self {
        return Self {p4_path: PathBuf::from("p4"), port: None, user: None, password: None, client: None, retries: None};
    }

    pub fn with_p4_path(mut self, p4_path: impl AsRef<Path>) -> Self {
        self.p4_path = p4_path.as_ref().to_path_buf();
        self
    }

    pub fn with_port(mut self, port: impl AsRef<str>) -> Self {
        self.port = Some(port.as_ref().to_string());
        self
    }

    pub fn with_user(mut self, user: impl AsRef<str>) -> Self {
        self.user = Some(user.as_ref().to_string());
        self
    }

    pub fn with_password(mut self, password: impl AsRef<str>) -> Self {
        self.password = Some(password.as_ref().to_string());
        self
    }
    
    pub fn with_client(mut self, client: impl AsRef<str>) -> Self {
        self.client = Some(client.as_ref().to_string());
        self
    }

    pub fn with_retries(mut self, retries: usize) -> Self {
        self.retries = Some(retries);
        self
    }

    fn build_cmd(&self, args: &[&str]) -> std::process::Command {
        let mut cmd = std::process::Command::new(&self.p4_path);
        cmd.args(&["-Mj", "-ztag"]);
        if let Some(client) = &self.client {
            cmd.arg("-c");
            cmd.arg(client);
        }
        if let Some(port) = &self.port {
            cmd.arg("-p");
            cmd.arg(port);
        }
        if let Some(user) = &self.user {
            cmd.arg("-u");
            cmd.arg(user);
        }
        if let Some(password) = &self.password {
            cmd.arg("-P");
            cmd.arg(password);
        }
        cmd.args(args);
        return cmd;
    }

    fn run(&self, args: &[&str]) -> Result<serde_json::Value, P4Error> {
        let mut cmd = self.build_cmd(args);
        log::debug!("Running command: {:?}", cmd);
        let output = cmd.output()?;
        log::debug!("Command output: {:?}\nError output: {:?}", output.stdout, output.stderr);
        let json = serde_json::from_slice(&output.stdout)?;
        if !output.status.success() {
            let error_response: ErrorResponse = serde_json::from_value(json)?;
            return Err(P4Error::CommandFailed(error_response.data));
        }
        Ok(json)
    }

    pub fn info(&self) -> Result<InfoResponse, P4Error> {
        let json = self.run(&["info"])?;
        let info_response: InfoResponse = serde_json::from_value(json)?;
        Ok(info_response)
    }

    pub fn changes(&self) -> ChangesCmd<'_> {
        ChangesCmd::new(self)
    }

    pub(crate) fn execute<C: P4Command>(&self, cmd: &C) -> Result<C::Response, P4Error> {
        let mut args: Vec<&str> = vec![C::command_name()];
        let arg_strings = cmd.args();
        args.extend(arg_strings.iter().map(|s| s.as_str()));
        let json = self.run(&args)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    data: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    #[serde(rename = "ServerID")]
    pub server_id: String,
    pub client_name: String,
    pub client_root: String,
    pub client_cwd: String,
    pub client_host: String,
    pub server_version: String,
    pub user_name: String,
}


mod tests {
    use super::*;

    #[test]
    fn test_p4_new() {
        let p4 = P4::new();
        assert!(matches!(p4.run(&["-h"]), Err(P4Error::JsonError(_))));
        assert!(matches!(p4.run(&["inf"]), Err(P4Error::CommandFailed(ref error)) if error.starts_with("Unknown command")));
        assert!(p4.info().is_ok());
    }
}