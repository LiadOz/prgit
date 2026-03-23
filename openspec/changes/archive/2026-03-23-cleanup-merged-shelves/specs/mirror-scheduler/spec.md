## ADDED Requirements

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
