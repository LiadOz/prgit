use std::sync::LazyLock;
use testcontainers::{core::WaitFor, runners::SyncRunner, GenericImage};
use p4rs::{P4, P4Command, ClientSpec, ClientMapping, ChangeStatus};
use tempfile::TempDir;

pub struct TestClient {
    pub p4: P4,
    client_name: String,
}

impl TestClient {
    pub fn new(p4: P4) -> Self {
        let test_name = format!("test_{}", uuid::Uuid::new_v4());
        let tmp_dir = TempDir::new().expect("Failed to create temp dir");
        let client_spec = ClientSpec::new(
            &test_name,
            tmp_dir.path().to_str().unwrap(),
            vec![ClientMapping::new(format!("//depot/{test_name}/..."), format!("//{test_name}/..."))],
        );
        p4.client().set(&client_spec).run().expect("Failed to create client");
        Self { p4: p4.client_name(&test_name), client_name: test_name }
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        log::debug!("Cleaning up test client {}", self.client_name);
        let pending_changes = self.p4.changes(&[]).status(ChangeStatus::Pending).run().expect("Failed to get changes");
        log::debug!("Deleting {} pending changes", pending_changes.len());
        for change in pending_changes {
            self.p4.change().delete(change.change).run().expect("Failed to delete change");
        }
        log::debug!("Reverting opened files");
        if let Ok(opened) = self.p4.opened(&["//..."]).run() {
            if opened.len() > 0 {
                self.p4.revert(&["//..."]).run().expect("Failed to revert all");
            }
        }
        log::debug!("Deleting client {}", self.client_name);
        self.p4.client().delete(&self.client_name).run().expect("Failed to delete client");
    }
}

pub struct P4Server {
    pub port: u16,
    _container: testcontainers::Container<GenericImage>,
}

impl P4Server {
    pub fn start() -> Self {
        let container = GenericImage::new("p4d-server", "latest")
            .with_exposed_port(1666.into())
            .with_wait_for(WaitFor::seconds(2))
            .start()
            .expect("Failed to start P4 server");

        let port = container.get_host_port_ipv4(1666).unwrap();
        Self { port, _container: container }
    }

    pub fn p4(&self) -> P4 {
        P4::new().port(format!("localhost:{}", self.port))
    }

    pub fn test_client(&self) -> TestClient {
        TestClient::new(self.p4())
    }
}

pub static SERVER: LazyLock<P4Server> = LazyLock::new(P4Server::start);