## Why

The current async shelve flow still blocks the push response until `prepare_shelve` completes (changelist creation). On slow P4 servers, even changelist creation can take significant time. By returning immediately after push validation — before any P4 interaction — the user gets instant feedback and can query shelve progress by branch name.

## What Changes

- **BREAKING**: Async push response no longer includes a CL number. The response confirms the push was accepted and shelving has been queued.
- **BREAKING**: All API endpoints move under `/api/v1/`. Existing unversioned `/api/` routes are dropped. New route layout:
  - `/api/v1/health`
  - `/api/v1/repos/{group}/{name}/shelve/status/{branch}` (was `/api/repos/.../shelve-status/{cl}`)
  - `/api/v1/repos/{group}/{name}/shelve/cl-alias` (was `/api/repos/.../cl-alias`)
- The shelve status endpoint is now branch-based (not CL-based). The response includes the current shelve state (queued, shelving, done, failed) and the CL number once available.
- The active shelves tracker is rekeyed from CL numbers to branch names, since the CL doesn't exist yet when the push returns.
- The two-phase shelve split (`prepare_shelve` + `PendingShelve::complete`) is removed. Since nothing is returned synchronously, the background task just calls the existing `shelve()` method directly. `prepare_shelve`, `PendingShelve`, and its `complete()` method can be deleted.

## Capabilities

### New Capabilities

_(none — this modifies existing capabilities)_

### Modified Capabilities

- `async-shelve`: The two-phase split is removed entirely. The background task calls `shelve()` directly instead of `prepare_shelve()` + `complete()`. `PendingShelve` struct is deleted.
- `shelve-status`: Endpoint moves to `/api/v1/.../shelve/status/{branch}`. Response includes richer state (queued/shelving/done/failed) and the CL when known.
- `push-shelve-intercept`: Async shelve feedback message changes — no CL in the immediate response. Sideband message confirms shelving is queued, not in progress.
- `cl-alias`: Route moves to `/api/v1/.../shelve/cl-alias`. No behavioral changes.

## Impact

- `src/shelf/shelver.rs` — Delete `PendingShelve` struct, `prepare_shelve()` method, and associated tests. Async mode reuses `shelve()` directly.
- `src/window/handlers.rs` — Push handler returns immediately in async mode (no `spawn_blocking` for prepare). Shelve status handler rekeyed to branch name. All routes versioned under `/api/v1/`.
- `src/window/mod.rs` — `ActiveShelves` changes from `HashSet<usize>` (CL) to a map of branch name → `ShelveState` (queued/shelving/done/failed + optional CL). All routes move to `/api/v1/`.
- Git sideband messages — async feedback no longer includes CL
- API consumers must update to `/api/v1/` prefix and query shelve status by branch name instead of CL
