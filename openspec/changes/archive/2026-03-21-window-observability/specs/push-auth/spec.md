## ADDED Requirements

### Requirement: Emit auth.failed event on authentication failure
The auth handler SHALL emit an `auth.failed` event when P4 ticket validation fails, including the attempted username and reason.

#### Scenario: Invalid ticket emits auth event
- **WHEN** a push request includes an invalid P4 ticket for user "jdoe"
- **THEN** an `auth.failed` event SHALL be emitted with user="jdoe" and reason="invalid_ticket"

#### Scenario: Missing auth emits auth event
- **WHEN** a push request has no Authorization header
- **THEN** an `auth.failed` event SHALL be emitted with user=None and reason="missing_credentials"
