use serde::Deserialize;

#[derive(thiserror::Error, Debug)]
pub enum P4Error {
    #[error("Failed to execute command: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse JSON result: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Command Failed: {0}")]
    CommandFailed(String),
}

#[derive(Deserialize, Debug)]
pub(crate) struct ErrorResponse {
    pub data: String,
}

