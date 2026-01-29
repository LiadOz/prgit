# changelist-builder Specification

## Purpose
TBD - created by archiving change 2026-01-29-batched-changelist-builder. Update Purpose after archive.
## Requirements
### Requirement: Builder collects operations without immediate execution

The `ChangelistBuilder` SHALL collect file operations (add, edit, delete, move) without executing P4 commands until explicitly flushed or submitted.

#### Scenario: Add multiple files without P4 calls
- **WHEN** user calls `builder.add("a.txt").add("b.txt")`
- **THEN** no P4 commands are executed until `submit()` or `flush()` is called

#### Scenario: Mixed operations are collected
- **WHEN** user calls `builder.add(...).edit(...).delete(...)`
- **THEN** all operations are stored internally without P4 calls

### Requirement: Flush executes batched P4 commands

The `flush()` method SHALL execute collected operations as batched P4 commands, grouped by action type and file type.

#### Scenario: Adds grouped by file type
- **WHEN** builder has 3 text files and 2 executable files to add
- **THEN** `flush()` executes 2 `p4 add` commands (one per file type)

#### Scenario: Edits and deletes batched separately
- **WHEN** builder has 5 edits and 3 deletes pending
- **THEN** `flush()` executes one `p4 edit` command and one `p4 delete` command

#### Scenario: Flush clears pending operations
- **WHEN** `flush()` completes successfully
- **THEN** the builder's pending operations list is empty

### Requirement: Submit flushes and submits the changelist

The `submit()` method SHALL flush any pending operations and then submit the changelist.

#### Scenario: Submit with pending operations
- **WHEN** builder has pending operations and user calls `submit()`
- **THEN** operations are flushed, then changelist is submitted

#### Scenario: Submit returns result
- **WHEN** `submit()` completes successfully
- **THEN** returns `SubmitResult` with the submitted change number

### Requirement: Immediate mode bypasses batching

The builder SHALL support an `immediate()` mode that executes each operation immediately without batching.

#### Scenario: Enable immediate mode
- **WHEN** user calls `builder.immediate()` before adding files
- **THEN** subsequent `add()` calls execute `p4 add` immediately

#### Scenario: Immediate mode per-operation
- **WHEN** user has immediate mode enabled
- **THEN** each `add()`, `edit()`, `delete()` executes its P4 command immediately

### Requirement: Builder creates changelist on construction

The builder SHALL create a pending changelist when constructed with a description.

#### Scenario: New builder creates changelist
- **WHEN** user creates `ChangelistBuilder::new(&p4, "description")`
- **THEN** a new pending changelist is created in P4
- **AND** the builder stores the changelist number

### Requirement: File type auto-detection

The builder SHALL auto-detect file types from the filesystem when possible, and allow explicit override when detection is not possible or desired.

#### Scenario: Detect executable file
- **WHEN** user calls `builder.add("script.sh")` and file has executable bit set
- **THEN** file type is detected as `text+x`

#### Scenario: Detect symlink
- **WHEN** user calls `builder.add("link")` and path is a symlink
- **THEN** file type is detected as `symlink`

#### Scenario: Default to text
- **WHEN** user calls `builder.add("file.txt")` and file is regular with no executable bit
- **THEN** file type is detected as `text`

#### Scenario: Explicit type overrides detection
- **WHEN** user calls `builder.add_with_type("file", FileType::binary())`
- **THEN** the explicit type is used, no detection performed

#### Scenario: Explicit type when file not on disk
- **WHEN** user calls `builder.add_with_type("future.txt", FileType::text())` and file doesn't exist
- **THEN** the explicit type is used without error

### Requirement: Batching groups by file type

The builder SHALL group operations by file type during flush to minimize P4 commands.

#### Scenario: Adds grouped by detected type
- **WHEN** builder has 3 text files and 2 executable files to add
- **THEN** `flush()` executes 2 `p4 add` commands (one per file type)

#### Scenario: Mixed explicit and detected types
- **WHEN** builder has adds with mixed detected and explicit types
- **THEN** operations are grouped by their final file type

### Requirement: Builder supports all file operations

The builder SHALL support add, edit, delete, and move operations on paths (P4 operations only, no file I/O).

#### Scenario: Add file with auto-detected type
- **WHEN** user calls `builder.add("path")`
- **THEN** file type is detected from filesystem and path is queued for `p4 add`

#### Scenario: Add file with explicit type
- **WHEN** user calls `builder.add_with_type("path", FileType::symlink())`
- **THEN** path is queued for `p4 add` with the specified file type

#### Scenario: Edit file with auto-detected type
- **WHEN** user calls `builder.edit("path")`
- **THEN** file type is detected and path is queued for `p4 edit`

#### Scenario: Edit file with explicit type
- **WHEN** user calls `builder.edit_with_type("path", FileType::text().executable())`
- **THEN** path is queued for `p4 edit` with the specified file type

#### Scenario: Delete file
- **WHEN** user calls `builder.delete("path")`
- **THEN** path is queued for `p4 delete`

#### Scenario: Move file
- **WHEN** user calls `builder.move_file("from", "to")`
- **THEN** source is queued for `p4 edit`, then `p4 move`

#### Scenario: Move file with explicit type
- **WHEN** user calls `builder.move_file_with_type("from", "to", FileType::text())`
- **THEN** move is queued with the specified file type

