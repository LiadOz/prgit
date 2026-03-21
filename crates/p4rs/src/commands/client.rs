use crate::commands::process::{CmdType, P4Command, P4Process};
use crate::commands::types::{extract_numbered, GenericResponse, LineEnding};
use crate::error::P4Error;
use crate::output::P4Output;
use crate::p4::P4;
use derive_setters::Setters;
use serde::Deserialize;
use std::collections::HashMap;

pub struct SetClient<'s> {
    client_spec: &'s ClientSpec,
}

pub struct GetClient<'s> {
    client_name: Option<&'s str>,
}

pub struct DeleteClient<'s> {
    client_name: &'s str,
}

pub struct Client<'p> {
    p4: &'p P4,
}

impl<'p> Client<'p> {
    pub fn new(p4: &'p P4) -> Self {
        Self { p4 }
    }

    pub fn get<'s>(&self, client_name: Option<&'s str>) -> ClientCommand<'p, GetClient<'s>> {
        ClientCommand::new(self.p4, GetClient { client_name })
    }

    pub fn set<'s>(&self, client_spec: &'s ClientSpec) -> ClientCommand<'p, SetClient<'s>> {
        ClientCommand::new(self.p4, SetClient { client_spec })
    }

    pub fn delete<'s>(&self, client_name: &'s str) -> ClientCommand<'p, DeleteClient<'s>> {
        ClientCommand::new(self.p4, DeleteClient { client_name })
    }
}

#[derive(Setters)]
#[setters(into, strip_option)]
pub struct ClientCommand<'p, T> {
    #[setters(skip)]
    p4: &'p P4,
    #[setters(skip)]
    command_specific: T,
    #[setters(bool)]
    force: bool,
}

impl<'p, T> ClientCommand<'p, T> {
    pub fn new(p4: &'p P4, command_specific: T) -> Self {
        Self {
            p4,
            command_specific,
            force: false,
        }
    }

    fn build_process(&self, cmd_type: CmdType) -> P4Process {
        let mut process = self.p4.build_cmd("client", cmd_type);
        process.flag(self.force, "-f");
        process
    }
}

impl<'p, 's> P4Command for ClientCommand<'p, GetClient<'s>> {
    type Response = ClientSpec;
    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let mut process = self.build_process(CmdType::FormOutput);
        if let Some(name) = self.command_specific.client_name {
            process.arg(name);
        }
        let json = self.p4.run(process)?;
        let raw: ClientSpecRaw = serde_json::from_value(json)?;
        Ok(P4Output::new(vec![raw.into()], vec![]))
    }
}

impl<'p, 's> P4Command for ClientCommand<'p, SetClient<'s>> {
    type Response = String;
    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let process = self.build_process(CmdType::FormInput);
        let stdin_data = self.command_specific.client_spec.to_string();
        let output = self.p4.run_command(process, Some(&stdin_data))?;
        let result = String::from_utf8_lossy(&output.stdout);
        let client_name: String = result
            .split_whitespace()
            .nth(1)
            .ok_or(P4Error::UnexpectedError(format!(
                "unexpected output: {}",
                result
            )))?
            .parse()
            .map_err(|_| P4Error::UnexpectedError(format!("unexpected output: {}", result)))?;
        Ok(P4Output::new(vec![client_name], vec![]))
    }
}

