## ADDED Requirements

### Requirement: Emit request.completed event for all git HTTP requests
The handler SHALL emit a `request.completed` event after every git HTTP request is served, capturing network-level metrics for capacity planning.

#### Scenario: Upload-pack request tracked
- **WHEN** a `POST git-upload-pack` request completes
- **THEN** a `request.completed` event SHALL be emitted with git_service="upload-pack", request_bytes from the body, response_bytes from the CGI output, and duration_ms

#### Scenario: Receive-pack request tracked
- **WHEN** a `POST git-receive-pack` request completes
- **THEN** a `request.completed` event SHALL be emitted with git_service="receive-pack", the authenticated user, request_bytes, response_bytes, and duration_ms

#### Scenario: Info/refs discovery tracked
- **WHEN** a `GET info/refs` request completes
- **THEN** a `request.completed` event SHALL be emitted with the git_service from the query parameter, request_bytes=0 (GET has no body), and response_bytes from the CGI output

#### Scenario: Unknown repo request tracked
- **WHEN** a git request targets a repo that does not exist
- **THEN** a `request.completed` event SHALL NOT be emitted (no repo context to attribute it to)
