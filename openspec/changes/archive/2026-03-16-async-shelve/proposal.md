## Why

Some P4 servers are slow enough that shelving even a single file takes ~1 minute, causing `git push` to block for the entire duration. Users on these servers need a way to get push feedback immediately while the shelve completes in the background.

## What Changes

- Add a `shelve` configuration section to per-repo YAML config, with an `async` boolean field (defaults to `false`)
- When `shelve.async` is enabled, the push handler creates the P4 changelist eagerly (fast), returns it to the user immediately, and completes the actual shelve operation (sync, apply, shelve) in a background task
- The sideband message changes to indicate the shelve is in progress rather than complete
- If the background shelve fails, the error is logged server-side; the push itself still succeeds
- The `ShelveClient` is refactored to separate changelist creation from the shelve execution, enabling the two-phase flow

## Capabilities

### New Capabilities

- `async-shelve`: Background shelving mode that returns the changelist immediately without waiting for the P4 shelve operation to complete

### Modified Capabilities

- `server-config`: Add `shelve` section to per-repo configuration with `async` field
- `push-shelve-intercept`: Support async shelving mode where changelist is returned before shelve completes

## Impact

- `src/window/mod.rs` - `RepoConfig` struct gains `shelve: ShelveSettings` with `async` field
- `src/shelf/shelve_client.rs` - `ShelveClient::run()` split into changelist creation + shelve execution
- `src/shelf/shelver.rs` - New `prepare_shelve()` method and `PendingShelve` struct for two-phase flow
- `src/window/handlers.rs` - Async shelve path in `shelve_branches` that fires and forgets background completion
- Config YAML files - Optional new `shelve.async` field per repo
