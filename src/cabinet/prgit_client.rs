use std::path::{Path, PathBuf};
use std::time::Duration;

use p4rs::P4;
use rusqlite::Connection;

use crate::mirror::{IntegrateStrategy, MirrorData};

use super::tables::ShelveConfig;

#[derive(Debug, Clone)]
pub struct P4Config {
    pub client_name: String,
    pub p4_path: PathBuf,
    pub p4port: String,
    pub p4user: String,
}

impl P4Config {
    pub fn p4(&self) -> P4 {
        P4::new()
            .p4_path(&self.p4_path)
            .port(&self.p4port)
            .p4_user(&self.p4user)
            .client_name(&self.client_name)
    }
}

#[derive(Debug, Clone)]
pub struct GitConfig {
    pub repo_path: PathBuf,
    pub synced_branch: String,
}

pub struct PrgitClient<'a> {
    conn: &'a Connection,
    pub client_id: u64,
    pub p4_config: P4Config,
    pub git_config: GitConfig,
    integrate_strategy: IntegrateStrategy,
    max_changes_query: Option<usize>,
}

impl<'a> PrgitClient<'a> {
    pub(super) fn new(
        conn: &'a Connection,
        client_id: u64,
        client_name: String,
        p4_path: PathBuf,
        p4port: String,
        p4user: String,
        repo_path: PathBuf,
        synced_branch: String,
        integrate_strategy: IntegrateStrategy,
        max_changes_query: Option<usize>,
    ) -> Self {
        Self {
            conn,
            client_id,
            p4_config: P4Config {
                client_name,
                p4_path,
                p4port,
                p4user,
            },
            git_config: GitConfig { repo_path, synced_branch },
            integrate_strategy,
            max_changes_query,
        }
    }

    pub fn repo_path(&self) -> &Path {
        &self.git_config.repo_path
    }

    pub fn p4(&self) -> P4 {
        self.p4_config.p4()
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
                "SELECT prgit_client_id, max_clients, timeout_secs, clients_root FROM shelve_config WHERE prgit_client_id = ?1",
                [self.client_id],
                |row| {
                    Ok(ShelveConfig {
                        prgit_client_id: row.get::<_, i64>(0)? as u64,
                        max_clients: row.get::<_, i64>(1)? as usize,
                        timeout: Duration::from_secs(row.get::<_, i64>(2)? as u64),
                        clients_root: PathBuf::from(row.get::<_, String>(3)?),
                    })
                },
            )
            .ok()
    }

    pub fn get_available_shelve_client(&self) -> Option<String> {
        self.conn
            .query_row(
                "SELECT client_name FROM shelve_clients WHERE prgit_client_id = ?1 AND status = 'available' LIMIT 1",
                [self.client_id],
                |row| row.get(0),
            )
            .ok()
    }

    pub fn get_timed_out_shelve_client(&self, timeout_millis: i64) -> Option<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as i64;
        self.conn
            .query_row(
                "SELECT client_name FROM shelve_clients WHERE prgit_client_id = ?1 AND status = 'in_use' AND locked_at < ?2 LIMIT 1",
                rusqlite::params![self.client_id, now - timeout_millis],
                |row| row.get(0),
            )
            .ok()
    }

    pub fn count_shelve_clients(&self) -> usize {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM shelve_clients WHERE prgit_client_id = ?1",
                [self.client_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|v| v as usize)
            .unwrap_or(0)
    }

    pub fn acquire_shelve_client(&self, client_name: &str) -> bool {
        let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
            return false;
        };
        self.conn
            .execute(
                "UPDATE shelve_clients SET status = 'in_use', locked_at = ?1 WHERE prgit_client_id = ?2 AND client_name = ?3",
                rusqlite::params![now.as_millis() as i64, self.client_id, client_name],
            )
            .map(|n| n > 0)
            .unwrap_or(false)
    }

    pub fn release_shelve_client(&self, client_name: &str) {
        let _ = self.conn.execute(
            "UPDATE shelve_clients SET status = 'available', locked_at = NULL WHERE prgit_client_id = ?1 AND client_name = ?2",
            rusqlite::params![self.client_id, client_name],
        );
    }

    pub fn register_shelve_client(&self, client_name: &str) {
        let _ = self.conn.execute(
            "INSERT INTO shelve_clients (prgit_client_id, client_name, status) VALUES (?1, ?2, 'available')",
            rusqlite::params![self.client_id, client_name],
        );
    }

    pub fn get_shelved_change_for_branch(&self, branch: &str) -> Option<usize> {
        self.conn
            .query_row(
                "SELECT shelved_change FROM branch_shelve_mapping WHERE prgit_client_id = ?1 AND branch = ?2",
                rusqlite::params![self.client_id, branch],
                |row| row.get::<_, i64>(0),
            )
            .map(|v| v as usize)
            .ok()
    }

    pub fn set_shelved_change_for_branch(&self, branch: &str, change: usize) {
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO branch_shelve_mapping (prgit_client_id, branch, shelved_change) VALUES (?1, ?2, ?3)",
            rusqlite::params![self.client_id, branch, change as i64],
        );
    }

    pub fn clear_shelved_change_for_branch(&self, branch: &str) {
        let _ = self.conn.execute(
            "DELETE FROM branch_shelve_mapping WHERE prgit_client_id = ?1 AND branch = ?2",
            rusqlite::params![self.client_id, branch],
        );
    }
}

