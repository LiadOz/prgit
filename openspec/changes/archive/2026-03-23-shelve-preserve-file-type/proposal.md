## Why

When the shelver edits an existing file, it replaces the P4 file type with one derived purely from git working copy attributes. This strips P4-specific modifiers (`+C` compressed, `+k` keyword expansion, `+l` exclusive lock, etc.) and can incorrectly change the executable bit. The only file metadata git carries is the executable permission — all other P4 type modifiers should be preserved from the depot.

Real-world impact: files submitted through prgit lose their P4 file type modifiers. For example, `text+x` files become `text+C` or `text`, breaking executable permissions and storage behavior.

## What Changes

- On edit, the shelver should only toggle the executable and symlink bits based on the git working copy, preserving all other P4 modifiers from the depot file type
- `FileType::Display` fixed to combine modifiers after a single `+` (was outputting `text+x+C` instead of `text+xC`)
- Bug-documenting tests added; fix will convert them to passing assertions

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities
- `shelve-client-management`: Edit path must preserve P4 file type modifiers, only changing executable/symlink based on git

## Impact

- **Modified files**: `src/shelf/shelve_client.rs` (edit path in `apply_changes`), `crates/p4rs/src/commands/types.rs` (`FileType::Display`)
- **No new dependencies**
- **Risk**: Low — the fix narrows the scope of `reopen` calls, making fewer type changes rather than more

## Observability

No new observability needed — shelve events already capture file_count. Type preservation is verified by P4 describe on the shelved CL.
