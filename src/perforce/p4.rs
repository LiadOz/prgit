use std::path::PathBuf;
use crate::perforce::error::{P4Error, ErrorResponse};
use crate::perforce::commands::{InfoCommand, ChangesCommand};
use derive_setters::Setters;

#[derive(Setters)]
#[setters(into, strip_option)]
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

    fn build_cmd(&self, args: &[&str]) -> std::process::Command {
        let mut cmd = std::process::Command::new(&self.p4_path);
        cmd.args(&["-Mj", "-ztag"]);
        if let Some(client) = &self.client {
            cmd.args(["-c", client]);
        }
        if let Some(port) = &self.port {
            cmd.args(["-p", port]);
        }
        if let Some(user) = &self.user {
            cmd.args(["-u", user]);
        }
        if let Some(password) = &self.password {
            cmd.args(["-P", password]);
        }
        cmd.args(args);
        cmd
    }

    fn run_cmd(&self, args: &[&str], multi_line: bool) -> Result<serde_json::Value, P4Error> {
        let mut cmd = self.build_cmd(args);
        log::debug!("Running command: {:?}", cmd);
        let output = cmd.output()?;
        log::debug!("Command output: {:?}\nError output: {:?}", output.stdout, output.stderr);
        if !output.status.success() {
            if output.stderr.starts_with(b"Perforce client error:") {
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                match stderr_str.lines().nth(1).unwrap_or("").trim() {
                    "Connect to server failed; check $P4PORT." => return Err(P4Error::ConnectionFailed),
                    _ => return Err(P4Error::UnknownError(stderr_str.into_owned())),
                }
            }

            let error_response: ErrorResponse = serde_json::from_slice(&output.stdout)?;
            return Err(P4Error::CommandFailed(error_response.data));
        }
        if multi_line {
            let json_array = format!("[{}]", 
                String::from_utf8_lossy(&output.stdout).lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect::<Vec<&str>>().join(",")
            );
            Ok(serde_json::from_str(&json_array)?)
        } else {
            Ok(serde_json::from_slice(&output.stdout)?)
        }
    }

    pub(crate) fn run(&self, args: &[&str]) -> Result<serde_json::Value, P4Error> {
        self.run_cmd(args, false)
    }

    pub(crate) fn run_multi_line(&self, args: &[&str]) -> Result<serde_json::Value, P4Error> {
        self.run_cmd(args, true)
    }

    pub fn info<'p>(&'p self) -> InfoCommand<'p> {
        InfoCommand::new(self)
    }

    pub fn changes<'p, 'f>(&'p self, files: &'f [&'f str]) -> ChangesCommand<'p, 'f> {
        ChangesCommand::new(self, files)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perforce::commands::P4Command;
    use crate::perforce::commands::types::ChangeStatus;

    #[test]
    fn test_p4_new() {
        let p4 = P4::new();
        assert!(matches!(p4.run(&["-h"]), Err(P4Error::JsonError(_))));
        assert!(matches!(p4.run(&["inf"]), Err(P4Error::CommandFailed(ref error)) if error.starts_with("Unknown command")));
        assert!(p4.info().short().run().is_ok());
        let changes = p4.changes(&[]).long().run().unwrap();
        assert!(!changes.is_empty());
        assert!(changes.len() == 3);
        assert!(changes[0].status == ChangeStatus::Submitted);
    }
}
