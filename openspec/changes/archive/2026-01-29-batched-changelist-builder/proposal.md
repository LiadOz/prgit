## Why

The `ChangelistBuilder` in testkit is useful for scaffolding P4 operations, but it's test-only and inefficient - each `add_file()`, `edit_file()`, `delete_file()` call makes an immediate P4 command. This violates p4rs's principle of minimizing P4 calls. The `ShelveClient` already demonstrates the right pattern (batching by action/type), but users have to implement this themselves.

Moving `ChangelistBuilder` to the main library with built-in batching gives users an ergonomic API that's efficient by default.

## What Changes

- Move `ChangelistBuilder` from `testkit.rs` to main library (new module)
- Add batching: collect operations, execute grouped P4 commands on `submit()` or explicit `flush()`
- Group by action type (add/edit/delete) and file type for minimal P4 calls
- Add `immediate()` method to disable batching for cases that need it
- Update testkit to use the library builder (no backward compat layer)

## Capabilities

### New Capabilities
- `changelist-builder`: Ergonomic changelist builder with batched P4 operations

### Modified Capabilities
- `testkit`: Update to use library's builder instead of its own implementation

## Impact

- `crates/p4rs/src/changelist.rs` (new): `ChangelistBuilder` with batching logic
- `crates/p4rs/src/lib.rs`: Export `ChangelistBuilder`
- `crates/p4rs/src/testkit.rs`: Remove inline builder, delegate to library version
- `src/shelf/shelve_client.rs`: Could potentially reuse builder's batching logic
