use crate::commands::process::{CmdType, P4Command};
use crate::error::P4Error;
use crate::p4::P4;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

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
    type Response = SubmitResult;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("submit", CmdType::Query);
        process.arg("-c").arg(self.changelist.to_string());
        let output = self.p4.run_command(process, None)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Ok(result) = serde_json::from_str::<SubmitResult>(line) {
                return Ok(result);
            }
        }
        Err(P4Error::UnexpectedError(
            "No submittedChange in response".to_string(),
        ))
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitResult {
    #[serde_as(as = "DisplayFromStr")]
    pub submitted_change: usize,
}
