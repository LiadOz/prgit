use rusqlite::Connection;

use crate::mirror::{IntegrateStrategy, MirrorData};

pub struct DBMirrorData {
    conn: Connection,
    p4_client: String,
    integrate_strategy: IntegrateStrategy,
    max_changes_query: Option<usize>,
}

impl DBMirrorData {
    pub(super) fn new(
        conn: Connection,
        p4_client: String,
        integrate_strategy: IntegrateStrategy,
        max_changes_query: Option<usize>,
    ) -> Self {
        Self {
            conn,
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
                "SELECT last_sync_change FROM sync_state WHERE id = 1",
                [],
                |row| row.get::<_, usize>(0),
            )
            .unwrap_or(0)
    }

    fn set_last_sync_change(&mut self, change: usize) {
        let _ = self.conn.execute(
            "UPDATE sync_state SET last_sync_change = ?1 WHERE id = 1",
            [change],
        );
    }

    fn get_related_branch(&self, change: usize) -> Option<String> {
        self.conn
            .query_row(
                "SELECT branch FROM branch_mapping WHERE change = ?1",
                [change],
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

