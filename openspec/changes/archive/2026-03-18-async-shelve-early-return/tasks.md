## 1. Delete two-phase shelve machinery

- [x] 1.1 Delete `PendingShelve` struct and its `impl` block from `src/shelf/shelver.rs`
- [x] 1.2 Delete `Shelver::prepare_shelve()` method from `src/shelf/shelver.rs`
- [x] 1.3 Revert `ShelveClientHandle` visibility to `pub(crate)` if it was made `pub` only for `PendingShelve`
- [x] 1.4 Delete `do_prepare_shelve()` and `complete_pending_shelves()` from `src/window/handlers.rs`
- [x] 1.5 Delete the three `prepare_shelve` tests (`test_prepare_shelve_creates_changelist`, `test_prepare_then_complete_shelve`, `test_prepare_shelve_reuses_existing_changelist`)

## 2. Rework ActiveShelves tracker

- [x] 2.1 Define `ShelveState` enum (`Queued`, `Shelving`, `Done { changelist, client }`, `Failed { error }`) in `src/window/mod.rs`
- [x] 2.2 Change `ActiveShelves` from `HashSet<usize>` to `HashMap<String, ShelveState>` keyed by `{group}/{name}/{branch}`
- [x] 2.3 Add methods: `set_queued`, `set_shelving`, `set_done`, `set_failed`, `get` (returns `Option<&ShelveState>`)
- [x] 2.4 Update `test_active_shelves_insert_remove_contains` to test new state transitions

## 3. Rework async push handler

- [x] 3.1 In `shelve_branches` async path: register each branch as `Queued` in tracker before returning response
- [x] 3.2 Return `HandlerShelveResult` with empty `shelved` vec in async mode (no CLs to report)
- [x] 3.3 Spawn a single `spawn_blocking` background task that calls `do_shelve` per branch, updating tracker to `Shelving` before and `Done`/`Failed` after each
- [x] 3.4 Update sideband message for async mode to `"Shelving branch '{branch}' in background"` (no CL)

## 4. Rework shelve status endpoint

- [x] 4.1 Change `shelve_status` handler to accept branch name instead of CL number
- [x] 4.2 Return `ShelveState`-based JSON response (`{ state, changelist?, client?, error? }`)
- [x] 4.3 Return 404 when branch has no tracker entry

## 5. Version and regroup API routes

- [x] 5.1 Move health endpoint to `/api/v1/health`
- [x] 5.2 Move shelve status to `/api/v1/repos/{group}/{name}/shelve/status/{branch}`
- [x] 5.3 Move cl-alias to `/api/v1/repos/{group}/{name}/shelve/cl-alias`
- [x] 5.4 Remove old unversioned routes

## 6. Tests

- [x] 6.1 Add test for `ShelveState` transitions in `ActiveShelves` (queued → shelving → done, queued → shelving → failed)
- [x] 6.2 Add test for shelve status endpoint returning each state variant
- [x] 6.3 Add test for shelve status 404 on unknown branch
- [x] 6.4 Verify existing sync shelve tests still pass (no changes to sync path)
