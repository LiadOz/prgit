## ADDED Requirements

### Requirement: ShelveClient initializes with clean workspace
ShelveClient::new() SHALL ensure the client workspace is clean before returning, regardless of previous run state.

#### Scenario: Clean workspace with open files from previous run
- **WHEN** ShelveClient::new() is called and files are open from a previous incomplete run
- **THEN** all open files are reverted
- **THEN** all synced files are removed from workspace
- **THEN** all untracked files are deleted from client_root

#### Scenario: Clean workspace with synced files from previous run
- **WHEN** ShelveClient::new() is called and synced files exist from a previous run
- **THEN** all synced files are removed via `p4 sync #none`
- **THEN** client_root contains no files

#### Scenario: Clean workspace with untracked files from previous run
- **WHEN** ShelveClient::new() is called and untracked files exist (e.g., reverted adds)
- **THEN** all files in client_root are deleted
- **THEN** client_root directory itself is preserved

### Requirement: ShelveClient cleans workspace on drop
ShelveClient::drop() SHALL clean the workspace to leave it ready for the next operation.

#### Scenario: Clean workspace after shelve operation
- **WHEN** ShelveClient is dropped after a successful shelve
- **THEN** all open files are reverted
- **THEN** all synced files are removed from workspace
- **THEN** all untracked files are deleted from client_root

#### Scenario: Clean workspace after failed operation
- **WHEN** ShelveClient is dropped after a failed operation
- **THEN** cleanup is attempted for all steps (revert, unsync, delete)
- **THEN** errors during cleanup are ignored to ensure best-effort cleanup
