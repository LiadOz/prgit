## MODIFIED Requirements

### Requirement: Shelve status endpoint returns active state
The server SHALL expose `GET /api/v1/repos/{group}/{name}/shelve/status/{branch}` that returns the shelve state for the given branch. The response SHALL include a `state` field (one of `queued`, `shelving`, `done`, `failed`) and, when available, a `changelist` field with the CL number and a `client` field with the client name.

#### Scenario: Branch with queued shelve
- **WHEN** a GET request is made to `/api/v1/repos/depot/main/shelve/status/feature-xyz` and branch `feature-xyz` has been registered but shelving has not started
- **THEN** the server SHALL respond with status 200 and body `{ "state": "queued" }`

#### Scenario: Branch with shelve in progress
- **WHEN** a GET request is made to `/api/v1/repos/depot/main/shelve/status/feature-xyz` and branch `feature-xyz` has a background shelve currently executing
- **THEN** the server SHALL respond with status 200 and body `{ "state": "shelving" }`

#### Scenario: Branch with completed shelve
- **WHEN** a GET request is made to `/api/v1/repos/depot/main/shelve/status/feature-xyz` and the background shelve for `feature-xyz` completed as CL 12345 on client `shelve-client-1`
- **THEN** the server SHALL respond with status 200 and body `{ "state": "done", "changelist": 12345, "client": "shelve-client-1" }`

#### Scenario: Branch with failed shelve
- **WHEN** a GET request is made to `/api/v1/repos/depot/main/shelve/status/feature-xyz` and the background shelve for `feature-xyz` failed
- **THEN** the server SHALL respond with status 200 and body `{ "state": "failed", "error": "<error message>" }`

#### Scenario: Branch with no shelve activity
- **WHEN** a GET request is made to `/api/v1/repos/depot/main/shelve/status/feature-xyz` and branch `feature-xyz` has no entry in the active shelves tracker
- **THEN** the server SHALL respond with status 404

#### Scenario: Non-existent repo
- **WHEN** a GET request is made to `/api/v1/repos/depot/nonexistent/shelve/status/feature-xyz` and the repo does not exist
- **THEN** the server SHALL respond with status 404

### Requirement: Active shelves tracker
The server SHALL maintain an in-memory map of branch names to shelve states. Each entry tracks the current state (`queued`, `shelving`, `done`, `failed`), and optionally the CL number, client name, or error message.

#### Scenario: Branch registered when async push is accepted
- **WHEN** a push is accepted for branch `feature-xyz` with async shelving enabled
- **THEN** the branch SHALL be added to the tracker with state `queued` before the push response is sent

#### Scenario: Branch updated when shelve starts
- **WHEN** the background task begins the `shelve()` call for branch `feature-xyz`
- **THEN** the tracker entry SHALL be updated to state `shelving`

#### Scenario: Branch updated when shelve completes
- **WHEN** the background shelve completes successfully for branch `feature-xyz` with CL 12345
- **THEN** the tracker entry SHALL be updated to state `done` with `changelist: 12345` and the client name

#### Scenario: Branch updated when shelve fails
- **WHEN** the background shelve fails for branch `feature-xyz`
- **THEN** the tracker entry SHALL be updated to state `failed` with the error message

#### Scenario: Tracker survives concurrent access
- **WHEN** multiple pushes register/update branches concurrently while status queries are in flight
- **THEN** the tracker SHALL handle all operations without data races or panics

## REMOVED Requirements

### Requirement: Shelve status endpoint returns active state
**Reason**: Replaced by the modified version above. The endpoint path changes from `/api/repos/{group}/{name}/shelve-status/{cl}` to `/api/v1/repos/{group}/{name}/shelve/status/{branch}`, and the response changes from a simple `{ "active": bool }` to a richer state object.
**Migration**: Consumers must update to the new `/api/v1/` path, use branch name instead of CL number, and handle the new response shape.

### Requirement: Active shelves tracker
**Reason**: Replaced by the modified version above. The tracker is rekeyed from CL numbers (`HashSet<usize>`) to branch names with richer state.
**Migration**: Internal change only — no external migration needed.
