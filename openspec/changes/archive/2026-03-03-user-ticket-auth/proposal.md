## Why

The per-user-shelve-client design requires an authenticated P4 instance per user, but currently has no mechanism for obtaining or storing user credentials. The server needs P4 tickets to perform operations on behalf of users.

Users should never need to send their P4 password to the prgit server. P4's native ticket system (`p4 login`) already handles authentication — the server only needs the resulting ticket.

## What Changes

- Store P4 tickets in the OS keyring (via secret-service)
- Track ticket expiry metadata in SQLite
- Provide a library interface to store, retrieve, and verify tickets
- Detect expired tickets and report auth failure

## Capabilities

### New Capabilities

- `user-ticket-storage`: Store and retrieve P4 tickets in keyring, keyed by P4 port and user
- `ticket-verification`: Verify a ticket via `p4 login -s` to extract user identity and expiry
- `ticket-expiry-tracking`: Track ticket expiry in SQLite, detect expired tickets before or during operations

### Modified Capabilities

- `shelve-client-management`: Can now retrieve a stored ticket to build an authenticated P4 instance for a user

## Impact

- New `keyring` crate dependency
- New `ticket_metadata` table in SQLite
- Integration point with `get_shelve_client` from per-user-shelve-client

## Future Considerations

- Server HTTP endpoint for ticket submission
- A `prgit login` CLI command that wraps `p4 login -h <server> -p` and sends the ticket to the server automatically
- A manual one-liner path: `p4 login -h <server> -p | curl ...` for environments without the prgit CLI
- The server hostname needed for `-h` could be exposed via an unauthenticated `/info` endpoint
- Keyring keying scheme (e.g. service `"prgit"`, username `"{p4port}:{p4user}"`)
- SQLite metadata schema for expiry tracking (p4port, p4user, expires_at, stored_at)
- Proactive expiry checks vs reactive auth failure detection
- Requiring `secret-service` (gnome-keyring/kwallet) as a server dependency — the keyring crate's file-based fallback does not provide equivalent security
