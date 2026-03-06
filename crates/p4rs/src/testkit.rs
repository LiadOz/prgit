use crate::changelist::ChangelistBuilder;
use crate::commands::protect::{Protection, ProtectionTable};
use crate::commands::types::FileType;
use crate::{ChangeStatus, ClientMapping, ClientSpec, P4Command, P4};
use std::fs;
use std::path::Path;
use std::sync::{LazyLock, Mutex};
use tempfile::TempDir;

#[cfg(all(feature = "testkit", not(feature = "testkit-local")))]
use testcontainers::{core::WaitFor, runners::SyncRunner, GenericImage};

#[cfg(all(feature = "testkit", not(feature = "testkit-local")))]
const DEFAULT_IMAGE: &str = "p4d-server";

pub const ADMIN_USER: &str = "admin";
pub const ADMIN_PASSWORD: &str = "admin123";

pub fn admin_p4(p4: &P4) -> P4 {
    p4.clone().p4_user(ADMIN_USER).password(ADMIN_PASSWORD)
}

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

    pub fn client_root(&self) -> &Path {
        self._tmp_dir.path()
    }

    pub fn changelist(&self, description: &str) -> ChangelistBuilder<'_> {
        ChangelistBuilder::new(&self.p4, self.client_root().to_path_buf(), description)
            .expect("Failed to create changelist")
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        let mut issues = Vec::new();
        if let Ok(opened) = self.p4.opened(&["//..."]).run() {
            if !opened.is_empty() {
                issues.push(format!("open files: {:?}", opened.iter().map(|f| &f.depot_file).collect::<Vec<_>>()));
            }
        }
        if let Ok(pending) = self.p4.changes(&[]).status(ChangeStatus::Pending).client(&self.client_name).run() {
            if !pending.is_empty() {
                issues.push(format!("pending changelists: {:?}", pending.iter().map(|c| c.change).collect::<Vec<_>>()));
            }
        }
        if !issues.is_empty() {
            let _ = self.p4.revert(&["//..."]).run();
            if let Ok(pending) = self.p4.changes(&[]).status(ChangeStatus::Pending).client(&self.client_name).run() {
                for change in pending {
                    let _ = self.p4.shelve().delete(change.change).run();
                    let _ = self.p4.change().delete(change.change).run();
                }
            }
            let _ = self.p4.client().delete(&self.client_name).run();
            panic!("Test left invalid state - {}", issues.join(", "));
        }
        self.p4.client().delete(&self.client_name).run().expect("Failed to delete client");
    }
}

fn write_file(root: &Path, path: &str, content: impl AsRef<[u8]>) {
    let full_path = root.join(path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create parent dirs");
    }
    fs::write(full_path, content).expect("Failed to write file");
}

impl<'p> ChangelistBuilder<'p> {
    pub fn add_file(mut self, path: &str, content: impl AsRef<[u8]>) -> Self {
        write_file(&self.root, path, content);
        self.add(path).expect("Failed to add file");
        self
    }

    pub fn add_file_with_opts(mut self, path: &str, content: impl AsRef<[u8]>, file_type: Option<FileType>) -> Self {
        write_file(&self.root, path, content);
        if let Some(ft) = file_type {
            self.add_with_type(path, ft).expect("Failed to add file");
        } else {
            self.add(path).expect("Failed to add file");
        }
        self
    }

    pub fn edit_file(self, path: &str, content: impl AsRef<[u8]>) -> Self {
        let full_path = self.root.join(path);
        let file_type = Self::determine_file_type(&full_path).expect("Failed to determine file type");
        self.p4.edit(&[full_path.to_string_lossy().as_ref()])
            .changelist(self.changelist)
            .file_type(file_type)
            .run()
            .expect("Failed to edit file");
        write_file(&self.root, path, content);
        self
    }

    pub fn edit_file_with_opts(self, path: &str, content: impl AsRef<[u8]>, file_type: Option<FileType>) -> Self {
        let full_path = self.root.join(path);
        let ft = file_type.unwrap_or_else(|| Self::determine_file_type(&full_path).expect("Failed to determine file type"));
        self.p4.edit(&[full_path.to_string_lossy().as_ref()])
            .changelist(self.changelist)
            .file_type(ft)
            .run()
            .expect("Failed to edit file");
        write_file(&self.root, path, content);
        self
    }

    pub fn delete_file(mut self, path: &str) -> Self {
        self.delete(path).expect("Failed to delete file");
        self
    }

    pub fn move_file_ext(mut self, from: &str, to: &str) -> Self {
        self.move_file(from, to).expect("Failed to move file");
        self
    }

    pub fn move_file_with_opts(mut self, from: &str, to: &str, content: Option<&[u8]>, file_type: Option<FileType>) -> Self {
        if let Some(ft) = file_type {
            self.move_file_with_type(from, to, ft).expect("Failed to move file");
        } else {
            self.move_file(from, to).expect("Failed to move file");
        }
        if let Some(c) = content {
            write_file(&self.root, to, c);
        }
        self
    }
}

