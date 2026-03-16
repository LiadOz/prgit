## ADDED Requirements

### Requirement: Remove tolerates missing files

`CommitBuilder::remove()` SHALL succeed without error when the target path does not exist in the git tree. Missing paths SHALL be skipped with a warning log.

#### Scenario: Remove a file that exists in the tree
- **WHEN** `remove("dir/file.txt")` is called and `dir/file.txt` exists in the base tree
- **THEN** the file SHALL be removed from the resulting tree

#### Scenario: Remove a file that does not exist in the tree
- **WHEN** `remove("dir/gone.txt")` is called and `dir/gone.txt` does not exist in the base tree
- **THEN** the remove SHALL be skipped without error
- **AND** a warning log SHALL be emitted: `Skipping remove of 'dir/gone.txt': not in tree`

#### Scenario: Double-delete from P4 history
- **WHEN** P4 history contains two consecutive Delete actions for the same file (e.g., #2 delete then #3 delete)
- **AND** the first delete was already mirrored (file removed from git)
- **THEN** the second delete SHALL be skipped without error during mirroring

#### Scenario: Delete of a file from a skipped Branch/Integrate action
- **WHEN** P4 reports a Delete action for a file that was never added to git (e.g., arrived via a skipped Branch/Integrate action)
- **THEN** the remove SHALL be skipped without error during mirroring

### Requirement: Removes are deferred until tree building

`CommitBuilder` SHALL defer remove operations until `build_tree()` is called, rather than queuing them immediately into the underlying `TreeUpdateBuilder`.

#### Scenario: Remove is collected but not applied immediately
- **WHEN** `remove("file.txt")` is called
- **THEN** the path SHALL be stored in a pending collection
- **AND** no call to `TreeUpdateBuilder::remove()` SHALL occur until `build_tree()`

#### Scenario: Pending removes are applied during build_tree
- **WHEN** `build_tree()` is called with pending removes
- **THEN** each pending path SHALL be checked against the resolved base tree
- **AND** only paths that exist in the base tree SHALL be passed to `TreeUpdateBuilder::remove()`
