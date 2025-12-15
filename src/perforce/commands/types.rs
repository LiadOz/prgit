use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeStatus {
    Pending,
    Submitted,
    Shelved,
}

impl std::fmt::Display for ChangeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ChangeStatus::Pending => "pending",
            ChangeStatus::Submitted => "submitted",
            ChangeStatus::Shelved => "shelved",
        };
        f.write_str(s)
    }
}