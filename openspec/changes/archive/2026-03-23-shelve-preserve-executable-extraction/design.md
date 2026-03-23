## Context

The shelver extracts files from git commits to a temp directory via `extract_files_to_temp` in `shelver.rs`. It uses `std::fs::write` which creates files with default `0o644` permissions. The git tree entry's filemode (`100755` for executable) is available via `entry.filemode()` but was not used.

Downstream, `determine_file_type` checks disk permissions to decide if a file is executable. Without the correct permissions on the extracted file, it always returns `text` (non-executable), causing the edit path to incorrectly strip `+x` from the depot type.

## Goals / Non-Goals

**Goals:**
- Set `0o755` on extracted executable files so `determine_file_type` returns the correct type

**Non-Goals:**
- Changing how symlinks are extracted (already handled by `copy_file`)
- Changing the edit path logic (already fixed in `shelve-preserve-file-type`)

## Decisions

### 1. Check `entry.filemode()` against `BlobExecutable`

**Decision:** After `std::fs::write`, check if `entry.filemode() == i32::from(git2::FileMode::BlobExecutable)` and set permissions to `0o755`.

**Why:** This is the git-native way to check executable status. The filemode is already available from the tree walk. No additional git operations needed.

## Risks / Trade-offs

None — this is a 3-line fix that adds permissions that should have always been set.
