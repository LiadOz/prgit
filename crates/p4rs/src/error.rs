use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P4Message {
    pub severity: u8,
    pub generic: u8,
    pub data: String,
}

impl P4Message {
    pub fn new(severity: u8, generic: u8, data: impl Into<String>) -> Self {
        Self {
            severity,
            generic,
            data: data.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity >= 3
    }

    pub fn is_warning(&self) -> bool {
        self.severity == 2
    }

    pub fn is_info(&self) -> bool {
        self.severity <= 1
    }
}

impl std::fmt::Display for P4Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.trim())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum P4Error {
    #[error("Failed to execute command: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse JSON result: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("P4 command error: {}", .errors.iter().map(|e| e.data.trim()).collect::<Vec<_>>().join("; "))]
    Command {
        errors: Vec<P4Message>,
        partial_results: Option<serde_json::Value>,
    },
    #[error("Connection failed")]
    ConnectionFailed,
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Unknown error: {0}")]
    UnexpectedError(String),
    #[error("Usage error: {0}")]
    UsageError(String),
}

const PERMISSION_PATTERNS: &[&str] = &["You don't have permission"];

fn is_permission_error(errors: &[P4Message]) -> bool {
    errors
        .iter()
        .any(|e| PERMISSION_PATTERNS.iter().any(|p| e.data.contains(p)))
}

fn format_errors(errors: &[P4Message]) -> String {
    errors
        .iter()
        .map(|e| e.data.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

impl P4Error {
    pub fn command(errors: Vec<P4Message>) -> Self {
        if is_permission_error(&errors) {
            return Self::PermissionDenied(format_errors(&errors));
        }
        Self::Command {
            errors,
            partial_results: None,
        }
    }

    pub fn command_with_partial(errors: Vec<P4Message>, partial: serde_json::Value) -> Self {
        if is_permission_error(&errors) {
            return Self::PermissionDenied(format_errors(&errors));
        }
        Self::Command {
            errors,
            partial_results: Some(partial),
        }
    }

    pub fn message(&self) -> String {
        match self {
            P4Error::Command { errors, .. } => format_errors(errors),
            _ => self.to_string(),
        }
    }

    pub fn contains(&self, pattern: &str) -> bool {
        match self {
            P4Error::Command { errors, .. } => errors.iter().any(|e| e.data.contains(pattern)),
            _ => self.to_string().contains(pattern),
        }
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct ErrorResponse {
    pub data: String,
    pub severity: usize,
    #[serde(default)]
    pub generic: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p4message_severity_classification() {
        assert!(P4Message::new(0, 0, "info").is_info());
        assert!(P4Message::new(1, 0, "info").is_info());
        assert!(P4Message::new(2, 0, "warning").is_warning());
        assert!(P4Message::new(3, 0, "error").is_error());
        assert!(P4Message::new(4, 0, "error").is_error());

        assert!(!P4Message::new(2, 0, "warning").is_info());
        assert!(!P4Message::new(3, 0, "error").is_warning());
    }

    #[test]
    fn test_p4message_display() {
        let msg = P4Message::new(3, 0, "  error message  ");
        assert_eq!(msg.to_string(), "error message");
    }

    #[test]
    fn test_p4error_message() {
        let err = P4Error::command(vec![
            P4Message::new(3, 0, "first error"),
            P4Message::new(3, 0, "second error"),
        ]);
        assert_eq!(err.message(), "first error\nsecond error");
    }

    #[test]
    fn test_p4error_contains() {
        let err = P4Error::command(vec![P4Message::new(3, 0, "file not found")]);
        assert!(err.contains("not found"));
        assert!(!err.contains("permission denied"));
    }

    #[test]
    fn test_command_detects_permission_denied() {
        let err = P4Error::command(vec![P4Message::new(
            3,
            0,
            "You don't have permission for this operation.",
        )]);
        assert!(matches!(err, P4Error::PermissionDenied(_)));

        let err = P4Error::command(vec![P4Message::new(3, 0, "file not found")]);
        assert!(matches!(err, P4Error::Command { .. }));
    }
}
