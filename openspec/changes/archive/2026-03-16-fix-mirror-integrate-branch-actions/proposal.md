## Why

The mirror currently silently drops `Branch` and `Integrate` file actions when syncing P4 changelists to git. In Perforce, these actions introduce real file content (e.g., branching a file copies it, integrating merges changes into it), but prgit's mirror match arm treats them as no-ops (`{}`). This means any changelist containing only branch/integrate actions produces an empty git commit, and mixed changelists lose those files entirely.

## What Changes

- Treat `FileAction::Branch` and `FileAction::Integrate` as content-bearing actions in the mirror, mapping them to git upserts (same as `Add`/`Edit`) since `p4 print` already fetches their content
- Add integration tests that create real P4 integrate and branch scenarios and verify the mirrored git repo contains the correct files

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `mirror-scheduler`: The mirror's file action handling requirements change — `Branch` and `Integrate` actions must produce git upserts instead of being silently dropped

## Impact

- `src/mirror/mirror.rs` — the `match file.action` block in `create_commit` needs to route `Branch`/`Integrate` to `builder.upsert`
- `tests/mirror_tests.rs` — new test cases for branch and integrate scenarios
- No API or dependency changes
