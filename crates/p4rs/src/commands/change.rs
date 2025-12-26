use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::{deserialize_p4_date, ChangeStatus};
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
}

impl<'p> P4Command for ChangeCommand<'p, usize> {
    type Response = ChangeSpec;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("change", CmdType::FormOutput);
        process.flag(self.force, "-f").arg(self.data.to_string());
        let json = self.p4.run(process)?;
        Ok(serde_json::from_value(json)?)
    }
}

impl<'p, 's> P4Command for ChangeCommand<'p, &'s ChangeSpec> {
    type Response = usize;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut process = self.p4.build_cmd("change", CmdType::FormInput);
        process.flag(self.force, "-f");
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
#[derive(Debug, PartialEq, Eq, Deserialize, Setters)]
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
    #[serde(default)]
    pub files: Vec<String>,
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
            files: Vec::new(),
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
            for file in &self.files {
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
    fn test_client_spec_single_line_description() {
        let spec = ChangeSpec {
            change: ChangeType::New,
            date: DateTime::default(),
            client: Some("my-client".into()),
            user: Some("testuser".into()),
            status: ChangeStatus::Pending,
            description: "Simple description".into(),
            files: vec![],
        };
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
    fn test_client_spec_multiline_description() {
        let spec = ChangeSpec {
            change: ChangeType::Number(42),
            date: DateTime::default(),
            client: Some("my-client".into()),
            user: Some("testuser".into()),
            status: ChangeStatus::Pending,
            description: "Line one\nLine two\nLine three".into(),
            files: vec![],
        };
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
    fn test_client_spec_with_files() {
        let spec = ChangeSpec {
            change: ChangeType::New,
            date: DateTime::default(),
            client: Some("my-client".into()),
            user: Some("testuser".into()),
            status: ChangeStatus::Submitted,
            description: "Test".into(),
            files: vec!["//depot/file1.txt".into(), "//depot/file2.txt".into()],
        };
        assert_eq!(
            spec.to_string(),
            "\
Change: new

Client: my-client

User:	testuser

Status: submitted

Description:
	Test

Files:
	//depot/file1.txt
	//depot/file2.txt
"
        );
    }

    #[test]
    fn test_client_spec_empty_files() {
        let spec = ChangeSpec {
            change: ChangeType::New,
            date: DateTime::default(),
            client: Some("my-client".into()),
            user: Some("testuser".into()),
            status: ChangeStatus::Pending,
            description: "No files".into(),
            files: vec![],
        };
        assert_eq!(
            spec.to_string(),
            "\
Change: new

Client: my-client

User:	testuser

Status: pending

Description:
	No files
"
        );
    }

    fn parse_date(s: &str) -> DateTime<Utc> {
        chrono::NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M:%S")
            .unwrap()
            .and_utc()
    }

    #[test]
    fn test_client_spec_deserialize() {
        let json = r#"{
            "Change": "12345",
            "Date": "2025/12/17 18:38:12",
            "Client": "test-client",
            "User": "testuser",
            "Status": "pending",
            "Description": "Test description"
        }"#;
        let spec: ChangeSpec = serde_json::from_str(json).unwrap();
        let expected = ChangeSpec {
            change: ChangeType::Number(12345),
            date: parse_date("2025/12/17 18:38:12"),
            client: Some("test-client".into()),
            user: Some("testuser".into()),
            status: ChangeStatus::Pending,
            description: "Test description".into(),
            files: vec![],
        };
        assert_eq!(spec, expected);
    }

    #[test]
    fn test_client_spec_deserialize_new_change() {
        let json = r#"{
            "Change": "new",
            "Date": "2025/12/17 18:38:12",
            "Client": "my-client",
            "User": "someuser",
            "Status": "pending",
            "Description": "New changelist"
        }"#;
        let spec: ChangeSpec = serde_json::from_str(json).unwrap();
        let expected = ChangeSpec {
            change: ChangeType::New,
            date: parse_date("2025/12/17 18:38:12"),
            client: Some("my-client".into()),
            user: Some("someuser".into()),
            status: ChangeStatus::Pending,
            description: "New changelist".into(),
            files: vec![],
        };
        assert_eq!(spec, expected);
    }
}
