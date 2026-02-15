## Context

The shelving system needs P4 client workspaces to perform shelve operations. Currently this is managed by a ClientPool that tracks multiple clients per prgit configuration, with acquire/release semantics, timeouts, and temporary overflow clients.

The actual usage is simpler: operations are initiated server-side, one at a time per user. The pool complexity isn't needed.

Shelve operations need to run as a specific P4 user (not the service account). The caller provides a P4 instance configured with the user's credentials.

## Goals / Non-Goals

**Goals:**
- Simplify shelve client management to one client per user
- Remove pool tracking overhead
- Make client lifecycle predictable (create on first use, persist)
- Clean interface: `get_shelve_client(prgit_client, user_id) -> Result<ShelveClient>`

**Non-Goals:**
- Supporting concurrent operations per user (caller's responsibility)
- Client cleanup/deletion (clients persist, workspaces are cleaned per-operation)
- Timeout-based recovery (future enhancement if needed)

## Decisions

**One client per user, deterministic naming**

Client name pattern: `{base_client}-{user_id}-shelve`
Client root pattern: `{clients_root}/{client_name}/`

User ID is extracted from the P4 instance provided by caller.

**Interface: caller provides configured P4**

```rust
get_shelve_client(prgit_client, user_p4: &P4) -> Result<ShelveClient>
```

The caller handles authentication (getting credentials from keyring, etc.) and passes a P4 instance ready to use. We extract user_id from it for naming the client.

**P4 client spec: no Host restriction**

When creating the shelve P4 client, the Host field must be empty/unset. This allows the client to be used from any machine.

**Create on demand**

When `get_shelve_client` is called:
1. Extract user_id from user_p4
2. Derive client_name and client_root from pattern
3. If P4 client doesn't exist, create it (copy view from base client, no Host)
4. If local directory doesn't exist, create it
5. Return ShelveClient

The P4 client persists across operations. The workspace is cleaned by ShelveClient::new() each time.

**File lock for concurrency protection (flock)**

Use OS-level `flock()` for automatic release on process death:
- Path: `{client_root}/.prgit.lock`
- Open file and acquire exclusive lock with `flock(LOCK_EX | LOCK_NB)`
- If lock fails (EWOULDBLOCK), return error (ClientBusy)
- Hold file descriptor open for duration of ShelveClient lifetime
- OS automatically releases lock when process dies or fd is closed

In Rust: use `fs2` crate's `FileExt::try_lock_exclusive()` or similar.

This provides defense in depth - caller should still coordinate, but crashes don't leave stale locks.

**Keep shelve_config table**

Keep the table for future configuration. Will include:
- `clients_root` - where shelve workspaces live
- `mode` (future) - admin mode vs user mode for shelving

## Risks / Trade-offs

**P4 clients accumulate** → Each unique user_id creates a persistent P4 client. Acceptable - these are lightweight server-side and the alternative (cleanup) is complex.

**No protection against concurrent access** → Two processes using the same user_id simultaneously would conflict. Acceptable - caller must coordinate.

**user_id must be safe for P4 client names** → Need to sanitize or validate user_id to ensure valid P4 client naming.
