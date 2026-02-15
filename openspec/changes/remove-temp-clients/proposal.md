## Why

P4 clients cannot be deleted if they have pending changelists (including shelves). The current ClientPool creates temporary clients when the pool is exhausted, but these temp clients create shelves and then cannot be deleted, leading to orphaned P4 clients accumulating on the server.

## What Changes

- Remove temporary client support from ClientPool
- Return an error when pool is exhausted instead of creating temp clients
- Simplify ClientLease by removing the lease type distinction

## Capabilities

### New Capabilities

None

### Modified Capabilities

- `client-pool`: ClientPool now fails fast with PoolExhausted error when no clients are available instead of creating temporary clients

## Impact

- `src/shelf/client_pool.rs` - Remove ClientLeaseType enum, remove create_temporary_client(), add PoolExhausted error
- Callers must handle PoolExhausted error (retry later or increase max_clients)
- No more orphaned P4 clients from temp client usage
