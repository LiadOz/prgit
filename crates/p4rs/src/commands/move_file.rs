use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::{FileAction, FileType};
use crate::error::P4Error;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct MoveCommand<'p> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    from: &'p str,
    #[setters(skip)]
    to: &'p str,
    changelist: Option<usize>,
    file_type: Option<FileType>,
    #[setters(bool)]
    preview: bool,
    #[setters(bool)]
    force: bool,
}

impl<'p> MoveCommand<'p> {
    pub fn new(p4: &'p P4, from: &'p str, to: &'p str) -> Self {
        Self {
            p4,
            from,
            to,
            changelist: None,
            file_type: None,
            preview: false,
            force: false,
        }
    }
}

impl<'p> P4Command for MoveCommand<'p> {
    type Response = Vec<MoveResult>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("move", CmdType::Query);
        process
            .opt("-c", &self.changelist)
            .opt("-t", &self.file_type)
            .flag(self.preview, "-n")
            .flag(self.force, "-f")
            .arg(self.from)
            .arg(self.to);
        let json = self.p4.run_multi_line(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveResult {
    pub depot_file: String,
    pub client_file: String,
    pub action: FileAction,
    #[serde(rename = "type", deserialize_with = "deserialize_file_type")]
    pub file_type: FileType,
}

fn deserialize_file_type<'de, D>(deserializer: D) -> Result<FileType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    // P4 sometimes returns numeric type codes like "000" for move operations.
    // Default to text when we can't parse the type.
    Ok(s.and_then(|s| s.parse().ok()).unwrap_or_else(FileType::text))
}