#[cfg(all(feature = "testkit", not(feature = "testkit-local")))]
static CONTAINER_ID: Mutex<Option<String>> = Mutex::new(None);

#[cfg(all(feature = "testkit", not(feature = "testkit-local")))]
pub struct P4Server {
    pub port: u16,
    #[allow(dead_code)]
    container: Option<testcontainers::Container<GenericImage>>,
}

#[cfg(all(feature = "testkit", not(feature = "testkit-local")))]
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
            .with_wait_for(WaitFor::seconds(1))
            .start()
            .expect("Failed to start P4 server");

        let port = container.get_host_port_ipv4(1666).expect("Failed to get container port");
        Self::wait_for_p4_ready(port);
        Self::setup_protections(port);
        *CONTAINER_ID.lock().unwrap() = Some(container.id().to_string());
        Self { port, container: Some(container) }
    }
}

#[cfg(all(feature = "testkit", not(feature = "testkit-local")))]
extern "C" fn cleanup_container() {
    if let Some(id) = CONTAINER_ID.lock().ok().and_then(|mut g| g.take()) {
        log::info!("Cleaning up P4 test container {}", id);
        let _ = std::process::Command::new("docker").args(["rm", "-f", &id]).output();
    }
}

#[cfg(all(feature = "testkit", not(feature = "testkit-local")))]
pub static SERVER: LazyLock<P4Server> = LazyLock::new(|| {
    unsafe { libc::atexit(cleanup_container) };
    P4Server::start()
});

#[cfg(feature = "testkit-local")]
static P4D_PID: Mutex<Option<u32>> = Mutex::new(None);

#[cfg(feature = "testkit-local")]
pub struct P4Server {
    pub port: u16,
    #[allow(dead_code)]
    child: Option<std::process::Child>,
    #[allow(dead_code)]
    p4_root: Option<TempDir>,
}

#[cfg(feature = "testkit-local")]
impl P4Server {
    fn find_available_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("Failed to bind to ephemeral port");
        listener.local_addr().unwrap().port()
    }

    pub fn start() -> Self {
        if let Ok(port_str) = std::env::var("P4RS_TEST_PORT") {
            let port = port_str
                .parse()
                .expect("P4RS_TEST_PORT must be a valid port number");
            log::info!("Using external P4 server on port {}", port);
            return Self { port, child: None, p4_root: None };
        }

        let p4_root = TempDir::new().expect("Failed to create temp dir for p4d root");
        let port = Self::find_available_port();

        let child = std::process::Command::new("p4d")
            .arg("-r")
            .arg(p4_root.path())
            .arg("-p")
            .arg(format!("localhost:{}", port))
            .arg("-q")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to start p4d - is it installed and in PATH?");

        log::info!("Started local p4d on port {} (pid {})", port, child.id());
        *P4D_PID.lock().unwrap() = Some(child.id());

        Self::wait_for_p4_ready(port);
        Self::setup_protections(port);
        Self { port, child: Some(child), p4_root: Some(p4_root) }
    }
}

#[cfg(feature = "testkit-local")]
impl Drop for P4Server {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            log::info!("Stopping local p4d (pid {})", child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(feature = "testkit-local")]
extern "C" fn cleanup_p4d() {
    if let Some(pid) = P4D_PID.lock().ok().and_then(|mut g| g.take()) {
        log::info!("Cleaning up local p4d process {}", pid);
        unsafe { libc::kill(pid as i32, libc::SIGTERM); }
    }
}

#[cfg(feature = "testkit-local")]
pub static SERVER: LazyLock<P4Server> = LazyLock::new(|| {
    unsafe { libc::atexit(cleanup_p4d) };
    P4Server::start()
});

impl P4Server {
    fn wait_for_p4_ready(port: u16) {
        let p4 = P4::new().port(format!("localhost:{}", port));
        for attempt in 0..30 {
            if p4.info().short().run().is_ok() {
                log::debug!("P4 server ready after {} attempts", attempt + 1);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!("P4 server failed to become ready after 3 seconds");
    }

    fn setup_protections(port: u16) {
        let admin = P4::new()
            .port(format!("localhost:{}", port))
            .p4_user(ADMIN_USER)
            .password(ADMIN_PASSWORD);
        let table = ProtectionTable::new(vec![
            Protection::super_user(ADMIN_USER, "*", "//..."),
            Protection::write_user("*", "*", "//..."),
        ]);
        admin
            .protect()
            .set(&table)
            .run()
            .expect("Failed to setup protections");
        log::info!("Protections configured: admin has super access, others have write access");
    }

    pub fn p4(&self) -> P4 {
        P4::new().port(format!("localhost:{}", self.port))
    }

    pub fn admin_p4(&self) -> P4 {
        admin_p4(&self.p4())
    }

    pub fn test_client(&self) -> TestClient {
        TestClient::new(self.p4())
    }
}
