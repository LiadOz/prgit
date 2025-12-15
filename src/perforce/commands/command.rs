use serde::de::DeserializeOwned;
use crate::perforce::error::P4Error;

pub trait P4Command {
    type Response: DeserializeOwned;
    fn run(&self) -> Result<Self::Response, P4Error>;
}