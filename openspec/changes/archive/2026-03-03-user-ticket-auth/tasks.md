## 1. p4rs Login Command

- [x] 1.1 Add `login` command to p4rs with `-s` (status) flag support
- [x] 1.2 Parse `p4 login -s` plain text output to extract username and seconds remaining
- [x] 1.3 Define return type for login status (user, seconds_remaining)
- [x] 1.4 Add tests for login status parsing (valid ticket, expired ticket, invalid ticket)

## 2. Database Schema

- [x] 2.1 Add `TicketMetadata` struct and table schema to `cabinet/tables.rs`
- [x] 2.2 Register table creation in database initialization

## 3. Ticket Store

- [x] 3.1 Add `keyring` crate dependency to Cargo.toml
- [x] 3.2 Create `src/cabinet/ticket_store.rs` with `TicketStore` struct
- [x] 3.3 Implement `store_ticket` — write ticket to keyring and upsert expiry metadata in SQLite
- [x] 3.4 Implement `get_ticket` — retrieve ticket from keyring
- [x] 3.5 Implement `delete_ticket` — remove ticket from keyring and delete expiry metadata
- [x] 3.6 Implement `is_expired` — check expiry metadata against current time

## 4. Error Types

- [x] 4.1 Add auth error variants: `NoTicketStored`, `TicketExpired`, `KeyringUnavailable`, `TicketInvalid`

## 5. Integration

- [x] 5.1 Add method to build authenticated P4 instance from stored ticket (lookup ticket, check expiry, construct P4)
- [x] 5.2 Wire into `get_shelve_client` path so it can retrieve tickets from the store
