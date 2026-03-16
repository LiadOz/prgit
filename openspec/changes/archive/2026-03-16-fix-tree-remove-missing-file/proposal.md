## Why

`CommitBuilder::remove()` directly calls `TreeUpdateBuilder::remove()`, which crashes with "failed to remove entry: file isn't in the tree" if the file doesn't exist in the git tree. This happens when P4 reports a Delete action on a file that was never added to git — for example, a file deleted multiple times (P4 allows #N delete after #N-1 delete via re-add cycles), or files from Branch/Integrate actions that were previously skipped. The mirror should tolerate these cases gracefully instead of crashing.

## What Changes

- Defer `remove()` calls in `CommitBuilder` — instead of immediately queuing into `TreeUpdateBuilder`, collect paths in a `pending_removes` vec
- In `build_tree()`, resolve the base tree first, then check each pending remove path actually exists in the tree before calling `TreeUpdateBuilder::remove()`
- Skip missing files with a warning log instead of crashing

## Capabilities

### New Capabilities
- `safe-tree-remove`: CommitBuilder tolerates remove operations on files that don't exist in the git tree, logging a warning instead of crashing

### Modified Capabilities

## Impact

- `src/mirror/commit_builder.rs`: `CommitBuilder` struct gains a `pending_removes: Vec<String>` field; `remove()` method changes from direct delegation to deferred collection; `build_tree()` gains existence-check loop
- No API changes — `remove(&mut self, path: &str)` signature stays the same
- No new dependencies