impl MirrorData for PrgitClient<'_> {
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
        &self.p4_config.client_name
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

    fn setup_prgit_client() -> (Database, u64) {
        let db = Database::open(":memory:").unwrap();
        let client_id = db
            .create_prgit_client("test-client", "/usr/bin/p4", "localhost:1666", "testuser")
            .unwrap();
        db.create_prgit_repo(client_id, "/path/to/repo", "master", IntegrateStrategy::MergeOurs, Some(50))
            .unwrap();
        (db, client_id)
    }

    #[test]
    fn prgit_client_p4_config() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert_eq!(client.p4_config.client_name, "test-client");
        assert_eq!(client.p4_config.p4port, "localhost:1666");
    }

    #[test]
    fn prgit_client_git_config() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert_eq!(client.git_config.repo_path, PathBuf::from("/path/to/repo"));
    }

    #[test]
    fn prgit_client_repo_path() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert_eq!(client.repo_path(), Path::new("/path/to/repo"));
    }

    #[test]
    fn prgit_client_p4_client() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert_eq!(client.p4_client(), "test-client");
    }

    #[test]
    fn prgit_client_integrate_strategy() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert!(matches!(client.integrate_strategy(), IntegrateStrategy::MergeOurs));
    }

    #[test]
    fn prgit_client_max_changes_query() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert_eq!(client.max_changes_query(), Some(50));
    }

    #[test]
    fn last_sync_change_defaults_to_zero() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert_eq!(client.last_sync_change(), 0);
    }

    #[test]
    fn set_and_get_last_sync_change() {
        let (db, client_id) = setup_prgit_client();
        let mut client = db.client(client_id).unwrap().unwrap();
        client.set_last_sync_change(12345);
        assert_eq!(client.last_sync_change(), 12345);
    }

    #[test]
    fn set_and_get_user_email() {
        let (db, client_id) = setup_prgit_client();
        let mut client = db.client(client_id).unwrap().unwrap();
        client.set_user_email("jdoe", "jdoe@example.com");
        assert_eq!(client.get_user_email("jdoe"), Some("jdoe@example.com".to_string()));
    }

    #[test]
    fn map_and_get_commit_change() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        client.map_commit_to_change("abc123", 100);
        assert_eq!(client.get_commit_for_change(100), Some("abc123".to_string()));
        assert_eq!(client.get_change_for_commit("abc123"), Some(100));
    }

    #[test]
    fn get_branch_for_change_returns_none() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert!(client.get_branch_for_change(100).is_none());
    }

    #[test]
    fn shelve_config_returns_none_when_not_set() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert!(client.shelve_config().is_none());
    }

    #[test]
    fn get_shelved_change_for_branch_returns_none_when_not_set() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        assert!(client.get_shelved_change_for_branch("feature/test").is_none());
    }

    #[test]
    fn set_and_get_shelved_change_for_branch() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        client.set_shelved_change_for_branch("feature/test", 12345);
        assert_eq!(client.get_shelved_change_for_branch("feature/test"), Some(12345));
    }

    #[test]
    fn set_shelved_change_overwrites_existing() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        client.set_shelved_change_for_branch("feature/test", 100);
        client.set_shelved_change_for_branch("feature/test", 200);
        assert_eq!(client.get_shelved_change_for_branch("feature/test"), Some(200));
    }

    #[test]
    fn clear_shelved_change_for_branch() {
        let (db, client_id) = setup_prgit_client();
        let client = db.client(client_id).unwrap().unwrap();
        client.set_shelved_change_for_branch("feature/test", 12345);
        client.clear_shelved_change_for_branch("feature/test");
        assert!(client.get_shelved_change_for_branch("feature/test").is_none());
    }
}
