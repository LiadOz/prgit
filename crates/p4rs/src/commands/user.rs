use crate::commands::process::{CmdType, P4Command};
use crate::error::P4Error;
use crate::output::P4Output;
use crate::p4::P4;
use serde::Deserialize;

pub struct GetUser<'a> {
    name: &'a str,
}

pub struct User<'p> {
    p4: &'p P4,
}

impl<'p> User<'p> {
    pub fn new(p4: &'p P4) -> Self {
        Self { p4 }
    }

    pub fn get<'a>(&self, name: &'a str) -> UserCommand<'p, GetUser<'a>> {
        UserCommand::new(self.p4, GetUser { name })
    }
}

pub struct UserCommand<'p, T> {
    p4: &'p P4,
    command_specific: T,
}

impl<'p, T> UserCommand<'p, T> {
    pub fn new(p4: &'p P4, command_specific: T) -> Self {
        Self {
            p4,
            command_specific,
        }
    }
}

impl<'p, 'a> P4Command for UserCommand<'p, GetUser<'a>> {
    type Response = UserInfo;
    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let mut process = self.p4.build_cmd("user", CmdType::FormOutput);
        process.arg(self.command_specific.name);
        let json = self.p4.run(process)?;
        let result: UserInfo = serde_json::from_value(json)?;
        Ok(P4Output::new(vec![result], vec![]))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserInfo {
    pub user: String,
    pub email: String,
    pub full_name: String,
}
