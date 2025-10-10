# Sync Action Base Class

## Purpose

Provide a foundational abstract base class for all synchronization actions between Git and Perforce. The SyncAction class establishes a consistent interface, enables type-safe action composition, and supports dependency injection for testing.

## Design Goals

- Consistent interface for all sync actions
- Type-safe action composition with strong typing
- Constructor-based dependency injection
- Independently testable actions
- Extensible for future common functionality (logging, metrics, etc.)
- Stateless execution model (configure once, execute via perform())
- Exception-based error handling (no result objects)

## Components

### SyncAction (Abstract Base Class)

Abstract base class that all sync actions inherit from. Actions are configured at construction and executed via parameterless `perform()` method.

Location: `src/prgit/sync/actions/base.py`

#### Constructor

```python
SyncAction(git: GitEngine, perforce: PerforceEngine)
```

Stores engine references in protected attributes for subclass access. Subclasses extend the constructor signature with action-specific parameters.

#### Methods

- `perform() -> None`: Execute the sync action (abstract method, raises exception on failure)

#### Protected Attributes

- `_git: GitEngine`: Git engine instance for git operations
- `_perforce: PerforceEngine`: Perforce engine instance for p4 operations

#### Design Decisions

**Why `perform()` takes no arguments:**
- Actions are fully configured at construction time
- Separates configuration from execution
- Enables action reuse (configure once, execute multiple times)
- Simplifies testing (clear setup vs execution phases)
- Makes action state explicit (stored as instance attributes)

**Why engines are passed to base constructor:**
- All sync actions need both engines
- Ensures consistent dependency injection
- Simplifies subclass constructors
- Enables base class to provide common helper methods in the future

**Why no result objects:**
- Actions either succeed (return None) or fail (raise exception)
- Simpler mental model than checking result.success
- Exception context provides error details
- Follows Python's "easier to ask for forgiveness" philosophy

### SyncActionError Exception Hierarchy

Base exception class for all sync operation errors, extending PrgitError.

Location: `src/prgit/sync/actions/exceptions.py`

```python
class SyncActionError(PrgitError):
    def __init__(self, message: str, action: SyncAction, **kwargs: Any) -> None:
        super().__init__(message, action_name=action.__class__.__name__, **kwargs)
        self.action = action
```

Base exception for all sync operations. Captures the action object for better error reporting and access to action state.

#### Exception Subclasses

```python
class SyncConfigurationError(SyncActionError):
    def __init__(self, message: str, action: SyncAction, parameter: str, **kwargs: Any) -> None:
        super().__init__(message, action=action, parameter=parameter, **kwargs)
        self.parameter = parameter
```

Raised when sync action is configured incorrectly (e.g., invalid branch name, missing parameters). Captures the specific parameter that caused the error.

```python
class SyncExecutionError(SyncActionError):
    def __init__(self, message: str, action: SyncAction, operation: str, **kwargs: Any) -> None:
        super().__init__(message, action=action, operation=operation, **kwargs)
        self.operation = operation
```

Raised when sync execution fails (e.g., git operation fails, perforce connection lost). Captures the specific operation that failed.

### Action Exports

All concrete action classes are exported from the actions module for easy access. Most actions are grouped into two files: `perforce_to_git_actions.py` for P4→Git sync actions and `git_to_perforce_actions.py` for Git→P4 sync actions.

Location: `src/prgit/sync/actions/__init__.py`

```python
from .perforce_to_git_actions import ImportPerforceChangelists
from .git_to_perforce_actions import (
    ExportBranchToShelf,
    UpdateShelfFromBranch,
    DeleteShelfForBranch,
)

__all__ = [
    "ImportPerforceChangelists",
    "ExportBranchToShelf",
    "UpdateShelfFromBranch",
    "DeleteShelfForBranch",
]
```

The module itself serves as the namespace for accessing actions:

```python
from prgit.sync import actions

action = actions.ImportPerforceChangelists(git, perforce, target_branch="master")
action.perform()
```

Or import actions directly:

```python
from prgit.sync.actions import ImportPerforceChangelists

action = ImportPerforceChangelists(git, perforce, target_branch="master")
action.perform()
```
