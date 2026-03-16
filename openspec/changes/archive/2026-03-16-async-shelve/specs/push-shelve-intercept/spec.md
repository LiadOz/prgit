## MODIFIED Requirements

### Requirement: Shelve on branch push
After a successful push, the server SHALL run the shelver for each pushed branch. When async shelving is enabled for the repo, the server SHALL use the two-phase shelve flow: create the changelist synchronously, then complete the shelve in the background.

#### Scenario: Feature branch push triggers shelve (sync mode)
- **WHEN** a user pushes to `refs/heads/feature-xyz` and async shelving is disabled
- **THEN** the server SHALL call `Shelver.shelve("feature-xyz", user_p4)` and wait for completion before responding

#### Scenario: Feature branch push triggers shelve (async mode)
- **WHEN** a user pushes to `refs/heads/feature-xyz` and async shelving is enabled
- **THEN** the server SHALL call `Shelver.prepare_shelve("feature-xyz", user_p4)`, return the changelist number in the git response, and complete the shelve in a background task

#### Scenario: Branch deletion does not trigger shelve
- **WHEN** a push deletes a branch (new-sha is all zeros)
- **THEN** the server SHALL NOT run the shelver for that ref

### Requirement: Shelve feedback messages
The server SHALL inject sideband messages into the git push response indicating the shelve status for each branch.

#### Scenario: Sync shelve feedback
- **WHEN** a branch is shelved synchronously
- **THEN** the sideband message SHALL read: "Shelved branch '{branch}' as CL {cl} on client '{client}'"

#### Scenario: Async shelve feedback
- **WHEN** a branch shelve is started asynchronously
- **THEN** the sideband message SHALL read: "Shelving branch '{branch}' as CL {cl} on client '{client}' (in background)"
