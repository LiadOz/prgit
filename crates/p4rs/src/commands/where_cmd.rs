use crate::commands::process::{CmdType, P4Command};
use crate::error::P4Error;
use crate::output::P4Output;
use crate::p4::P4;
use serde::Deserialize;

pub struct WhereCommand<'p, 'f> {
    p4: &'p P4,
    files: &'f [&'f str],
}

impl<'p, 'f> WhereCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self { p4, files }
    }
}

impl<'p, 'f> P4Command for WhereCommand<'p, 'f> {
    type Response = WhereResult;

    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let mut process = self.p4.build_cmd("where", CmdType::Query);
        process.args(self.files);
        self.p4.run_parsed(process, true)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhereResult {
    pub depot_file: String,
    pub client_file: String,
    pub path: String,
}
