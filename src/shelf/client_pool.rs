use std::path::PathBuf;

use p4rs::{ClientMapping, ClientSpec, P4, P4Command, P4Error};
use thiserror::Error;

use crate::cabinet::{PrgitClient, ShelveConfig};

#[derive(Debug)]
pub enum ClientLeaseType {
    Pooled,
    Temporary,
}

pub struct ClientLease<'a> {
    prgit_client: &'a PrgitClient<'a>,
    p4: P4,
    pub client_name: String,
    client_root: PathBuf,
    lease_type: ClientLeaseType,
}

impl<'a> ClientLease<'a> {
    fn new_pooled(prgit_client: &'a PrgitClient<'a>, p4: P4, client_name: String, client_root: PathBuf) -> Self {
        Self {
            prgit_client,
            p4,
            client_name,
            client_root,
            lease_type: ClientLeaseType::Pooled,
        }
    }

    fn new_temporary(prgit_client: &'a PrgitClient<'a>, p4: P4, client_name: String, client_root: PathBuf) -> Self {
        Self {
            prgit_client,
            p4,
            client_name,
            client_root,
            lease_type: ClientLeaseType::Temporary,
        }
    }

    pub fn p4(&self) -> &P4 {
        &self.p4
    }

    pub fn client_root(&self) -> &PathBuf {
        &self.client_root
    }
}

impl Drop for ClientLease<'_> {
    fn drop(&mut self) {
        match self.lease_type {
            ClientLeaseType::Pooled => {
                self.prgit_client.release_shelve_client(&self.client_name);
            }
            ClientLeaseType::Temporary => {
                let _ = self.p4.client().delete(&self.client_name).run();
                let _ = std::fs::remove_dir_all(&self.client_root);
            }
        }
    }
}

pub struct ClientPool<'a> {
    prgit_client: &'a PrgitClient<'a>,
    config: ShelveConfig,
}

impl<'a> ClientPool<'a> {
    pub fn new(prgit_client: &'a PrgitClient<'a>) -> Result<Self, ClientPoolError> {
        let config = prgit_client
            .shelve_config()
            .ok_or(ClientPoolError::NoShelveConfig)?;
        Ok(Self { prgit_client, config })
    }

    pub fn acquire(&self) -> Result<ClientLease<'a>, ClientPoolError> {
        if let Some(client_name) = self.prgit_client.get_available_shelve_client() {
            if self.prgit_client.acquire_shelve_client(&client_name) {
                let client_root = self.config.clients_root.join(&client_name);
                let p4 = self.prgit_client.p4().client_name(&client_name);
                return Ok(ClientLease::new_pooled(self.prgit_client, p4, client_name, client_root));
            }
        }

        let current_count = self.prgit_client.count_shelve_clients();
        if current_count < self.config.max_clients {
            return self.create_pooled_client();
        }

        let timeout_millis = self.config.timeout.as_millis() as i64;
        if let Some(client_name) = self.prgit_client.get_timed_out_shelve_client(timeout_millis) {
            if self.prgit_client.acquire_shelve_client(&client_name) {
                let client_root = self.config.clients_root.join(&client_name);
                let p4 = self.prgit_client.p4().client_name(&client_name);
                return Ok(ClientLease::new_pooled(self.prgit_client, p4, client_name, client_root));
            }
        }

