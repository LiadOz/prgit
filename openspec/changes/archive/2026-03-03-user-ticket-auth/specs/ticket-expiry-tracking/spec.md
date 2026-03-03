## ADDED Requirements

### Requirement: Store ticket expiry metadata
The system SHALL store ticket expiry metadata in SQLite when a ticket is stored, including p4port, p4user, expiry timestamp, and storage timestamp.

#### Scenario: Record expiry on ticket storage
- **WHEN** a verified ticket is stored in the keyring
- **THEN** the system records p4port, p4user, expires_at, and stored_at in the ticket_metadata table

#### Scenario: Update expiry on ticket replacement
- **WHEN** a new ticket is stored for an existing p4port/p4user combination
- **THEN** the expiry metadata is updated to reflect the new ticket

### Requirement: Detect expired tickets
The system SHALL check ticket expiry metadata before attempting P4 operations and report an auth failure if the ticket has expired.

#### Scenario: Ticket not expired
- **WHEN** a ticket's expires_at is in the future
- **THEN** the system proceeds with the operation

#### Scenario: Ticket expired
- **WHEN** a ticket's expires_at is in the past
- **THEN** the system returns an auth failure error without attempting the P4 operation

#### Scenario: No expiry metadata
- **WHEN** no ticket metadata exists for the given p4port/p4user
- **THEN** the system returns an error indicating no ticket is available
