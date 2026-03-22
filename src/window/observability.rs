use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum ObservabilityEvent {
    // Request events
    #[serde(rename = "request.completed")]
    RequestCompleted {
        timestamp: DateTime<Utc>,
        repo: String,
        method: String,
        git_service: String,
        request_bytes: usize,
        response_bytes: usize,
        user: Option<String>,
        duration_ms: u64,
    },

    // Push events
    #[serde(rename = "push.received")]
    PushReceived {
        timestamp: DateTime<Utc>,
        user: String,
        repo: String,
        payload_bytes: usize,
        ref_count: usize,
    },
    #[serde(rename = "push.branch_created")]
    PushBranchCreated {
        timestamp: DateTime<Utc>,
        user: String,
        repo: String,
        branch: String,
    },
    #[serde(rename = "push.branch_updated")]
    PushBranchUpdated {
        timestamp: DateTime<Utc>,
        user: String,
        repo: String,
        branch: String,
    },
    #[serde(rename = "push.branch_deleted")]
    PushBranchDeleted {
        timestamp: DateTime<Utc>,
        user: String,
        repo: String,
        branch: String,
    },
    #[serde(rename = "push.rejected")]
    PushRejected {
        timestamp: DateTime<Utc>,
        user: Option<String>,
        repo: String,
        branch: Option<String>,
        reason: String,
    },

    // Shelve events
    #[serde(rename = "shelve.started")]
    ShelveStarted {
        timestamp: DateTime<Utc>,
        user: String,
        repo: String,
        branch: String,
        r#async: bool,
    },
    #[serde(rename = "shelve.completed")]
    ShelveCompleted {
        timestamp: DateTime<Utc>,
        user: String,
        repo: String,
        branch: String,
        changelist: usize,
        client_name: String,
        duration_ms: u64,
        file_count: usize,
        r#async: bool,
        commits_in_branch: usize,
    },
    #[serde(rename = "shelve.reshelved")]
    ShelveReshelved {
        timestamp: DateTime<Utc>,
        user: String,
        repo: String,
        branch: String,
        changelist: usize,
        client_name: String,
        duration_ms: u64,
        file_count: usize,
        r#async: bool,
        commits_in_branch: usize,
    },
    #[serde(rename = "shelve.failed")]
    ShelveFailed {
        timestamp: DateTime<Utc>,
        user: String,
        repo: String,
        branch: String,
        error: String,
        duration_ms: u64,
        r#async: bool,
    },
    #[serde(rename = "shelve.merged")]
    ShelveMerged {
        timestamp: DateTime<Utc>,
        repo: String,
        branch: String,
        shelved_cl: usize,
        submitted_cl: usize,
        shelver_user: String,
    },

    // Mirror events
    #[serde(rename = "mirror.cycle_started")]
    MirrorCycleStarted {
        timestamp: DateTime<Utc>,
        repo: String,
        last_sync_change: usize,
    },
    #[serde(rename = "mirror.cycle_completed")]
    MirrorCycleCompleted {
        timestamp: DateTime<Utc>,
        repo: String,
        changes_synced: usize,
        new_last_sync: usize,
        duration_ms: u64,
    },
    #[serde(rename = "mirror.cycle_failed")]
    MirrorCycleFailed {
        timestamp: DateTime<Utc>,
        repo: String,
        error: String,
        duration_ms: u64,
    },
    #[serde(rename = "mirror.change_committed")]
    MirrorChangeCommitted {
        timestamp: DateTime<Utc>,
        repo: String,
        p4_change: usize,
        commit_hash: String,
        user: String,
        file_count: usize,
        duration_ms: u64,
        merge_parent: Option<String>,
        merge_strategy: Option<String>,
    },

    #[serde(rename = "mirror.file_skipped")]
    MirrorFileSkipped {
        timestamp: DateTime<Utc>,
        repo: String,
        p4_change: usize,
        depot_path: String,
        reason: String,
    },

    // Auth events
    #[serde(rename = "auth.failed")]
    AuthFailed {
        timestamp: DateTime<Utc>,
        user: Option<String>,
        repo: String,
        reason: String,
    },
}