impl<'p, 's> P4Command for ClientCommand<'p, DeleteClient<'s>> {
    type Response = GenericResponse;
    fn run(&self) -> Result<P4Output<Self::Response>, P4Error> {
        let mut process = self.build_process(CmdType::Query);
        process.arg("-d");
        process.flag(self.force, "-f");
        process.arg(self.command_specific.client_name);
        let json = self.p4.run(process)?;
        let response: GenericResponse = serde_json::from_value(json)?;
        let deleted_re = regex::Regex::new(r"^Client .+ deleted\.$").expect("invalid regex");
        if !deleted_re.is_match(response.data.trim()) {
            return Err(P4Error::command(vec![crate::P4Message::new(
                3,
                0,
                response.data,
            )]));
        }
        Ok(P4Output::new(vec![response], vec![]))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientMapping {
    pub depot: String,
    pub client: String,
}

impl ClientMapping {
    pub fn new(depot: impl Into<String>, client: impl Into<String>) -> Self {
        Self {
            depot: depot.into(),
            client: client.into(),
        }
    }
}

impl std::str::FromStr for ClientMapping {
    type Err = P4Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(P4Error::UnexpectedError(format!(
                "Invalid view mapping: {}",
                s
            )));
        }
        Ok(Self::new(parts[0], parts[1]))
    }
}

impl std::fmt::Display for ClientMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.depot, self.client)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ClientSpecRaw {
    client: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    description: Option<String>,
    root: String,
    #[serde(default)]
    options: Option<String>,
    #[serde(default)]
    submit_options: Option<String>,
    #[serde(default)]
    line_end: Option<String>,
    #[serde(default)]
    backup: Option<String>,
    #[serde(default, rename = "Type")]
    client_type: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Debug, Setters)]
#[setters(into, strip_option)]
pub struct ClientSpec {
    #[setters(skip)]
    pub client: String,
    pub owner: Option<String>,
    pub host: Option<String>,
    pub description: Option<String>,
    pub root: String,
    pub options: Option<String>,
    pub submit_options: Option<String>,
    pub line_end: Option<LineEnding>,
    pub backup: Option<String>,
    pub client_type: Option<String>,
    pub view: Vec<ClientMapping>,
}

impl From<ClientSpecRaw> for ClientSpec {
    fn from(raw: ClientSpecRaw) -> Self {
        Self {
            client: raw.client,
            owner: raw.owner,
            host: raw.host,
            description: raw.description,
            root: raw.root,
            options: raw.options,
            submit_options: raw.submit_options,
            line_end: raw.line_end.and_then(|s| s.parse().ok()),
            backup: raw.backup,
            client_type: raw.client_type,
            view: extract_numbered(&raw.extra, "View"),
        }
    }
}

impl ClientSpec {
    pub fn new(name: impl Into<String>, root: impl Into<String>, view: Vec<ClientMapping>) -> Self {
        Self {
            client: name.into(),
            owner: None,
            host: None,
            description: None,
            root: root.into(),
            options: None,
            submit_options: None,
            line_end: None,
            backup: None,
            client_type: None,
            view,
        }
    }
    pub fn new_with_default_mapping(
        name: impl Into<String>,
        root: impl Into<String>,
        depot_path: impl Into<String>,
    ) -> Self {
        let name = name.into();
        let client_path = format!("//{}/...", &name);
        Self::new(
            name,
            root,
            vec![ClientMapping::new(depot_path, client_path)],
        )
    }
}

