## ADDED Requirements

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

### Requirement: Only the shelver can create an alias
The server SHALL verify that the authenticated user is the same user who created the original shelved CL. The server SHALL determine the shelver by looking up which user owns the shelved CL in the `branch_shelve_mapping` — the shelver is the P4 user whose ticket was used when the branch was shelved. If the authenticated user does not match the shelver, the request SHALL be rejected.

#### Scenario: Shelver creates alias
- **WHEN** user `jdoe` who shelved CL 100 sends a POST to create an alias for CL 100
- **THEN** the server SHALL allow the alias creation

#### Scenario: Non-shelver rejected
- **WHEN** user `other` who did NOT shelve CL 100 sends a POST to create an alias for CL 100
- **THEN** the server SHALL return HTTP 403 with an error message indicating only the shelver can create aliases

#### Scenario: Shelved CL not found
- **WHEN** a user sends a POST with a `shelved_cl` that has no branch mapping
- **THEN** the server SHALL return HTTP 404 with an error message indicating the shelved CL was not found

### Requirement: CL alias is one-to-one
Each alias CL SHALL map to exactly one shelved CL, and each shelved CL SHALL have at most one alias. Creating a new alias for a shelved CL that already has one SHALL replace the existing alias.

#### Scenario: One alias per shelved CL
- **WHEN** an alias `200 → 100` exists and the user creates alias `300 → 100`
- **THEN** the server SHALL replace the old alias so only `300 → 100` exists

#### Scenario: Alias CL cannot map to multiple shelved CLs
- **WHEN** an alias `200 → 100` exists and the user creates alias `200 → 150`
- **THEN** the server SHALL replace the old alias so `200` now maps to `150`

### Requirement: Alias authentication uses P4 ticket
The alias endpoint SHALL require HTTP Basic authentication with a valid P4 ticket, using the same authentication mechanism as push operations.

#### Scenario: Missing credentials
- **WHEN** a request to the alias endpoint has no Authorization header
- **THEN** the server SHALL return HTTP 401 with WWW-Authenticate header

#### Scenario: Invalid P4 ticket
- **WHEN** a request to the alias endpoint has an invalid P4 ticket
- **THEN** the server SHALL return HTTP 401

### Requirement: Mirror resolves branch via alias fallback
When the mirror processes a submitted changelist, it SHALL first look up the branch directly via `branch_shelve_mapping`. If no direct mapping is found, it SHALL check the `shelve_cl_alias` table to see if the submitted CL is an alias for a shelved CL. If an alias exists, the mirror SHALL use the aliased shelved CL to resolve the related branch.

#### Scenario: Direct branch mapping exists
- **WHEN** the mirror processes CL 100 and `branch_shelve_mapping` has a direct entry for CL 100
- **THEN** the mirror SHALL use the directly mapped branch (alias table is not consulted)

#### Scenario: Alias resolves to branch
- **WHEN** the mirror processes CL 200, no direct mapping exists, but alias `200 → 100` exists and CL 100 maps to branch `feature-x`
- **THEN** the mirror SHALL resolve branch `feature-x` as the related branch for CL 200

#### Scenario: No mapping and no alias
- **WHEN** the mirror processes CL 300 with no direct mapping and no alias
- **THEN** the mirror SHALL return no related branch (existing behavior)

### Requirement: Alias shelver identity storage
The server SHALL store the P4 username of the user who shelved each branch alongside the branch-shelve mapping, so that alias authorization can verify the shelver's identity.

#### Scenario: Shelver username stored on shelve
- **WHEN** user `jdoe` pushes a branch and it is shelved as CL 100
- **THEN** the system SHALL store `jdoe` as the shelver for that branch-shelve mapping

#### Scenario: Shelver username available for alias auth
- **WHEN** a user requests to create an alias for CL 100 shelved by `jdoe`
- **THEN** the system SHALL be able to retrieve `jdoe` as the shelver to compare against the authenticated user