impl ObservabilityEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::RequestCompleted { .. } => "request.completed",
            Self::PushReceived { .. } => "push.received",
            Self::PushBranchCreated { .. } => "push.branch_created",
            Self::PushBranchUpdated { .. } => "push.branch_updated",
            Self::PushBranchDeleted { .. } => "push.branch_deleted",
            Self::PushRejected { .. } => "push.rejected",
            Self::ShelveStarted { .. } => "shelve.started",
            Self::ShelveCompleted { .. } => "shelve.completed",
            Self::ShelveReshelved { .. } => "shelve.reshelved",
            Self::ShelveFailed { .. } => "shelve.failed",
            Self::ShelveMerged { .. } => "shelve.merged",
            Self::MirrorCycleStarted { .. } => "mirror.cycle_started",
            Self::MirrorCycleCompleted { .. } => "mirror.cycle_completed",
            Self::MirrorCycleFailed { .. } => "mirror.cycle_failed",
            Self::MirrorChangeCommitted { .. } => "mirror.change_committed",
            Self::MirrorFileSkipped { .. } => "mirror.file_skipped",
            Self::AuthFailed { .. } => "auth.failed",
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::RequestCompleted { timestamp, .. }
            | Self::PushReceived { timestamp, .. }
            | Self::PushBranchCreated { timestamp, .. }
            | Self::PushBranchUpdated { timestamp, .. }
            | Self::PushBranchDeleted { timestamp, .. }
            | Self::PushRejected { timestamp, .. }
            | Self::ShelveStarted { timestamp, .. }
            | Self::ShelveCompleted { timestamp, .. }
            | Self::ShelveReshelved { timestamp, .. }
            | Self::ShelveFailed { timestamp, .. }
            | Self::ShelveMerged { timestamp, .. }
            | Self::MirrorCycleStarted { timestamp, .. }
            | Self::MirrorCycleCompleted { timestamp, .. }
            | Self::MirrorCycleFailed { timestamp, .. }
            | Self::MirrorChangeCommitted { timestamp, .. }
            | Self::MirrorFileSkipped { timestamp, .. }
            | Self::AuthFailed { timestamp, .. } => *timestamp,
        }
    }

    pub fn repo(&self) -> &str {
        match self {
            Self::RequestCompleted { repo, .. }
            | Self::PushReceived { repo, .. }
            | Self::PushBranchCreated { repo, .. }
            | Self::PushBranchUpdated { repo, .. }
            | Self::PushBranchDeleted { repo, .. }
            | Self::PushRejected { repo, .. }
            | Self::ShelveStarted { repo, .. }
            | Self::ShelveCompleted { repo, .. }
            | Self::ShelveReshelved { repo, .. }
            | Self::ShelveFailed { repo, .. }
            | Self::ShelveMerged { repo, .. }
            | Self::MirrorCycleStarted { repo, .. }
            | Self::MirrorCycleCompleted { repo, .. }
            | Self::MirrorCycleFailed { repo, .. }
            | Self::MirrorChangeCommitted { repo, .. }
            | Self::MirrorFileSkipped { repo, .. }
            | Self::AuthFailed { repo, .. } => repo,
        }
    }

    pub fn user(&self) -> Option<&str> {
        match self {
            Self::RequestCompleted { user, .. } => user.as_deref(),
            Self::PushReceived { user, .. }
            | Self::PushBranchCreated { user, .. }
            | Self::PushBranchUpdated { user, .. }
            | Self::PushBranchDeleted { user, .. }
            | Self::ShelveStarted { user, .. }
            | Self::ShelveCompleted { user, .. }
            | Self::ShelveReshelved { user, .. }
            | Self::ShelveFailed { user, .. } => Some(user),
            Self::ShelveMerged { shelver_user, .. } => Some(shelver_user),
            Self::MirrorChangeCommitted { user, .. } => Some(user),
            Self::PushRejected { user, .. } | Self::AuthFailed { user, .. } => user.as_deref(),
            Self::MirrorCycleStarted { .. }
            | Self::MirrorCycleCompleted { .. }
            | Self::MirrorCycleFailed { .. }
            | Self::MirrorFileSkipped { .. } => None,
        }
    }
}

/// Non-blocking event emitter. Wraps a bounded channel sender.
/// All emit operations are best-effort: failures are logged, never propagated.
#[derive(Clone)]
pub struct EventEmitter {
    sender: tokio::sync::mpsc::Sender<ObservabilityEvent>,
}

impl EventEmitter {
    pub fn new(sender: tokio::sync::mpsc::Sender<ObservabilityEvent>) -> Self {
        Self { sender }
    }