impl std::fmt::Display for ClientSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Client: {}", self.client)?;
        if let Some(owner) = &self.owner {
            writeln!(f)?;
            writeln!(f, "Owner: {}", owner)?;
        }
        if let Some(host) = &self.host {
            writeln!(f)?;
            writeln!(f, "Host: {}", host)?;
        }
        if let Some(description) = &self.description {
            writeln!(f)?;
            writeln!(f, "Description:")?;
            for line in description.lines() {
                writeln!(f, "\t{}", line)?;
            }
        }
        writeln!(f)?;
        writeln!(f, "Root: {}", self.root)?;
        if let Some(options) = &self.options {
            writeln!(f)?;
            writeln!(f, "Options: {}", options)?;
        }
        if let Some(submit_options) = &self.submit_options {
            writeln!(f)?;
            writeln!(f, "SubmitOptions: {}", submit_options)?;
        }
        if let Some(line_end) = &self.line_end {
            writeln!(f)?;
            writeln!(f, "LineEnd: {}", line_end)?;
        }
        if !self.view.is_empty() {
            writeln!(f)?;
            writeln!(f, "View:")?;
            for mapping in self.view.iter() {
                writeln!(f, "\t{}", mapping)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_spec(json: &str) -> ClientSpec {
        let raw: ClientSpecRaw = serde_json::from_str(json).unwrap();
        raw.into()
    }

    #[test]
    fn test_client_spec_deserialize() {
        let json = r#"{"Access":"2025/12/26 19:26:10","Backup":"enable","Client":"dummy","Description":"Created by ozonzono.\n","Host":"DESKTOP-1B13Q6A","LineEnd":"local","Options":"noallwrite noclobber nocompress unlocked nomodtime normdir noaltsync","Owner":"ozonzono","Root":"/home/ozonzono/tools/perforce/clients/1","SubmitOptions":"submitunchanged","Type":"writeable","Update":"2025/12/26 19:26:10","View0":"//depot/... //dummy/...","View1":"//depot/b/... //dummy/..."}"#;
        let spec = parse_spec(json);
        assert_eq!(spec.client, "dummy");
        assert_eq!(spec.owner, Some("ozonzono".into()));
        assert_eq!(spec.host, Some("DESKTOP-1B13Q6A".into()));
        assert_eq!(spec.root, "/home/ozonzono/tools/perforce/clients/1");
        assert_eq!(spec.backup, Some("enable".into()));
        assert_eq!(spec.client_type, Some("writeable".into()));
        assert_eq!(spec.view.len(), 2);
        assert_eq!(
            spec.view,
            vec![
                ClientMapping::new("//depot/...", "//dummy/..."),
                ClientMapping::new("//depot/b/...", "//dummy/...")
            ]
        );
    }

    #[test]
    fn test_client_spec_display() {
        let spec = ClientSpec::new(
            "my-client",
            "/home/user/workspace",
            vec![
                ClientMapping::new("//depot/main/...", "//my-client/main/..."),
                ClientMapping::new("//depot/dev/...", "//my-client/dev/..."),
            ],
        )
        .owner("testuser")
        .host("myhost")
        .description("Test client")
        .options("noallwrite noclobber nocompress unlocked nomodtime normdir")
        .submit_options("submitunchanged")
        .line_end(LineEnding::Local);
        assert_eq!(
            spec.to_string(),
            "\
Client: my-client

Owner: testuser

Host: myhost

Description:
\tTest client

Root: /home/user/workspace

Options: noallwrite noclobber nocompress unlocked nomodtime normdir

SubmitOptions: submitunchanged

LineEnd: local

View:
\t//depot/main/... //my-client/main/...
\t//depot/dev/... //my-client/dev/...
"
        );
    }

    #[test]
    fn test_client_spec_minimal() {
        let spec = ClientSpec::new(
            "minimal-client",
            "/tmp/root",
            vec![ClientMapping::new("//depot/...", "//minimal-client/...")],
        );
        assert_eq!(
            spec.to_string(),
            "\
Client: minimal-client

Root: /tmp/root

View:
\t//depot/... //minimal-client/...
"
        );
    }

    #[test]
    fn test_client_mapping_display() {
        let mapping = ClientMapping::new("//depot/main/...", "//client/main/...");
        assert_eq!(mapping.to_string(), "//depot/main/... //client/main/...");
    }

    #[test]
    fn test_client_mapping_parse() {
        let mapping: ClientMapping = "//depot/... //client/...".parse().unwrap();
        assert_eq!(mapping, ClientMapping::new("//depot/...", "//client/..."));
    }

    #[test]
    fn test_client_spec_new_with_default_mapping() {
        let spec = ClientSpec::new_with_default_mapping(
            "my-client",
            "/home/user/workspace",
            "//depot/...",
        );
        assert_eq!(spec.view.len(), 1);
        assert_eq!(
            spec.view,
            vec![ClientMapping::new("//depot/...", "//my-client/...")]
        );
    }
}
