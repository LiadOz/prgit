# shelve-client-management Specification

## Purpose
Manages construction of authenticated P4 instances for shelve operations by retrieving stored tickets and validating their expiry before use.

## Requirements
### Requirement: Build authenticated P4 instance from stored ticket
The system SHALL retrieve a stored ticket for a given user and use it to construct an authenticated P4 instance for shelve operations.

#### Scenario: Ticket available and valid
- **WHEN** get_shelve_client is called for a user with a stored, non-expired ticket
- **THEN** the system retrieves the ticket from keyring, builds a P4 instance with it, and returns a ShelveClient

#### Scenario: Ticket expired
- **WHEN** get_shelve_client is called for a user whose ticket has expired
- **THEN** the system returns an auth failure error indicating re-authentication is needed

#### Scenario: No ticket stored
- **WHEN** get_shelve_client is called for a user with no stored ticket
- **THEN** the system returns an error indicating the user must authenticate first
