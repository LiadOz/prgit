## Why

We need data-driven insight into how prgit is used: who benefits, how they work, where performance gaps exist, and whether the git interface is practical at scale. Currently all we have is unstructured log lines. Without structured observability we can't answer basic questions like "how many active users do we have" or "is the mirror keeping up" or "do we need LFS support."

Observability must never obstruct normal operation — collection errors are logged but never block requests or shelving.

## What Changes

- Introduce a structured event system within the window module that emits events at key points in the request, shelve, and mirror lifecycles
- Add a SQLite-backed event store for durable local storage (leveraging the existing cabinet)
- Track 5 event categories: request (network), push, shelve, mirror, and auth
- Expose event data through an API endpoint for querying
- All collection is best-effort: failures are logged, never propagated

## Capabilities

### New Capabilities
- `observability-events`: Defines the event taxonomy — all event types, their fields, and when they are emitted. Covers request, push, shelve, mirror, and auth events.
- `observability-store`: Local SQLite event storage with retention policies. Events are written asynchronously via a bounded channel to avoid blocking request handling.
- `observability-api`: API endpoints for querying collected event data (counts, aggregations, raw events).

### Modified Capabilities
- `push-shelve-intercept`: Emit push and shelve events at existing hook points (push received, branch created/updated/deleted, shelve started/completed/reshelved/failed/merged)
- `mirror-scheduler`: Emit mirror cycle and per-change events (cycle started/completed/failed, change committed with merge info)
- `git-http-serving`: Emit request-level events with network metrics (request/response bytes, duration, git service type)
- `push-auth`: Emit auth failure events

## Impact

- **New files**: `src/window/observability.rs` (event types, channel, store)
- **Modified files**: `src/window/handlers.rs` (emit events at request/push/shelve/auth points), `src/window/mirror_task.rs` (emit mirror events), `src/window/mod.rs` (wire up event channel in AppState), `src/cabinet/database.rs` (event table schema)
- **Dependencies**: None new — uses existing tokio channels and SQLite
- **Performance**: Bounded channel with `try_send` ensures zero latency impact on hot paths. SQLite writes batched on collector task.
