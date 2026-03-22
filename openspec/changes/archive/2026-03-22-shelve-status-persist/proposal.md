## Why

The shelve status endpoint (`GET /api/v1/repos/{group}/{name}/shelve/status/{branch}`) only queries the in-memory `ActiveShelves` tracker. After a server restart, all completed shelve statuses are lost and return 404, even though the shelve data is persisted in `branch_shelve_mapping`. This makes the endpoint unreliable for any client that polls after a restart.

## What Changes

- Fall back to querying the database (`branch_shelve_mapping`) when the in-memory tracker has no entry for a branch
- Return `done` status with the changelist and shelver from the DB when a mapping exists

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `shelve-status`: The endpoint now returns persisted shelve results after server restart, not just in-flight/recent statuses.

## Impact

- `src/window/handlers.rs` — `shelve_status` handler gains a DB fallback
- No API contract change — the response shape is identical (`done` state with changelist + client)

## Observability

No new observability needed — the existing `shelve.completed` events already cover successful shelves.
