use std::path::PathBuf;
use std::io::Write;
use std::process::{Stdio};
use crate::perforce::commands::command::{CmdType, P4Process};
use crate::perforce::error::{P4Error, ErrorResponse};
use crate::perforce::commands::{InfoCommand, ChangesCommand};
use crate::perforce::commands::change::{ChangeSpec, ChangeCommand};
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

    pub(crate) fn build_cmd(&self, cmd_name: &str, cmd_type: CmdType) -> P4Process {
        let mut cmd = std::process::Command::new(&self.p4_path);
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
        match cmd_type {
            CmdType::FormOutput => cmd.args(["-Mj", "-ztag", cmd_name, "-o"]),
            CmdType::FormInput => cmd.args([cmd_name, "-i"]),
            CmdType::Query => cmd.args(["-Mj", "-ztag", cmd_name]),
        };
        P4Process {
            cmd: cmd,
            cmd_type: cmd_type,
        }
    }

    fn run_process(&self, p4_process: &mut P4Process, stdin_data: Option<&str>) -> Result<std::process::Output, P4Error> {
        log::debug!("Running command: {:?}", p4_process.cmd);
        let output;
        if let Some(stdin_data) = stdin_data {
            log::debug!("Writing stdin data: {:?}", stdin_data);
            let mut child = p4_process.cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            child.stdin.as_mut().ok_or(P4Error::CommandFailed("Failed to set stdin".to_string()))?.write_all(stdin_data.as_bytes())?;
            output = child.wait_with_output()?;
        }
        else {
            output = p4_process.cmd.output()?;
        }
        log::debug!("Command output: {:?}", String::from_utf8_lossy(&output.stdout));
        log::debug!("Error output: {:?}", String::from_utf8_lossy(&output.stderr));
        Ok(output)
    }

    pub(crate) fn run_command(&self, mut p4_process: P4Process, stdin_data: Option<&str>) -> Result<std::process::Output, P4Error> {
        let output = self.run_process(&mut p4_process, stdin_data)?;
        if !output.status.success() {
            if output.stderr.starts_with(b"Perforce client error:") {
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                match stderr_str.lines().nth(1).unwrap_or("").trim() {
                    "Connect to server failed; check $P4PORT." => return Err(P4Error::ConnectionFailed),
                    _ => return Err(P4Error::UnexpectedError(stderr_str.into_owned())),
                }
            }

            match p4_process.cmd_type {
                CmdType::FormInput => {
                    return Err(P4Error::CommandFailed(String::from_utf8_lossy(&output.stderr).into_owned()));
                }
                _ => {
                    let error_response: ErrorResponse = serde_json::from_slice(&output.stdout)?;
                    return Err(P4Error::CommandFailed(error_response.data));
                }
            }
        }
        Ok(output)
    }

    pub fn extract_json_output(&self, output: &std::process::Output, multi_line: bool) -> Result<serde_json::Value, P4Error> {
        if multi_line {
            let json_array = format!("[{}]", 
                String::from_utf8_lossy(&output.stdout).lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect::<Vec<&str>>().join(",")
            );
            Ok(serde_json::from_str(&json_array)?)
        } else {
            Ok(serde_json::from_slice(&output.stdout)?)
        }
    }

    pub(crate) fn run(&self, p4_process: P4Process) -> Result<serde_json::Value, P4Error> {
        let output = self.run_command(p4_process, None)?;
        self.extract_json_output(&output, false)
    }

    pub(crate) fn run_multi_line(&self, p4_process: P4Process) -> Result<serde_json::Value, P4Error> {
        let output = self.run_command(p4_process, None)?;
        self.extract_json_output(&output, true)
    }

    pub fn info<'p>(&'p self) -> InfoCommand<'p> {
        InfoCommand::new(self)
    }

    pub fn changes<'p, 'f>(&'p self, files: &'f [&'f str]) -> ChangesCommand<'p, 'f> {
        ChangesCommand::new(self, files)
    }

    pub fn set_change<'p, 's>(&'p self, change_spec: &'s ChangeSpec) -> ChangeCommand<'p, &'s ChangeSpec> {
        ChangeCommand::new(self, change_spec)
    }

    pub fn get_change<'p>(&'p self, change: usize) -> ChangeCommand<'p, usize> {
        ChangeCommand::<'p, usize>::new(self, change)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perforce::commands::P4Command;
    use crate::perforce::commands::types::ChangeStatus;
    use crate::perforce::commands::change::ChangeType;
    use test_log::test;

    #[test]
    fn test_p4_new() {
        let p4 = P4::new();
        assert!(matches!(p4.run(p4.build_cmd("-h", CmdType::Query)), Err(P4Error::JsonError(_))));
        assert!(matches!(p4.run(p4.build_cmd("inf", CmdType::Query)), Err(P4Error::CommandFailed(ref error)) if error.starts_with("Unknown command")));
        assert!(p4.info().short().run().is_ok());
        let changes = p4.changes(&[]).long().run().unwrap();
        assert!(!changes.is_empty());
        assert!(changes.len() >= 3);
        assert!(changes.last().unwrap().status == ChangeStatus::Submitted);
        let change_spec = ChangeSpec::new(ChangeType::New).description("Test change".to_string());
        let change_number = p4.set_change(&change_spec).run().unwrap();
        assert!(change_number > 0);
        let result_spec = p4.get_change(change_number).run().unwrap();
        assert!(result_spec.description.trim() == change_spec.description.trim());
    }
}
