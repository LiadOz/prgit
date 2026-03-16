## MODIFIED Requirements

### Requirement: Shelve on branch push
After a successful push, the server SHALL run the shelver for each pushed branch. When async shelving is enabled for the repo, the server SHALL use the two-phase shelve flow: create the changelist synchronously, then complete the shelve in the background. The server SHALL register each CL in the active shelves tracker before spawning the background task, and deregister it when the background task completes.

#### Scenario: Feature branch push triggers shelve (sync mode)
- **WHEN** a user pushes to `refs/heads/feature-xyz` and async shelving is disabled
- **THEN** the server SHALL call `Shelver.shelve("feature-xyz", user_p4)` and wait for completion before responding

#### Scenario: Feature branch push triggers shelve (async mode)
- **WHEN** a user pushes to `refs/heads/feature-xyz` and async shelving is enabled
- **THEN** the server SHALL call `Shelver.prepare_shelve("feature-xyz", user_p4)`, return the changelist number in the git response, register the CL in the active shelves tracker, and complete the shelve in a background task that deregisters the CL on completion

#### Scenario: Branch deletion does not trigger shelve
- **WHEN** a push deletes a branch (new-sha is all zeros)
- **THEN** the server SHALL NOT run the shelver for that ref
