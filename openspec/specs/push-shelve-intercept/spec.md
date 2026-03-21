## Purpose
Push interception that rejects writes to the synced branch, triggers shelving on feature branch pushes, and provides user feedback via git sideband protocol.

## Requirements

### Requirement: Reject pushes to synced branch
The server SHALL reject any push that targets the synced branch before proxying to `git-http-backend`.

#### Scenario: Push to synced branch rejected
- **WHEN** a user pushes to the synced branch (e.g., `refs/heads/main`)
- **THEN** the server SHALL reject the push with an error indicating that the synced branch is read-only

#### Scenario: Push to non-synced branch allowed
- **WHEN** a user pushes to a branch that is not the synced branch
- **THEN** the server SHALL allow the push to proceed

### Requirement: Shelve on branch push
After a successful push, the server SHALL run the shelver for each pushed branch. When async shelving is enabled for the repo, the server SHALL register the branch in the active shelves tracker with state `queued` and return the push response immediately — before any P4 interaction. The shelve SHALL then run entirely in the background using the standard `shelve()` method.

#### Scenario: Feature branch push triggers shelve (sync mode)
- **WHEN** a user pushes to `refs/heads/feature-xyz` and async shelving is disabled
- **THEN** the server SHALL call `Shelver.shelve("feature-xyz", user_p4)` and wait for completion before responding

#### Scenario: Feature branch push triggers shelve (async mode)
- **WHEN** a user pushes to `refs/heads/feature-xyz` and async shelving is enabled
- **THEN** the server SHALL register `feature-xyz` as `queued` in the active shelves tracker and return the push response immediately, then execute `Shelver.shelve("feature-xyz", user_p4)` in a background task that updates the tracker on completion

#### Scenario: Branch deletion does not trigger shelve
- **WHEN** a push deletes a branch (new-sha is all zeros)
- **THEN** the server SHALL NOT run the shelver for that ref

### Requirement: Shelve feedback messages
The server SHALL inject sideband messages into the git push response indicating the shelve status for each branch.

#### Scenario: Sync shelve feedback
- **WHEN** a branch is shelved synchronously
- **THEN** the sideband message SHALL read: "Shelved branch '{branch}' as CL {cl} on client '{client}'"

#### Scenario: Async shelve feedback
- **WHEN** a branch shelve is queued asynchronously
- **THEN** the sideband message SHALL read: "Shelving branch '{branch}' in background"

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
The server SHALL NOT guarantee successful shelving when multiple users push to the same branch, because the P4 shelved changelist is owned by the user who first created it.

#### Scenario: Second user pushes to branch with existing shelve
- **WHEN** user B pushes to a branch that was previously shelved by user A
- **THEN** the shelver SHALL attempt the shelve but MAY fail due to P4 changelist ownership. This is a known limitation.
