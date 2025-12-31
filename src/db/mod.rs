mod mirror_data;

use rusqlite::Connection;

pub use mirror_data::DBMirrorData;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sync_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                last_sync_change INTEGER NOT NULL DEFAULT 0
            );
            INSERT OR IGNORE INTO sync_state (id, last_sync_change) VALUES (1, 0);
            CREATE TABLE IF NOT EXISTS branch_mapping (
                change INTEGER PRIMARY KEY,
                branch TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_mapping (
                user TEXT PRIMARY KEY,
                email TEXT NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn mirror_data(
        self,
        p4_client: String,
        integrate_strategy: crate::mirror::IntegrateStrategy,
        max_changes_query: Option<usize>,
    ) -> DBMirrorData {
        DBMirrorData::new(self.conn, p4_client, integrate_strategy, max_changes_query)
    }
}

