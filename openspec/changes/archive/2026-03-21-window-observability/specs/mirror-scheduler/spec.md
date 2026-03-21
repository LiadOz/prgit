## ADDED Requirements

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
