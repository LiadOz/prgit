use chrono::Utc;
use p4rs::P4;
use rusqlite::Connection;
use thiserror::Error;

const KEYRING_SERVICE: &str = "prgit";

#[derive(Error, Debug)]
pub enum TicketStoreError {
    #[error("No ticket stored for {p4port}:{p4user}")]
    NoTicketStored { p4port: String, p4user: String },
    #[error("Ticket expired for {p4port}:{p4user}")]
    TicketExpired { p4port: String, p4user: String },
    #[error("Keyring unavailable: {0}")]
    KeyringUnavailable(String),
    #[error("Ticket is invalid: {0}")]
    TicketInvalid(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

pub trait SecretBackend {
    fn set_secret(&self, key: &str, value: &str) -> Result<(), TicketStoreError>;
    fn get_secret(&self, key: &str) -> Result<String, TicketStoreError>;
    fn delete_secret(&self, key: &str) -> Result<(), TicketStoreError>;
}

pub struct KeyringBackend;

impl SecretBackend for KeyringBackend {
    fn set_secret(&self, key: &str, value: &str) -> Result<(), TicketStoreError> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, key)
            .map_err(|e| TicketStoreError::KeyringUnavailable(e.to_string()))?;
        entry
            .set_password(value)
            .map_err(|e| TicketStoreError::KeyringUnavailable(e.to_string()))
    }

    fn get_secret(&self, key: &str) -> Result<String, TicketStoreError> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, key)
            .map_err(|e| TicketStoreError::KeyringUnavailable(e.to_string()))?;
        entry.get_password().map_err(|e| match e {
            keyring::Error::NoEntry => {
                // Parse the key back to p4port:p4user for the error message
                let (p4port, p4user) = key.split_once(':').unwrap_or(("", key));
                TicketStoreError::NoTicketStored {
                    p4port: p4port.to_string(),
                    p4user: p4user.to_string(),
                }
            }
            _ => TicketStoreError::KeyringUnavailable(e.to_string()),
        })
    }

    fn delete_secret(&self, key: &str) -> Result<(), TicketStoreError> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, key)
            .map_err(|e| TicketStoreError::KeyringUnavailable(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(TicketStoreError::KeyringUnavailable(e.to_string())),
        }
    }
}

pub struct TicketStore<'a, B: SecretBackend = KeyringBackend> {
    conn: &'a Connection,
    backend: B,
}

impl<'a> TicketStore<'a, KeyringBackend> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            backend: KeyringBackend,
        }
    }
}

impl<'a, B: SecretBackend> TicketStore<'a, B> {
    pub fn with_backend(conn: &'a Connection, backend: B) -> Self {
        Self { conn, backend }
    }

    fn keyring_username(p4port: &str, p4user: &str) -> String {
        format!("{}:{}", p4port, p4user)
    }

    /// Store a ticket in the keyring and record expiry metadata in SQLite.
    pub fn store_ticket(
        &self,
        p4port: &str,
        p4user: &str,
        ticket: &str,
        expires_at: i64,
    ) -> Result<(), TicketStoreError> {
        let key = Self::keyring_username(p4port, p4user);
        self.backend.set_secret(&key, ticket)?;

        let now = Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR REPLACE INTO ticket_metadata (p4port, p4user, expires_at, stored_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![p4port, p4user, expires_at, now],
        )?;

        Ok(())
    }

    /// Retrieve a ticket from the keyring.
    pub fn get_ticket(&self, p4port: &str, p4user: &str) -> Result<String, TicketStoreError> {
        let key = Self::keyring_username(p4port, p4user);
        self.backend.get_secret(&key)
    }

    /// Delete a ticket from the keyring and remove expiry metadata.
    pub fn delete_ticket(&self, p4port: &str, p4user: &str) -> Result<(), TicketStoreError> {
        let key = Self::keyring_username(p4port, p4user);
        self.backend.delete_secret(&key)?;

        self.conn.execute(
            "DELETE FROM ticket_metadata WHERE p4port = ?1 AND p4user = ?2",
            rusqlite::params![p4port, p4user],
        )?;

        Ok(())
    }

    /// Check if the ticket for a given user/port has expired based on stored metadata.
    pub fn is_expired(&self, p4port: &str, p4user: &str) -> Result<bool, TicketStoreError> {
        let expires_at: i64 = self
            .conn
            .query_row(
                "SELECT expires_at FROM ticket_metadata WHERE p4port = ?1 AND p4user = ?2",
                rusqlite::params![p4port, p4user],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => TicketStoreError::NoTicketStored {
                    p4port: p4port.to_string(),
                    p4user: p4user.to_string(),
                },
                _ => TicketStoreError::Database(e),
            })?;

