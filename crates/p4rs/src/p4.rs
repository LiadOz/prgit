use std::path::PathBuf;
use crate::commands::process::{CmdType, P4Process};
use crate::error::{P4Error, ErrorResponse};
use crate::commands::{InfoCommand, ChangesCommand, EditCommand, OpenedCommand, RevertCommand};
use crate::commands::change::{ChangeSpec, ChangeCommand};
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
        P4Process::new(cmd, cmd_type)
    }

    fn run_process(&self, process: &mut P4Process, stdin_data: Option<&str>) -> Result<std::process::Output, P4Error> {
        log::debug!("Running process: {:?}", process);
        let output = match stdin_data {
            Some(data) => process.run_with_stdin(data)?,
            None => process.output()?,
        };
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

    fn extract_json_output(&self, output: &std::process::Output, multi_line: bool) -> Result<serde_json::Value, P4Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::P4Command;
    use crate::commands::types::ChangeStatus;
    use crate::commands::change::ChangeType;
    use crate::commands::edit::EditAction;
    use crate::commands::opened::OpenAction;
    use crate::commands::process::CmdType;
    use test_log::test;

    #[test]
    fn test_p4_new() {
        let p4 = P4::new();
        assert!(matches!(p4.run(p4.build_cmd("-h", CmdType::Query)), Err(P4Error::JsonError(_))));
        assert!(matches!(p4.run(p4.build_cmd("inf", CmdType::Query)), Err(P4Error::CommandFailed(ref error)) if error.starts_with("Unknown command")));
        assert!(p4.info().short().run().is_ok());
        let changes = p4.changes(&[]).long().run().expect("Failed to get changes");
        assert!(!changes.is_empty());
        assert!(changes.len() >= 3);
        assert!(changes.last().expect("No changes found").status == ChangeStatus::Submitted);
        let change_spec = ChangeSpec::new(ChangeType::New).description("Test change".to_string());
        let change_number = p4.set_change(&change_spec).run().expect("Failed to set change");
        assert!(change_number > 0);
        let result_spec = p4.get_change(change_number).run().expect("Failed to get change");
        assert!(result_spec.description.trim() == change_spec.description.trim());
    }

    #[test]
    fn test_edit_opened_revert() {
        let p4 = P4::new();
        let test_file = "//depot/testing/test_file";

        p4.revert(&[test_file]).run().ok();

        let results = p4.edit(&[test_file]).run().expect("Failed to edit file");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, EditAction::Edit);
        assert!(results[0].depot_file.ends_with("test_file"));

        let opened = p4.opened(&[test_file]).run().expect("Failed to get opened files");
        assert_eq!(opened.len(), 1);
        assert_eq!(opened[0].action, OpenAction::Edit);

        let reverted = p4.revert(&[test_file]).run().expect("Failed to revert file");
        assert_eq!(reverted.len(), 1);
        assert!(reverted[0].depot_file.ends_with("test_file"));

        let opened_after = p4.opened(&[test_file]).run().expect("Failed to get opened files after revert");
        assert!(opened_after.is_empty());
    }

    #[test]
    fn test_edit_with_changelist() {
        let p4 = P4::new();
        let test_file = "//depot/testing/another_file";

        p4.revert(&[test_file]).run().ok();

        let change_spec = ChangeSpec::new(ChangeType::New).description("Test CL for edit".to_string());
        let cl = p4.set_change(&change_spec).run().expect("Failed to set change");

        let results = p4.edit(&[test_file]).changelist(cl).run().expect("Failed to edit file with changelist");
        assert_eq!(results.len(), 1);

        let opened = p4.opened(&[test_file]).run().expect("Failed to get opened files");
        assert_eq!(opened.len(), 1);
        assert!(opened[0].depot_file.ends_with("another_file"));
        assert_eq!(opened[0].change.number(), Some(cl));

        p4.revert(&[test_file]).run().expect("Failed to revert file");
    }
}