        self.create_temporary_client()
    }

    fn create_pooled_client(&self) -> Result<ClientLease<'a>, ClientPoolError> {
        let uuid = uuid::Uuid::new_v4();
        let client_name = format!("{}-{}", self.prgit_client.p4_config.client_name, uuid);
        let client_root = self.config.clients_root.join(&client_name);

        std::fs::create_dir_all(&client_root)?;

        let p4 = self.prgit_client.p4();
        self.create_p4_client(&p4, &client_name, &client_root)?;

        self.prgit_client.register_shelve_client(&client_name);
        self.prgit_client.acquire_shelve_client(&client_name);

        let p4 = p4.client_name(&client_name);
        Ok(ClientLease::new_pooled(self.prgit_client, p4, client_name, client_root))
    }

    fn create_temporary_client(&self) -> Result<ClientLease<'a>, ClientPoolError> {
        let uuid = uuid::Uuid::new_v4();
        let client_name = format!("{}-tmp-{}", self.prgit_client.p4_config.client_name, uuid);
        let client_root = self.config.clients_root.join(&client_name);

        std::fs::create_dir_all(&client_root)?;

        let p4 = self.prgit_client.p4();
        self.create_p4_client(&p4, &client_name, &client_root)?;

        let p4 = p4.client_name(&client_name);
        Ok(ClientLease::new_temporary(self.prgit_client, p4, client_name, client_root))
    }

    fn create_p4_client(&self, p4: &P4, client_name: &str, client_root: &PathBuf) -> Result<(), ClientPoolError> {
        let base_spec = p4.client().get(None).run()?;
        let base_client = &self.prgit_client.p4_config.client_name;
        let new_view: Vec<ClientMapping> = base_spec
            .view
            .iter()
            .map(|m| {
                ClientMapping::new(
                    &m.depot,
                    m.client.replace(&format!("//{}/", base_client), &format!("//{}/", client_name)),
                )
            })
            .collect();
        let new_spec = ClientSpec::new(
            client_name,
            client_root.to_str().ok_or(ClientPoolError::InvalidPath)?,
            new_view,
        );
        p4.client().set(&new_spec).run()?;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ClientPoolError {
    #[error("No shelve config found")]
    NoShelveConfig,
    #[error("P4 error: {0}")]
    P4(#[from] P4Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid path")]
    InvalidPath,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cabinet::Database;
    use crate::mirror::IntegrateStrategy;
    use p4rs::testkit::{TestClient, SERVER};
    use std::time::Duration;

    fn setup_test_env() -> (TestClient, Database, PathBuf) {
        let client = SERVER.test_client();
        let db = Database::open(":memory:").unwrap();
        let clients_root = client.client_root().join("shelve_clients");
        std::fs::create_dir_all(&clients_root).unwrap();
        (client, db, clients_root)
    }

    fn setup_prgit_client<'a>(
        test_client: &TestClient,
        db: &'a Database,
        clients_root: &PathBuf,
        max_clients: usize,
        timeout_secs: u64,
    ) -> PrgitClient<'a> {
        let client_id = db
            .create_prgit_client(
                &test_client.client_name,
                "p4",
                &format!("localhost:{}", SERVER.port),
                "",
            )
            .unwrap();
        db.create_prgit_repo(
            client_id,
            test_client.client_root().to_str().unwrap(),
            "master",
            IntegrateStrategy::MergeOurs,
            None,
        )
        .unwrap();
        db.create_shelve_config(
            client_id,
            max_clients,
            Duration::from_secs(timeout_secs),
            clients_root.to_str().unwrap(),
        )
        .unwrap();
        db.client(client_id).unwrap().unwrap()
    }

    #[test]
    fn test_acquire_creates_new_client_when_pool_empty() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root, 3, 300);

        let pool = ClientPool::new(&prgit_client).unwrap();
        let lease = pool.acquire().unwrap();

        assert!(lease.client_name.starts_with(&test_client.client_name));
        assert!(lease.client_root().exists());
        assert!(matches!(lease.lease_type, ClientLeaseType::Pooled));
        assert_eq!(prgit_client.count_shelve_clients(), 1);
    }

    #[test]
    fn test_acquire_reuses_available_client() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root, 3, 300);

        let pool = ClientPool::new(&prgit_client).unwrap();

        let first_name;
        {
            let lease = pool.acquire().unwrap();
            first_name = lease.client_name.clone();
        }

        {
            let lease = pool.acquire().unwrap();
            assert_eq!(lease.client_name, first_name);
        }

        assert_eq!(prgit_client.count_shelve_clients(), 1);
    }

    #[test]
    fn test_acquire_creates_multiple_clients_when_all_in_use() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root, 3, 300);

        let pool = ClientPool::new(&prgit_client).unwrap();

        let lease1 = pool.acquire().unwrap();
        let lease2 = pool.acquire().unwrap();
        let lease3 = pool.acquire().unwrap();

        assert_ne!(lease1.client_name, lease2.client_name);
        assert_ne!(lease2.client_name, lease3.client_name);
        assert_eq!(prgit_client.count_shelve_clients(), 3);
    }

    #[test]
    fn test_acquire_creates_temporary_when_max_reached() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root, 2, 300);

        let pool = ClientPool::new(&prgit_client).unwrap();

        let _lease1 = pool.acquire().unwrap();
        let _lease2 = pool.acquire().unwrap();
        let lease3 = pool.acquire().unwrap();

        assert!(lease3.client_name.contains("-tmp-"));
        assert!(matches!(lease3.lease_type, ClientLeaseType::Temporary));
        assert_eq!(prgit_client.count_shelve_clients(), 2);
    }

    #[test]
    fn test_temporary_client_deleted_on_drop() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root, 1, 300);

        let pool = ClientPool::new(&prgit_client).unwrap();

        let _lease1 = pool.acquire().unwrap();

        let temp_client_root;
        {
            let lease2 = pool.acquire().unwrap();
            temp_client_root = lease2.client_root().clone();
            assert!(temp_client_root.exists());
            assert!(lease2.client_name.contains("-tmp-"));
        }

        assert!(!temp_client_root.exists());
    }

    #[test]
    fn test_pooled_client_released_on_drop() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root, 3, 300);

        let pool = ClientPool::new(&prgit_client).unwrap();

        let client_name;
        {
            let lease = pool.acquire().unwrap();
            client_name = lease.client_name.clone();
        }

        assert!(prgit_client.get_available_shelve_client().is_some());
        assert_eq!(
            prgit_client.get_available_shelve_client().unwrap(),
            client_name
        );
    }

    #[test]
    fn test_acquire_timed_out_client() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root, 1, 0);

        let pool = ClientPool::new(&prgit_client).unwrap();

        let first_lease = pool.acquire().unwrap();
        let first_name = first_lease.client_name.clone();

        std::mem::forget(first_lease);

        std::thread::sleep(Duration::from_millis(100));

        let second_lease = pool.acquire().unwrap();
        assert_eq!(second_lease.client_name, first_name);
        assert!(matches!(second_lease.lease_type, ClientLeaseType::Pooled));
    }

    #[test]
    fn test_no_shelve_config_error() {
        let db = Database::open(":memory:").unwrap();
        let client_id = db
            .create_prgit_client("test", "p4", "localhost:1666", "user")
            .unwrap();
        db.create_prgit_repo(client_id, "/repo", "master", IntegrateStrategy::MergeOurs, None)
            .unwrap();
        let prgit_client = db.client(client_id).unwrap().unwrap();

        let result = ClientPool::new(&prgit_client);
        assert!(matches!(result, Err(ClientPoolError::NoShelveConfig)));
    }
}
