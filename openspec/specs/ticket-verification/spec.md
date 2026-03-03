# ticket-verification Specification

## Purpose
Verifies P4 tickets by running `p4 login -s` and extracts the authenticated user identity and ticket expiry information.

## Requirements
### Requirement: Verify ticket and extract identity
The system SHALL verify a P4 ticket by running `p4 login -s` with the ticket and extract the authenticated user identity and ticket expiry.

#### Scenario: Valid ticket
- **WHEN** a valid P4 ticket is provided
- **THEN** the system returns the P4 username and expiry (in seconds remaining)

#### Scenario: Invalid or expired ticket
- **WHEN** an invalid or expired ticket is provided
- **THEN** the system returns an error indicating the ticket is not valid
