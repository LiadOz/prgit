use std::fs::{self, File};
use std::os::unix::io::AsRawFd;
use std::path::Path;

use p4rs::{ClientMapping, ClientSpec, P4, P4Command, P4Error};
use thiserror::Error;

use crate::cabinet::PrgitClient;

use super::shelve_client::ShelveClient;

pub struct ShelveClientHandle {
    pub shelve_client: ShelveClient,
    _lock_file: File,
}

pub fn get_shelve_client(
    prgit_client: &PrgitClient,
    user_p4: &P4,
) -> Result<ShelveClientHandle, ShelveClientError> {
    let config = prgit_client
        .shelve_config()
        .ok_or(ShelveClientError::NoShelveConfig)?;

    let user_id = extract_user_id(user_p4)?;
    let base_client = &prgit_client.p4_config.client_name;
    let client_name = format!("{}-{}-shelve", base_client, user_id);
    let client_root = config.clients_root.join(&client_name);

    fs::create_dir_all(&client_root)?;

    let lock_file = acquire_lock(&client_root)?;

    let p4 = user_p4.clone().client_name(&client_name);
    ensure_p4_client_exists(&p4, &prgit_client.p4(), base_client, &client_name, &client_root)?;

    let shelve_client = ShelveClient::new(p4, &client_name, client_root)?;

    Ok(ShelveClientHandle {
        shelve_client,
        _lock_file: lock_file,
    })
}

fn extract_user_id(p4: &P4) -> Result<String, ShelveClientError> {
    let info = p4.info().short().run()?.single()?;
    Ok(info.user_name)
}

fn acquire_lock(client_root: &Path) -> Result<File, ShelveClientError> {
    let lock_path = client_root.join(".prgit.lock");
    let file = File::create(&lock_path)?;
    let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        return Err(ShelveClientError::ClientBusy);
    }
    Ok(file)
}

fn ensure_p4_client_exists(
    user_p4: &P4,
    service_p4: &P4,
    base_client: &str,
    client_name: &str,
    client_root: &Path,
) -> Result<(), ShelveClientError> {
    let base_spec = service_p4.client().get(None).run()?.single()?;
    let new_view: Vec<ClientMapping> = base_spec
        .view
        .iter()
        .map(|m| {
            ClientMapping::new(
                &m.depot,
                m.client.replace(
                    &format!("//{}/", base_client),
                    &format!("//{}/", client_name),
                ),
            )
        })
        .collect();
    let new_spec = ClientSpec::new(
        client_name,
        client_root.to_str().ok_or(ShelveClientError::InvalidPath)?,
        new_view,
    );
    user_p4.client().set(&new_spec).run()?;
    Ok(())
}

#[derive(Error, Debug)]
pub enum ShelveClientError {
    #[error("No shelve config found")]
    NoShelveConfig,
    #[error("Client is busy (locked by another operation)")]
    ClientBusy,
    #[error("P4 error: {0}")]
    P4(#[from] P4Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid path")]
    InvalidPath,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use super::*;
    use crate::cabinet::Database;
    use crate::mirror::IntegrateStrategy;
    use p4rs::testkit::SERVER;

    fn setup_test_env() -> (p4rs::testkit::TestClient, Database, PathBuf) {
        let client = SERVER.test_client();
        let db = Database::open(":memory:").unwrap();
        let clients_root = client.client_root().join("shelve_clients");
        std::fs::create_dir_all(&clients_root).unwrap();
        (client, db, clients_root)
    }

    fn setup_prgit_client<'a>(
        test_client: &p4rs::testkit::TestClient,
        db: &'a Database,
        clients_root: &PathBuf,
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
            clients_root.to_str().unwrap(),
        )
        .unwrap();
        db.client(client_id).unwrap().unwrap()
    }

    #[test]
    fn test_get_shelve_client_creates_client() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root);

        let handle = get_shelve_client(&prgit_client, &test_client.p4).unwrap();
        let expected_name = format!("{}-{}-shelve", test_client.client_name, test_client.p4.info().short().run().unwrap().single().unwrap().user_name);
        let client_root = clients_root.join(&expected_name);
        assert!(client_root.exists());
        drop(handle);
    }

    #[test]
    fn test_get_shelve_client_reuses_existing() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root);

        {
            let _handle = get_shelve_client(&prgit_client, &test_client.p4).unwrap();
        }
        // Second call should succeed (reuses existing P4 client)
        {
            let _handle = get_shelve_client(&prgit_client, &test_client.p4).unwrap();
        }
    }

    #[test]
    fn test_concurrent_access_returns_client_busy() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root);

        let _handle = get_shelve_client(&prgit_client, &test_client.p4).unwrap();
        let result = get_shelve_client(&prgit_client, &test_client.p4);
        assert!(matches!(result, Err(ShelveClientError::ClientBusy)));
    }

    #[test]
    fn test_lock_released_on_drop() {
        let (test_client, db, clients_root) = setup_test_env();
        let prgit_client = setup_prgit_client(&test_client, &db, &clients_root);

        {
            let _handle = get_shelve_client(&prgit_client, &test_client.p4).unwrap();
            // lock held here
        }
        // lock released on drop, should be able to acquire again
        let _handle = get_shelve_client(&prgit_client, &test_client.p4).unwrap();
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

        let p4 = P4::new().port("localhost:1666").p4_user("user");
        let result = get_shelve_client(&prgit_client, &p4);
        assert!(matches!(result, Err(ShelveClientError::NoShelveConfig)));
    }
}
