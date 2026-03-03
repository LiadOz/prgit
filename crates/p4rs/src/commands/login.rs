use crate::commands::process::{CmdType, P4Command};
use crate::error::P4Error;
use crate::output::P4Output;
use crate::p4::P4;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct LoginStatus {
    pub user: String,
    pub expires_in_secs: u64,
}

pub struct Login<'p> {
    p4: &'p P4,
}

impl<'p> Login<'p> {
    pub fn new(p4: &'p P4) -> Self {
        Self { p4 }
    }

    /// Check the current ticket status via `p4 login -s`.
    /// Returns the user and expiry if the ticket is valid.
    pub fn status(&self) -> Result<LoginStatus, P4Error> {
        let output: P4Output<LoginResponse> = self.run()?;
        let resp = output.single()?;
        let expires_in_secs: u64 = resp.ticket_expiration.parse().map_err(|_| {
            P4Error::UnexpectedError("Failed to parse ticket expiry seconds".into())
        })?;
        Ok(LoginStatus {
            user: resp.user,
            expires_in_secs,
        })
    }
}

impl<'p> P4Command for Login<'p> {
    type Response = LoginResponse;

    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let mut process = self.p4.build_cmd("login", CmdType::Query);
        process.arg("-s");
        self.p4.run_parsed(process, false)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct LoginResponse {
    pub user: String,
    pub ticket_expiration: String,
}
