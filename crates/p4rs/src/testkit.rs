use crate::commands::types::FileType;
use crate::{ChangeSpec, ChangeStatus, ChangeType, ClientMapping, ClientSpec, P4Command, P4};
use std::fs;
use std::sync::{LazyLock, Mutex};
use tempfile::TempDir;
use testcontainers::{core::WaitFor, runners::SyncRunner, GenericImage};

const DEFAULT_IMAGE: &str = "p4d-server";

pub struct TestClient {
    pub p4: P4,
    pub client_name: String,
    _tmp_dir: TempDir,
}

impl TestClient {
    pub fn new(p4: P4) -> Self {
        let test_name = format!("test_{}", uuid::Uuid::new_v4());
        let tmp_dir = TempDir::new().expect("Failed to create temp dir");
        let client_spec = ClientSpec::new(
            &test_name,
            tmp_dir.path().to_str().expect("Path is not valid UTF-8"),
            vec![ClientMapping::new(
                format!("//depot/{test_name}/..."),
                format!("//{test_name}/..."),
            )],
        );
        p4.client()
            .set(&client_spec)
            .run()
            .expect("Failed to create client");
        Self {
            p4: p4.client_name(&test_name),
            client_name: test_name,
            _tmp_dir: tmp_dir,
        }
    }

    pub fn client_root(&self) -> &std::path::Path {
        self._tmp_dir.path()
    }

    pub fn changelist(&self, description: &str) -> ChangelistBuilder<'_> {
        ChangelistBuilder::new(self, description)
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        log::debug!("Cleaning up test client {}", self.client_name);
        let pending_changes = self
            .p4
            .changes(&[])
            .status(ChangeStatus::Pending)
            .client(&self.client_name)
            .run()
            .expect("Failed to get changes");
        log::debug!("Deleting {} pending changes", pending_changes.len());
        for change in pending_changes {
            self.p4
                .change()
                .delete(change.change)
                .run()
                .expect("Failed to delete change");
        }
        log::debug!("Reverting opened files");
        if let Ok(opened) = self.p4.opened(&["//..."]).run() {
            if !opened.is_empty() {
                self.p4
                    .revert(&["//..."])
                    .run()
                    .expect("Failed to revert all");
            }
        }
        log::debug!("Deleting client {}", self.client_name);
        self.p4
            .client()
            .delete(&self.client_name)
            .run()
            .expect("Failed to delete client");
    }
}

/// A helper for setting up test changelists without boilerplate.
///
/// Use this builder for scaffolding/preconditions where you don't need to
/// inspect command outputs. For testing specific command behavior and verifying
/// return values, use the P4 commands directly.
///
/// ```ignore
/// // Builder: just need files to exist
/// test_client.changelist("Setup")
///     .add_file("foo.txt", b"content")
///     .add_file("bar.txt", b"more")
///     .submit();
///
/// // Direct P4 command: testing move behavior
/// let results = test_client.p4.move_file(from, to).run()?;
/// assert_eq!(results[0].action, FileAction::MoveAdd);
/// ```
pub struct ChangelistBuilder<'a> {
    client: &'a TestClient,
    pub changelist: usize,
}

impl<'a> ChangelistBuilder<'a> {
    pub fn new(client: &'a TestClient, description: &str) -> Self {
        let changelist = client
            .p4
            .change()
            .set(&ChangeSpec::new(ChangeType::New).description(description))
            .run()
            .expect("Failed to create changelist")
            .single()
            .expect("Expected single changelist");
        Self { client, changelist }
    }

    fn resolve_path(&self, path: &str) -> String {
        self.client
            .client_root()
            .join(path)
            .to_string_lossy()
            .into_owned()
    }

