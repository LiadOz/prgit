# Git to Perforce Sync Actions

## Purpose

Provide concrete sync action implementations for synchronizing Git branches into Perforce shelved changelists. These actions enable one-way sync from Git to Perforce, converting Git branch changes into shelves for code review and submission.

## Design Goals

- Map Git branches to Perforce shelved changelists
- Preserve file differences between base and feature branches
- Support shelf creation and updates
- Preserve file metadata (executable permissions, symlinks)
- Provide clear error messages for sync failures
- Follow the SyncAction base class patterns
- Keep Perforce metadata minimal (no Git commit history embedded)

## Components

### ExportBranchToShelf

Creates a new Perforce shelved changelist from a Git branch, containing the diff between the base branch and the feature branch tip.

Location: `src/prgit/sync/actions/git_to_perforce_actions.py`

#### Constructor

```python
ExportBranchToShelf(
    git: GitEngine,
    perforce: PerforceEngine,
    branch_name: str,
    base_branch: str = "master"
)
```

**Parameters:**
- `branch_name: str` - The Git branch to export as a shelf
- `base_branch: str` - The base branch to diff against (default: "master")

**Validation:**
- Branch must exist in Git repository
- Base branch must exist in Git repository
- Branch cannot be the same as base branch (no self-diff)
- Raises `SyncConfigurationError` if branch validation fails (parameter: "branch_name" or "base_branch")

#### Methods

##### perform() -> int

Executes the branch export operation, creating a new shelved changelist in Perforce and returning its number.

**Operation Steps:**
1. Validate branch and base branch exist in Git
2. Get commits in branch that aren't in base branch (for verification)
3. Get file differences between base branch tip and feature branch tip
4. Extract file content and metadata (permissions, symlink targets)
5. Create a new pending changelist in Perforce with branch description
6. Shelve the changed files to the changelist
7. Return the changelist number

**Changelist Description Format:**
```
Branch: {branch_name}
```

Simple description indicating the branch name. No Git metadata (commit hashes, authors, timestamps) is embedded in Perforce.

**File Handling:**
- Binary files: Shelved as-is with correct file type
- Text files: Shelved with appropriate content
- Symlinks: Preserved as symlinks with correct target path
- Executable files: Preserved with execute permission set
- Deleted files: Marked for delete in the shelf
- Added files: Marked for add in the shelf
- Modified files: Marked for edit in the shelf

**File Metadata Preservation:**
- Execute bit: Files with execute permission in Git must maintain that permission in Perforce
- Symlinks: Symbolic links must be stored as symlinks (not as file content) with the target path preserved
- File type detection: Perforce file types (binary, text, symlink, executable) must match Git file properties

**File Path Mapping:**
Git relative paths are converted to Perforce depot paths using the client mappings:
- Git path: `project/src/file.py`
- Client mapping: `("//depot/project/...", Path("/workspace/project"))`
- Depot path: `//depot/project/src/file.py`

**Error Handling:**

Raises `SyncConfigurationError` when:
- Branch does not exist in Git (parameter: "branch_name")
- Base branch does not exist in Git (parameter: "base_branch")
- Branch is the same as base branch (parameter: "branch_name")

Raises `SyncExecutionError` when:
- No file differences between branches (operation: "calculate_diff")
- Changelist creation fails (operation: "create_changelist")
- File shelving fails (operation: "shelve_files")

#### Protected Attributes

Inherited from SyncAction:
- `_git: GitEngine` - Git engine for branch and file operations
- `_perforce: PerforceEngine` - Perforce engine for changelist and shelf operations

Instance attributes:
- `_branch_name: str` - The branch name to export
- `_base_branch: str` - The base branch to diff against

#### Usage Example

