use crate::commands::process::{CmdType, P4Command};
use crate::error::P4Error;
use crate::p4::P4;

pub struct SubmitCommand<'p> {
    p4: &'p P4,
    changelist: usize,
}

impl<'p> SubmitCommand<'p> {
    pub fn new(p4: &'p P4, changelist: usize) -> Self {
        Self { p4, changelist }
    }
}

impl<'p> P4Command for SubmitCommand<'p> {
    type Response = ();
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("submit", CmdType::Query);
        process.arg("-c").arg(self.changelist.to_string());
        self.p4.run_command(process, None)?;
        Ok(())
    }
}

