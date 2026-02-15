## Why

ShelveClient workspaces accumulate files over multiple operations. Synced files remain after revert, and added files remain as untracked. This causes disk space growth and potential conflicts with subsequent operations.

## What Changes

- ShelveClient::new() fully cleans the workspace before starting (handles incomplete previous runs)
- ShelveClient::drop() fully cleans the workspace after completion (keeps workspace clean for next run)
- Cleanup includes: revert open files, unsync all files, delete remaining untracked files

## Capabilities

### New Capabilities

- `shelve-client-workspace-cleanup`: ShelveClient ensures a clean workspace at initialization by reverting, unsyncing, and removing leftover files

### Modified Capabilities

## Impact

- `src/shelf/shelve_client.rs` - ShelveClient::new() initialization
- Workspace will be clean after each ShelveClient is created
- Slightly more overhead at startup but guarantees clean state
