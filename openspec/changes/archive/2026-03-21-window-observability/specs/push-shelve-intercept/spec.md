## ADDED Requirements

### Requirement: Emit push events on ref updates
After parsing ref updates from a receive-pack request, the handler SHALL emit push events for each ref: `push.branch_created`, `push.branch_updated`, or `push.branch_deleted` based on the old/new SHA values. It SHALL also emit a single `push.received` event with payload_bytes and ref_count.

#### Scenario: Push with mixed ref updates
- **WHEN** a push contains a branch creation, an update, and a deletion
- **THEN** the handler SHALL emit one `push.branch_created`, one `push.branch_updated`, one `push.branch_deleted`, and one `push.received` event

### Requirement: Emit shelve lifecycle events
The handler SHALL emit `shelve.started` before invoking the shelver and `shelve.completed`, `shelve.reshelved`, or `shelve.failed` after, with duration_ms measured from start to completion. The event SHALL include the async flag and commits_in_branch count.

#### Scenario: Successful first shelve emits completed
- **WHEN** a branch is shelved for the first time
- **THEN** `shelve.started` and `shelve.completed` events SHALL be emitted with duration_ms and commits_in_branch

#### Scenario: Reshelve emits reshelved event
- **WHEN** a branch with an existing shelve is pushed again
- **THEN** `shelve.started` and `shelve.reshelved` events SHALL be emitted

#### Scenario: Failed shelve emits failed event
- **WHEN** the shelver fails
- **THEN** `shelve.started` and `shelve.failed` events SHALL be emitted with the error

### Requirement: Emit push.rejected on synced branch or auth failure
When a push is rejected due to synced branch protection, the handler SHALL emit a `push.rejected` event with reason "synced_branch".

#### Scenario: Synced branch rejection emits event
- **WHEN** a push targets the synced branch
- **THEN** a `push.rejected` event SHALL be emitted before returning the error response
