use crate::error::P4Error;
use std::ffi::OsStr;
use std::io::Write;
use std::process::{Output, Stdio};

#[derive(Debug, PartialEq, Eq)]
pub enum CmdType {
    FormInput,
    FormOutput,
    Query,
}

pub trait P4Command {
    type Response;
    fn run(&self) -> Result<Self::Response, P4Error>;
}

#[derive(Debug)]
pub struct P4Process {
    cmd: std::process::Command,
    pub(crate) cmd_type: CmdType,
}

impl P4Process {
    pub fn new(cmd: std::process::Command, cmd_type: CmdType) -> Self {
        Self { cmd, cmd_type }
    }

    pub fn flag(&mut self, condition: bool, flag: &str) -> &mut Self {
        if condition {
            self.cmd.arg(flag);
        }
        self
    }

    pub fn opt<T: ToString>(&mut self, flag: &str, value: &Option<T>) -> &mut Self {
        if let Some(v) = value {
            self.cmd.args([flag, &v.to_string()]);
        }
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.cmd.args(args);
        self
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.cmd.arg(arg);
        self
    }

    pub fn output(&mut self) -> std::io::Result<Output> {
        self.cmd.output()
    }

    pub fn run_with_stdin(&mut self, stdin_data: &str) -> std::io::Result<Output> {
        log::debug!("Writing stdin data: {:?}", stdin_data);
        let mut child = self
            .cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(stdin) = child.stdin.take() {
            std::io::BufWriter::new(stdin).write_all(stdin_data.as_bytes())?;
        }
        child.wait_with_output()
    }
}