```python
from prgit.sync.actions import ExportBranchToShelf
from pathlib import Path

perforce_engine = PerforceEngine([
    ("//depot/project/...", Path("/workspace/project"))
])

action = ExportBranchToShelf(
    git=git_engine,
    perforce=perforce_engine,
    branch_name="feature/new-api",
    base_branch="master"
)

try:
    changelist_number = action.perform()
    print(f"Created shelf in changelist {changelist_number}")
except SyncConfigurationError as e:
    print(f"Configuration error for {e.parameter}: {e.message}")
except SyncExecutionError as e:
    print(f"Export failed during {e.operation}: {e.message}")
```

### UpdateShelfFromBranch

Updates an existing Perforce shelved changelist with the current state of a Git branch, replacing the previous shelf contents.

Location: `src/prgit/sync/actions/git_to_perforce_actions.py`

#### Constructor

```python
UpdateShelfFromBranch(
    git: GitEngine,
    perforce: PerforceEngine,
    branch_name: str,
    changelist_number: int,
    base_branch: str = "master"
)
```

**Parameters:**
- `branch_name: str` - The Git branch to export as a shelf
- `changelist_number: int` - The existing changelist to update
- `base_branch: str` - The base branch to diff against (default: "master")

**Validation:**
- All validations from ExportBranchToShelf apply
- Changelist must exist in Perforce
- Changelist must be in pending or shelved status (not submitted)
- Raises `SyncConfigurationError` for branch validation failures
- Raises `SyncConfigurationError` if changelist doesn't exist or is submitted (parameter: "changelist_number")

#### Methods

##### perform() -> None

Executes the shelf update operation, replacing the existing shelf with new branch content.

**Operation Steps:**
1. Validate changelist exists and is updatable (pending or shelved status)
2. Validate branch and base branch exist in Git
3. Get file differences between base branch tip and feature branch tip
4. Extract file content and metadata (permissions, symlink targets)
5. Update the changelist description (maintains simple format)
6. Re-shelve all files to the changelist (replaces previous shelf contents)

**Changelist Description Update:**
```
Branch: {branch_name}
```

The description is updated to reflect the current branch name, maintaining the simple format with no Git metadata.

**Shelf Replacement:**
- Previous shelf contents are completely replaced
- All new file changes are shelved
- Files that were in the previous shelf but not in the new diff are removed from the shelf

**Error Handling:**

Raises `SyncConfigurationError` when:
- Branch validation fails (same as ExportBranchToShelf)
- Changelist does not exist (parameter: "changelist_number")
- Changelist is already submitted (parameter: "changelist_number")
- Changelist status is invalid (parameter: "changelist_number")

Raises `SyncExecutionError` when:
- No file differences between branches (operation: "calculate_diff")
- Changelist description update fails (operation: "update_changelist")
- File re-shelving fails (operation: "shelve_files")

#### Protected Attributes

Inherited from SyncAction:
- `_git: GitEngine` - Git engine for branch and file operations
- `_perforce: PerforceEngine` - Perforce engine for changelist and shelf operations

Instance attributes:
- `_branch_name: str` - The branch name to export
- `_changelist_number: int` - The changelist to update
- `_base_branch: str` - The base branch to diff against

#### Usage Example

```python
from prgit.sync.actions import UpdateShelfFromBranch

action = UpdateShelfFromBranch(
    git=git_engine,
    perforce=perforce_engine,
    branch_name="feature/new-api",
    changelist_number=12345,
    base_branch="master"
)

try:
    action.perform()
    print(f"Updated shelf in changelist 12345")
except SyncConfigurationError as e:
    print(f"Configuration error for {e.parameter}: {e.message}")
except SyncExecutionError as e:
    print(f"Update failed during {e.operation}: {e.message}")
```

## Implementation Notes

### Git Engine Requirements

These actions require methods to extract file differences between branches. There are multiple approaches to achieve this, each with different tradeoffs.

#### Option 1: Add Dedicated Diff Method (Recommended)

Add a new method to `GitEngine` interface that directly returns file differences:

