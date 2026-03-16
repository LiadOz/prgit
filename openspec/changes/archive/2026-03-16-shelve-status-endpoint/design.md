## Context

When async shelving is enabled, `shelve_branches()` in `handlers.rs` creates a CL immediately via `do_prepare_shelve()`, returns it to the user in sideband messages, then fires off `complete_pending_shelves()` in a background `spawn_blocking`. There is currently no server-side state tracking which CLs have background shelves in flight — the `PendingShelve` structs are consumed by the background task and forgotten.

The server already uses `Arc<AppState>` shared across all handlers, with axum's `State` extractor. Routes are defined in `build_app()` in `window/mod.rs`.

## Goals / Non-Goals

**Goals:**
- Expose an HTTP endpoint to check if a CL has an active background shelve
- Track active background shelves in server memory
- Clean up tracking state automatically when background shelves complete (or fail)

**Non-Goals:**
- Persisting shelve status across server restarts (in-memory only)
- Tracking historical shelve outcomes (success/failure) — just active/not-active
- Authentication on the status endpoint (matches `/api/health` pattern)

## Decisions

### In-memory `ActiveShelves` tracker on `AppState`

**Decision:** Add an `ActiveShelves` struct wrapping `Arc<Mutex<HashSet<usize>>>` to `AppState`. The set contains CL numbers with background shelves currently in progress.

**Rationale:** A `HashSet<usize>` is the simplest possible data structure — CLs are unique identifiers and we only need presence/absence. `Mutex` (not `RwLock`) is fine because contention is minimal: writes happen at push start/end, reads happen on status queries. The inner `Arc` allows cloning a handle into background tasks without borrowing `AppState`.

**Alternative considered:** `DashMap` for lock-free reads. Rejected — overkill for a set that will typically contain 0–5 entries.

### Register before spawn, deregister in background task

**Decision:** In the async path of `shelve_branches()`:
1. After `do_prepare_shelve()` returns, insert each CL into `ActiveShelves`
2. Clone the `ActiveShelves` handle into the background closure
3. In `complete_pending_shelves()`, remove each CL after it completes (whether success or failure)

**Rationale:** Registering before the background spawn ensures the status endpoint never misses an in-flight shelve. Deregistering on both success and failure ensures no leaks.

### Endpoint: `GET /api/repos/{group}/{name}/shelve-status/{cl}`

**Decision:** Return JSON `{ "active": true/false }`. 404 if the repo doesn't exist. The CL is scoped to a repo path for consistency with the URL structure, though the underlying tracker is global.

**Rationale:** Scoping to repo path matches the existing URL structure and prevents leaking information about other repos. Simple boolean response — callers just need to know "is it still going?"

**Alternative considered:** A bulk endpoint returning all active CLs for a repo. Rejected for now — single-CL queries are the primary use case and simpler. Can be added later if needed.

## Risks / Trade-offs

- **[Server restart clears state]** → All in-flight shelves are lost from the tracker. The shelves themselves may still be running (tokio runtime shuts down blocking tasks) or may be lost. This is acceptable — callers can fall back to checking P4 directly if the server restarts.
- **[No repo scoping in tracker]** → The `HashSet<usize>` is global, not per-repo. CL numbers are globally unique in P4, so this is correct. The repo in the URL path is just for routing consistency.
