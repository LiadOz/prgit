# Exception System

## Purpose

Provide a minimal exception system for prgit that allows attaching contextual information for error display purposes.

## Design Goals

- Single base exception class
- Support arbitrary context via kwargs
- Context restricted to error display code only
- Subclassable for domain-specific exceptions

## Components

### PrgitError

Location: `src/prgit/exceptions.py`

```python
from typing import Any


class _ContextKey:
    pass


class PrgitError(Exception):
    def __init__(self, message: str, **kwargs: Any) -> None:
        super().__init__(message)
        self.message = message
        self.__context = kwargs
    
    def get_context(self, key: _ContextKey) -> dict[str, Any]:
        return self.__context

    def get_context_info(self) -> str:
        return ', '.join(f"{k}: {v}" for k, v in self.__context.items())


def create_context_key_for_testing() -> _ContextKey:
    return _ContextKey()


__all__ = ["PrgitError", "create_context_key_for_testing"]
```

## Usage Example

```python
def divide(a: int, b: int) -> float:
    if b == 0:
        raise PrgitError(
            "Division by zero",
            numerator=a,
            denominator=b
        )
    return a / b


try:
    result = divide(10, 0)
except PrgitError as e:
    print(e.message)
    print(f"Context: {e.get_context_info()}")
```

Output:
```
Division by zero
Context: numerator: 10, denominator: 0
```

## Context Access

### Public Access: get_context_info()

For basic display and logging, use `get_context_info()` which returns a formatted string:

```python
try:
    operation()
except PrgitError as e:
    log.error(f"{e.message} - {e.get_context_info()}")
```

This method is public and provides read-only access to context information suitable for error messages and logs.

### Restricted Access: get_context()

The `get_context()` method requires a `_ContextKey` instance to access the full context dictionary.

**Within the exceptions module:**

Error display/formatting code can directly instantiate `_ContextKey()`:

```python
def format_error_for_display(error: PrgitError) -> str:
    key = _ContextKey()
    context = error.get_context(key)
    return custom_format(error.message, context)
```

**For testing:**

Tests use `create_context_key_for_testing()` since they cannot import `_ContextKey`:

```python
from prgit.exceptions import PrgitError, create_context_key_for_testing

def test_get_context() -> None:
    error = PrgitError("Error", key="value")
    key = create_context_key_for_testing()
    context = error.get_context(key)
    assert context == {"key": "value"}
```

This design ensures:
- Error display/formatting code lives within the exceptions module
- `_ContextKey` cannot be instantiated outside the module (except via the testing function)
- Tests can verify context behavior using `create_context_key_for_testing()`
- Basic display needs are served by the simpler `get_context_info()`

## Subclassing

```python
class DivisionError(PrgitError):
    pass


raise DivisionError("Division by zero", numerator=10, denominator=0)
```

## Testing Strategy

Tests in `tests/test_exceptions.py`:

- Test PrgitError with message only
- Test PrgitError with message and kwargs
- Test get_context_info() returns formatted string
- Test get_context_info() with empty context returns empty string
- Test get_context() with create_context_key_for_testing() returns kwargs dict
- Test get_context() without key raises TypeError
- Test create_context_key_for_testing() returns usable key
- Test subclass inherits behavior
- Test exception chaining with `raise ... from e`
