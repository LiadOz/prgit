use serde::Deserialize;

#[derive(thiserror::Error, Debug)]
pub enum P4Error {
    #[error("Failed to execute command: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse JSON result: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Command Failed: {0} (severity: {1})")]
    CommandFailed(String, usize),
    #[error("Connection failed")]
    ConnectionFailed,
    #[error("Unknown error: {0}")]
    UnexpectedError(String),
    #[error("Usage error: {0}")]
    UsageError(String),
    #[error("Command failed with specific error: {0} (severity: {1})")]
    CommandSpecificError(String, usize),
}

#[derive(Deserialize, Debug)]
pub(crate) struct ErrorResponse {
    pub data: String,
    pub severity: usize,
}
