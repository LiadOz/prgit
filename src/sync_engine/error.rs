use std::fmt;

#[derive(Debug)]
pub enum SyncError {
    IoError(std::io::Error),
    InvalidState(String),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::IoError(e) => write!(f, "IO error: {}", e),
            SyncError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
        }
    }
}

impl std::error::Error for SyncError {}