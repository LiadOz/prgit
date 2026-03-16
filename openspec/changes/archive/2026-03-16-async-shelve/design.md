## Context

Currently, when a user pushes a branch, `shelve_branches()` in `handlers.rs` calls `do_shelve()` synchronously via `spawn_blocking`. The shelver performs multiple P4 round-trips: sync base files, create/reuse changelist, apply file changes, execute `p4 shelve`, and clean up the workspace. On slow P4 servers, this blocks `git push` for minutes even for single-file changes.

The config (`RepoConfig`) has no mechanism to control shelve behavior per-repo.

## Goals / Non-Goals

**Goals:**
- Allow per-repo opt-in to background shelving via config
- Return the P4 changelist number to the user immediately after push
- Complete the actual shelve operation asynchronously without blocking the git response
- Preserve existing synchronous behavior as the default

**Non-Goals:**
- Retry logic for failed background shelves
- Notification mechanism for background shelve completion/failure
- Changing the shelve client locking model

## Decisions

### Two-phase shelve via `ShelveClient` split

Split `ShelveClient::run()` into two public methods:
- `create_or_reuse_changelist(description, original_change) -> Result<usize>` — creates a pending changelist or returns the existing one. Single `p4 change -i` round-trip.
- `shelve_changelist(cl, base_change, base_dir, changes) -> Result<()>` — performs sync, apply, shelve, cleanup. Multiple slow P4 round-trips.

The existing `run()` calls both sequentially, preserving current behavior for sync mode. This is a pure refactor with no behavioral change — changelist creation doesn't depend on sync state.

**Why split at this boundary:** Changelist creation is the only P4 operation whose result (the CL number) the user needs to see. Everything else can happen after the response is sent.

### `PendingShelve` as owned completion token

Introduce `PendingShelve` in `shelver.rs` that owns everything needed to complete a shelve:
- `ShelveClientHandle` — keeps the file lock held so no concurrent shelve can interfere
- `tempfile::TempDir` — extracted git file contents
- `Vec<ChangedFile>` — owned file action list
- `base_change: usize` and `changelist: usize`

`PendingShelve::complete(self)` consumes the struct, runs `shelve_changelist()`, and drops the lock + temp dir on completion. All fields are `Send`, so the struct can move into `tokio::spawn_blocking`.

**Alternative considered:** Re-acquiring the shelve client in the background task (release lock, then re-lock). Rejected because another push could grab the lock in between, and re-computing git changes is wasteful.

### `Shelver::prepare_shelve()` for the fast path

New method alongside existing `shelve()`:
1. All git operations (find branch, merge base, compute diff) — local, fast
2. `get_shelve_client()` — acquires lock, ensures P4 client exists
3. `create_or_reuse_changelist()` — one P4 round-trip
4. `extract_files_to_temp()` — local filesystem, fast
5. `set_shelved_change_for_branch()` — DB write

Returns `(ShelveResult, PendingShelve)`. The caller gets the CL number immediately and can complete the shelve later.

### Handler async path with fire-and-forget background

In `shelve_branches()`, when `config.shelve_async()` is true (helper on `RepoConfig`):
1. `spawn_blocking(do_prepare_shelve(...))` — returns `(HandlerShelveResult, Vec<PendingShelve>)`
2. Return CLs to the user via sideband messages with "(in background)" suffix
3. `spawn_blocking(complete_pending_shelves(...))` — fire-and-forget, not awaited

Background failures are logged at `error` level. The push has already succeeded and the CL exists as a pending changelist. On the next push to the same branch, `existing_shelve` will find this CL and the shelver will retry with it.

**Why fire-and-forget `spawn_blocking`:** Dropping a `spawn_blocking` JoinHandle does not cancel the task — it continues running on the blocking thread pool. This gives us safe background execution without needing a separate task queue.

### Per-repo `shelve` config section

Add a `ShelveSettings` struct with an `async` field (`r#async: bool`), and nest it under `RepoConfig` as `shelve: Option<ShelveSettings>`. When omitted, all shelve settings default (async = false). This section is extensible for future shelve-related config. Example:

```yaml
repos:
  - group: depot
    name: main
    shelve:
      async: true
    # ...other fields...
```

## Risks / Trade-offs

**Lock held during background shelve** — The `ShelveClientHandle` file lock is held until the background task completes. If a user pushes again while the previous shelve is still running, they'll get `ClientBusy`. This is the same behavior as today's sync mode, so no regression. → Mitigation: This is acceptable; concurrent pushes to the same branch by the same user are rare.

**Silent background failures** — If the background shelve fails, the user sees a CL number but the shelve may be empty. → Mitigation: Errors are logged server-side. On the next push, the shelver retries using the same CL number. Admins can monitor logs for persistent failures.

**Changelist exists but may be empty** — Between prepare and complete, the CL exists as a pending changelist with no shelved files. Anyone querying P4 during this window will see an empty CL. → Mitigation: The window is typically seconds to minutes. The "(in background)" message signals to the user that the shelve is not yet complete.
