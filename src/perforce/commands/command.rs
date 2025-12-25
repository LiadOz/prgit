use crate::perforce::error::P4Error;

#[derive(Debug)]
pub(crate) enum CmdType {
    FormInput,
    FormOutput,
    Query,
}

pub trait P4Command {
    type Response;
    fn run(&self) -> Result<Self::Response, P4Error>;
}


#[derive(Debug)]
pub(crate) struct P4Process {
    pub cmd: std::process::Command,
    pub cmd_type: CmdType,
}