```python
@dataclass(frozen=True)
class FileDiff:
    path: Path
    content: bytes | None
    is_executable: bool
    is_symlink: bool
    symlink_target: str | None
    operation: FileOperationType

class FileOperationType(StrEnum):
    ADD = "add"
    MODIFY = "modify"
    DELETE = "delete"

def get_diff_files(
    self, 
    from_commit: str, 
    to_commit: str
) -> list[FileDiff]:
    pass
```

**Advantages:**
- Clean, purpose-built interface
- Handles all metadata (permissions, symlinks) in one place
- Single method call to get all needed information
- Efficient implementation possible (GitPython's diff utilities)

**Disadvantages:**
- Requires new types and method in Git engine interface
- Adds complexity to the engine

#### Option 2: Add File Content Extraction Method

Add a method to get all files from a specific commit:

```python
def get_commit_files(self, commit_hash: str) -> dict[Path, bytes]:
    pass

def get_file_metadata(self, commit_hash: str, path: Path) -> FileMetadata:
    pass
```

Then implement diff logic in the action:
1. Get files from base branch tip: `base_files = git.get_commit_files(base_commit)`
2. Get files from feature branch tip: `feature_files = git.get_commit_files(feature_commit)`
3. Compare dictionaries to find added, modified, deleted files
4. Extract metadata for each file

**Advantages:**
- Simpler Git engine methods (just file extraction)
- Diff logic is in the action (easier to customize)
- Reusable methods for other purposes

**Disadvantages:**
- Less efficient (loads all files from both commits)
- Diff logic must be implemented in action
- Multiple method calls needed
- Metadata extraction requires separate calls

#### Option 3: Use Existing Commit Objects

Use existing `get_commits()` and `get_commit()` methods with extended `Commit` dataclass:

**Extend Commit dataclass:**
```python
@dataclass(frozen=True)
class Commit:
    hash: str
    author: Author
    timestamp: datetime
    message: str
    parent_hashes: list[str]
    files: dict[Path, FileInfo]  # NEW

@dataclass(frozen=True)
class FileInfo:
    content: bytes
    is_executable: bool
    is_symlink: bool
    symlink_target: str | None
```

Then implement diff in action:
1. Get base commit: `base_commit = git.get_commit(base_hash)`
2. Get feature commit: `feature_commit = git.get_commit(feature_hash)`
3. Compare `base_commit.files` vs `feature_commit.files`

**Advantages:**
- No new methods needed
- All information in one place
- Natural Git model (commits contain files)

**Disadvantages:**
- Changes existing Commit dataclass (breaking change)
- Loads all files for commits (memory intensive)
- May not match existing code that expects lightweight Commit objects

#### Option 4: Leverage GitPython Directly (Real Engine Only)

For `RealGitEngine`, use GitPython's diff utilities directly in the action:

```python
def perform(self) -> int:
    base = self._git.repo.commit(base_hash)
    feature = self._git.repo.commit(feature_hash)
    diffs = base.diff(feature)
    
    for diff in diffs:
        # Extract file path, content, metadata
        pass
```

**Advantages:**
- No Git engine changes needed
- Leverages GitPython's optimized diff
- Most efficient approach

**Disadvantages:**
- Breaks abstraction (action directly uses GitPython)
- Doesn't work with VirtualGitEngine
- Not testable without real Git repository
- Tightly couples action to GitPython implementation

#### Recommendation

**Option 1 (Dedicated Diff Method)** is recommended because:
- Maintains clean abstraction boundaries
- Provides all needed information in one call
- Allows efficient implementation in both Real and Virtual engines
- Future-proof for other actions that need diffs
- Explicit about metadata handling (permissions, symlinks)

The `FileDiff` dataclass should be added to `src/prgit/sync/git/types.py` and the method should be added to the `GitEngine` abstract interface.

### File Path Conversion

**Git to Depot Path Mapping:**

Git stores files as relative paths from the repository root. These must be converted to Perforce depot paths using client mappings:

1. Get Git file path (relative): `src/module/file.py`
2. Find matching client mapping: `("//depot/project/...", Path("/workspace/project"))`
3. Construct depot path: `//depot/project/src/module/file.py`

**Implementation:**
```python
def git_path_to_depot_path(
    git_path: Path,
    mappings: list[tuple[str, Path]]
) -> str:
    for depot_pattern, local_base in mappings:
        depot_root = depot_pattern.rstrip("/...").rstrip("/")
        return f"{depot_root}/{git_path}"
    raise ValueError(f"No mapping found for {git_path}")
```

### File Metadata Handling

**Executable Files:**

Files with the execute bit set in Git must be shelved with the correct Perforce file type:
- Detect execute permission from Git file mode
- Set Perforce file type to `+x` (executable)
- Perforce file types: `text+x`, `binary+x`, etc.

**Symlinks:**

Symbolic links must be preserved as symlinks in Perforce:
- Detect symlink from Git file mode
- Extract symlink target path
- Set Perforce file type to `symlink`
- Store target path (not file content) in Perforce

**File Type Detection:**

Git file modes:
- `100644` - Regular file
- `100755` - Executable file
- `120000` - Symbolic link

The Git engine's diff method must provide this information through the `FileDiff` dataclass (`is_executable`, `is_symlink`, `symlink_target` fields).

### Processing Diffs for Perforce

Once file diffs are obtained from Git, they must be processed and converted to Perforce shelf format. Here's the complete flow:

#### Step 1: Convert Git Paths to Depot Paths

For each `FileDiff` object, convert the Git relative path to a Perforce depot path:

```python
file_diffs = git.get_diff_files(base_commit, feature_commit)

depot_files: dict[str, bytes] = {}
for diff in file_diffs:
    depot_path = git_path_to_depot_path(diff.path, perforce._mappings)
```

#### Step 2: Prepare File Content Based on Type

Handle different file types appropriately:

**Regular Files:**
```python
if not diff.is_symlink and diff.operation != FileOperationType.DELETE:
    depot_files[depot_path] = diff.content
```

**Symlinks:**
```python
if diff.is_symlink and diff.operation != FileOperationType.DELETE:
    depot_files[depot_path] = diff.symlink_target.encode('utf-8')
```

For symlinks, the content is the target path encoded as bytes.

**Deleted Files:**
```python
if diff.operation == FileOperationType.DELETE:
    depot_files[depot_path] = None
```

Use `None` to signal deletion (or handle separately based on Perforce engine implementation).

#### Step 3: Handle File Metadata

The file metadata (executable bit, symlink flag) must be communicated to Perforce. There are two approaches:

**Approach A: Extend shelve_files Interface**

Modify the Perforce engine's `shelve_files` signature to accept metadata:

```python
@dataclass(frozen=True)
class ShelveFile:
    depot_path: str
    content: bytes | None
    is_executable: bool
    is_symlink: bool

def shelve_files(
    self, 
    changelist_number: int, 
    files: list[ShelveFile]
) -> ShelvedChange:
    pass
```

This allows passing all metadata explicitly.

**Approach B: Encode Metadata in File Type**

Keep the current interface but determine file type from content/path:

```python
def shelve_files(
    self, 
    changelist_number: int, 
    files: dict[str, bytes]
) -> ShelvedChange:
    # Implementation infers file types
    # - Detects binary vs text from content
    # - Symlinks passed with special marker
    # - No way to specify executable bit
```

**Recommendation: Approach A** - Extend the interface to explicitly handle metadata. This makes the contract clear and allows proper handling of executable files and symlinks.

#### Step 4: Create Changelist and Shelve

```python
changelist = perforce.create_changelist(f"Branch: {branch_name}")

shelve_files_list = [
    ShelveFile(
        depot_path=depot_path,
        content=content,
        is_executable=diff.is_executable,
        is_symlink=diff.is_symlink
    )
    for diff in file_diffs
    for depot_path in [git_path_to_depot_path(diff.path, perforce._mappings)]
    for content in [diff.symlink_target.encode('utf-8') if diff.is_symlink else diff.content]
]

perforce.shelve_files(changelist.number, shelve_files_list)
```

#### Complete Example Flow

```python
def perform(self) -> int:
    base_commit = self._git.get_branches()  # get base branch commit
    feature_commit = self._git.get_branches()  # get feature branch commit
    
    file_diffs = self._git.get_diff_files(base_commit, feature_commit)
    
    if not file_diffs:
        raise SyncExecutionError(
            "No differences between branches",
            action=self,
            operation="calculate_diff"
        )
    
    changelist = self._perforce.create_changelist(f"Branch: {self._branch_name}")
    
    try:
        shelve_files_list = []
        for diff in file_diffs:
            depot_path = self._git_path_to_depot_path(diff.path)
            
            if diff.operation == FileOperationType.DELETE:
                content = None
            elif diff.is_symlink:
                content = diff.symlink_target.encode('utf-8')
            else:
                content = diff.content
            
            shelve_files_list.append(ShelveFile(
                depot_path=depot_path,
                content=content,
                is_executable=diff.is_executable,
                is_symlink=diff.is_symlink
            ))
        
        self._perforce.shelve_files(changelist.number, shelve_files_list)
        return changelist.number
        
    except Exception as e:
        # Cleanup: try to delete the changelist we created
        try:
            self._perforce.delete_changelist(changelist.number)
        except:
            pass
        raise SyncExecutionError(
            f"Failed to shelve files: {e}",
            action=self,
            operation="shelve_files"
        )
```

#### Perforce Engine Implementation Details

The Perforce engine's `shelve_files` implementation must:

1. **Write files to workspace:**
   ```python
   for shelve_file in files:
       local_path = self._depot_to_local(shelve_file.depot_path)
       local_path.parent.mkdir(parents=True, exist_ok=True)
       
       if shelve_file.content is not None:
           local_path.write_bytes(shelve_file.content)
   ```

2. **Mark files in Perforce:**
   ```python
   if shelve_file.content is None:
       # Deleted file
       self._p4.run_delete("-c", str(changelist_number), str(local_path))
   elif file_exists_in_depot:
       # Modified file
       self._p4.run_edit("-c", str(changelist_number), str(local_path))
   else:
       # New file
       self._p4.run_add("-c", str(changelist_number), str(local_path))
   ```

3. **Set file type flags:**
   ```python
   if shelve_file.is_symlink:
       self._p4.run_reopen("-t", "symlink", str(local_path))
   elif shelve_file.is_executable:
       # Determine base type (text or binary)
       base_type = self._detect_file_type(shelve_file.content)
       self._p4.run_reopen("-t", f"{base_type}+x", str(local_path))
   ```

4. **Execute shelf command:**
   ```python
   self._p4.run_shelve("-c", str(changelist_number))
   ```

This ensures all file metadata is properly preserved in the Perforce shelf.

### Atomic Operations

Both operations should be atomic:
- Either complete successfully or leave no trace (for ExportBranchToShelf)
- On failure during changelist creation, attempt to clean up the pending changelist
- On failure during shelf update, the previous shelf remains intact
- Perforce's shelf mechanism provides some atomicity (shelve replaces atomically)

### Empty Diff Handling

If the branch has no differences from the base branch:
- Raise `SyncExecutionError` with operation "calculate_diff"
- Message should indicate no changes to shelve
- This prevents creating empty shelves

## Action Exports

Both action classes are exported from the actions module for easy access.

Location: `src/prgit/sync/actions/__init__.py`

```python
from .git_to_perforce_actions import (
    ExportBranchToShelf,
    UpdateShelfFromBranch,
)

__all__ = [
    "ExportBranchToShelf",
    "UpdateShelfFromBranch",
]
```

The module serves as the namespace for accessing actions:

```python
from prgit.sync import actions

action = actions.ExportBranchToShelf(git, perforce, branch_name="feature-x")
changelist_num = action.perform()
```

Or import actions directly:

```python
from prgit.sync.actions import ExportBranchToShelf

action = ExportBranchToShelf(git, perforce, branch_name="feature-x")
changelist_num = action.perform()
```

