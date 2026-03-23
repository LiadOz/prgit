## Purpose
Background mirror task scheduling that periodically syncs P4 changes to git, with failure resilience and observability.
## Requirements
### Requirement: Background mirror task per repo
The server SHALL spawn one background task per configured repo that periodically runs the mirror to sync P4 changes to git.

#### Scenario: Mirror runs on interval
- **WHEN** a repo is configured with `mirror_interval_secs: 60`
- **THEN** the server SHALL run `Mirror.run()` for that repo approximately every 60 seconds

#### Scenario: Mirror tasks start on server startup
- **WHEN** the server starts with configured repos
- **THEN** the server SHALL spawn a mirror task for each repo and run the first mirror iteration immediately

### Requirement: Mirror failure resilience
A mirror task failure SHALL NOT crash the server or affect other repos.

#### Scenario: Mirror iteration fails
- **WHEN** a mirror iteration fails (e.g., P4 server unreachable)
- **THEN** the server SHALL log the error and continue scheduling the next iteration

### Requirement: Blocking P4 calls run off async runtime
Mirror operations (which use synchronous P4 commands) SHALL run via `spawn_blocking` to avoid blocking the tokio async runtime.

#### Scenario: Mirror does not block async tasks
- **WHEN** a mirror iteration runs a slow P4 query
- **THEN** HTTP request handling and other mirror tasks SHALL continue to operate without blocking

### Requirement: Mirror handles branch file actions
The mirror SHALL treat `FileAction::Branch` as a content-bearing action and upsert the file into the git tree, identical to `Add` or `Edit`.

#### Scenario: Branched file appears in git
- **WHEN** a P4 changelist contains a file with `branch` action
- **THEN** the mirrored git commit SHALL contain that file with the correct content from `p4 print`

#### Scenario: Changelist with only branch actions produces non-empty commit
- **WHEN** a P4 changelist contains only `branch` action files
- **THEN** the mirrored git commit SHALL include all branched files (not an empty diff)

### Requirement: Mirror handles integrate file actions
The mirror SHALL treat `FileAction::Integrate` as a content-bearing action and upsert the file into the git tree, identical to `Add` or `Edit`.

#### Scenario: Integrated file appears in git
- **WHEN** a P4 changelist contains a file with `integrate` action
- **THEN** the mirrored git commit SHALL contain that file with the correct content from `p4 print`

#### Scenario: Changelist with mixed integrate and regular actions
- **WHEN** a P4 changelist contains both `integrate` and `add`/`edit` action files
- **THEN** the mirrored git commit SHALL include all files regardless of action type

### Requirement: Emit mirror cycle events
The mirror task SHALL emit `mirror.cycle_started` before calling `mirror.run()` and `mirror.cycle_completed` or `mirror.cycle_failed` after, with duration_ms and changes_synced count.

#### Scenario: Successful mirror cycle
- **WHEN** a mirror iteration completes with 5 changes synced
- **THEN** `mirror.cycle_started` and `mirror.cycle_completed` SHALL be emitted with changes_synced=5 and the measured duration_ms

#### Scenario: Failed mirror cycle
- **WHEN** a mirror iteration fails
- **THEN** `mirror.cycle_started` and `mirror.cycle_failed` SHALL be emitted with the error

### Requirement: Emit per-change commit events
The mirror SHALL emit a `mirror.change_committed` event after each P4 change is committed to git, including the merge parent and merge strategy if MergeOurs was used.

#### Scenario: Regular change committed
- **WHEN** a P4 change is mirrored without a merge parent
- **THEN** a `mirror.change_committed` event SHALL be emitted with merge_parent=None and file_count from the change context

#### Scenario: Merged change committed
- **WHEN** a P4 change with old_change maps to a branch in branch_shelve_mapping
- **THEN** a `mirror.change_committed` event SHALL be emitted with merge_parent set to the branch name and merge_strategy="merge_ours"

### Requirement: Emit shelve.merged on branch reintegration
When the mirror detects that a P4 change's old_change maps to a shelved branch, the mirror SHALL emit a `shelve.merged` event with the shelved CL, submitted CL, branch name, and original shelver user.

#### Scenario: Shelved branch submitted in P4
- **WHEN** a P4 change is committed with old_change=123 and branch_shelve_mapping maps CL 123 to branch "feature-x" shelved by user "jdoe"
- **THEN** a `shelve.merged` event SHALL be emitted with shelved_cl=123, submitted_cl=(the new change number), branch="feature-x", shelver_user="jdoe"

### Requirement: Delete shelved CL after merge detection
When the mirror detects that a shelved branch has been submitted in P4, it SHALL delete the shelved changelist using `p4 shelve -d` and remove the `branch_shelve_mapping` entry for that branch.

#### Scenario: Shelved CL cleaned up after submit
- **WHEN** the mirror detects a merge for branch "feature-x" with shelved CL 123
- **THEN** the mirror SHALL call `p4 shelve -d -c 123` and remove the branch_shelve_mapping entry for "feature-x"

#### Scenario: Alias CL resolved before cleanup
- **WHEN** the submitted CL was created through a CL alias (submitted CL differs from the original shelved CL)
- **THEN** the mirror SHALL resolve the alias to the original shelved CL and delete that CL

#### Scenario: Cleanup failure does not block mirroring
- **WHEN** `p4 shelve -d` fails (e.g. CL already deleted, permission error)
- **THEN** the mirror SHALL log the error at warn level and continue processing

