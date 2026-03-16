## Context

The mirror (`src/mirror/mirror.rs`) maps P4 file actions to git tree operations in `create_commit`. Currently the match arm for `FileAction::Branch` and `FileAction::Integrate` is an empty block `{}`, silently discarding these files. However, `p4 print` already fetches the content of branch/integrate files into the temp directory — the data is there, it's just not wired into the git tree builder.

In Perforce:
- **Branch** action: file was copied/branched from another depot path. The file has real content at the target.
- **Integrate** action: file was merged from another depot path. The resulting file has real content reflecting the merge result.

Both actions produce files with content that `p4 print` retrieves, just like `Add` or `Edit`.

## Goals / Non-Goals

**Goals:**
- Mirror `Branch` and `Integrate` file actions as git upserts so the mirrored git repo accurately reflects the P4 depot
- Add failing integration tests that demonstrate the bug, then make them pass with the fix
- Keep the fix minimal — this is a one-line change in a match arm

**Non-Goals:**
- Handling integrate merge topology (recording which branch was the source) — this is already handled separately via `old_change`/`get_related_branch`
- Adding `p4 integrate` command wrapper to p4rs — tests will use raw `std::process::Command` for P4 operations that p4rs doesn't wrap

## Decisions

### Route Branch/Integrate to upsert

**Decision**: Treat `FileAction::Branch` and `FileAction::Integrate` identically to `FileAction::Add | FileAction::Edit` — call `builder.upsert()` with the file content from the temp directory.

**Rationale**: `p4 print` already retrieves the file content for these actions. The temp directory already contains the correct file data. We just need to wire it through to the git tree builder. There's no semantic difference from git's perspective — the file exists and has content.

**Alternative considered**: Treating integrate differently (e.g., logging, special metadata). Rejected because git has no concept of "integrate" at the file level — it's just a file that exists in a commit. The merge parent tracking is already handled separately.

### Test approach: use raw p4 commands for integrate/branch

**Decision**: Tests will create a second test client mapping to a different depot path, add files there, then use `std::process::Command` to run `p4 integrate` and `p4 resolve -at` to create real branch/integrate actions in the primary client's workspace.

**Rationale**: p4rs doesn't have an `integrate` command wrapper, and adding one is out of scope. Using raw commands in tests is simple and tests the real P4 behavior.

## Risks / Trade-offs

- **Risk**: `p4 print` might not return content for some edge cases of integrate (e.g., integrate with delete). → Mitigation: The existing `Delete` handling covers `integrate` actions that result in deletions, since P4 would report those as `delete` action, not `integrate`.
- **Trade-off**: Tests depend on `p4 integrate` CLI being available in the test environment. This is already true since tests require a p4d server.
