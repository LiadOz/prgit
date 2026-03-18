## MODIFIED Requirements

### Requirement: Two-phase shelve operation
The shelver SHALL support an async mode where the entire shelve operation runs in the background. When async shelving is enabled, the push handler SHALL register the branch in the active shelves tracker and spawn a background task that calls `shelve()` directly. The `prepare_shelve` method and `PendingShelve` struct are removed.

#### Scenario: Async shelve runs entirely in background
- **WHEN** a push is received for branch `feature-xyz` and async shelving is enabled
- **THEN** the system SHALL register the branch as queued in the active shelves tracker and spawn a background task that calls `shelve("feature-xyz", user_p4, shelver_user)` — no P4 interaction occurs before the push response is sent

#### Scenario: Background task updates tracker on completion
- **WHEN** the background shelve task completes successfully for branch `feature-xyz`
- **THEN** the system SHALL update the tracker entry to done state with the resulting CL number and client name

#### Scenario: Background task updates tracker on failure
- **WHEN** the background shelve task fails for branch `feature-xyz`
- **THEN** the system SHALL update the tracker entry to failed state with the error message

### Requirement: PendingShelve holds lock until completion
This requirement is removed as `PendingShelve` no longer exists. The shelve client lock is held for the duration of the `shelve()` call within the background task.

#### Scenario: Lock held during background shelve
- **WHEN** a background shelve is in progress for a user
- **THEN** the shelve client lock SHALL be held for the duration of the `shelve()` call only

## REMOVED Requirements

### Requirement: PendingShelve holds lock until completion
**Reason**: The two-phase split (prepare + complete) is removed. The background task calls `shelve()` directly, which manages its own lock internally.
**Migration**: No migration needed — the lock is still held during the shelve operation, just within a single `shelve()` call instead of across prepare/complete.
