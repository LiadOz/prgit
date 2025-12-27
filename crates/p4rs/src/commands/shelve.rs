use crate::commands::process::{CmdType, P4Command, P4Process};
use crate::commands::types::GenericResponse;
use crate::error::P4Error;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelveResult {
    pub action: String,
    #[serde(default)]
    pub change: Option<String>,
    pub depot_file: String,
    pub rev: String,
}

pub struct SetShelve {
    changelist: usize,
    replace: bool,
}

pub struct DeleteShelve {
    changelist: usize,
}

pub struct Shelve<'p> {
    p4: &'p P4,
}

impl<'p> Shelve<'p> {
    pub fn new(p4: &'p P4) -> Self {
        Self { p4 }
    }

    pub fn set(&self, changelist: usize) -> ShelveCommand<'p, SetShelve> {
        ShelveCommand::new(
            self.p4,
            SetShelve {
                changelist,
                replace: false,
            },
        )
    }

    pub fn delete(&self, changelist: usize) -> ShelveCommand<'p, DeleteShelve> {
        ShelveCommand::new(self.p4, DeleteShelve { changelist })
    }
}

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct ShelveCommand<'p, T> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    command_specific: T,
    #[setters(bool)]
    force: bool,
}

impl<'p, T> ShelveCommand<'p, T> {
    pub fn new(p4: &'p P4, command_specific: T) -> Self {
        Self {
            p4,
            command_specific,
            force: false,
        }
    }

    fn build_process(&self, cmd_type: CmdType) -> P4Process {
        let mut process = self.p4.build_cmd("shelve", cmd_type);
        process.flag(self.force, "-f");
        process
    }
}

impl<'p> ShelveCommand<'p, SetShelve> {
    pub fn replace(mut self) -> Self {
        self.command_specific.replace = true;
        self
    }
}

impl<'p> P4Command for ShelveCommand<'p, SetShelve> {
    type Response = Vec<ShelveResult>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.build_process(CmdType::Query);
        process.flag(self.command_specific.replace, "-r");
        process.arg("-c");
        process.arg(self.command_specific.changelist.to_string());
        let json = self.p4.run_multi_line(process)?;
        let response: Vec<ShelveResult> = serde_json::from_value(json)?;
        Ok(response)
    }
}

impl<'p> P4Command for ShelveCommand<'p, DeleteShelve> {
    type Response = GenericResponse;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.build_process(CmdType::Query);
        process.arg("-d");
        process.arg("-c");
        process.arg(self.command_specific.changelist.to_string());
        let json = self.p4.run(process)?;
        let response: GenericResponse = serde_json::from_value(json)?;
        Ok(response)
    }
}
