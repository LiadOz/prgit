use rusqlite::Connection;

use crate::mirror::{IntegrateStrategy, MirrorData};

pub struct DBMirrorData {
    conn: Connection,
    prgit_client_id: u64,
    p4_client: String,
    integrate_strategy: IntegrateStrategy,
    max_changes_query: Option<usize>,
}

impl DBMirrorData {
    pub(super) fn new(conn: Connection, prgit_client_id: u64) -> Self {
        let (p4_client, integrate_strategy, max_changes_query) = conn
            .query_row(
                "SELECT p.client_name, r.integrate_strategy, r.max_changes_query
                 FROM prgit_clients p
                 JOIN prgit_repos r ON p.id = r.prgit_client_id
                 WHERE p.id = ?1",
                [prgit_client_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        IntegrateStrategy::from_db(row.get(1)?),
                        row.get::<_, Option<i64>>(2)?.map(|v| v as usize),
                    ))
                },
            )
            .expect("prgit_client and prgit_repo must exist");

        Self {
            conn,
            prgit_client_id,
            p4_client,
            integrate_strategy,
            max_changes_query,
        }
    }
}

impl MirrorData for DBMirrorData {
    fn last_sync_change(&self) -> usize {
        self.conn
            .query_row(
                "SELECT last_sync_change FROM prgit_repos WHERE prgit_client_id = ?1",
                [self.prgit_client_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|v| v as usize)
            .unwrap_or(0)
    }

    fn set_last_sync_change(&mut self, change: usize) {
        let _ = self.conn.execute(
            "UPDATE prgit_repos SET last_sync_change = ?1 WHERE prgit_client_id = ?2",
            rusqlite::params![change as i64, self.prgit_client_id],
        );
    }

    fn get_related_branch(&self, change: usize) -> Option<String> {
        self.conn
            .query_row(
                "SELECT branch FROM branch_mapping WHERE prgit_client_id = ?1 AND change = ?2",
                rusqlite::params![self.prgit_client_id, change as i64],
                |row| row.get(0),
            )
            .ok()
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
        &self.p4_client
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

    fn setup_mirror_data() -> DBMirrorData {
        let db = Database::open(":memory:").unwrap();
        let client_id = db
            .create_prgit_client("test-client", "/usr/bin/p4", "localhost:1666", "testuser")
            .unwrap();
        db.create_prgit_repo(client_id, "/path/to/repo", IntegrateStrategy::MergeOurs, Some(50))
            .unwrap();
        db.mirror_data(client_id)
    }

    #[test]
    fn mirror_data_returns_correct_p4_client() {
        let md = setup_mirror_data();
        assert_eq!(md.p4_client(), "test-client");
    }

    #[test]
    fn mirror_data_returns_correct_integrate_strategy() {
        let md = setup_mirror_data();
        assert!(matches!(md.integrate_strategy(), IntegrateStrategy::MergeOurs));
    }

    #[test]
    fn mirror_data_returns_correct_max_changes_query() {
        let md = setup_mirror_data();
        assert_eq!(md.max_changes_query(), Some(50));
    }

    #[test]
    fn last_sync_change_defaults_to_zero() {
        let md = setup_mirror_data();
        assert_eq!(md.last_sync_change(), 0);
    }

    #[test]
    fn set_and_get_last_sync_change() {
        let mut md = setup_mirror_data();
        md.set_last_sync_change(12345);
        assert_eq!(md.last_sync_change(), 12345);
    }

    #[test]
    fn get_user_email_returns_none_when_not_set() {
        let md = setup_mirror_data();
        assert!(md.get_user_email("unknown").is_none());
    }

    #[test]
    fn set_and_get_user_email() {
        let mut md = setup_mirror_data();
        md.set_user_email("jdoe", "jdoe@example.com");
        assert_eq!(md.get_user_email("jdoe"), Some("jdoe@example.com".to_string()));
    }

    #[test]
    fn set_user_email_overwrites_existing() {
        let mut md = setup_mirror_data();
        md.set_user_email("jdoe", "old@example.com");
        md.set_user_email("jdoe", "new@example.com");
        assert_eq!(md.get_user_email("jdoe"), Some("new@example.com".to_string()));
    }

    #[test]
    fn get_related_branch_returns_none_when_not_set() {
        let md = setup_mirror_data();
        assert!(md.get_related_branch(100).is_none());
    }
}
