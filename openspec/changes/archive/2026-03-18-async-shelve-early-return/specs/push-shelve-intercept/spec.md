## MODIFIED Requirements

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
