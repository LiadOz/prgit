use crate::commands::change::Change;
use crate::commands::client::Client;
use crate::commands::print::Print;
use crate::commands::process::{CmdType, P4Process};
use crate::commands::shelve::Shelve;
use crate::commands::sync::SyncCommand;
use crate::commands::user::User;
use crate::commands::where_cmd::WhereCommand;
use crate::commands::{
    AddCommand, ChangesCommand, DeleteCommand, DescribeCommand, EditCommand, FilesCommand,
    InfoCommand, MoveCommand, OpenedCommand, ReopenCommand, RevertCommand, SubmitCommand,
};
use crate::error::{ErrorResponse, P4Error, P4Message};
use crate::output::P4Output;
use derive_setters::Setters;
use std::path::PathBuf;

#[derive(Setters, Clone)]
#[setters(into, strip_option)]
pub struct P4 {
    p4_path: PathBuf,
    port: Option<String>,
    #[setters(rename = "p4_user")]
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
                    let msg = P4Message::new(3, 0, String::from_utf8_lossy(&output.stderr).into_owned());
                    return Err(P4Error::command(vec![msg]));
                }
                _ => {
                    let messages = Self::extract_messages(&output.stdout);
                    let errors: Vec<_> = messages.into_iter().filter(|m| m.is_error()).collect();
                    if !errors.is_empty() {
                        return Err(P4Error::command(errors));
                    }
                }
            }
        }
        Ok(output)
    }

    fn extract_messages(stdout: &[u8]) -> Vec<P4Message> {
        let stdout_str = String::from_utf8_lossy(stdout);
        stdout_str
            .lines()
            .filter_map(|line| serde_json::from_str::<ErrorResponse>(line).ok())
            .map(|e| P4Message::new(e.severity as u8, e.generic.unwrap_or(0) as u8, e.data))
            .collect()
    }


    fn is_message_line(line: &str) -> bool {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            json.get("severity").is_some() || json.get("level").is_some()
        } else {
            false
        }
    }

    fn parse_output_with_messages(
        &self,
        output: &std::process::Output,
        multi_line: bool,
    ) -> Result<(serde_json::Value, Vec<P4Message>), P4Error> {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let mut data_lines = Vec::new();
        let mut messages = Vec::new();

        for line in stdout_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if Self::is_message_line(line) {
                if let Ok(resp) = serde_json::from_str::<ErrorResponse>(line) {
                    messages.push(P4Message::new(
                        resp.severity as u8,
                        resp.generic.unwrap_or(0) as u8,
                        resp.data,
                    ));
                }
            } else {
                data_lines.push(line);
            }
        }

        let json = if multi_line {
            let json_array = format!("[{}]", data_lines.join(","));
            serde_json::from_str(&json_array)?
        } else if data_lines.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(data_lines.first().unwrap_or(&"null"))?
        };

        Ok((json, messages))
    }

    pub(crate) fn run_parsed<T: serde::de::DeserializeOwned>(
        &self,
        p4_process: P4Process,
        multi_line: bool,
    ) -> Result<P4Output<T>, P4Error> {
        let output = self.run_command(p4_process, None)?;
        let (json, messages) = self.parse_output_with_messages(&output, multi_line)?;
        
        let (errors, warnings): (Vec<_>, Vec<_>) = messages.into_iter().partition(|m| m.is_error());
        
        if !errors.is_empty() {
            return Err(P4Error::command_with_partial(errors, json));
        }

        let results: Vec<T> = if json.is_null() {
            Vec::new()
        } else if multi_line {
            serde_json::from_value(json)?
        } else {
            vec![serde_json::from_value(json)?]
        };

        Ok(P4Output::new(results, warnings))
    }

    #[cfg(feature = "extensible")]
    pub fn run(&self, p4_process: P4Process) -> Result<serde_json::Value, P4Error> {
        self.run_json_inner(p4_process, false)
    }

    #[cfg(not(feature = "extensible"))]
    pub(crate) fn run(&self, p4_process: P4Process) -> Result<serde_json::Value, P4Error> {
        self.run_json_inner(p4_process, false)
    }

    fn run_json_inner(&self, p4_process: P4Process, multi_line: bool) -> Result<serde_json::Value, P4Error> {
        let output = self.run_command(p4_process, None)?;
        let (json, messages) = self.parse_output_with_messages(&output, multi_line)?;
        
        let errors: Vec<_> = messages.into_iter().filter(|m| m.is_error()).collect();
        if !errors.is_empty() {
            return Err(P4Error::command_with_partial(errors, json));
        }
        
        Ok(json)
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

    pub fn files<'p, 'f>(&'p self, files: &'f [&'f str]) -> FilesCommand<'p, 'f> {
        FilesCommand::new(self, files)
    }

    pub fn move_file<'p>(&'p self, from: &'p str, to: &'p str) -> MoveCommand<'p> {
        MoveCommand::new(self, from, to)
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

    pub fn user(&self) -> User<'_> {
        User::new(self)
    }

    pub fn where_cmd<'p, 'f>(&'p self, files: &'f [&'f str]) -> WhereCommand<'p, 'f> {
        WhereCommand::new(self, files)
    }
}

impl Default for P4 {
    fn default() -> Self {
        Self::new()
    }
}
