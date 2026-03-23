## Context

The mirror task already detects merged shelves in `mirror_task.rs` — when `get_related_branch` finds a match and the branch has a shelved CL in `branch_shelve_mapping`, it creates a `ShelveMergedInfo` struct. Currently this only emits a `shelve.merged` event. The cleanup adds P4 deletion and DB cleanup at the same point.

## Goals / Non-Goals

**Goals:**
- Delete the shelved CL from P4 after it's been submitted
- Clean up the branch_shelve_mapping entry
- Handle alias resolution

**Non-Goals:**
- Deleting the P4 pending changelist (only the shelved files)
- Cleaning up the shelve_cl_alias table (left for reference)
- Cleaning up the git branch (that's the user's responsibility)

## Decisions

### 1. Cleanup in the mirror task spawn_blocking block

**Decision:** Add cleanup calls inside the existing `spawn_blocking` block in `mirror_task.rs`, right after building `ShelveMergedInfo`. This already has DB access and can make P4 calls.

**Why:** The mirror task already runs P4 commands in this context. Adding `p4 shelve -d` and a DB delete is natural. No new async boundaries needed.

### 2. Use the shelver's P4 identity for deletion

**Decision:** Use the same P4 instance the mirror uses (admin/service account). The shelved CL may be owned by a different user, but `p4 shelve -d` works if the caller has admin privileges or is the CL owner.

**Why:** The mirror's P4 instance typically has elevated privileges. If it doesn't, the delete will fail gracefully (logged, not blocking).

### 3. Add `clear_shelved_change_for_branch` to PrgitClient

**Decision:** Add a method that DELETEs from `branch_shelve_mapping` for a given branch. This is the inverse of `set_shelved_change_for_branch`.

**Why:** Clean separation. The existing `set_shelved_change_for_branch` inserts/updates; the new method removes.

## Risks / Trade-offs

**[P4 permission for shelve delete]** → The mirror's P4 user may not have permission to delete another user's shelved CL. Mitigation: log and continue. The shelve stays but doesn't affect functionality.

**[Race condition with concurrent push]** → A user could push to the same branch between the merge detection and cleanup. Mitigation: extremely unlikely timing window; if it happens, the next push will create a new shelve entry.
