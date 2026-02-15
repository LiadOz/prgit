use crate::commands::process::{CmdType, P4Command};
use crate::commands::types::extract_numbered;
use crate::error::P4Error;
use crate::output::P4Output;
use crate::p4::P4;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    List,
    Read,
    Open,
    Write,
    Admin,
    Super,
    Review,
}

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessLevel::List => write!(f, "list"),
            AccessLevel::Read => write!(f, "read"),
            AccessLevel::Open => write!(f, "open"),
            AccessLevel::Write => write!(f, "write"),
            AccessLevel::Admin => write!(f, "admin"),
            AccessLevel::Super => write!(f, "super"),
            AccessLevel::Review => write!(f, "review"),
        }
    }
}

impl std::str::FromStr for AccessLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "list" => Ok(AccessLevel::List),
            "read" => Ok(AccessLevel::Read),
            "open" => Ok(AccessLevel::Open),
            "write" => Ok(AccessLevel::Write),
            "admin" => Ok(AccessLevel::Admin),
            "super" => Ok(AccessLevel::Super),
            "review" => Ok(AccessLevel::Review),
            _ => Err(format!("unknown access level: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionKind {
    User,
    Group,
}

impl std::fmt::Display for ProtectionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtectionKind::User => write!(f, "user"),
            ProtectionKind::Group => write!(f, "group"),
        }
    }
}

impl std::str::FromStr for ProtectionKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(ProtectionKind::User),
            "group" => Ok(ProtectionKind::Group),
            _ => Err(format!("unknown protection kind: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protection {
    pub access: AccessLevel,
    pub kind: ProtectionKind,
    pub name: String,
    pub host: String,
    pub path: String,
}

impl Protection {
    pub fn new(
        access: AccessLevel,
        kind: ProtectionKind,
        name: impl Into<String>,
        host: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            access,
            kind,
            name: name.into(),
            host: host.into(),
            path: path.into(),
        }
    }

    pub fn super_user(name: impl Into<String>, host: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(AccessLevel::Super, ProtectionKind::User, name, host, path)
    }

    pub fn write_user(name: impl Into<String>, host: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(AccessLevel::Write, ProtectionKind::User, name, host, path)
    }

    pub fn read_user(name: impl Into<String>, host: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(AccessLevel::Read, ProtectionKind::User, name, host, path)
    }
}

impl std::fmt::Display for Protection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {} {} {}", self.access, self.kind, self.name, self.host, self.path)
    }
}

impl std::str::FromStr for Protection {
    type Err = P4Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(P4Error::UnexpectedError(format!("invalid protection line: {}", s)));
        }
        Ok(Self {
            access: parts[0].parse().map_err(P4Error::UnexpectedError)?,
            kind: parts[1].parse().map_err(P4Error::UnexpectedError)?,
            name: parts[2].to_string(),
            host: parts[3].to_string(),
            path: parts[4].to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectionTable {
    pub protections: Vec<Protection>,
}

impl ProtectionTable {
    pub fn new(protections: Vec<Protection>) -> Self {
        Self { protections }
    }
}

impl std::fmt::Display for ProtectionTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Protections:")?;
        for protection in &self.protections {
            writeln!(f, "\t{}", protection)?;
        }
        Ok(())
    }
}

pub struct GetProtect;
pub struct SetProtect<'s> {
    table: &'s ProtectionTable,
}

pub struct Protect<'p> {
    p4: &'p P4,
}

impl<'p> Protect<'p> {
    pub fn new(p4: &'p P4) -> Self {
        Self { p4 }
    }

    pub fn get(&self) -> ProtectCommand<'p, GetProtect> {
        ProtectCommand::new(self.p4, GetProtect)
    }

    pub fn set<'s>(&self, table: &'s ProtectionTable) -> ProtectCommand<'p, SetProtect<'s>> {
        ProtectCommand::new(self.p4, SetProtect { table })
    }
}

pub struct ProtectCommand<'p, T> {
    p4: &'p P4,
    command_specific: T,
}

impl<'p, T> ProtectCommand<'p, T> {
    pub fn new(p4: &'p P4, command_specific: T) -> Self {
        Self { p4, command_specific }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ProtectionTableRaw {
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

impl<'p> P4Command for ProtectCommand<'p, GetProtect> {
    type Response = ProtectionTable;
    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let process = self.p4.build_cmd("protect", CmdType::FormOutput);
        let json = self.p4.run(process)?;
        let raw: ProtectionTableRaw = serde_json::from_value(json)?;
        let protections: Vec<Protection> = extract_numbered(&raw.extra, "Protections");
        Ok(P4Output::new(vec![ProtectionTable::new(protections)], vec![]))
    }
}

impl<'p, 's> P4Command for ProtectCommand<'p, SetProtect<'s>> {
    type Response = String;
    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let process = self.p4.build_cmd("protect", CmdType::FormInput);
        let stdin_data = self.command_specific.table.to_string();
        let output = self.p4.run_command(process, Some(&stdin_data))?;
        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(P4Output::new(vec![result], vec![]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_level_display() {
        assert_eq!(AccessLevel::Super.to_string(), "super");
        assert_eq!(AccessLevel::Write.to_string(), "write");
        assert_eq!(AccessLevel::Read.to_string(), "read");
    }

    #[test]
    fn test_access_level_from_str() {
        assert_eq!("super".parse::<AccessLevel>().unwrap(), AccessLevel::Super);
        assert_eq!("write".parse::<AccessLevel>().unwrap(), AccessLevel::Write);
        assert_eq!("read".parse::<AccessLevel>().unwrap(), AccessLevel::Read);
        assert!("invalid".parse::<AccessLevel>().is_err());
    }

    #[test]
    fn test_protection_kind_display() {
        assert_eq!(ProtectionKind::User.to_string(), "user");
        assert_eq!(ProtectionKind::Group.to_string(), "group");
    }

    #[test]
    fn test_protection_display() {
        let p = Protection::super_user("admin", "*", "//...");
        assert_eq!(p.to_string(), "super user admin * //...");
    }

    #[test]
    fn test_protection_from_str() {
        let p: Protection = "write user john * //depot/...".parse().unwrap();
        assert_eq!(p.access, AccessLevel::Write);
        assert_eq!(p.kind, ProtectionKind::User);
        assert_eq!(p.name, "john");
        assert_eq!(p.host, "*");
        assert_eq!(p.path, "//depot/...");
    }

    #[test]
    fn test_protection_table_display() {
        let table = ProtectionTable::new(vec![
            Protection::super_user("admin", "*", "//..."),
            Protection::write_user("*", "*", "//..."),
        ]);
        let expected = "Protections:\n\tsuper user admin * //...\n\twrite user * * //...\n";
        assert_eq!(table.to_string(), expected);
    }
}
