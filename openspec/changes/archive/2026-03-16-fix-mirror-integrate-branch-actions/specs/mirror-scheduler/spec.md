## ADDED Requirements

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
