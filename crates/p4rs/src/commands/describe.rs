use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::{
    deserialize_unix_timestamp, ChangeListType, ChangeStatus, FileAction, FileType, NumberedFields,
};
use crate::error::P4Error;
use crate::output::P4Output;
use crate::p4::P4;
use chrono::{DateTime, Utc};
use derive_setters::Setters;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct DescribeCommand<'p> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    changelists: Vec<usize>,
    #[setters(bool)]
    short: bool,
    #[setters(bool)]
    shelved: bool,
    max_files: Option<usize>,
}

impl<'p> DescribeCommand<'p> {
    pub fn new(p4: &'p P4, changelists: &[usize]) -> Self {
        Self {
            p4,
            changelists: changelists.to_vec(),
            short: false,
            shelved: false,
            max_files: None,
        }
    }
}

impl<'p> P4Command for DescribeCommand<'p> {
    type Response = DescribeResult;

    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let mut process = self.p4.build_cmd("describe", CmdType::Query);
        process
            .flag(self.short, "-s")
            .flag(self.shelved, "-S")
            .opt("-m", &self.max_files);
        for cl in &self.changelists {
            process.arg(cl.to_string());
        }
        let output: P4Output<DescribeResultRaw> = self.p4.run_parsed(process, true)?;
        Ok(P4Output::new(
            output.results.into_iter().map(Into::into).collect(),
            output.warnings,
        ))
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DescribeResultRaw {
    #[serde_as(as = "DisplayFromStr")]
    change: usize,
    user: String,
    client: String,
    #[serde(deserialize_with = "deserialize_unix_timestamp")]
    time: DateTime<Utc>,
    desc: String,
    status: ChangeStatus,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    change_type: Option<ChangeListType>,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Debug)]
pub struct DescribeResult {
    pub change: usize,
    pub user: String,
    pub client: String,
    pub time: DateTime<Utc>,
    pub description: String,
    pub status: ChangeStatus,
    pub path: Option<String>,
    pub change_type: Option<ChangeListType>,
    pub files: Vec<DescribeFile>,
}

#[derive(Debug)]
pub struct DescribeFile {
    pub depot_file: String,
    pub action: FileAction,
    pub file_type: FileType,
    pub rev: Option<usize>,
    pub file_size: Option<usize>,
    pub digest: Option<String>,
}

impl From<DescribeResultRaw> for DescribeResult {
    fn from(raw: DescribeResultRaw) -> Self {
        let files = NumberedFields::new(&raw.extra).map_each("depotFile", |f| DescribeFile {
            depot_file: f.get("depotFile").unwrap_or_default(),
            action: f.get("action").unwrap_or(FileAction::Edit),
            file_type: f.get("type").unwrap_or(FileType::text()),
            rev: f.get("rev"),
            file_size: f.get("fileSize"),
            digest: f.get("digest"),
        });

        Self {
            change: raw.change,
            user: raw.user,
            client: raw.client,
            time: raw.time,
            description: raw.desc,
            status: raw.status,
            path: raw.path,
            change_type: raw.change_type,
            files,
        }
    }
}
