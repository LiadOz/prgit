# Git Engine Interface

## Purpose

Provide an abstraction layer for git operations within the sync engine. The interface supports both real git commands (via GitPython) and a virtual in-memory implementation for testing.

## Design Goals

- Strong typing throughout the interface
- Testability without mocking or patching
- Support extended git operations needed by sync engine
- Clean separation between interface and implementation
- Constructor-based dependency injection

## Components

### GitEngine (Abstract Base Class)

Abstract interface defining all git operations. Implementations must provide concrete behavior.

Location: `src/prgit/sync/git/abstract_engine.py`

#### Methods

- `init_repo(path: Path) -> None`: Initialize a new git repository
- `clone_repo(source: str, target_path: Path) -> None`: Clone a repository from source URL/path
- `export_repository() -> Repository`: Export current repository state as a Repository object
- `get_commits(branch: str | None = None) -> list[Commit]`: Get commits from a branch (or current branch if None)
- `get_commit(commit_hash: str) -> Commit`: Get a specific commit by hash
- `get_branches() -> list[Branch]`: List all branches
- `get_current_branch() -> Branch | None`: Get current branch (None if detached HEAD)
- `create_branch(name: str, from_commit: str | None = None) -> Branch`: Create a new branch
- `checkout(branch_or_commit: str) -> None`: Checkout a branch or commit
- `delete_branch(name: str, force: bool = False) -> None`: Delete a branch
- `get_file_status() -> list[FileStatus]`: Get working directory status
- `stage_and_commit(files: dict[Path, bytes], message: str, author: Author, timestamp: datetime | None = None) -> Commit`: Stage files and create a commit
- `merge(branch: str, message: str | None = None) -> Commit`: Merge a branch

### RealGitEngine

Implementation using GitPython library for actual git operations on filesystem.

Location: `src/prgit/sync/git/real_engine.py`

#### Constructor

```python
RealGitEngine(repo_path: Path)
```

#### Implementation Notes

- Wraps GitPython's `Repo` class
- Adds comprehensive type hints where GitPython lacks them
- Converts GitPython objects to our dataclasses
- Raises appropriate exceptions on git errors
- `export_repository()` reads all commits, branches, and HEAD from the filesystem repository and creates a `Repository` object

### VirtualGitEngine

In-memory implementation simulating git behavior for testing.

Location: `src/prgit/sync/git/virtual_engine.py`

#### Constructor

```python
VirtualGitEngine()
```

#### Implementation Notes

- Maintains in-memory state (commits, branches, files)
- No filesystem interaction
- Simulates git behavior accurately enough for testing
- For cloning, queries the `VirtualGitRegistry` singleton to resolve source strings to `Repository` objects
- When cloning, imports the Repository's commits, branches, and HEAD into its internal state

#### State Management

Internal state structure:
- `_commits: dict[str, Commit]`: All commits by hash
- `_branches: dict[str, str]`: Branch names to commit hashes
- `_head: str | None`: Current HEAD (commit hash or branch name)
- `_working_files: dict[Path, bytes]`: Working directory files

`export_repository()` creates a `Repository` from the internal state, copying commits, branches, and HEAD.

### VirtualGitRegistry

Type alias to generic `VirtualRegistry[Repository]` for testing. Acts as the central repository host for virtual engines.

Location: `src/prgit/sync/git/__init__.py`

```python
from prgit.sync.virtual_registry import VirtualRegistry
from prgit.sync.git.types import Repository

VirtualGitRegistry = VirtualRegistry[Repository]
```

#### Methods

- `instance() -> VirtualRegistry[Repository]`: Get singleton instance
- `register(identifier: str, repository: Repository) -> None`: Register a Repository with an identifier
- `unregister(identifier: str) -> None`: Remove a repository from the registry
- `get(identifier: str) -> Repository`: Retrieve a registered Repository by identifier
- `clear() -> None`: Clear all registered repositories (useful for test cleanup)

#### Usage Pattern

```python
from prgit.sync.git import VirtualGitRegistry

registry = VirtualGitRegistry.instance()
registry.register("virtual://my-repo", repository)
```

#### Purpose

Provides a centralized registry for `Repository` objects that can be cloned. When `VirtualGitEngine.clone_repo()` is called with a registered identifier, it queries this registry to get the `Repository` and imports its state. Repositories can be manually constructed or exported from engines.

