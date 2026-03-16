## ADDED Requirements

### Requirement: Two-phase shelve operation
The shelver SHALL support splitting a shelve into a prepare phase and a completion phase. The prepare phase creates the P4 changelist and returns immediately. The completion phase performs the actual P4 shelve operation.

#### Scenario: Prepare creates changelist without shelving
- **WHEN** `prepare_shelve` is called for a branch with changes
- **THEN** the system SHALL create a pending P4 changelist, store the branch-to-changelist mapping, and return the changelist number without executing the P4 shelve command

#### Scenario: Completion executes the shelve
- **WHEN** `PendingShelve::complete()` is called with a previously prepared shelve
- **THEN** the system SHALL sync base files, apply changes, execute `p4 shelve`, and clean up the workspace

#### Scenario: Prepare reuses existing changelist
- **WHEN** `prepare_shelve` is called for a branch that already has a shelved changelist
- **THEN** the system SHALL reuse the existing changelist number rather than creating a new one

### Requirement: PendingShelve holds lock until completion
The `PendingShelve` struct SHALL hold the shelve client file lock for the entire duration between prepare and complete, preventing concurrent shelve operations on the same client.

#### Scenario: Concurrent push during pending shelve
- **WHEN** a second push arrives for the same user while a `PendingShelve` is still pending
- **THEN** the system SHALL return a client-busy error for the second push

#### Scenario: Lock released after completion
- **WHEN** `PendingShelve::complete()` finishes (success or failure)
- **THEN** the file lock SHALL be released

### Requirement: Background shelve failure is non-fatal
When a background shelve completion fails, the failure SHALL be logged but SHALL NOT affect the already-completed git push response.

#### Scenario: Background shelve fails
- **WHEN** the background `PendingShelve::complete()` encounters a P4 error
- **THEN** the server SHALL log the error at error level and the pending changelist SHALL remain in P4 for the next push to retry