    fn write_file(&self, path: &str, content: impl AsRef<[u8]>) {
        let full_path = self.client.client_root().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dirs");
        }
        fs::write(full_path, content).expect("Failed to write file");
    }

    pub fn add_file(self, path: &str, content: impl AsRef<[u8]>) -> Self {
        self.add_file_with_opts(path, content, None)
    }

    pub fn add_file_with_opts(self, path: &str, content: impl AsRef<[u8]>, file_type: Option<FileType>) -> Self {
        self.write_file(path, content);
        let path_str = self.resolve_path(path);
        let files = [path_str.as_str()];
        let mut cmd = self.client.p4.add(&files).changelist(self.changelist);
        if let Some(ft) = file_type {
            cmd = cmd.file_type(ft);
        }
        cmd.run().expect("Failed to add file");
        self
    }

    pub fn edit_file(self, path: &str, content: impl AsRef<[u8]>) -> Self {
        self.edit_file_with_opts(path, content, None)
    }

    pub fn edit_file_with_opts(self, path: &str, content: impl AsRef<[u8]>, file_type: Option<FileType>) -> Self {
        let path_str = self.resolve_path(path);
        let files = [path_str.as_str()];
        let mut cmd = self.client.p4.edit(&files).changelist(self.changelist);
        if let Some(ft) = file_type {
            cmd = cmd.file_type(ft);
        }
        cmd.run().expect("Failed to edit file");
        self.write_file(path, content);
        self
    }

    pub fn delete_file(self, path: &str) -> Self {
        let path_str = self.resolve_path(path);
        let files = [path_str.as_str()];
        self.client.p4.delete(&files).changelist(self.changelist).run().expect("Failed to delete file");
        self
    }

    pub fn move_file(self, from: &str, to: &str) -> Self {
        self.move_file_with_opts(from, to, None, None)
    }

    pub fn move_file_with_opts(self, from: &str, to: &str, content: Option<&[u8]>, file_type: Option<FileType>) -> Self {
        let from_str = self.resolve_path(from);
        let to_str = self.resolve_path(to);
        // P4 requires the source file to be opened for edit before moving
        self.client.p4.edit(&[from_str.as_str()])
            .changelist(self.changelist)
            .run().expect("Failed to open file for edit before move");
        let mut cmd = self.client.p4.move_file(&from_str, &to_str).changelist(self.changelist);
        if let Some(ft) = file_type {
            cmd = cmd.file_type(ft);
        }
        cmd.run().expect("Failed to move file");
        if let Some(c) = content {
            self.write_file(to, c);
        }
        self
    }

    pub fn submit(self) -> crate::commands::submit::SubmitResult {
        self.client.p4.submit(self.changelist).run().expect("Failed to submit").single().expect("Expected submit result")
    }
}

static CONTAINER_ID: Mutex<Option<String>> = Mutex::new(None);

pub struct P4Server {
    pub port: u16,
    #[allow(dead_code)]
    container: Option<testcontainers::Container<GenericImage>>,
}

impl P4Server {
    pub fn start() -> Self {
        if let Ok(port_str) = std::env::var("P4RS_TEST_PORT") {
            let port = port_str
                .parse()
                .expect("P4RS_TEST_PORT must be a valid port number");
            log::info!("Using external P4 server on port {}", port);
            return Self { port, container: None };
        }

        let image = std::env::var("P4RS_TEST_IMAGE").unwrap_or_else(|_| DEFAULT_IMAGE.to_string());
        let container = GenericImage::new(image, "latest".to_string())
            .with_exposed_port(1666.into())
            .with_wait_for(WaitFor::seconds(2))
            .start()
            .expect("Failed to start P4 server");

        let port = container.get_host_port_ipv4(1666).expect("Failed to get container port");
        *CONTAINER_ID.lock().unwrap() = Some(container.id().to_string());
        Self { port, container: Some(container) }
    }

    pub fn p4(&self) -> P4 {
        P4::new().port(format!("localhost:{}", self.port))
    }

    pub fn test_client(&self) -> TestClient {
        TestClient::new(self.p4())
    }
}

extern "C" fn cleanup_container() {
    if let Some(id) = CONTAINER_ID.lock().ok().and_then(|mut g| g.take()) {
        log::info!("Cleaning up P4 test container {}", id);
        let _ = std::process::Command::new("docker").args(["rm", "-f", &id]).output();
    }
}

pub static SERVER: LazyLock<P4Server> = LazyLock::new(|| {
    unsafe { libc::atexit(cleanup_container) };
    P4Server::start()
});
