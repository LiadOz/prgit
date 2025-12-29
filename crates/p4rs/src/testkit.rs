use crate::{ChangeStatus, ClientMapping, ClientSpec, P4Command, P4};
use std::sync::LazyLock;
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

#[allow(dead_code)]
enum ContainerMode {
    External(u16),
    Managed(Box<testcontainers::Container<GenericImage>>),
}

pub struct P4Server {
    pub port: u16,
    _container: ContainerMode,
}

impl P4Server {
    pub fn start() -> Self {
        if let Ok(port_str) = std::env::var("P4RS_TEST_PORT") {
            let port = port_str
                .parse()
                .expect("P4RS_TEST_PORT must be a valid port number");
            log::info!("Using external P4 server on port {}", port);
            return Self {
                port,
                _container: ContainerMode::External(port),
            };
        }

        let image = std::env::var("P4RS_TEST_IMAGE").unwrap_or_else(|_| DEFAULT_IMAGE.to_string());
        let container = GenericImage::new(image, "latest".to_string())
            .with_exposed_port(1666.into())
            .with_wait_for(WaitFor::seconds(2))
            .start()
            .expect("Failed to start P4 server");

        let port = container
            .get_host_port_ipv4(1666)
            .expect("Failed to get container port");
        Self {
            port,
            _container: ContainerMode::Managed(Box::new(container)),
        }
    }

    pub fn p4(&self) -> P4 {
        P4::new().port(format!("localhost:{}", self.port))
    }

    pub fn test_client(&self) -> TestClient {
        TestClient::new(self.p4())
    }
}

pub static SERVER: LazyLock<P4Server> = LazyLock::new(P4Server::start);
