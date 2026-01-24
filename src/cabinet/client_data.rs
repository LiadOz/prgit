use std::path::{Path, PathBuf};
use std::time::Duration;

use p4rs::P4;
use rusqlite::Connection;

use crate::mirror::{IntegrateStrategy, MirrorData};

use super::tables::{PrgitClient, ShelveConfig};

pub struct ClientData<'a> {
    conn: &'a Connection,
    pub client_id: u64,
    client_name: String,
    p4_path: PathBuf,
    p4port: String,
    p4user: String,
    repo_path: PathBuf,
    integrate_strategy: IntegrateStrategy,
    max_changes_query: Option<usize>,
}

impl<'a> ClientData<'a> {
    pub(super) fn new(
        conn: &'a Connection,
        client_id: u64,
        client_name: String,
        p4_path: PathBuf,
        p4port: String,
        p4user: String,
        repo_path: PathBuf,
        integrate_strategy: IntegrateStrategy,
        max_changes_query: Option<usize>,
    ) -> Self {
        Self {
            conn,
            client_id,
            client_name,
            p4_path,
            p4port,
            p4user,
            repo_path,
            integrate_strategy,
            max_changes_query,
        }
    }

    pub fn info(&self) -> PrgitClient {
        PrgitClient {
            id: self.client_id,
            client_name: self.client_name.clone(),
            p4_path: self.p4_path.clone(),
            p4port: self.p4port.clone(),
            p4user: self.p4user.clone(),
        }
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub fn p4(&self) -> P4 {
        P4::new()
            .p4_path(&self.p4_path)
            .port(&self.p4port)
            .p4_user(&self.p4user)
            .client_name(&self.client_name)
    }

    pub fn get_commit_for_change(&self, change: usize) -> Option<String> {
        self.conn
            .query_row(
                "SELECT commit_hash FROM commit_change_mapping WHERE prgit_client_id = ?1 AND change = ?2",
                rusqlite::params![self.client_id, change as i64],
                |row| row.get(0),
            )
            .ok()
    }

    pub fn get_change_for_commit(&self, commit: &str) -> Option<usize> {
        self.conn
            .query_row(
                "SELECT change FROM commit_change_mapping WHERE prgit_client_id = ?1 AND commit_hash = ?2",
                rusqlite::params![self.client_id, commit],
                |row| row.get::<_, i64>(0),
            )
            .map(|v| v as usize)
            .ok()
    }

    pub fn map_commit_to_change(&self, commit: &str, change: usize) {
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO commit_change_mapping (prgit_client_id, change, commit_hash) VALUES (?1, ?2, ?3)",
            rusqlite::params![self.client_id, change as i64, commit],
        );
    }

    pub fn get_branch_for_change(&self, change: usize) -> Option<String> {
        self.conn
            .query_row(
                "SELECT branch FROM branch_mapping WHERE prgit_client_id = ?1 AND change = ?2",
                rusqlite::params![self.client_id, change as i64],
                |row| row.get(0),
            )
            .ok()
    }

    pub fn shelve_config(&self) -> Option<ShelveConfig> {
        self.conn
            .query_row(
                "SELECT prgit_client_id, max_clients, timeout_secs, grow_threshold FROM shelve_config WHERE prgit_client_id = ?1",
                [self.client_id],
                |row| {
                    Ok(ShelveConfig {
                        prgit_client_id: row.get::<_, i64>(0)? as u64,
                        max_clients: row.get::<_, i64>(1)? as usize,
                        timeout: Duration::from_secs(row.get::<_, i64>(2)? as u64),
                        grow_threshold: row.get::<_, i64>(3)? as usize,
                    })
                },
            )
            .ok()
    }
}

