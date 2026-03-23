## ADDED Requirements

### Requirement: Preserve P4 file type modifiers on edit
When editing an existing file, the shelver SHALL preserve all P4 file type modifiers from the depot (e.g. `+C`, `+k`, `+l`, `+F`, `+D`, `+w`). Only the executable bit and base type (text/binary/symlink) SHALL be derived from the git working copy. All other modifiers SHALL be inherited from the depot file's existing type.

#### Scenario: Edit file with compressed modifier
- **WHEN** a depot file has type `text+Cx` and the git working copy edits the content without changing permissions
- **THEN** the shelved file SHALL retain type `text+Cx`

#### Scenario: Edit adds executable to file with modifiers
- **WHEN** a depot file has type `text+C` (compressed, not executable) and the git working copy has the executable bit set
- **THEN** the shelved file SHALL have type `text+Cx` (compressed preserved, executable added)

#### Scenario: Edit removes executable from file with modifiers
- **WHEN** a depot file has type `text+kx` (keyword expansion + executable) and the git working copy does not have the executable bit set
- **THEN** the shelved file SHALL have type `text+k` (keyword expansion preserved, executable removed)

#### Scenario: Edit changes file to symlink
- **WHEN** a depot file has type `text+C` and the git working copy is a symlink
- **THEN** the shelved file SHALL have type `symlink` (base type change overrides modifiers)

### Requirement: Only reopen when type actually changes
The shelver SHALL only call `p4 reopen` when the effective file type (considering git-derivable attributes applied to the depot type) differs from the depot type. If only content changed and the type is unchanged, no `reopen` SHALL occur.

#### Scenario: Content-only edit on file with modifiers
- **WHEN** a `text+Cx` file is edited with no permission change
- **THEN** no `p4 reopen` SHALL be called (type is unchanged)
