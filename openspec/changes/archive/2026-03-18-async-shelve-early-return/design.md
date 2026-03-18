## Context

The async shelve feature (commits `ad7e2a4`..`dd040fd`) introduced a two-phase shelve split: `prepare_shelve` creates a CL synchronously, then `PendingShelve::complete()` runs in the background. This was needed because the push response included the CL number, so it had to exist before the response was sent.

This change removes that constraint — the push response no longer includes a CL. The entire shelve runs in the background, and the user queries status by branch name. This lets us delete the two-phase machinery and simplify back to a single `shelve()` call.

Current async flow in `handlers.rs`:
```
spawn_blocking(do_prepare_shelve)  →  insert CLs into tracker  →  spawn_blocking(complete_pending_shelves)
```

New async flow:
```
insert branches as queued in tracker  →  return response  →  spawn_blocking(do_shelve) with tracker updates
```

## Goals / Non-Goals

**Goals:**
- Instant push response in async mode (no P4 interaction before responding)
- Branch-based shelve status with state machine (queued → shelving → done/failed)
- Delete `PendingShelve`, `prepare_shelve`, `do_prepare_shelve`, `complete_pending_shelves` and their tests
- Version all API routes under `/api/v1/`, group shelve endpoints under `/shelve/`

**Non-Goals:**
- Changing the sync shelve flow (it stays as-is)
- Changing the shelve-to-branch mapping in the database (that stays CL-based)
- Adding retry logic for failed background shelves
- Supporting concurrent async shelves for the same branch (existing client lock prevents this)

## Decisions

### 1. Delete the two-phase split, reuse `shelve()` directly

The `prepare_shelve` method duplicates most of `shelve()` but splits it at the changelist creation point. Since we no longer need the CL before responding, the background task can call `shelve()` as-is. This deletes:

- `PendingShelve` struct and its `complete()` method
- `Shelver::prepare_shelve()` method
- `do_prepare_shelve()` handler function
- `complete_pending_shelves()` handler function
- Three `prepare_shelve` tests (`test_prepare_shelve_creates_changelist`, `test_prepare_then_complete_shelve`, `test_prepare_shelve_reuses_existing_changelist`)

The `ShelveClientHandle` visibility can also revert — it was made `pub` only for `PendingShelve`.

**Alternative**: Keep `prepare_shelve` for future use. Rejected — YAGNI, and it's trivially re-creatable from git history if ever needed.

### 2. `ActiveShelves` changes from `HashSet<usize>` to state map

Current: `Arc<Mutex<HashSet<usize>>>` keyed by CL number.

New: `Arc<Mutex<HashMap<String, ShelveState>>>` keyed by branch name (scoped per repo).

```rust
enum ShelveState {
    Queued,
    Shelving,
    Done { changelist: usize, client: String },
    Failed { error: String },
}
```

The key format is `{group}/{name}/{branch}` to avoid collisions across repos. The tracker key must include the repo identifier since `ActiveShelves` is shared across all repos in `AppState`.

**Alternative**: Use `DashMap` for lock-free concurrent access. Rejected — the current `Mutex<HashMap>` is fine since lock hold times are microseconds (just inserting/reading a small map entry). The simplicity wins.

### 3. Async handler flow

The `shelve_branches` function in async mode becomes:

1. For each branch, insert `(repo/branch → ShelveState::Queued)` into tracker (synchronous, before response)
2. Return `HandlerShelveResult` with empty `shelved` vec (no CLs to report)
3. Spawn a single `spawn_blocking` that calls `do_shelve` for each branch, updating the tracker state as it progresses

The background task updates the tracker to `Shelving` before calling `shelve()`, then to `Done` or `Failed` after.

### 4. Versioned API routes

All routes move to `/api/v1/`:
- `/api/v1/health`
- `/api/v1/repos/{group}/{name}/shelve/status/{branch}`
- `/api/v1/repos/{group}/{name}/shelve/cl-alias`

The git HTTP backend fallback stays unversioned (it's not our API — it's git protocol).

Old `/api/` routes are dropped entirely (no redirect/compatibility shim).

### 5. Sideband message for async mode

Changes from:
```
"Shelving branch '{branch}' as CL {cl} on client '{client}' (in background)"
```
To:
```
"Shelving branch '{branch}' in background"
```

The sync message stays unchanged since it still has the CL.

## Risks / Trade-offs

- **[No CL in push response]** → Clients that parsed the CL from the sideband message will break. Mitigation: this is called out as BREAKING. The status endpoint provides the CL once available.
- **[Branch status is ephemeral]** → The in-memory tracker loses state on server restart. A push that was `shelving` when the server restarts will show as 404 on the status endpoint. Mitigation: acceptable for now — the shelve either completed (branch mapping exists in DB) or failed (user re-pushes). The DB mapping is the source of truth, not the tracker.
- **[Stale entries]** → `Done` and `Failed` entries stay in the map indefinitely. Mitigation: acceptable for now — the map is small (one entry per recently-pushed branch). Can add TTL-based cleanup later if needed.
