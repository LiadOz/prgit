use crate::commands::change::Change;
use crate::commands::client::Client;
use crate::commands::print::Print;
use crate::commands::process::{CmdType, P4Process};
use crate::commands::shelve::Shelve;
use crate::commands::sync::SyncCommand;
use crate::commands::{
    AddCommand, ChangesCommand, DeleteCommand, DescribeCommand, EditCommand, InfoCommand,
    OpenedCommand, ReopenCommand, RevertCommand, SubmitCommand,
};
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
    client_name: Option<String>,
    retries: Option<usize>,
}

impl P4 {
    pub fn new() -> Self {
        Self {
            p4_path: PathBuf::from("p4"),
            port: None,
            user: None,
            password: None,
            client_name: None,
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
        if let Some(client) = &self.client_name {
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
                        3,
                    ));
                }
                _ => {
                    if let Some((data, severity)) = Self::extract_errors(&output.stdout) {
                        return Err(P4Error::CommandFailed(data, severity));
                    }
                }
            }
        }
        if p4_process.cmd_type != CmdType::FormInput {
            if let Some((data, severity)) = Self::extract_errors(&output.stdout) {
                return Err(P4Error::CommandSpecificError(data, severity));
            }
        }
        Ok(output)
    }

    fn extract_errors(stdout: &[u8]) -> Option<(String, usize)> {
        let stdout_str = String::from_utf8_lossy(stdout);
        let errors: Vec<ErrorResponse> = stdout_str
            .lines()
            .filter_map(|line| serde_json::from_str::<ErrorResponse>(line).ok())
            .collect();

        if errors.is_empty() {
            return None;
        }

        let combined_data = errors
            .iter()
            .map(|e| e.data.as_str())
            .collect::<Vec<_>>()
            .join("");
        let max_severity = errors.iter().map(|e| e.severity).max().unwrap_or(0);
        Some((combined_data, max_severity))
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

    pub fn describe(&self, changelists: &[usize]) -> DescribeCommand<'_> {
        DescribeCommand::new(self, changelists)
    }

    pub fn change(&self) -> Change<'_> {
        Change::new(self)
    }

    pub fn add<'p, 'f>(&'p self, files: &'f [&'f str]) -> AddCommand<'p, 'f> {
        AddCommand::new(self, files)
    }

    pub fn delete<'p, 'f>(&'p self, files: &'f [&'f str]) -> DeleteCommand<'p, 'f> {
        DeleteCommand::new(self, files)
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

    pub fn reopen<'p, 'f>(&'p self, files: &'f [&'f str]) -> ReopenCommand<'p, 'f> {
        ReopenCommand::new(self, files)
    }

    pub fn submit(&self, changelist: usize) -> SubmitCommand<'_> {
        SubmitCommand::new(self, changelist)
    }

    pub fn client(&self) -> Client<'_> {
        Client::new(self)
    }

    pub fn shelve(&self) -> Shelve<'_> {
        Shelve::new(self)
    }

    pub fn print(&self) -> Print<'_> {
        Print::new(self)
    }

    pub fn sync<'p, 'f>(&'p self, files: &'f [&'f str]) -> SyncCommand<'p, 'f> {
        SyncCommand::new(self, files)
    }
}

impl Default for P4 {
    fn default() -> Self {
        Self::new()
    }
}
