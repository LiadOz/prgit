# Virtual Registry

## Purpose

Provide a generic, thread-safe singleton registry for storing and sharing state across virtual engine instances during testing. Eliminates code duplication between VirtualGitRegistry and VirtualPerforceRegistry.

## Design Goals

- Generic type support for any data type
- Thread-safe singleton pattern
- Simple interface (register, get, unregister, clear)
- Used exclusively for testing virtual engines
- No dependencies on specific engine types

## Components

### VirtualRegistry (Generic Class)

Thread-safe singleton registry that stores arbitrary typed data keyed by string identifiers.

Location: `src/prgit/sync/virtual_registry.py`

#### Type Parameters

```python
T = TypeVar("T")

class VirtualRegistry(Generic[T]):
    ...
```

#### Class Variables

```python
_instances: dict[type, "VirtualRegistry"] = {}
_lock: threading.Lock = threading.Lock()
```

Note: Uses dictionary to store separate singleton instances per type parameter.

#### Instance Variables

```python
_data: dict[str, T]
_data_lock: threading.Lock
```

#### Methods

- `instance(cls: type[VirtualRegistry[T]]) -> VirtualRegistry[T]`: Get singleton instance for type T
- `register(identifier: str, data: T) -> None`: Register data with an identifier
- `unregister(identifier: str) -> None`: Remove data by identifier
- `get(identifier: str) -> T`: Retrieve data by identifier (raises ValueError if not found)
- `clear() -> None`: Clear all registered data

## Implementation Details

### Singleton Pattern

Uses type-specific singleton instances stored in class-level dictionary to support different types simultaneously:

```python
@classmethod
def instance(cls: type[VirtualRegistry[T]]) -> VirtualRegistry[T]:
    if cls not in cls._instances:
        with cls._lock:
            if cls not in cls._instances:
                cls._instances[cls] = cls()
    return cls._instances[cls]
```

This allows:
```python
git_registry = VirtualRegistry[Repository].instance()
p4_registry = VirtualRegistry[tuple[...]].instance()
```

Each maintains separate singleton instances.

### Thread Safety

All data access operations are protected by `_data_lock` using context managers.

### Error Handling

`get()` raises `ValueError` with descriptive message when identifier not found.

## Migration Strategy

### VirtualGitRegistry

Remove class entirely from `src/prgit/sync/git/virtual_engine.py`.

Update imports in `src/prgit/sync/git/__init__.py`:
```python
from prgit.sync.virtual_registry import VirtualRegistry
from prgit.sync.git.types import Repository

VirtualGitRegistry = VirtualRegistry[Repository]
```

### VirtualPerforceRegistry

Remove class entirely from `src/prgit/sync/perforce/virtual_engine.py`.

Update imports in `src/prgit/sync/perforce/__init__.py`:
```python
from prgit.sync.virtual_registry import VirtualRegistry
from prgit.sync.perforce.types import Changelist

PerforceState = tuple[dict[int, Changelist], dict[str, dict[int, bytes]]]
VirtualPerforceRegistry = VirtualRegistry[PerforceState]
```

## Usage Pattern

### Git Engine
```python
from prgit.sync.git import VirtualGitRegistry

registry = VirtualGitRegistry.instance()
registry.register("virtual://test-repo", repository)
repo = registry.get("virtual://test-repo")
```

### Perforce Engine
```python
from prgit.sync.perforce import VirtualPerforceRegistry

registry = VirtualPerforceRegistry.instance()
registry.register("test-state", (changelists, file_revisions))
state = registry.get("test-state")
```

### Testing
```python
import pytest
from prgit.sync.git import VirtualGitRegistry

@pytest.fixture(autouse=True)
def clear_registry():
    VirtualGitRegistry.instance().clear()
    yield
    VirtualGitRegistry.instance().clear()
```

## Package Structure

```
src/prgit/sync/
├── virtual_registry.py          # Generic VirtualRegistry[T]
├── git/
│   ├── virtual_engine.py        # VirtualGitEngine only
│   └── __init__.py              # Export VirtualGitRegistry = VirtualRegistry[Repository]
└── perforce/
    ├── virtual_engine.py        # VirtualPerforceEngine only
    └── __init__.py              # Export VirtualPerforceRegistry = VirtualRegistry[PerforceState]
```

## Testing Strategy

Tests in `tests/sync/test_virtual_registry.py`:

- Test singleton instance creation
- Test type-specific singleton isolation (different types get different instances)
- Test register and get operations
- Test unregister operation
- Test clear operation
- Test error handling (get non-existent identifier)
- Test thread safety with concurrent access
- Verify multiple types can coexist

## Test Updates Required

All test files need to update their imports:

**Git tests:** No change needed, still import from `prgit.sync.git`
**Perforce tests:** No change needed, still import from `prgit.sync.perforce`

The only difference is that `VirtualGitRegistry` and `VirtualPerforceRegistry` are now type aliases to the generic registry instead of separate classes.

## Benefits

- Eliminates ~30 lines of duplicated code per registry
- Single source of truth for registry pattern
- Easier to maintain and extend
- Type-safe with generics
- Can add more registries in future without duplication (e.g., VirtualSvnRegistry)
