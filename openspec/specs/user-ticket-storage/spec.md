# user-ticket-storage Specification

## Purpose
Manages storage, retrieval, and deletion of P4 tickets in the OS keyring using the secret-service API, ensuring secure credential management without file-based fallbacks.

## Requirements
### Requirement: Store P4 ticket in keyring
The system SHALL store a P4 ticket in the OS keyring using the secret-service API, keyed by P4 server port and P4 username.

#### Scenario: Store a new ticket
- **WHEN** a P4 ticket is provided for a user and P4 port
- **THEN** the system stores the ticket in the keyring with service "prgit" and username "{p4port}:{p4user}"

#### Scenario: Overwrite an existing ticket
- **WHEN** a ticket is stored for a user/port combination that already has a ticket
- **THEN** the new ticket replaces the old one

### Requirement: Retrieve P4 ticket from keyring
The system SHALL retrieve a stored P4 ticket from the keyring given a P4 port and username.

#### Scenario: Retrieve an existing ticket
- **WHEN** a ticket has been stored for a given p4port and p4user
- **THEN** the system returns the ticket value

#### Scenario: No ticket stored
- **WHEN** no ticket exists for the given p4port and p4user
- **THEN** the system returns an error indicating no ticket is available

### Requirement: Delete P4 ticket from keyring
The system SHALL support deleting a stored ticket from the keyring.

#### Scenario: Delete an existing ticket
- **WHEN** a delete is requested for a stored p4port and p4user
- **THEN** the ticket is removed from the keyring

### Requirement: Require secret-service backend
The system SHALL require a secret-service provider (gnome-keyring or kwallet) on the host. The keyring crate's file-based fallback SHALL NOT be used.

#### Scenario: No secret-service available
- **WHEN** the system attempts to access the keyring and no secret-service provider is running
- **THEN** the system returns an error indicating secret-service is required