impl MirrorData for ClientData<'_> {
    fn last_sync_change(&self) -> usize {
        self.conn
            .query_row(
                "SELECT last_sync_change FROM prgit_repos WHERE prgit_client_id = ?1",
                [self.client_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|v| v as usize)
            .unwrap_or(0)
    }

    fn set_last_sync_change(&mut self, change: usize) {
        let _ = self.conn.execute(
            "UPDATE prgit_repos SET last_sync_change = ?1 WHERE prgit_client_id = ?2",
            rusqlite::params![change as i64, self.client_id],
        );
    }

    fn get_related_branch(&self, change: usize) -> Option<String> {
        self.get_branch_for_change(change)
    }

    fn get_user_email(&self, user: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT email FROM user_mapping WHERE user = ?1",
                [user],
                |row| row.get(0),
            )
            .ok()
    }

    fn set_user_email(&mut self, user: &str, email: &str) {
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO user_mapping (user, email) VALUES (?1, ?2)",
            [user, email],
        );
    }

    fn p4_client(&self) -> &str {
        &self.client_name
    }

    fn integrate_strategy(&self) -> IntegrateStrategy {
        self.integrate_strategy
    }

    fn max_changes_query(&self) -> Option<usize> {
        self.max_changes_query
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cabinet::Database;

    fn setup_client_data() -> (Database, u64) {
        let db = Database::open(":memory:").unwrap();
        let client_id = db
            .create_prgit_client("test-client", "/usr/bin/p4", "localhost:1666", "testuser")
            .unwrap();
        db.create_prgit_repo(client_id, "/path/to/repo", IntegrateStrategy::MergeOurs, Some(50))
            .unwrap();
        (db, client_id)
    }

    #[test]
    fn client_data_info() {
        let (db, client_id) = setup_client_data();
        let cd = db.client(client_id).unwrap().unwrap();
        let info = cd.info();
        assert_eq!(info.client_name, "test-client");
        assert_eq!(info.p4port, "localhost:1666");
    }

    #[test]
    fn client_data_repo_path() {
        let (db, client_id) = setup_client_data();
        let cd = db.client(client_id).unwrap().unwrap();
        assert_eq!(cd.repo_path(), Path::new("/path/to/repo"));
    }

    #[test]
    fn client_data_p4_client() {
        let (db, client_id) = setup_client_data();
        let cd = db.client(client_id).unwrap().unwrap();
        assert_eq!(cd.p4_client(), "test-client");
    }

    #[test]
    fn client_data_integrate_strategy() {
        let (db, client_id) = setup_client_data();
        let cd = db.client(client_id).unwrap().unwrap();
        assert!(matches!(cd.integrate_strategy(), IntegrateStrategy::MergeOurs));
    }

    #[test]
    fn client_data_max_changes_query() {
        let (db, client_id) = setup_client_data();
        let cd = db.client(client_id).unwrap().unwrap();
        assert_eq!(cd.max_changes_query(), Some(50));
    }

    #[test]
    fn last_sync_change_defaults_to_zero() {
        let (db, client_id) = setup_client_data();
        let cd = db.client(client_id).unwrap().unwrap();
        assert_eq!(cd.last_sync_change(), 0);
    }

    #[test]
    fn set_and_get_last_sync_change() {
        let (db, client_id) = setup_client_data();
        let mut cd = db.client(client_id).unwrap().unwrap();
        cd.set_last_sync_change(12345);
        assert_eq!(cd.last_sync_change(), 12345);
    }

    #[test]
    fn set_and_get_user_email() {
        let (db, client_id) = setup_client_data();
        let mut cd = db.client(client_id).unwrap().unwrap();
        cd.set_user_email("jdoe", "jdoe@example.com");
        assert_eq!(cd.get_user_email("jdoe"), Some("jdoe@example.com".to_string()));
    }

    #[test]
    fn map_and_get_commit_change() {
        let (db, client_id) = setup_client_data();
        let cd = db.client(client_id).unwrap().unwrap();
        cd.map_commit_to_change("abc123", 100);
        assert_eq!(cd.get_commit_for_change(100), Some("abc123".to_string()));
        assert_eq!(cd.get_change_for_commit("abc123"), Some(100));
    }

    #[test]
    fn get_branch_for_change_returns_none() {
        let (db, client_id) = setup_client_data();
        let cd = db.client(client_id).unwrap().unwrap();
        assert!(cd.get_branch_for_change(100).is_none());
    }

    #[test]
    fn shelve_config_returns_none_when_not_set() {
        let (db, client_id) = setup_client_data();
        let cd = db.client(client_id).unwrap().unwrap();
        assert!(cd.shelve_config().is_none());
    }
}
