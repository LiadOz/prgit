use crate::commands::process::{CmdType, P4Command, P4Process};
use crate::commands::types::{deserialize_p4_date, ChangeStatus, FilesPrefix, NumberedVec};
use crate::error::P4Error;
use crate::p4::P4;
use chrono::{DateTime, Utc};
use derive_setters::Setters;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct ChangeCommand<'p, T> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    data: T,
    #[setters(bool)]
    force: bool,
}

impl<'p, T> ChangeCommand<'p, T> {
    pub fn new(p4: &'p P4, data: T) -> Self {
        Self {
            p4,
            data,
            force: false,
        }
    }

    fn build_process(&self, cmd_type: CmdType) -> P4Process {
        let mut process = self.p4.build_cmd("change", cmd_type);
        process.flag(self.force, "-f");
        process
    }
}

impl<'p> P4Command for ChangeCommand<'p, usize> {
    type Response = ChangeSpec;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.build_process(CmdType::FormOutput);
        process.arg(self.data.to_string());
        let json = self.p4.run(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

impl<'p, 's> P4Command for ChangeCommand<'p, &'s ChangeSpec> {
    type Response = usize;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let process = self.build_process(CmdType::FormInput);
        let stdin_data = self.data.to_string();
        let output = self.p4.run_command(process, Some(&stdin_data))?;
        let result = String::from_utf8_lossy(&output.stdout);
        let change: usize = result
            .split_whitespace()
            .nth(1)
            .ok_or(P4Error::UnexpectedError(format!(
                "unexpected output: {}",
                result
            )))?
            .parse()
            .map_err(|_| P4Error::UnexpectedError(format!("unexpected output: {}", result)))?;
        Ok(change)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChangeType {
    New,
    Number(usize),
    Default,
}

impl ChangeType {
    pub fn number(&self) -> Option<usize> {
        match self {
            ChangeType::Number(n) => Some(*n),
            _ => None,
        }
    }
}

impl std::str::FromStr for ChangeType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "new" => ChangeType::New,
            "default" => ChangeType::Default,
            n => ChangeType::Number(n.parse().unwrap_or(0)),
        })
    }
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::New => write!(f, "new"),
            ChangeType::Default => write!(f, "default"),
            ChangeType::Number(n) => write!(f, "{}", n),
        }
    }
}
#[serde_as]
#[derive(Debug, Deserialize, Setters)]
#[serde(rename_all = "PascalCase")]
#[setters(into, strip_option)]
pub struct ChangeSpec {
    #[setters(skip)]
    #[serde_as(as = "DisplayFromStr")]
    pub change: ChangeType,
    #[setters(skip)]
    #[serde(deserialize_with = "deserialize_p4_date")]
    pub date: DateTime<Utc>,
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[setters(skip)]
    pub status: ChangeStatus,
    pub description: String,
    #[serde(default, rename = "Type")]
    pub change_type: Option<String>,
    #[serde(flatten)]
    pub files: NumberedVec<FilesPrefix>,
}

impl ChangeSpec {
    pub fn new(change: ChangeType) -> Self {
        Self {
            change,
            date: DateTime::default(),
            client: None,
            user: None,
            status: ChangeStatus::Pending,
            description: String::new(),
            change_type: None,
            files: Default::default(),
        }
    }
}

impl std::fmt::Display for ChangeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Change: {}", self.change)?;
        if let Some(client) = &self.client {
            writeln!(f)?;
            writeln!(f, "Client: {}", client)?;
        }
        if let Some(user) = &self.user {
            writeln!(f)?;
            writeln!(f, "User:\t{}", user)?;
        }
        writeln!(f)?;
        writeln!(f, "Status: {}", self.status)?;
        writeln!(f)?;
        writeln!(f, "Description:")?;
        for line in self.description.lines() {
            writeln!(f, "\t{}", line)?;
        }
        if !self.files.is_empty() {
            writeln!(f)?;
            writeln!(f, "Files:")?;
            for file in self.files.iter() {
                writeln!(f, "\t{}", file)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_spec_single_line_description() {
        let spec = ChangeSpec::new(ChangeType::New)
            .client("my-client")
            .user("testuser")
            .description("Simple description");
        assert_eq!(
            spec.to_string(),
            "\
Change: new

Client: my-client

User:	testuser

Status: pending

Description:
	Simple description
"
        );
    }

    #[test]
    fn test_change_spec_multiline_description() {
        let spec = ChangeSpec::new(ChangeType::Number(42))
            .client("my-client")
            .user("testuser")
            .description("Line one\nLine two\nLine three");
        assert_eq!(
            spec.to_string(),
            "\
Change: 42

Client: my-client

User:	testuser

Status: pending

Description:
	Line one
	Line two
	Line three
"
        );
    }

    #[test]
    fn test_change_spec_with_files() {
        let json = r#"{"Change":"1","Date":"2025/12/17 18:38:12","Status":"submitted","Description":"Test","Files0":"//depot/file1.txt","Files1":"//depot/file2.txt"}"#;
        let spec: ChangeSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.files.len(), 2);
        assert_eq!(spec.files[0], "//depot/file1.txt");
        assert_eq!(spec.files[1], "//depot/file2.txt");
    }

    #[test]
    fn test_change_spec_empty_files() {
        let spec = ChangeSpec::new(ChangeType::New).description("No files");
        assert!(spec.files.is_empty());
    }

    fn parse_date(s: &str) -> DateTime<Utc> {
        chrono::NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M:%S")
            .unwrap()
            .and_utc()
    }

    #[test]
    fn test_change_spec_deserialize() {
        let json = r#"{
            "Change": "12345",
            "Date": "2025/12/17 18:38:12",
            "Client": "test-client",
            "User": "testuser",
            "Status": "pending",
            "Description": "Test description"
        }"#;
        let spec: ChangeSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.change, ChangeType::Number(12345));
        assert_eq!(spec.date, parse_date("2025/12/17 18:38:12"));
        assert_eq!(spec.client, Some("test-client".into()));
        assert_eq!(spec.status, ChangeStatus::Pending);
        assert!(spec.files.is_empty());
    }

    #[test]
    fn test_change_spec_deserialize_new() {
        let json = r#"{
            "Change": "new",
            "Date": "2025/12/17 18:38:12",
            "Client": "my-client",
            "User": "someuser",
            "Status": "pending",
            "Description": "New changelist"
        }"#;
        let spec: ChangeSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.change, ChangeType::New);
        assert_eq!(spec.client, Some("my-client".into()));
    }

    #[test]
    fn test_real_p4_output() {
        let json = r#"{"Change":"79","Client":"dummy","Date":"2025/12/26 19:48:25","Description":"c\n","Files0":"//depot/b/c","Status":"pending","Type":"public","User":"ozonzono"}"#;
        let spec: ChangeSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.change, ChangeType::Number(79));
        assert_eq!(spec.client, Some("dummy".into()));
        assert_eq!(spec.change_type, Some("public".into()));
        assert_eq!(spec.files.len(), 1);
        assert_eq!(spec.files[0], "//depot/b/c");
    }
}