    /// Try to emit an event. Never blocks, never panics.
    /// Returns true if sent, false if dropped (channel full or closed).
    pub fn try_emit(&self, event: ObservabilityEvent) -> bool {
        match self.sender.try_send(event) {
            Ok(()) => true,
            Err(tokio::sync::mpsc::error::TrySendError::Full(e)) => {
                log::warn!("Event channel full, dropping {} event", e.event_type());
                false
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(e)) => {
                log::warn!("Event channel closed, dropping {} event", e.event_type());
                false
            }
        }
    }
}

/// Spawn the collector task that drains events and writes to SQLite.
pub fn spawn_collector(
    mut receiver: tokio::sync::mpsc::Receiver<ObservabilityEvent>,
    db_path: String,
    retention_days: u32,
) {
    tokio::spawn(async move {
        let mut last_prune = std::time::Instant::now();
        let prune_interval = std::time::Duration::from_secs(3600);

        while let Some(event) = receiver.recv().await {
            if let Err(e) = write_event(&db_path, &event) {
                log::warn!("Failed to write {} event: {e}", event.event_type());
            }

            if last_prune.elapsed() >= prune_interval {
                if let Err(e) = prune_events(&db_path, retention_days) {
                    log::warn!("Failed to prune events: {e}");
                }
                last_prune = std::time::Instant::now();
            }
        }
        log::info!("Event collector shutting down");
    });
}

fn write_event(
    db_path: &str,
    event: &ObservabilityEvent,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let conn = rusqlite::Connection::open(db_path)?;
    let payload = serde_json::to_string(event)?;
    let timestamp_ms = event.timestamp().timestamp_millis();
    conn.execute(
        "INSERT INTO events (event_type, timestamp, repo, user, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            event.event_type(),
            timestamp_ms,
            event.repo(),
            event.user(),
            payload,
        ],
    )?;
    Ok(())
}

fn prune_events(
    db_path: &str,
    retention_days: u32,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let conn = rusqlite::Connection::open(db_path)?;
    let cutoff_ms = (Utc::now() - chrono::Duration::days(retention_days as i64)).timestamp_millis();
    let deleted = conn.execute(
        "DELETE FROM events WHERE timestamp < ?1",
        rusqlite::params![cutoff_ms],
    )?;
    if deleted > 0 {
        log::info!("Pruned {deleted} events older than {retention_days} days");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_emitter_try_emit_succeeds() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let emitter = EventEmitter::new(tx);

        let sent = emitter.try_emit(ObservabilityEvent::PushReceived {
            timestamp: Utc::now(),
            user: "alice".into(),
            repo: "depot/main".into(),
            payload_bytes: 1024,
            ref_count: 1,
        });
        assert!(sent);

        let event = rx.recv().await.unwrap();
        assert_eq!(event.event_type(), "push.received");
    }

    #[tokio::test]
    async fn test_emitter_try_emit_full_channel_does_not_panic() {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let emitter = EventEmitter::new(tx);

        // Fill the channel
        emitter.try_emit(ObservabilityEvent::PushReceived {
            timestamp: Utc::now(),
            user: "alice".into(),
            repo: "depot/main".into(),
            payload_bytes: 0,
            ref_count: 0,
        });

        // This should not panic
        let sent = emitter.try_emit(ObservabilityEvent::PushReceived {
            timestamp: Utc::now(),
            user: "bob".into(),
            repo: "depot/main".into(),
            payload_bytes: 0,
            ref_count: 0,
        });
        assert!(!sent);
    }

    #[test]
    fn test_retention_pruning() {
        let db = crate::cabinet::Database::open(":memory:").unwrap();
        let conn = db.conn();

        // Insert an old event (60 days ago) and a recent event
        let old_ts = (Utc::now() - chrono::Duration::days(60)).timestamp_millis();
        let new_ts = Utc::now().timestamp_millis();

        conn.execute(
            "INSERT INTO events (event_type, timestamp, repo, user, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["push.received", old_ts, "depot/main", "alice", "{}"],
        ).unwrap();
        conn.execute(
            "INSERT INTO events (event_type, timestamp, repo, user, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["push.received", new_ts, "depot/main", "bob", "{}"],
        ).unwrap();

        // Verify 2 rows
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        // Prune with 30-day retention — should delete the old one
        let cutoff_ms = (Utc::now() - chrono::Duration::days(30)).timestamp_millis();
        conn.execute(
            "DELETE FROM events WHERE timestamp < ?1",
            rusqlite::params![cutoff_ms],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // Remaining event should be the recent one
        let remaining_user: String = conn
            .query_row("SELECT user FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(remaining_user, "bob");
    }
}
