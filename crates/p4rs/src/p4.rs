use crate::commands::change::{ChangeCommand, ChangeSpec};
use crate::commands::process::{CmdType, P4Process};
use crate::commands::{ChangesCommand, EditCommand, InfoCommand, OpenedCommand, RevertCommand};
use crate::error::{ErrorResponse, P4Error};
use derive_setters::Setters;
use std::path::PathBuf;

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

    #[cfg(feature = "extensible")]
    pub fn build_cmd(&self, cmd_name: &str, cmd_type: CmdType) -> P4Process {
        self.build_cmd_inner(cmd_name, cmd_type)
    }

    #[cfg(not(feature = "extensible"))]
    pub(crate) fn build_cmd(&self, cmd_name: &str, cmd_type: CmdType) -> P4Process {
        self.build_cmd_inner(cmd_name, cmd_type)
    }

    fn build_cmd_inner(&self, cmd_name: &str, cmd_type: CmdType) -> P4Process {
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
        P4Process::new(cmd, cmd_type)
    }

    fn run_process(
        &self,
        process: &mut P4Process,
        stdin_data: Option<&str>,
    ) -> Result<std::process::Output, P4Error> {
        log::debug!("Running process: {:?}", process);
        let output = match stdin_data {
            Some(data) => process.run_with_stdin(data)?,
            None => process.output()?,
        };
        log::debug!(
            "Command output: {:?}",
            String::from_utf8_lossy(&output.stdout)
        );
        log::debug!(
            "Error output: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(output)
    }

    #[cfg(feature = "extensible")]
    pub fn run_command(
        &self,
        p4_process: P4Process,
        stdin_data: Option<&str>,
    ) -> Result<std::process::Output, P4Error> {
        self.run_command_inner(p4_process, stdin_data)
    }

    #[cfg(not(feature = "extensible"))]
    pub(crate) fn run_command(
        &self,
        p4_process: P4Process,
        stdin_data: Option<&str>,
    ) -> Result<std::process::Output, P4Error> {
        self.run_command_inner(p4_process, stdin_data)
    }

    fn run_command_inner(
        &self,
        mut p4_process: P4Process,
        stdin_data: Option<&str>,
    ) -> Result<std::process::Output, P4Error> {
        let output = self.run_process(&mut p4_process, stdin_data)?;
        if !output.status.success() {
            if output.stderr.starts_with(b"Perforce client error:") {
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                match stderr_str.lines().nth(1).unwrap_or("").trim() {
                    "Connect to server failed; check $P4PORT." => {
                        return Err(P4Error::ConnectionFailed)
                    }
                    _ => return Err(P4Error::UnexpectedError(stderr_str.into_owned())),
                }
            }

            match p4_process.cmd_type {
                CmdType::FormInput => {
                    return Err(P4Error::CommandFailed(
                        String::from_utf8_lossy(&output.stderr).into_owned(),
                    ));
                }
                _ => {
                    let error_response: ErrorResponse = serde_json::from_slice(&output.stdout)?;
                    return Err(P4Error::CommandFailed(error_response.data));
                }
            }
        }
        Ok(output)
    }

    fn extract_json_output(
        &self,
        output: &std::process::Output,
        multi_line: bool,
    ) -> Result<serde_json::Value, P4Error> {
        if multi_line {
            let json_array = format!(
                "[{}]",
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<&str>>()
                    .join(",")
            );
            Ok(serde_json::from_str(&json_array)?)
        } else {
            Ok(serde_json::from_slice(&output.stdout)?)
        }
    }

    #[cfg(feature = "extensible")]
    pub fn run(&self, p4_process: P4Process) -> Result<serde_json::Value, P4Error> {
        self.run_inner(p4_process)
    }

    #[cfg(not(feature = "extensible"))]
    pub(crate) fn run(&self, p4_process: P4Process) -> Result<serde_json::Value, P4Error> {
        self.run_inner(p4_process)
    }

    fn run_inner(&self, p4_process: P4Process) -> Result<serde_json::Value, P4Error> {
        let output = self.run_command(p4_process, None)?;
        self.extract_json_output(&output, false)
    }

    #[cfg(feature = "extensible")]
    pub fn run_multi_line(&self, p4_process: P4Process) -> Result<serde_json::Value, P4Error> {
        self.run_multi_line_inner(p4_process)
    }

    #[cfg(not(feature = "extensible"))]
    pub(crate) fn run_multi_line(
        &self,
        p4_process: P4Process,
    ) -> Result<serde_json::Value, P4Error> {
        self.run_multi_line_inner(p4_process)
    }

    fn run_multi_line_inner(&self, p4_process: P4Process) -> Result<serde_json::Value, P4Error> {
        let output = self.run_command(p4_process, None)?;
        self.extract_json_output(&output, true)
    }

    pub fn info<'p>(&'p self) -> InfoCommand<'p> {
        InfoCommand::new(self)
    }

    pub fn changes<'p, 'f>(&'p self, files: &'f [&'f str]) -> ChangesCommand<'p, 'f> {
        ChangesCommand::new(self, files)
    }

    pub fn set_change<'p, 's>(
        &'p self,
        change_spec: &'s ChangeSpec,
    ) -> ChangeCommand<'p, &'s ChangeSpec> {
        ChangeCommand::new(self, change_spec)
    }

    pub fn get_change<'p>(&'p self, change: usize) -> ChangeCommand<'p, usize> {
        ChangeCommand::<'p, usize>::new(self, change)
    }

    pub fn edit<'p, 'f>(&'p self, files: &'f [&'f str]) -> EditCommand<'p, 'f> {
        EditCommand::new(self, files)
    }

    pub fn opened<'p, 'f>(&'p self, files: &'f [&'f str]) -> OpenedCommand<'p, 'f> {
        OpenedCommand::new(self, files)
    }

    pub fn revert<'p, 'f>(&'p self, files: &'f [&'f str]) -> RevertCommand<'p, 'f> {
        RevertCommand::new(self, files)
    }
}

impl Default for P4 {
    fn default() -> Self {
        Self::new()
    }
}
