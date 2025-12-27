use std::sync::LazyLock;
use testcontainers::{core::WaitFor, runners::SyncRunner, GenericImage};
use p4rs::P4;

pub struct P4Server {
    pub p4: P4,
    pub port: u16,
    _container: testcontainers::Container<GenericImage>,
}

impl P4Server {
    pub fn start() -> Self {
        let container = GenericImage::new("p4d-server", "latest")
            .with_exposed_port(1666.into())
            .with_wait_for(WaitFor::seconds(2))
            .start()
            .unwrap();

        let port = container.get_host_port_ipv4(1666).unwrap();
        let p4 = P4::new().port(format!("localhost:{port}"));

        Self { p4, port, _container: container }
    }
}

pub static SERVER: LazyLock<P4Server> = LazyLock::new(P4Server::start);

