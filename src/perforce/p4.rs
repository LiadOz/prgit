use std::path::{Path, PathBuf};
use crate::perforce::error::{P4Error, ErrorResponse};
use crate::perforce::commands::{InfoCommand, InfoResponse};

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
        Self {
            p4_path: PathBuf::from("p4"),
            port: None,
            user: None,
            password: None,
            client: None,
            retries: None,
        }
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
        cmd
    }

    pub(crate) fn run(&self, args: &[&str]) -> Result<serde_json::Value, P4Error> {
        let mut cmd = self.build_cmd(args);
        log::debug!("Running command: {:?}", cmd);
        println!("Running command: {:?}", cmd);
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

    pub fn new_info<'a>(&self, test: &'a str) -> InfoCommand<'_, 'a> {
        InfoCommand::new(self, test)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perforce::commands::P4Command;

    #[test]
    fn test_p4_new() {
        let p4 = P4::new();
        assert!(matches!(p4.run(&["-h"]), Err(P4Error::JsonError(_))));
        assert!(matches!(p4.run(&["inf"]), Err(P4Error::CommandFailed(ref error)) if error.starts_with("Unknown command")));
        println!("{:?}", p4.new_info("test").short(true).run());
        assert!(p4.new_info("test").short(true).run().is_ok());
    }
}
