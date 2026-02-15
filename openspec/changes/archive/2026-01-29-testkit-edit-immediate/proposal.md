## Why

The testkit's `edit_file` helper fails when editing P4-synced files because it queues the P4 edit operation but tries to write content immediately. Since P4 syncs files as read-only, the write fails with "Permission denied".

## What Changes

- Modify `edit_file` (and similar content-providing helpers) to execute P4 edit immediately rather than queuing it, ensuring the file is writable before content is written

## Capabilities

### New Capabilities

- `testkit-content-edit`: Testkit helpers that provide content when editing files must execute the P4 edit immediately to ensure files are writable

### Modified Capabilities

## Impact

- `crates/p4rs/src/testkit.rs` - `edit_file`, `edit_file_with_opts` helpers
- All tests using these helpers will work correctly with P4-synced read-only files
