## Purpose
P4 ticket-based authentication for git push operations, with anonymous read access and auth failure observability.
## Requirements
### Requirement: P4 ticket authentication on push
The server SHALL require HTTP basic auth on push endpoints (`git-receive-pack`), where the username is the P4 user and the password is the P4 ticket.

#### Scenario: Valid P4 ticket
- **WHEN** a push request includes basic auth with a valid P4 username and ticket
- **THEN** the server SHALL validate the ticket by running `p4 login -s` against the P4 server, and if valid, allow the push to proceed

#### Scenario: Invalid P4 ticket
- **WHEN** a push request includes basic auth with an invalid or expired P4 ticket
- **THEN** the server SHALL return HTTP 401 and reject the push

#### Scenario: Missing auth on push
- **WHEN** a push request has no Authorization header
- **THEN** the server SHALL return HTTP 401 with a `WWW-Authenticate: Basic` header

### Requirement: Anonymous read access
The server SHALL allow unauthenticated access to read operations (clone/fetch).

#### Scenario: Clone without auth
- **WHEN** a client sends a `git-upload-pack` request without an Authorization header
- **THEN** the server SHALL proxy the request to `git-http-backend` without requiring credentials

### Requirement: Emit auth.failed event on authentication failure
The auth handler SHALL emit an `auth.failed` event when P4 ticket validation fails, including the attempted username and reason.

#### Scenario: Invalid ticket emits auth event
- **WHEN** a push request includes an invalid P4 ticket for user "jdoe"
- **THEN** an `auth.failed` event SHALL be emitted with user="jdoe" and reason="invalid_ticket"

#### Scenario: Missing auth emits auth event
- **WHEN** a push request has no Authorization header
- **THEN** an `auth.failed` event SHALL be emitted with user=None and reason="missing_credentials"

