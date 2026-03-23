## Why

The shelver's `extract_files_to_temp` writes all files from the git tree with default `0o644` permissions, losing the executable bit from git's `100755` filemode. When the shelve client later calls `determine_file_type` on the extracted file, it returns `text` instead of `text+x`. This causes `apply_git_type_to_depot` to compute the wrong effective type, triggering an unnecessary `p4 reopen` that strips the `+x` modifier — and on P4 servers with compression defaults, the file type changes to `text+C`.

This is a companion fix to the earlier `shelve-preserve-file-type` change which fixed the `apply_changes` edit path but missed the extraction step upstream.

## What Changes

- `extract_files_to_temp` in `shelver.rs` now sets `0o755` permissions on files whose git tree entry has `BlobExecutable` filemode
- End-to-end test through the full Shelver path verifies executable files preserve their type

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities
- `shelve-client-management`: Extracted files preserve the executable bit from the git tree entry

## Impact

- **Modified files**: `src/shelf/shelver.rs` (3-line fix in `extract_files_to_temp`, new e2e test)
- **No new dependencies**
- **Risk**: None — only adds permissions that should have been there

## Observability

No new observability needed — the existing shelve events capture file_count and the fix is verified by P4 describe on the shelved CL.
