## MODIFIED Requirements

### Requirement: Create CL alias via POST endpoint
The window server SHALL expose a POST endpoint at `/api/v1/repos/{group}/{name}/shelve/cl-alias` that accepts a JSON body with `shelved_cl` (the original shelved CL) and `alias_cl` (the new CL the user submitted instead). On success, the server SHALL return 201 with the created alias as JSON.

#### Scenario: Successful alias creation
- **WHEN** an authenticated user POSTs `{"shelved_cl": 100, "alias_cl": 200}` to `/api/v1/repos/depot/main/shelve/cl-alias`
- **THEN** the server SHALL store the alias mapping and return HTTP 201 with `{"shelved_cl": 100, "alias_cl": 200}`

#### Scenario: Repo not found
- **WHEN** a user POSTs to `/api/v1/repos/unknown/repo/shelve/cl-alias`
- **THEN** the server SHALL return HTTP 404

#### Scenario: Invalid JSON body
- **WHEN** a user POSTs a body that cannot be parsed as a valid alias request
- **THEN** the server SHALL return HTTP 400

### Requirement: Alias authentication uses P4 ticket
The alias endpoint SHALL require HTTP Basic authentication with a valid P4 ticket, using the same authentication mechanism as push operations.

#### Scenario: Missing credentials
- **WHEN** a request to the alias endpoint has no Authorization header
- **THEN** the server SHALL return HTTP 401 with WWW-Authenticate header

#### Scenario: Invalid P4 ticket
- **WHEN** a request to the alias endpoint has an invalid P4 ticket
- **THEN** the server SHALL return HTTP 401
