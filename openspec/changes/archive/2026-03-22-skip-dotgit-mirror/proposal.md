## Why

When someone submits a file named `.git` into Perforce (common with git submodules/worktrees that use a `.git` file), the mirror crashes. libgit2 refuses to create a tree entry named `.git` for security reasons, causing the entire mirror cycle to fail and block all subsequent changes from syncing.

## What Changes

- Skip files whose depot path contains `.git` as a path component during mirroring
- Log a warning when a file is skipped so operators know it happened
- Emit an observability event when files are skipped so it can be tracked

## Capabilities

### New Capabilities

None — this is a defensive fix within the existing mirror capability.

### Modified Capabilities

None — no spec-level behavior changes. The mirror already processes files from P4 changes; this adds a filter for paths that git cannot represent.

## Impact

- `src/mirror/mirror.rs` — file processing loop in `create_commit`
- `src/window/observability.rs` — new event variant for skipped files

## Observability

New event: `mirror.file_skipped` — emitted when a file is skipped during mirroring due to a path component that git cannot represent (e.g. `.git`). Includes the depot path and the reason for skipping. Allows operators to identify problematic files in the depot and fix them upstream.
