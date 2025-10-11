# Perforce to Git Sync Actions

## Purpose

Provide concrete sync action implementations for synchronizing Perforce changelists into Git commits. These actions enable one-way sync from Perforce to Git, converting Perforce changelists into properly formatted Git commits with correct authorship and metadata.

## Design Goals

- Preserve Perforce changelist metadata in Git commits
- Maintain correct authorship information
- Support single changelist imports
- Handle Perforce-specific metadata gracefully
- Provide clear error messages for sync failures
- Follow the SyncAction base class patterns

## Components

### ImportChangelist

Imports a single Perforce changelist and creates a corresponding Git commit to the HEAD of the current branch with proper metadata and authorship.

Location: `src/prgit/sync/actions/perforce_to_git_actions.py`

#### Constructor

```python
ImportChangelist(
    git: GitEngine,
    perforce: PerforceEngine,
    changelist: int
)
```

**Parameters:**
- `changelist: int` - The Perforce changelist number to import

**Validation:**
- Changelist existence and validity is verified by querying the Perforce engine
- Raises `SyncExecutionError` if changelist does not exist or cannot be fetched (operation: "fetch_changelist")

#### Methods

##### perform() -> None

Executes the changelist import operation, creating a commit at the HEAD of the current Git branch.

**Operation Steps:**
1. Fetch changelist details from Perforce (description, author, timestamp, email, files)
2. Filter files based on client view mappings (only process files that match the client mappings)
3. Sync the filtered changelist files from Perforce to the working directory
4. Stage all changes in Git (including file permissions, symlinks, deletions)
5. Create a Git commit with formatted message and correct authorship

**Commit Message Format:**
```
{original_description}

[CL: {changelist_number}, user: {perforce_user}, date: {perforce_timestamp}]
```

The commit message consists of:
- Original changelist description
- Blank line separator
- Perforce metadata in bracket format

**Commit Authorship:**
- Author name: Perforce user's full name (from user information)
- Author email: Perforce user's email address (from user information)
- Author date: Original Perforce changelist submission timestamp
- Committer: Same as author (not current git user)
- Commit date: Same as author date

**File Handling:**
- Client view filtering: Only files matching the client mappings are processed (files outside the mapped depot paths are skipped)
- Binary files: Preserved as-is
- Text files: Applied with appropriate content
- Symlinks: Preserved as symlinks with correct target
- Execute bit: Applied to files that have execute permission in Perforce
- Deleted files: Removed from Git
- Renamed files: Handled as delete + add (Git will detect renames)

**Error Handling:**

Raises `SyncExecutionError` when:
- Changelist does not exist in Perforce (operation: "fetch_changelist")
- Changelist files cannot be synced (operation: "sync_changelist")
- Git staging fails (operation: "stage_files")
- Git commit creation fails (operation: "create_commit")
- Working directory is not clean (operation: "check_clean_state")

#### Protected Attributes

Inherited from SyncAction:
- `_git: GitEngine` - Git engine for commit operations
- `_perforce: PerforceEngine` - Perforce engine for changelist operations

Instance attributes:
- `_changelist: int` - The changelist number to import

#### Usage Example

```python
from prgit.sync.actions import ImportChangelist
from pathlib import Path

perforce_engine = PerforceEngine([
    ("//depot/project/...", Path("/workspace/project"))
])

action = ImportChangelist(
    git=git_engine,
    perforce=perforce_engine,
    changelist=12345
)

try:
    action.perform()
except SyncExecutionError as e:
    print(f"Import failed during {e.operation}: {e.message}")
```

**Client View Filtering Example:**

If a changelist contains files from multiple depot paths:
- `//depot/project/file1.py` (included)
- `//depot/project/subfolder/file2.py` (included)
- `//depot/other/file3.py` (excluded - not in client view)

And the Perforce engine is configured with mapping `//depot/project/...`, only files under `//depot/project/` will be synced and committed to Git. Files from `//depot/other/` will be silently skipped.

## Implementation Notes

### Clean State Validation

Before importing, verify that the working directory is clean:
- No uncommitted changes
- No untracked files that would conflict
- Raise `SyncExecutionError` with clear message if not clean

### Atomic Operations

The import must be atomic:
- Either complete successfully or leave no trace
- Use Git's staging area to prepare all changes
- Only commit if all files are staged successfully
- On failure, reset to previous state

## Testing Strategy

Tests in `tests/sync/actions/test_import_changelist.py`:

- Test basic changelist import with single file
- Test changelist import with user information
- Test importing multiple files in single changelist
- Test importing changelist with delete operations
- Test error handling for nonexistent changelists
- Test error handling for files with no revision
- Test error handling for missing file content
- Test automatic user creation when user not in system
- Test fallback authorship when user fetch fails
- Test client view filtering (files outside mappings are excluded from import)

