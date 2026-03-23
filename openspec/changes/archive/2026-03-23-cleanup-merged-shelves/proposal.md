## Why

When a shelved changelist created by prgit is submitted in P4, the mirror detects it and emits a `shelve.merged` event. But the original shelved CL remains in P4 as stale data. Over time these accumulate, cluttering `p4 shelve -l` output and consuming server resources. prgit created them, so prgit should clean them up.

## What Changes

- After the mirror detects a branch merge (via `get_related_branch`), delete the shelved CL from P4 using `p4 shelve -d`
- Also clean up the `branch_shelve_mapping` entry so the branch is no longer tracked as shelved
- Handle the alias case: if the submitted CL came through an alias, resolve to the original shelved CL before deleting

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities
- `mirror-scheduler`: After detecting shelve.merged, delete the shelved CL and clean up mappings

## Impact

- **Modified files**: `src/window/mirror_task.rs` (add cleanup after merge detection), `src/cabinet/prgit_client.rs` (add method to clear branch_shelve_mapping entry)
- **No new dependencies** — `p4.shelve().delete()` already exists in p4rs
- **Risk**: Low — cleanup happens after the merge commit is already created, so failure to delete doesn't affect mirroring

## Observability

No new events needed. The existing `shelve.merged` event already captures the relevant data. Cleanup failures should be logged at warn level without blocking the mirror.
