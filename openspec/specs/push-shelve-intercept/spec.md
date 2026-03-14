## ADDED Requirements

### Requirement: Reject pushes to synced branch
The server SHALL reject any push that targets the synced branch before proxying to `git-http-backend`.

#### Scenario: Push to synced branch rejected
- **WHEN** a user pushes to the synced branch (e.g., `refs/heads/main`)
- **THEN** the server SHALL reject the push with an error indicating that the synced branch is read-only

#### Scenario: Push to non-synced branch allowed
- **WHEN** a user pushes to a branch that is not the synced branch
- **THEN** the server SHALL allow the push to proceed

### Requirement: Shelve on branch push
After a successful push, the server SHALL run the shelver in-process for each pushed branch.

#### Scenario: Feature branch push triggers shelve
- **WHEN** a user pushes to `refs/heads/feature-xyz`
- **THEN** the server SHALL call `Shelver.shelve("feature-xyz", user_p4)` using the P4 identity from the push's auth credentials

#### Scenario: Branch deletion does not trigger shelve
- **WHEN** a push deletes a branch (new-sha is all zeros)
- **THEN** the server SHALL NOT run the shelver for that ref

### Requirement: Shelve uses authenticated P4 identity
The shelver SHALL create the P4 shelved changelist using the P4 user and ticket extracted from the push request's HTTP basic auth.

#### Scenario: Changelist created as pushing user
- **WHEN** user `jdoe` pushes a branch with a valid P4 ticket
- **THEN** the shelved changelist in P4 SHALL be owned by `jdoe`

### Requirement: Shelve failure does not fail the push
If the shelver fails after a successful push, the push SHALL still succeed. The shelve error SHALL be logged.

#### Scenario: Shelve error after successful push
- **WHEN** a push succeeds but the shelver encounters an error (e.g., P4 server unreachable)
- **THEN** the git push SHALL return success to the client, and the server SHALL log the shelve error

### Requirement: Multiple users on same branch (known limitation)
When multiple users push to the same branch, shelve ownership may conflict because the P4 shelved changelist is owned by the user who first created it. A subsequent user pushing to the same branch may fail to update the existing shelve.

#### Scenario: Second user pushes to branch with existing shelve
- **WHEN** user B pushes to a branch that was previously shelved by user A
- **THEN** the shelver MAY fail due to P4 changelist ownership. This is a known limitation to be addressed in a future change.
