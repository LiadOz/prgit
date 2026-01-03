use std::path::PathBuf;
use std::time::Duration;

use rusqlite::Connection;

use crate::mirror::IntegrateStrategy;

use super::mirror_data::DBMirrorData;
use super::tables::{
    BranchMapping, PrgitClient, PrgitRepo, ShelveClient, ShelveConfig, Table,
    UserMapping,
};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(PrgitClient::SCHEMA)?;
        conn.execute_batch(ShelveConfig::SCHEMA)?;
        conn.execute_batch(ShelveClient::SCHEMA)?;
        conn.execute_batch(BranchMapping::SCHEMA)?;
        conn.execute_batch(UserMapping::SCHEMA)?;
        conn.execute_batch(PrgitRepo::SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn get_prgit_client(&self, id: u64) -> Result<Option<PrgitClient>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, client_name, p4_path, p4port, p4user FROM prgit_clients WHERE id = ?1",
                [id],
                |row| {
                    Ok(PrgitClient {
                        id: row.get::<_, i64>(0)? as u64,
                        client_name: row.get(1)?,
                        p4_path: PathBuf::from(row.get::<_, String>(2)?),
                        p4port: row.get(3)?,
                        p4user: row.get(4)?,
                    })
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(e),
            })
    }

    pub fn get_prgit_client_by_name(
        &self,
        name: &str,
    ) -> Result<Option<PrgitClient>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, client_name, p4_path, p4port, p4user FROM prgit_clients WHERE client_name = ?1",
                [name],
                |row| {
                    Ok(PrgitClient {
                        id: row.get::<_, i64>(0)? as u64,
                        client_name: row.get(1)?,
                        p4_path: PathBuf::from(row.get::<_, String>(2)?),
                        p4port: row.get(3)?,
                        p4user: row.get(4)?,
                    })
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(e),
            })
    }

    pub fn create_prgit_client(
        &self,
        client_name: &str,
        p4_path: &str,
        p4port: &str,
        p4user: &str,
    ) -> Result<u64, rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO prgit_clients (client_name, p4_path, p4port, p4user) VALUES (?1, ?2, ?3, ?4)",
            [client_name, p4_path, p4port, p4user],
        )?;
        Ok(self.conn.last_insert_rowid() as u64)
    }

    // pub fn add_client_views(
    //     &self,
    //     prgit_client_id: u64,
    //     views: &[(&str, &str)],
    // ) -> Result<(), rusqlite::Error> {
    //     for (depot, client) in views {
    //         self.conn.execute(
    //             "INSERT INTO client_views (prgit_client_id, depot, client) VALUES (?1, ?2, ?3)",
    //             rusqlite::params![prgit_client_id, depot, client],
    //         )?;
    //     }
    //     Ok(())
    // }

    pub fn get_prgit_repo(
        &self,
        prgit_client_id: u64,
    ) -> Result<Option<PrgitRepo>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, prgit_client_id, repo_path, last_sync_change, integrate_strategy, max_changes_query FROM prgit_repos WHERE prgit_client_id = ?1",
                [prgit_client_id],
                |row| {
                    Ok(PrgitRepo {
                        id: row.get::<_, i64>(0)? as u64,
                        prgit_client_id: row.get::<_, i64>(1)? as u64,
                        repo_path: PathBuf::from(row.get::<_, String>(2)?),
                        last_sync_change: row.get::<_, i64>(3)? as usize,
                        integrate_strategy: IntegrateStrategy::from_db(row.get(4)?),
                        max_changes_query: row.get::<_, Option<i64>>(5)?.map(|v| v as usize),
                    })
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(e),
            })
    }

    pub fn create_prgit_repo(
        &self,
        prgit_client_id: u64,
        repo_path: &str,
        integrate_strategy: IntegrateStrategy,
        max_changes_query: Option<usize>,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO prgit_repos (prgit_client_id, repo_path, integrate_strategy, max_changes_query) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![prgit_client_id, repo_path, integrate_strategy.to_db(), max_changes_query.map(|v| v as i64)],
        )?;
        Ok(())
    }

    pub fn get_shelve_config(
        &self,
        prgit_client_id: u64,
    ) -> Result<Option<ShelveConfig>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT prgit_client_id, max_clients, timeout_secs, grow_threshold FROM shelve_config WHERE prgit_client_id = ?1",
                [prgit_client_id],
                |row| {
                    Ok(ShelveConfig {
                        prgit_client_id: row.get::<_, i64>(0)? as u64,
                        max_clients: row.get::<_, i64>(1)? as usize,
                        timeout: Duration::from_secs(row.get::<_, i64>(2)? as u64),
                        grow_threshold: row.get::<_, i64>(3)? as usize,
                    })
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(e),
            })
    }

    pub fn create_shelve_config(
        &self,
        prgit_client_id: u64,
        max_clients: usize,
        timeout: Duration,
        grow_threshold: usize,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO shelve_config (prgit_client_id, max_clients, timeout_secs, grow_threshold) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![prgit_client_id, max_clients as i64, timeout.as_secs() as i64, grow_threshold as i64],
        )?;
        Ok(())
    }

    pub fn mirror_data(self, prgit_client_id: u64) -> DBMirrorData {
        DBMirrorData::new(self.conn, prgit_client_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open(":memory:").unwrap()
    }

    #[test]
    fn create_and_get_prgit_client() {
        let db = test_db();
        let id = db
            .create_prgit_client("test-client", "/usr/bin/p4", "localhost:1666", "testuser")
            .unwrap();
        assert_eq!(id, 1);

        let client = db.get_prgit_client(id).unwrap().unwrap();
        assert_eq!(client.client_name, "test-client");
        assert_eq!(client.p4_path, PathBuf::from("/usr/bin/p4"));
        assert_eq!(client.p4port, "localhost:1666");
        assert_eq!(client.p4user, "testuser");
    }

    #[test]
    fn get_prgit_client_by_name() {
        let db = test_db();
        db.create_prgit_client("my-client", "p4", "server:1666", "user")
            .unwrap();

        let client = db.get_prgit_client_by_name("my-client").unwrap().unwrap();
        assert_eq!(client.client_name, "my-client");

        let missing = db.get_prgit_client_by_name("nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn get_nonexistent_client_returns_none() {
        let db = test_db();
        let client = db.get_prgit_client(999).unwrap();
        assert!(client.is_none());
    }

    // #[test]
    // fn add_client_views() {
    //    let db = test_db();
    //    let id = db.create_prgit_client("client", "p4", "port", "user").unwrap();
    //    db.add_client_views(id, &[("//depot/...", "//client/depot/...")]).unwrap();
    //}

    #[test]
    fn create_and_get_prgit_repo() {
        let db = test_db();
        let client_id = db.create_prgit_client("client", "p4", "port", "user").unwrap();
        db.create_prgit_repo(client_id, "/path/to/repo", IntegrateStrategy::MergeOurs, Some(100))
            .unwrap();

        let repo = db.get_prgit_repo(client_id).unwrap().unwrap();
        assert_eq!(repo.prgit_client_id, client_id);
        assert_eq!(repo.repo_path, PathBuf::from("/path/to/repo"));
        assert_eq!(repo.last_sync_change, 0);
        assert_eq!(repo.max_changes_query, Some(100));
    }

    #[test]
    fn get_nonexistent_repo_returns_none() {
        let db = test_db();
        let repo = db.get_prgit_repo(999).unwrap();
        assert!(repo.is_none());
    }

    #[test]
    fn create_and_get_shelve_config() {
        let db = test_db();
        let client_id = db.create_prgit_client("client", "p4", "port", "user").unwrap();
        db.create_shelve_config(client_id, 5, Duration::from_secs(300), 10)
            .unwrap();

        let config = db.get_shelve_config(client_id).unwrap().unwrap();
        assert_eq!(config.prgit_client_id, client_id);
        assert_eq!(config.max_clients, 5);
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert_eq!(config.grow_threshold, 10);
    }

    #[test]
    fn get_nonexistent_shelve_config_returns_none() {
        let db = test_db();
        let config = db.get_shelve_config(999).unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn create_multiple_clients() {
        let db = test_db();
        let id1 = db.create_prgit_client("client1", "p4", "port1", "user1").unwrap();
        let id2 = db.create_prgit_client("client2", "p4", "port2", "user2").unwrap();
        assert_ne!(id1, id2);

        let c1 = db.get_prgit_client(id1).unwrap().unwrap();
        let c2 = db.get_prgit_client(id2).unwrap().unwrap();
        assert_eq!(c1.client_name, "client1");
        assert_eq!(c2.client_name, "client2");
    }

    #[test]
    //fn add_multiple_client_views() {
    //    let db = test_db();
    //    let id = db.create_prgit_client("client", "p4", "port", "user").unwrap();
    //    db.add_client_views(id, &[
    //        ("//depot/main/...", "//client/main/..."),
    //        ("//depot/dev/...", "//client/dev/..."),
    //    ]).unwrap();
    //}

    #[test]
    fn create_repo_without_max_changes() {
        let db = test_db();
        let client_id = db.create_prgit_client("client", "p4", "port", "user").unwrap();
        db.create_prgit_repo(client_id, "/repo", IntegrateStrategy::MergeOurs, None).unwrap();

        let repo = db.get_prgit_repo(client_id).unwrap().unwrap();
        assert_eq!(repo.max_changes_query, None);
    }

    #[test]
    fn shelve_config_timeout_preserved() {
        let db = test_db();
        let client_id = db.create_prgit_client("client", "p4", "port", "user").unwrap();
        db.create_shelve_config(client_id, 10, Duration::from_secs(3600), 5).unwrap();

        let config = db.get_shelve_config(client_id).unwrap().unwrap();
        assert_eq!(config.timeout.as_secs(), 3600);
    }
}
