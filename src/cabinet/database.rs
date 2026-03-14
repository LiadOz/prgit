use std::path::PathBuf;

use rusqlite::Connection;

use crate::mirror::IntegrateStrategy;

use super::prgit_client::PrgitClient;
use super::tables::{
    PrgitClientInfo, PrgitRepo, ShelveConfig, Table, TicketMetadata, UserMapping,
};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn open(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(PrgitClientInfo::SCHEMA)?;
        conn.execute_batch(ShelveConfig::SCHEMA)?;
        conn.execute_batch(UserMapping::SCHEMA)?;
        conn.execute_batch(PrgitRepo::SCHEMA)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS commit_change_mapping (
                prgit_client_id INTEGER NOT NULL REFERENCES prgit_clients(id),
                change INTEGER NOT NULL,
                commit_hash TEXT NOT NULL,
                PRIMARY KEY (prgit_client_id, change)
            );"
        )?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS branch_shelve_mapping (
                prgit_client_id INTEGER NOT NULL REFERENCES prgit_clients(id),
                branch TEXT NOT NULL,
                shelved_change INTEGER NOT NULL,
                PRIMARY KEY (prgit_client_id, branch)
            );"
        )?;
        conn.execute_batch(TicketMetadata::SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn get_prgit_client_info(&self, id: u64) -> Result<Option<PrgitClientInfo>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, client_name, p4_path, p4port, p4user FROM prgit_clients WHERE id = ?1",
                [id],
                |row| {
                    Ok(PrgitClientInfo {
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

    pub fn get_prgit_client_info_by_name(
        &self,
        name: &str,
    ) -> Result<Option<PrgitClientInfo>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT id, client_name, p4_path, p4port, p4user FROM prgit_clients WHERE client_name = ?1",
                [name],
                |row| {
                    Ok(PrgitClientInfo {
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
                "SELECT id, prgit_client_id, repo_path, synced_branch, last_sync_change, integrate_strategy, max_changes_query FROM prgit_repos WHERE prgit_client_id = ?1",
                [prgit_client_id],
                |row| {
                    Ok(PrgitRepo {
                        id: row.get::<_, i64>(0)? as u64,
                        prgit_client_id: row.get::<_, i64>(1)? as u64,
                        repo_path: PathBuf::from(row.get::<_, String>(2)?),
                        synced_branch: row.get(3)?,
                        last_sync_change: row.get::<_, i64>(4)? as usize,
                        integrate_strategy: IntegrateStrategy::from_db(row.get(5)?),
                        max_changes_query: row.get::<_, Option<i64>>(6)?.map(|v| v as usize),
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
        synced_branch: &str,
        integrate_strategy: IntegrateStrategy,
        max_changes_query: Option<usize>,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO prgit_repos (prgit_client_id, repo_path, synced_branch, integrate_strategy, max_changes_query) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![prgit_client_id, repo_path, synced_branch, integrate_strategy.to_db(), max_changes_query.map(|v| v as i64)],
        )?;
        Ok(())
    }

    pub fn get_shelve_config(
        &self,
        prgit_client_id: u64,
    ) -> Result<Option<ShelveConfig>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT prgit_client_id, clients_root FROM shelve_config WHERE prgit_client_id = ?1",
                [prgit_client_id],
                |row| {
                    Ok(ShelveConfig {
                        prgit_client_id: row.get::<_, i64>(0)? as u64,
                        clients_root: PathBuf::from(row.get::<_, String>(1)?),
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
        clients_root: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO shelve_config (prgit_client_id, clients_root) VALUES (?1, ?2)",
            rusqlite::params![prgit_client_id, clients_root],
        )?;
        Ok(())
    }

    pub fn client(&self, id: u64) -> Result<Option<PrgitClient<'_>>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT p.client_name, p.p4_path, p.p4port, p.p4user, r.repo_path, r.synced_branch, r.integrate_strategy, r.max_changes_query
                 FROM prgit_clients p
                 JOIN prgit_repos r ON p.id = r.prgit_client_id
                 WHERE p.id = ?1",
                [id],
                |row| {
                    Ok(PrgitClient::new(
                        &self.conn,
                        id,
                        row.get::<_, String>(0)?,
                        PathBuf::from(row.get::<_, String>(1)?),
                        row.get(2)?,
                        row.get(3)?,
                        PathBuf::from(row.get::<_, String>(4)?),
                        row.get(5)?,
                        IntegrateStrategy::from_db(row.get(6)?),
                        row.get::<_, Option<i64>>(7)?.map(|v| v as usize),
                    ))
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(e),
            })
    }

    pub fn client_by_name(&self, name: &str) -> Result<Option<PrgitClient<'_>>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT p.id, p.client_name, p.p4_path, p.p4port, p.p4user, r.repo_path, r.synced_branch, r.integrate_strategy, r.max_changes_query
                 FROM prgit_clients p
                 JOIN prgit_repos r ON p.id = r.prgit_client_id
                 WHERE p.client_name = ?1",
                [name],
                |row| {
                    Ok(PrgitClient::new(
                        &self.conn,
                        row.get::<_, i64>(0)? as u64,
                        row.get::<_, String>(1)?,
                        PathBuf::from(row.get::<_, String>(2)?),
                        row.get(3)?,
                        row.get(4)?,
                        PathBuf::from(row.get::<_, String>(5)?),
                        row.get(6)?,
                        IntegrateStrategy::from_db(row.get(7)?),
                        row.get::<_, Option<i64>>(8)?.map(|v| v as usize),
                    ))
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(e),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open(":memory:").unwrap()
    }

    #[test]
    fn create_and_get_prgit_client_info() {
        let db = test_db();
        let id = db
            .create_prgit_client("test-client", "/usr/bin/p4", "localhost:1666", "testuser")
            .unwrap();
        assert_eq!(id, 1);

        let client = db.get_prgit_client_info(id).unwrap().unwrap();
        assert_eq!(client.client_name, "test-client");
        assert_eq!(client.p4_path, PathBuf::from("/usr/bin/p4"));
        assert_eq!(client.p4port, "localhost:1666");
        assert_eq!(client.p4user, "testuser");
    }

    #[test]
    fn get_prgit_client_info_by_name() {
        let db = test_db();
        db.create_prgit_client("my-client", "p4", "server:1666", "user")
            .unwrap();

        let client = db.get_prgit_client_info_by_name("my-client").unwrap().unwrap();
        assert_eq!(client.client_name, "my-client");

        let missing = db.get_prgit_client_info_by_name("nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn get_nonexistent_client_info_returns_none() {
        let db = test_db();
        let client = db.get_prgit_client_info(999).unwrap();
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
        db.create_prgit_repo(client_id, "/path/to/repo", "master", IntegrateStrategy::MergeOurs, Some(100))
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
        db.create_shelve_config(client_id, "/shelve/clients")
            .unwrap();

        let config = db.get_shelve_config(client_id).unwrap().unwrap();
        assert_eq!(config.prgit_client_id, client_id);
        assert_eq!(config.clients_root, PathBuf::from("/shelve/clients"));
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

        let c1 = db.get_prgit_client_info(id1).unwrap().unwrap();
        let c2 = db.get_prgit_client_info(id2).unwrap().unwrap();
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
        db.create_prgit_repo(client_id, "/repo", "master", IntegrateStrategy::MergeOurs, None).unwrap();

        let repo = db.get_prgit_repo(client_id).unwrap().unwrap();
        assert_eq!(repo.max_changes_query, None);
    }

    #[test]
    fn shelve_config_clients_root_preserved() {
        let db = test_db();
        let client_id = db.create_prgit_client("client", "p4", "port", "user").unwrap();
        db.create_shelve_config(client_id, "/my/clients").unwrap();

        let config = db.get_shelve_config(client_id).unwrap().unwrap();
        assert_eq!(config.clients_root, PathBuf::from("/my/clients"));
    }
}
