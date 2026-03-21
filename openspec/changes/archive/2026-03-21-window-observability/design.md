## Context

prgit currently logs operational information via unstructured `log::info!` and `log::error!` calls. There is no structured data collection, making it impossible to answer questions about usage patterns, user adoption, mirror health, or network capacity without parsing log files.

The system already has:
- SQLite database (cabinet) for persistent state
- `tokio` async runtime with `spawn_blocking` for CPU/IO work
- `AppState` shared via `Arc` across all handlers
- Clear event boundaries: push handling, shelve lifecycle, mirror cycles

## Goals / Non-Goals

**Goals:**
- Structured event collection at all key system boundaries
- Zero impact on request latency (async, non-blocking collection)
- Durable local storage queryable via API
- Graceful degradation: collection failures never affect core functionality
- Answer: user adoption, branch lifecycle, mirror health, push sizes, feature usage

**Non-Goals:**
- Multi-server aggregation (deferred to future work)
- Real-time dashboards or alerting
- Prometheus/OpenTelemetry integration (can be layered later on the event store)
- Historical backfill of existing data

## Decisions

### 1. Event type: Rust enum with serde

**Decision:** Define an `ObservabilityEvent` enum with a variant per event type. Each variant carries typed fields. Serialize to JSON for storage.

**Why:** Type safety at emit sites catches missing fields at compile time. JSON payload in SQLite keeps the schema simple (one table) while preserving queryability. Alternatives considered:
- Structured log lines (hard to query programmatically)
- Separate tables per event type (schema migration burden, many tables)
- Protocol buffers (overkill, adds dependency)

### 2. Bounded mpsc channel for decoupling

**Decision:** `tokio::sync::mpsc::channel` with a fixed capacity (e.g., 4096). Emitters call `try_send()`. A single collector task drains and writes to SQLite.

**Why:** `try_send` is non-blocking and infallible (returns Err if full, which we log and discard). This guarantees zero latency impact on handlers and the mirror. Alternatives considered:
- Direct SQLite writes in handlers (adds latency, risks blocking async runtime)
- `broadcast` channel (unnecessary, only one consumer)
- Unbounded channel (memory risk under load)

### 3. Single events table with JSON payload

**Decision:** One table: `events(id INTEGER PRIMARY KEY, event_type TEXT, timestamp INTEGER, repo TEXT, user TEXT, payload TEXT)`. The `event_type`, `repo`, and `user` columns are extracted for indexing; `payload` holds the full JSON.

**Why:** Keeps schema evolution simple — adding new event types requires no migration. Indexed columns enable efficient filtering for the common queries (by type, repo, user, time range). The full payload is available for ad-hoc analysis.

### 4. EventEmitter handle in AppState

**Decision:** Add an `EventEmitter` struct (wrapping the `mpsc::Sender`) to `AppState`. Pass it to handlers via axum state extraction. For the mirror task, pass a clone of the sender.

**Why:** Follows the existing pattern — handlers already receive `State(state): State<Arc<AppState>>`. The emitter is just another field on the shared state. No new patterns to learn.

### 5. commits_in_branch via revwalk

**Decision:** Count commits between merge base and branch tip using `repo.revwalk()` from target_oid, hiding commits reachable from base_oid. This runs inside the shelver's existing `spawn_blocking` context.

**Why:** Cheap git operation, both OIDs are already available in `Shelver::shelve`. No additional I/O. The revwalk is bounded by branch length (typically small).

### 6. shelve.merged detection in mirror

**Decision:** In `Mirror::process_change`, after `get_related_branch` returns a hit, emit `shelve.merged` with the branch name, shelved CL, submitted CL, and shelver user (looked up from branch_shelve_mapping).

**Why:** This is the only point where we know a shelved branch was consumed in P4. The data is already available — `get_related_branch` returns the branch, and `get_shelver_for_change` returns the user. No additional P4 queries needed.

### 7. Retention via periodic DELETE

**Decision:** The collector task runs a `DELETE FROM events WHERE timestamp < ?` every hour. Default retention: 30 days, configurable in server config.

**Why:** Simple, no additional background task. The hourly cadence means at most 1 hour of over-retention, which is acceptable. SQLite handles DELETE efficiently with the timestamp index.

## Risks / Trade-offs

**[Channel capacity too small]** → If the system processes many concurrent pushes, the 4096-element buffer could fill. Mitigation: log dropped events with a counter so we can monitor and tune. Channel size is configurable.

**[SQLite write contention]** → The collector writes to the same database file as the cabinet. Mitigation: SQLite WAL mode (already used) allows concurrent readers. The collector is the only writer to the events table, so no contention with cabinet writes on different tables.

**[JSON payload size]** → Events with large error messages or many fields could bloat the database. Mitigation: retention policy prunes old data. Event payloads are bounded by design (no file contents, just metadata).

**[Mirror event emission requires MirrorData trait change]** → The mirror operates through the `MirrorData` trait. To emit events from within mirror processing, we either pass the emitter through the trait or return event data to the caller. Decision: return event data from `process_change` and let the mirror task emit. This avoids polluting the trait.

## Open Questions

- Should the API endpoints require authentication? Currently leaning no (operational data, not sensitive), but worth revisiting.
- Should we add a `GET /api/v1/events/stream` SSE endpoint for live event tailing? Useful for debugging but adds complexity. Deferred unless requested.
