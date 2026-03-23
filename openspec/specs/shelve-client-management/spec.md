# shelve-client-management Specification

## Purpose
Manages construction of authenticated P4 instances for shelve operations by retrieving stored tickets and validating their expiry before use.
## Requirements
### Requirement: Build authenticated P4 instance from stored ticket
The system SHALL retrieve a stored ticket for a given user and use it to construct an authenticated P4 instance for shelve operations.

#### Scenario: Ticket available and valid
- **WHEN** get_shelve_client is called for a user with a stored, non-expired ticket
- **THEN** the system retrieves the ticket from keyring, builds a P4 instance with it, and returns a ShelveClient

#### Scenario: Ticket expired
- **WHEN** get_shelve_client is called for a user whose ticket has expired
- **THEN** the system returns an auth failure error indicating re-authentication is needed

#### Scenario: No ticket stored
- **WHEN** get_shelve_client is called for a user with no stored ticket
- **THEN** the system returns an error indicating the user must authenticate first

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

### Requirement: Preserve executable bit during file extraction
When the shelver extracts files from a git commit to a temp directory, it SHALL set the executable permission (`0o755`) on files whose git tree entry has `BlobExecutable` filemode (`100755`).

#### Scenario: Executable file extracted with correct permissions
- **WHEN** a git tree entry has filemode `100755` (BlobExecutable)
- **THEN** the extracted file SHALL have executable permissions (`0o755`)

#### Scenario: Non-executable file extracted with default permissions
- **WHEN** a git tree entry has filemode `100644` (Blob)
- **THEN** the extracted file SHALL have default permissions (`0o644`)

