### Requirement: Content-providing edit helpers execute P4 edit immediately
Testkit helpers that provide content when editing files (`edit_file`, `edit_file_with_opts`) SHALL execute the P4 edit operation immediately before writing content.

#### Scenario: Edit synced read-only file with content
- **WHEN** `edit_file` is called on a P4-synced read-only file
- **THEN** the P4 edit command executes immediately
- **THEN** the file becomes writable
- **THEN** the content is written successfully

#### Scenario: Edit with file type override
- **WHEN** `edit_file_with_opts` is called with a file type
- **THEN** the P4 edit command executes immediately with the specified file type
- **THEN** the content is written successfully
