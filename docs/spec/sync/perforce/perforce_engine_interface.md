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
RealPerforceEngine()
```

#### Implementation Notes

- Wraps P4Python's `P4` class
- Converts P4Python objects to our dataclasses
- Raises ValueError on P4Exception errors
- Handles file encoding/decoding

### VirtualPerforceEngine

In-memory implementation simulating Perforce behavior for testing.

Location: `src/prgit/sync/perforce/virtual_engine.py`

#### Constructor

```python
VirtualPerforceEngine()
```

#### Implementation Notes

- Maintains in-memory state (changelists, shelves, files)
- No actual Perforce server interaction
- Simulates Perforce behavior for testing
- Generates sequential changelist numbers
- Thread-safe operations

#### State Management

Internal state structure:
- `_changelists: dict[int, Changelist]`: All changelists by number
- `_shelved_files: dict[int, dict[str, bytes]]`: Shelved files by changelist number
- `_file_revisions: dict[str, dict[int, bytes]]`: File content by depot path and revision
- `_next_changelist_number: int`: Counter for changelist generation

### VirtualPerforceRegistry

Type alias to generic `VirtualRegistry[PerforceState]` for testing. Allows multiple virtual engines to share the same depot history.

Location: `src/prgit/sync/perforce/__init__.py`

```python
from prgit.sync.virtual_registry import VirtualRegistry
from prgit.sync.perforce.types import Changelist

PerforceState = tuple[dict[int, Changelist], dict[str, dict[int, bytes]]]
VirtualPerforceRegistry = VirtualRegistry[PerforceState]
```

#### Methods

- `instance() -> VirtualRegistry[PerforceState]`: Get singleton instance
- `register(identifier: str, data: PerforceState) -> None`: Register Perforce state (changelists, file_revisions tuple)
- `unregister(identifier: str) -> None`: Remove from registry
- `get(identifier: str) -> PerforceState`: Retrieve registered state
- `clear() -> None`: Clear all registered state

#### Usage Pattern

```python
from prgit.sync.perforce import VirtualPerforceRegistry

registry = VirtualPerforceRegistry.instance()
registry.register("test-state", (changelists, file_revisions))
```

#### Purpose

Provides centralized registry for shared Perforce state in tests. Allows multiple virtual engines to share the same depot history.

#### Implementation Notes

- Uses generic `VirtualRegistry[T]` from `src/prgit/sync/virtual_registry.py`
- See `docs/spec/sync/virtual_registry.md` for implementation details
- Singleton pattern ensures single registry instance per type
- Thread-safe for concurrent test execution

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

## Usage Pattern

### Production

```python
from prgit.sync.perforce import RealPerforceEngine

p4 = RealPerforceEngine()

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

```python
from prgit.sync.perforce import VirtualPerforceEngine, ChangelistStatus

p4 = VirtualPerforceEngine()

cl = p4.create_changelist("Test feature")
files = {
    "//depot/project/file.py": b"print('hello')"
}
shelved = p4.shelve_files(cl.number, files)

changelists = p4.get_changelists(status=ChangelistStatus.SHELVED)
assert len(changelists) == 1
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
]
```

## Testing Strategy

Basic tests in `tests/sync/perforce/test_virtual_engine.py`:

- Test changelist creation and retrieval
- Test changelist filtering (status, max_results)
- Test changelist description updates
- Test shelve operations
- Test file content retrieval from changelists
- Test error conditions (invalid changelist, etc.)
- Verify thread safety with VirtualPerforceRegistry
