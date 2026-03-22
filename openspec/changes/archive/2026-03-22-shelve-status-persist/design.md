## Context

The `shelve_status` handler checks `ActiveShelves` (an `Arc<Mutex<HashMap>>`) for in-flight shelve state. This works for tracking async shelves in progress but loses all data on restart. The `branch_shelve_mapping` table already stores `(prgit_client_id, branch, shelved_change, shelver_user)` for every completed shelve.

## Goals / Non-Goals

**Goals:**
- Return `done` status for previously-shelved branches after server restart
- No change to the response format

**Non-Goals:**
- Populating `ActiveShelves` from DB on startup (unnecessary overhead)
- Changing the in-memory tracker behavior for in-flight shelves

## Decisions

**DB fallback in the handler, not preloading into memory**

The handler already has access to `db_path` and `client_id` via `AppState`. When `ActiveShelves` returns `None`, open a DB connection and query `get_shelved_change_for_branch`. This is a simple indexed query on a small table — the cost is negligible compared to the git/P4 operations.

**Reuse existing `PrgitClient` methods**

`PrgitClient::get_shelved_change_for_branch` and `get_shelver_for_change` already exist. We also need to get the shelve client name — we can use `shelver_user` as the client identifier since that's what's stored.

## Risks / Trade-offs

- **DB open per status query** — acceptable for this low-frequency endpoint. If it becomes hot, we can add connection pooling later.
- **Stale data** — if someone manually deletes the mapping from DB, the endpoint will still return 404. This is correct behavior.
