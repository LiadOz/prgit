## ADDED Requirements

### Requirement: Preserve executable bit during file extraction
When the shelver extracts files from a git commit to a temp directory, it SHALL set the executable permission (`0o755`) on files whose git tree entry has `BlobExecutable` filemode (`100755`).

#### Scenario: Executable file extracted with correct permissions
- **WHEN** a git tree entry has filemode `100755` (BlobExecutable)
- **THEN** the extracted file SHALL have executable permissions (`0o755`)

#### Scenario: Non-executable file extracted with default permissions
- **WHEN** a git tree entry has filemode `100644` (Blob)
- **THEN** the extracted file SHALL have default permissions (`0o644`)
