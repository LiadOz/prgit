## Context

The per-user-shelve-client design established that the caller provides an authenticated P4 instance via `get_shelve_client(prgit_client, user_p4: &P4)`. This change fills the gap: how does the caller obtain and manage P4 tickets for each user?

P4 tickets are opaque auth tokens issued by `p4 login`. The P4 struct already accepts a password field (`-P` flag) which also accepts tickets. The `p4 login -s` command can verify a ticket and return the user identity and expiry without needing the original password.

There is no `login` command in p4rs currently — it needs to be added for ticket verification.

## Goals / Non-Goals

**Goals:**
- Library interface to store, retrieve, and delete P4 tickets using the OS keyring
- Verify tickets via P4 to extract user identity and expiry
- Track ticket expiry in SQLite to enable fast expiry checks without hitting P4
- Integrate with `get_shelve_client` so it can build authenticated P4 instances

**Non-Goals:**
- HTTP endpoints for ticket submission (future — needs server infrastructure)
- CLI commands for user login flow (future — `prgit login`)
- Ticket refresh or auto-renewal
- Supporting the keyring crate's file-based fallback

## Decisions

**Keyring keying scheme**

Use the `keyring` crate with:
- Service: `"prgit"`
- Username: `"{p4port}:{p4user}"` (e.g. `"ssl:perforce.corp:1666:bob"`)

This naturally namespaces tickets per P4 server and user. The `keyring` crate handles the secret-service D-Bus interaction on Linux.

Alternative considered: separate service per P4 port (`"prgit:ssl:perforce:1666"` + `"bob"`). Rejected — single service name is simpler and the username field can carry the composite key.

**Ticket verification via p4 login -s**

Add a `login` command to p4rs that supports the `-s` (status) flag. Run `p4 -P <ticket> login -s` to get output like:

```
User bob ticket expires in 43100 seconds.
```

Parse this to extract `(user, seconds_remaining)`. This serves two purposes:
1. Validate the ticket is real before storing it
2. Extract the expiry for metadata tracking

The `-s` flag doesn't require JSON output mode — it produces plain text that needs regex parsing.

**Expiry metadata in SQLite**

New table alongside existing tables in `cabinet/tables.rs`:

```sql
CREATE TABLE IF NOT EXISTS ticket_metadata (
    p4port TEXT NOT NULL,
    p4user TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    stored_at INTEGER NOT NULL,
    PRIMARY KEY (p4port, p4user)
);
```

`expires_at` and `stored_at` are Unix timestamps. On ticket replacement, upsert the row.

This table lives in the existing prgit database — no new database needed.

**Ticket store in the cabinet module**

Add ticket store to the existing `src/cabinet/` module alongside other database-backed functionality:
- `src/cabinet/ticket_store.rs` — keyring operations (store, get, delete) + expiry metadata

The ticket store takes a reference to the database connection (for expiry metadata) and manages keyring access internally. This fits naturally with cabinet since it already owns the SQLite connection and table definitions.

```rust
pub struct TicketStore<'a> {
    db: &'a Connection,
}

impl<'a> TicketStore<'a> {
    pub fn store_ticket(&self, p4port: &str, p4user: &str, ticket: &str, expires_at: i64) -> Result<()>;
    pub fn get_ticket(&self, p4port: &str, p4user: &str) -> Result<String>;
    pub fn delete_ticket(&self, p4port: &str, p4user: &str) -> Result<()>;
    pub fn is_expired(&self, p4port: &str, p4user: &str) -> Result<bool>;
}
```

Alternative considered: storing tickets in the database instead of keyring. Rejected — SQLite doesn't encrypt at rest, keyring via secret-service does.

**Integration with get_shelve_client**

The `get_shelve_client` interface changes from accepting a `user_p4: &P4` to being able to build one internally:

1. Look up ticket for `(p4port, p4user)` via `TicketStore`
2. Check expiry metadata — if expired, return auth error
3. Build `P4::new().port(p4port).p4_user(p4user).password(ticket)`
4. Pass to existing shelve client logic

This keeps the per-user-shelve-client change's interface clean — it still receives an authenticated P4, but now the auth module builds it.

**Error types**

New error variants for auth failures:
- `NoTicketStored` — user hasn't authenticated yet
- `TicketExpired` — ticket exists but has expired
- `KeyringUnavailable` — secret-service not running
- `TicketInvalid` — `p4 login -s` rejected the ticket

These are distinct from P4 command errors so callers can distinguish "need to re-login" from "P4 server error".

## Risks / Trade-offs

**Secret-service dependency on server** → Requires `gnome-keyring` or `kwallet` daemon running on the prgit server. This is an operational dependency that must be documented. If the daemon isn't running, ticket operations fail with a clear error.

**Expiry metadata can drift** → If a P4 admin changes a user's ticket lifetime or revokes a ticket, our stored expiry won't reflect that. Mitigation: treat expiry as advisory — if a P4 operation fails with an auth error at runtime, surface it as a re-login prompt regardless of what the metadata says.

**No login command in p4rs** → The `p4 login -s` command produces plain text, not JSON. The parsing is simple but fragile if P4 changes the output format. Mitigation: keep the regex narrow and fail clearly if the format doesn't match.
