use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::{deserialize_unix_timestamp, FileAction, FileType, LineEnding};
use crate::error::P4Error;
use crate::p4::P4;
use chrono::{DateTime, Utc};
use derive_setters::Setters;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

pub struct PrintToFile {
    output_path: String,
}

pub struct PrintContent {
    offset: Option<usize>,
    size: Option<usize>,
}

pub struct Print<'p> {
    p4: &'p P4,
}

impl<'p> Print<'p> {
    pub fn new(p4: &'p P4) -> Self {
        Self { p4 }
    }

    pub fn to_file<'f>(
        &self,
        files: &'f [&'f str],
        output_path: &str,
    ) -> PrintCommand<'p, 'f, PrintToFile> {
        PrintCommand::new(
            self.p4,
            files,
            PrintToFile {
                output_path: output_path.to_string(),
            },
        )
    }

    pub fn content<'f>(&self, files: &'f [&'f str]) -> PrintCommand<'p, 'f, PrintContent> {
        PrintCommand::new(
            self.p4,
            files,
            PrintContent {
                offset: None,
                size: None,
            },
        )
    }
}

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct PrintCommand<'p, 'f, T> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    files: &'f [&'f str],
    #[setters(skip)]
    command_specific: T,
    #[setters(bool)]
    all_revisions: bool,
    max_files: Option<usize>,
    line_ending: Option<LineEnding>,
}

impl<'p, 'f, T> PrintCommand<'p, 'f, T> {
    pub fn new(p4: &'p P4, files: &'f [&'f str], command_specific: T) -> Self {
        Self {
            p4,
            files,
            command_specific,
            all_revisions: false,
            max_files: None,
            line_ending: None,
        }
    }

    fn build_base_args(&self, process: &mut crate::commands::process::P4Process) {
        process
            .flag(self.all_revisions, "-a")
            .opt("-m", &self.max_files)
            .opt("-L", &self.line_ending);
    }
}

impl<'p, 'f> PrintCommand<'p, 'f, PrintContent> {
    pub fn offset(mut self, offset: usize) -> Self {
        self.command_specific.offset = Some(offset);
        self
    }

    pub fn size(mut self, size: usize) -> Self {
        self.command_specific.size = Some(size);
        self
    }
}

impl<'p, 'f> P4Command for PrintCommand<'p, 'f, PrintToFile> {
    type Response = Vec<PrintFileInfo>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("print", CmdType::Query);
        self.build_base_args(&mut process);
        process
            .arg("-o")
            .arg(&self.command_specific.output_path)
            .args(self.files);
        let output = self.p4.run_command(process, None)?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let results: Vec<PrintFileInfo> = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<PrintFileInfo>(line).ok())
            .collect();

        Ok(results)
    }
}

impl<'p, 'f> P4Command for PrintCommand<'p, 'f, PrintContent> {
    type Response = Vec<PrintResult>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("print", CmdType::Query);
        self.build_base_args(&mut process);
        process
            .opt("--offset", &self.command_specific.offset)
            .opt("--size", &self.command_specific.size)
            .args(self.files);
        let output = self.p4.run_command(process, None)?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut results: Vec<PrintResult> = Vec::new();
        let mut current_info: Option<PrintFileInfo> = None;
        let mut current_data = String::new();

        for line in stdout.lines() {
            if let Ok(info) = serde_json::from_str::<PrintFileInfo>(line) {
                // Save previous file if any
                if let Some(prev_info) = current_info.take() {
                    results.push(PrintResult {
                        info: prev_info,
                        data: std::mem::take(&mut current_data),
                    });
                }
                current_info = Some(info);
            } else if let Ok(data_line) = serde_json::from_str::<PrintDataLine>(line) {
                current_data.push_str(&data_line.data);
            }
        }

        // Save last file
        if let Some(info) = current_info {
            results.push(PrintResult {
                info,
                data: current_data,
            });
        }

        Ok(results)
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintFileInfo {
    pub depot_file: String,
    pub action: FileAction,
    #[serde_as(as = "DisplayFromStr")]
    pub change: usize,
    #[serde_as(as = "DisplayFromStr")]
    pub rev: usize,
    #[serde(default)]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub file_size: Option<usize>,
    #[serde(deserialize_with = "deserialize_unix_timestamp")]
    pub time: DateTime<Utc>,
    #[serde(rename = "type")]
    #[serde_as(as = "DisplayFromStr")]
    pub file_type: FileType,
}

#[derive(Debug, Deserialize)]
struct PrintDataLine {
    data: String,
}

#[derive(Debug)]
pub struct PrintResult {
    pub info: PrintFileInfo,
    pub data: String,
}
