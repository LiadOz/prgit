## 1. Event types and emitter infrastructure

- [x] 1.1 Define `ObservabilityEvent` enum with all event variants and typed fields (request, push, shelve, mirror, auth) in `src/window/observability.rs`
- [x] 1.2 Implement `EventEmitter` struct wrapping `tokio::sync::mpsc::Sender<ObservabilityEvent>` with a `try_emit()` method that logs on failure
- [x] 1.3 Add `EventEmitter` field to `AppState` and wire up channel creation in `build_app()`

## 2. Event storage

- [x] 2.1 Add `events` table schema to database initialization (id, event_type, timestamp, repo, user, payload JSON)
- [x] 2.2 Add index on (event_type, timestamp) and (repo, timestamp) columns
- [x] 2.3 Implement collector background task that drains the mpsc channel and writes to SQLite
- [x] 2.4 Implement retention pruning (DELETE events older than configurable period, runs hourly in collector)

## 3. Emit request events

- [x] 3.1 Emit `request.completed` event in `handle_git_request` with method, git_service, request_bytes, response_bytes, user, duration_ms

## 4. Emit push events

- [x] 4.1 Emit `push.received` event with payload_bytes and ref_count after parsing ref updates
- [x] 4.2 Emit `push.branch_created`, `push.branch_updated`, `push.branch_deleted` per ref update based on old/new SHA
- [x] 4.3 Emit `push.rejected` event when synced branch protection triggers

## 5. Emit shelve events

- [x] 5.1 Emit `shelve.started` before invoking the shelver (with async flag)
- [x] 5.2 Emit `shelve.completed` or `shelve.reshelved` on success (with changelist, client_name, duration_ms, file_count, async, commits_in_branch)
- [x] 5.3 Add `commits_in_branch` count via `repo.revwalk()` in the shelver, return it in `ShelveResult`
- [x] 5.4 Distinguish first shelve vs reshelve based on existing_shelve being Some/None
- [x] 5.5 Emit `shelve.failed` on shelver error (with error, duration_ms, async flag)

## 6. Emit mirror events

- [x] 6.1 Emit `mirror.cycle_started` and `mirror.cycle_completed`/`mirror.cycle_failed` in the mirror task loop with duration_ms and changes_synced
- [x] 6.2 Emit `mirror.change_committed` per change with p4_change, commit_hash, user, file_count, duration_ms, merge_parent, merge_strategy
- [x] 6.3 Emit `shelve.merged` when `get_related_branch` returns a hit during mirror processing (with shelved_cl, submitted_cl, branch, shelver_user)

## 7. Emit auth events

- [x] 7.1 Emit `auth.failed` in `authenticate_push` on ticket validation failure or missing credentials

## 8. API endpoints

- [x] 8.1 Implement `GET /api/v1/events` with query params: event_type, since, until, repo, user, limit
- [x] 8.2 Implement `GET /api/v1/events/counts` returning event_type → count aggregation with same filters
- [x] 8.3 Implement `GET /api/v1/events/users` returning distinct users with push_count and active_branches
- [x] 8.4 Add routes to the axum router in `build_app()`

## 9. Configuration

- [x] 9.1 Add optional `observability` section to `ServerConfig` with channel_capacity (default 4096) and retention_days (default 30)

## 10. Testing

- [x] 10.1 Unit test: EventEmitter try_emit succeeds and event appears on receiver
- [x] 10.2 Unit test: EventEmitter try_emit on full channel does not panic, logs warning
- [x] 10.3 Integration test: push triggers push.received and push.branch_created events queryable via API
- [x] 10.4 Integration test: events API returns correct filtered results
- [x] 10.5 Unit test: retention pruning deletes old events and keeps recent ones
