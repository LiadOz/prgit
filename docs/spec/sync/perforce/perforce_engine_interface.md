# Perforce Engine Interface

## Purpose

Provide an abstraction layer for Perforce operations within the sync engine. The interface supports both real Perforce commands (via P4Python) and a virtual in-memory implementation for testing.

## Design Goals

- Strong typing throughout the interface
- Testability without mocking or patching
- Support Perforce operations needed by sync engine
- Clean separation between interface and implementation
- Constructor-based dependency injection

## Components

### PerforceEngine (Abstract Base Class)

Abstract interface defining all Perforce operations. Implementations must provide concrete behavior.

Location: `src/prgit/sync/perforce/abstract_engine.py`

#### Methods

- `export_client() -> Client`: Export current client state as a Client object
- `get_changelist(number: int) -> Changelist`: Get changelist details by number
- `get_changelists(status: ChangelistStatus | None = None, max_results: int | None = None) -> list[Changelist]`: Query changelists with filters
- `get_changelist_file_content(depot_path: str, revision: int) -> bytes`: Get file content at specific revision
- `create_changelist(description: str) -> Changelist`: Create new pending changelist
- `update_changelist_description(number: int, description: str) -> Changelist`: Update changelist description
- `shelve_files(changelist_number: int, files: dict[str, bytes]) -> ShelvedChange`: Shelve files to changelist

### RealPerforceEngine

Implementation using P4Python library for actual Perforce operations.

Location: `src/prgit/sync/perforce/real_engine.py`

#### Constructor

```python
RealPerforceEngine(client_mappings: list[tuple[str, Path]])
```

#### Implementation Notes

- Wraps P4Python's `P4` class
- Converts P4Python objects to our dataclasses
- Raises ValueError on P4Exception errors
- Handles file encoding/decoding
- Constructor creates a P4 client with the provided mappings (depot paths to local paths)
- `export_client()` queries all changelists and file revisions relevant to the client mappings and creates a `Client` object

### VirtualPerforceEngine

In-memory implementation simulating Perforce behavior for testing.

Location: `src/prgit/sync/perforce/virtual_engine.py`

#### Constructor

```python
VirtualPerforceEngine(client_mappings: list[tuple[str, Path]])
```

#### Implementation Notes

- Maintains in-memory state (changelists, shelves, files)
- No actual Perforce server interaction
- Simulates Perforce behavior for testing
- Generates sequential changelist numbers
- Thread-safe operations
- Queries `VirtualPerforceRegistry` singleton using the first depot path from mappings to get the `Client`
- If no matching client found in registry, starts with empty state
- Imports the Client's changelists and file revisions into internal state

#### State Management

Internal state structure:
- `_changelists: dict[int, Changelist]`: All changelists by number
- `_shelved_files: dict[int, dict[str, bytes]]`: Shelved files by changelist number
- `_file_revisions: dict[str, dict[int, bytes]]`: File content by depot path and revision
- `_next_changelist_number: int`: Counter for changelist generation

`export_client()` creates a `Client` from the internal state, copying changelists and file revisions.

### VirtualPerforceRegistry

Type alias to generic `VirtualRegistry[Client]` for testing. Acts as a shared client state registry for tests.

Location: `src/prgit/sync/perforce/__init__.py`

```python
from prgit.sync.virtual_registry import VirtualRegistry
from prgit.sync.perforce.types import Client

VirtualPerforceRegistry = VirtualRegistry[Client]
```

#### Methods

- `instance() -> VirtualRegistry[Client]`: Get singleton instance
- `register(identifier: str, client: Client) -> None`: Register a Client with an identifier
- `unregister(identifier: str) -> None`: Remove a client from the registry
- `get(identifier: str) -> Client`: Retrieve a registered Client by identifier
- `clear() -> None`: Clear all registered clients (useful for test cleanup)

#### Usage Pattern

```python
from prgit.sync.perforce import VirtualPerforceRegistry

registry = VirtualPerforceRegistry.instance()
registry.register("//depot/project", client)
```

#### Purpose

Provides a centralized registry for `Client` objects that acts as a virtual Perforce server. Tests register a `Client` with a depot path identifier, and `VirtualPerforceEngine` automatically queries the registry based on its mappings to retrieve the client state. Clients can be manually constructed or exported from engines.

#### Implementation Notes

- Uses generic `VirtualRegistry[T]` from `src/prgit/sync/virtual_registry.py`
- See `docs/spec/sync/virtual_registry.md` for implementation details
- Singleton pattern ensures single registry instance per type
- Thread-safe for concurrent test execution
- Used exclusively with `VirtualPerforceEngine` for testing

### Dataclasses

Location: `src/prgit/sync/perforce/types.py`

#### Changelist

```python
@dataclass(frozen=True)
class Changelist:
    number: int
    description: str
    user: str
    client: str
    timestamp: datetime
    status: ChangelistStatus
    files: list[FileAction]
```

#### ChangelistStatus (StrEnum)