#### Implementation Notes

- Uses generic `VirtualRegistry[T]` from `src/prgit/sync/virtual_registry.py`
- See `docs/spec/sync/virtual_registry.md` for implementation details
- Singleton pattern ensures single registry instance per type
- Thread-safe for concurrent test execution
- Used exclusively with `VirtualGitEngine` for testing

### Dataclasses

Location: `src/prgit/sync/git/types.py`

#### Author

```python
@dataclass(frozen=True)
class Author:
    name: str
    email: str
```

#### Commit

```python
@dataclass(frozen=True)
class Commit:
    hash: str
    author: Author
    timestamp: datetime
    message: str
    parent_hashes: list[str]
```

#### Branch

```python
@dataclass(frozen=True)
class Branch:
    name: str
    commit_hash: str
```

#### FileStatus

```python
@dataclass(frozen=True)
class FileStatus:
    path: Path
    status: FileStatusType
```

#### FileStatusType (StrEnum)

```python
class FileStatusType(StrEnum):
    ADDED = "added"
    MODIFIED = "modified"
    DELETED = "deleted"
    UNTRACKED = "untracked"
```

#### Repository

```python
@dataclass(frozen=True)
class Repository:
    commits: dict[str, Commit]
    branches: dict[str, str]
    head: str
```

Represents a complete git repository state. Contains all commits indexed by hash, all branches with their commit hashes, and the current HEAD reference (branch name or commit hash). Files are reconstructed from commits when needed.

## Usage Pattern

### Production

```python
from pathlib import Path
from prgit.sync.git import RealGitEngine

git = RealGitEngine(Path("/path/to/repo"))
commits = git.get_commits()
```

### Testing

#### Manual Repository Construction

```python
from pathlib import Path
from datetime import datetime
from prgit.sync.git import VirtualGitEngine, VirtualGitRegistry, Author, Commit, Repository

author = Author("Test User", "test@example.com")
commit = Commit(
    hash="abc123",
    author=author,
    timestamp=datetime.now(),
    message="Initial commit",
    parent_hashes=[]
)

repository = Repository(
    commits={"abc123": commit},
    branches={"main": "abc123"},
    head="main"
)

registry = VirtualGitRegistry.instance()
registry.register("virtual://test-repo", repository)

cloned_git = VirtualGitEngine()
cloned_git.clone_repo("virtual://test-repo", Path("/fake/clone"))
```

#### Export from Engine

```python
from pathlib import Path
from prgit.sync.git import RealGitEngine, VirtualGitEngine, VirtualGitRegistry

real_git = RealGitEngine(Path("/real/repo"))
repository = real_git.export_repository()

registry = VirtualGitRegistry.instance()
registry.register("virtual://imported-repo", repository)

virtual_git = VirtualGitEngine()
virtual_git.clone_repo("virtual://imported-repo", Path("/fake/clone"))
```

## Dependencies

- **GitPython**: Real git implementation (add to pyproject.toml)
- **pytest**: Testing framework (already in dev dependencies)

## Package Exports

`src/prgit/sync/git/__init__.py` exports:

```python
from prgit.sync.git.abstract_engine import GitEngine
from prgit.sync.git.real_engine import RealGitEngine
from prgit.sync.git.virtual_engine import VirtualGitEngine, VirtualGitRegistry
from prgit.sync.git.types import (
    Author,
    Branch,
    Commit,
    FileStatus,
    FileStatusType,
    Repository,
)

__all__ = [
    "GitEngine",
    "RealGitEngine",
    "VirtualGitEngine",
    "VirtualGitRegistry",
    "Author",
    "Branch",
    "Commit",
    "FileStatus",
    "FileStatusType",
    "Repository",
]
```

## Testing Strategy

Basic tests in `tests/sync/git/test_virtual_engine.py`:

- Test repository initialization
- Test manual Repository construction
- Test repository cloning using VirtualGitRegistry
- Test commit creation and retrieval using stage_and_commit
- Test branch operations (create, list, checkout, delete)
- Test file status
- Test basic merge operations
- Test export_repository() on virtual engine
- Verify virtual engine maintains consistent state
- Verify cloned repository has identical commits and branches
- Test registry registration, retrieval, and cleanup
- Test importing real repository via export_repository()