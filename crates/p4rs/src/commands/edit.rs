use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::FileType;
use crate::error::P4Error;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct EditCommand<'p, 'f> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
    changelist: Option<usize>,
    file_type: Option<FileType>,
    #[setters(bool)]
    keep: bool,
    #[setters(bool)]
    preview: bool,
}

impl<'p, 'f> EditCommand<'p, 'f> {
    pub fn new(p4: &'p P4, files: &'f [&'f str]) -> Self {
        Self {
            p4,
            files,
            changelist: None,
            file_type: None,
            keep: false,
            preview: false,
        }
    }
}

impl<'p, 'f> P4Command for EditCommand<'p, 'f> {
    type Response = Vec<EditResult>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("edit", CmdType::Query);
        process
            .opt("-c", &self.changelist)
            .opt("-t", &self.file_type)
            .flag(self.keep, "-k")
            .flag(self.preview, "-n")
            .args(self.files);
        let json = self.p4.run_multi_line(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EditAction {
    Edit,
    Add,
}

impl std::fmt::Display for EditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditAction::Edit => write!(f, "edit"),
            EditAction::Add => write!(f, "add"),
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditResult {
    pub depot_file: String,
    pub client_file: String,
    pub work_rev: String,
    pub action: EditAction,
    #[serde(rename = "type")]
    #[serde_as(as = "DisplayFromStr")]
    pub file_type: FileType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_action_display() {
        assert_eq!(EditAction::Edit.to_string(), "edit");
        assert_eq!(EditAction::Add.to_string(), "add");
    }
}