        let now = Utc::now().timestamp();
        Ok(now >= expires_at)
    }

    /// Build an authenticated P4 instance from a stored ticket.
    /// Returns an error if no ticket is stored or the ticket has expired.
    pub fn authenticated_p4(&self, p4port: &str, p4user: &str) -> Result<P4, TicketStoreError> {
        if self.is_expired(p4port, p4user)? {
            return Err(TicketStoreError::TicketExpired {
                p4port: p4port.to_string(),
                p4user: p4user.to_string(),
            });
        }

        let ticket = self.get_ticket(p4port, p4user)?;
        Ok(P4::new().port(p4port).p4_user(p4user).password(ticket))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cabinet::Database;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockBackend {
        store: Mutex<HashMap<String, String>>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    impl SecretBackend for MockBackend {
        fn set_secret(&self, key: &str, value: &str) -> Result<(), TicketStoreError> {
            self.store
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn get_secret(&self, key: &str) -> Result<String, TicketStoreError> {
            self.store.lock().unwrap().get(key).cloned().ok_or_else(|| {
                TicketStoreError::NoTicketStored {
                    p4port: String::new(),
                    p4user: key.to_string(),
                }
            })
        }

        fn delete_secret(&self, key: &str) -> Result<(), TicketStoreError> {
            self.store.lock().unwrap().remove(key);
            Ok(())
        }
    }

    fn test_store(db: &Database) -> TicketStore<'_, MockBackend> {
        TicketStore::with_backend(db.conn(), MockBackend::new())
    }

    #[test]
    fn store_and_retrieve_ticket() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let future = Utc::now().timestamp() + 3600;
        store
            .store_ticket("localhost:1666", "bob", "TICKET123", future)
            .unwrap();

        let ticket = store.get_ticket("localhost:1666", "bob").unwrap();
        assert_eq!(ticket, "TICKET123");
    }

    #[test]
    fn get_ticket_not_stored() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let err = store.get_ticket("localhost:1666", "bob").unwrap_err();
        assert!(matches!(err, TicketStoreError::NoTicketStored { .. }));
    }

    #[test]
    fn overwrite_existing_ticket() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let future = Utc::now().timestamp() + 3600;
        store
            .store_ticket("localhost:1666", "bob", "OLD_TICKET", future)
            .unwrap();
        store
            .store_ticket("localhost:1666", "bob", "NEW_TICKET", future)
            .unwrap();

        let ticket = store.get_ticket("localhost:1666", "bob").unwrap();
        assert_eq!(ticket, "NEW_TICKET");
    }

    #[test]
    fn delete_ticket() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let future = Utc::now().timestamp() + 3600;
        store
            .store_ticket("localhost:1666", "bob", "TICKET123", future)
            .unwrap();
        store.delete_ticket("localhost:1666", "bob").unwrap();

        let err = store.get_ticket("localhost:1666", "bob").unwrap_err();
        assert!(matches!(err, TicketStoreError::NoTicketStored { .. }));
    }

    #[test]
    fn delete_nonexistent_ticket_is_ok() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);
        store.delete_ticket("localhost:1666", "nobody").unwrap();
    }

    #[test]
    fn is_expired_with_future_expiry() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let future = Utc::now().timestamp() + 3600;
        store
            .store_ticket("localhost:1666", "bob", "TICKET", future)
            .unwrap();

        assert!(!store.is_expired("localhost:1666", "bob").unwrap());
    }

    #[test]
    fn is_expired_with_past_expiry() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let past = Utc::now().timestamp() - 3600;
        store
            .store_ticket("localhost:1666", "bob", "TICKET", past)
            .unwrap();

        assert!(store.is_expired("localhost:1666", "bob").unwrap());
    }

    #[test]
    fn is_expired_no_metadata() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let err = store.is_expired("localhost:1666", "bob").unwrap_err();
        assert!(matches!(err, TicketStoreError::NoTicketStored { .. }));
    }

    #[test]
    fn authenticated_p4_with_valid_ticket() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let future = Utc::now().timestamp() + 3600;
        store
            .store_ticket("localhost:1666", "bob", "TICKET123", future)
            .unwrap();

        let p4 = store.authenticated_p4("localhost:1666", "bob").unwrap();
        // P4 was constructed — we can't inspect private fields, but no error means success
        drop(p4);
    }

    #[test]
    fn authenticated_p4_with_expired_ticket() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let past = Utc::now().timestamp() - 3600;
        store
            .store_ticket("localhost:1666", "bob", "TICKET", past)
            .unwrap();

        let result = store.authenticated_p4("localhost:1666", "bob");
        assert!(matches!(
            result,
            Err(TicketStoreError::TicketExpired { .. })
        ));
    }

    #[test]
    fn authenticated_p4_with_no_ticket() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let result = store.authenticated_p4("localhost:1666", "bob");
        assert!(matches!(
            result,
            Err(TicketStoreError::NoTicketStored { .. })
        ));
    }

    #[test]
    fn multiple_users_same_port() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let future = Utc::now().timestamp() + 3600;
        store
            .store_ticket("localhost:1666", "alice", "ALICE_TKT", future)
            .unwrap();
        store
            .store_ticket("localhost:1666", "bob", "BOB_TKT", future)
            .unwrap();

        assert_eq!(
            store.get_ticket("localhost:1666", "alice").unwrap(),
            "ALICE_TKT"
        );
        assert_eq!(
            store.get_ticket("localhost:1666", "bob").unwrap(),
            "BOB_TKT"
        );
    }

    #[test]
    fn same_user_different_ports() {
        let db = Database::open(":memory:").unwrap();
        let store = test_store(&db);

        let future = Utc::now().timestamp() + 3600;
        store
            .store_ticket("server1:1666", "bob", "TKT_S1", future)
            .unwrap();
        store
            .store_ticket("server2:1666", "bob", "TKT_S2", future)
            .unwrap();

        assert_eq!(store.get_ticket("server1:1666", "bob").unwrap(), "TKT_S1");
        assert_eq!(store.get_ticket("server2:1666", "bob").unwrap(), "TKT_S2");
    }
}
