# Virtual Registry

## Purpose
Provide a generic, thread-safe singleton registry for storing and sharing objects of a specific type during program execution and testing. This pattern allows easy dependency injection for testing.

## Design Goals

- Generic type support for any data type
- Thread-safe singleton pattern
- Simple interface (register, get, unregister, clear)

## Components

### VirtualRegistry
Location: `src/patterns/virtual_registry.rs`

Thread-safe singleton trait registry that stores arbitrary typed data keyed by string identifiers.

#### Methods
- `register(identifier: &str, data: T) -> None`: Register data with an identifier
- `get(identifier: &str) -> Option<&T>`: Retrieve data by identifier (returns None if not found)
- `unregister(identifier: &str) -> None`: Remove data by identifier
- `clear() -> None`: Clear all registered data

## Example high level usage
You want to write some test that clones a git repository and then performs some operations on it. Instead of setting up a real git repository and performing the operations on it, you can use the virtual registry to register a virtual git repository and then perform the operations on it. So in normal runs, the virtual registry will return actual git repositories, but in tests, it will return virtual git repositories defined in the test.