```python
class ChangelistStatus(StrEnum):
    PENDING = "pending"
    SHELVED = "shelved"
    SUBMITTED = "submitted"
```

#### FileAction

```python
@dataclass(frozen=True)
class FileAction:
    depot_path: str
    action: FileActionType
    revision: int | None
```

#### FileActionType (StrEnum)

```python
class FileActionType(StrEnum):
    ADD = "add"
    EDIT = "edit"
    DELETE = "delete"
    BRANCH = "branch"
    INTEGRATE = "integrate"
    MOVE_ADD = "move/add"
    MOVE_DELETE = "move/delete"
```

#### ShelvedChange

```python
@dataclass(frozen=True)
class ShelvedChange:
    changelist: Changelist
    files: dict[str, bytes]
```

#### Client

```python
@dataclass(frozen=True)
class Client:
    changelists: dict[int, Changelist]
    file_revisions: dict[str, dict[int, bytes]]
```

Represents a Perforce client state. Contains all changelists relevant to the client indexed by number and all file revisions indexed by depot path and revision number.

## Usage Pattern

### Production

```python
from pathlib import Path
from prgit.sync.perforce import RealPerforceEngine

mappings = [
    ("//depot/project/...", Path("/workspace/project")),
    ("//depot/shared/...", Path("/workspace/shared")),
]

p4 = RealPerforceEngine(mappings)

changelists = p4.get_changelists(status=ChangelistStatus.SUBMITTED, max_results=100)
for cl in changelists:
    for file_action in cl.files:
        if file_action.action != FileActionType.DELETE:
            content = p4.get_changelist_file_content(
                file_action.depot_path,
                file_action.revision
            )
```

### Testing

#### Manual Client Construction

```python
from pathlib import Path
from datetime import datetime
from prgit.sync.perforce import (
    VirtualPerforceEngine,
    VirtualPerforceRegistry,
    Changelist,
    ChangelistStatus,
    FileAction,
    FileActionType,
    Client
)

changelist = Changelist(
    number=1,
    description="Initial commit",
    user="testuser",
    client="testclient",
    timestamp=datetime.now(),
    status=ChangelistStatus.SUBMITTED,
    files=[
        FileAction(
            depot_path="//depot/project/file.py",
            action=FileActionType.ADD,
            revision=1
        )
    ]
)

client = Client(
    changelists={1: changelist},
    file_revisions={
        "//depot/project/file.py": {1: b"print('hello')"}
    }
)

registry = VirtualPerforceRegistry.instance()
registry.register("//depot/project", client)

mappings = [("//depot/project/...", Path("/workspace/project"))]
p4 = VirtualPerforceEngine(mappings)

changelists = p4.get_changelists(status=ChangelistStatus.SUBMITTED)
assert len(changelists) == 1
```

#### Export from Real Engine

```python
from pathlib import Path
from prgit.sync.perforce import (
    RealPerforceEngine,
    VirtualPerforceEngine,
    VirtualPerforceRegistry
)

real_mappings = [("//depot/project/...", Path("/workspace/project"))]
real_p4 = RealPerforceEngine(real_mappings)
client = real_p4.export_client()

registry = VirtualPerforceRegistry.instance()
registry.register("//depot/project", client)

virtual_mappings = [("//depot/project/...", Path("/test/workspace"))]
virtual_p4 = VirtualPerforceEngine(virtual_mappings)

changelists = virtual_p4.get_changelists()
```

## Dependencies

- **P4Python**: Real Perforce implementation (add to pyproject.toml)
- **pytest**: Testing framework (already in dev dependencies)

## Package Exports

`src/prgit/sync/perforce/__init__.py` exports:

```python
from prgit.sync.perforce.abstract_engine import PerforceEngine
from prgit.sync.perforce.real_engine import RealPerforceEngine
from prgit.sync.perforce.virtual_engine import VirtualPerforceEngine, VirtualPerforceRegistry
from prgit.sync.perforce.types import (
    Changelist,
    ChangelistStatus,
    FileAction,
    FileActionType,
    ShelvedChange,
    Client,
)

__all__ = [
    "PerforceEngine",
    "RealPerforceEngine",
    "VirtualPerforceEngine",
    "VirtualPerforceRegistry",
    "Changelist",
    "ChangelistStatus",
    "FileAction",
    "FileActionType",
    "ShelvedChange",
    "Client",
]
```

## Testing Strategy

Basic tests in `tests/sync/perforce/test_virtual_engine.py`:

- Test manual Client construction and initialization
- Test virtual engine with empty client (None)
- Test virtual engine with pre-populated client
- Test changelist creation and retrieval
- Test changelist filtering (status, max_results)
- Test changelist description updates
- Test shelve operations
- Test file content retrieval from changelists
- Test export_client() on virtual engine
- Test error conditions (invalid changelist, etc.)
- Verify virtual engine maintains consistent state
- Verify client initialization preserves all changelists and file revisions
- Test registry registration, retrieval, and cleanup
- Test importing real client via export_client()
