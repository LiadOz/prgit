## Why

The current ClientPool architecture is overly complex for the actual use case. It manages a pool of shelve clients with max limits, timeouts, and temporary overflow clients. However:

1. P4 clients cannot be deleted if they have pending shelves, making temporary clients problematic
2. The pool model assumes concurrent operations per user, which isn't the actual usage pattern
3. The complexity of pool management (acquire/release/timeout) adds maintenance burden

A simpler model of one shelve client per user fits the actual requirements better.

## What Changes

- Replace ClientPool with a simple `get_shelve_client(user_id)` function
- One P4 client per user, named `{base_client}-{user_id}-shelve`
- Client created on first use, persists for future operations
- Remove pool tracking from database
- Caller responsible for ensuring no concurrent operations per user

## Capabilities

### New Capabilities

None (simplification, not new functionality)

### Modified Capabilities

- `shelve-client-management`: Simplified from pool-based to per-user model

## Impact

- `src/shelf/client_pool.rs` - Replace entirely with simpler logic
- `src/shelf/shelver.rs` - Update to use new interface
- `src/cabinet/tables.rs` - Simplify shelve_config, remove shelve_clients table
- `src/cabinet/prgit_client.rs` - Remove pool management methods
- `src/cabinet/database.rs` - Remove shelve_clients operations
