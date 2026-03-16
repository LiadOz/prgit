## ADDED Requirements

### Requirement: Shelve status endpoint returns active state

The server SHALL expose `GET /api/repos/{group}/{name}/shelve-status/{cl}` that returns whether the given CL has a background shelve currently in progress.

#### Scenario: CL with active background shelve
- **WHEN** a GET request is made to `/api/repos/depot/main/shelve-status/12345` and CL 12345 has a background shelve in progress
- **THEN** the server SHALL respond with status 200 and body `{ "active": true }`

#### Scenario: CL with no active background shelve
- **WHEN** a GET request is made to `/api/repos/depot/main/shelve-status/12345` and CL 12345 has no background shelve in progress
- **THEN** the server SHALL respond with status 200 and body `{ "active": false }`

#### Scenario: Non-existent repo
- **WHEN** a GET request is made to `/api/repos/depot/nonexistent/shelve-status/12345` and the repo does not exist
- **THEN** the server SHALL respond with status 404

#### Scenario: Invalid CL number
- **WHEN** a GET request is made with a non-numeric CL (e.g., `/api/repos/depot/main/shelve-status/abc`)
- **THEN** the server SHALL respond with status 400

### Requirement: Active shelves tracker

The server SHALL maintain an in-memory set of CL numbers that have background shelves currently in progress.

#### Scenario: CL registered when background shelve starts
- **WHEN** an async shelve is prepared and the background task is about to be spawned
- **THEN** the CL number SHALL be added to the active shelves set before the background task starts

#### Scenario: CL deregistered when background shelve completes
- **WHEN** a background shelve completes successfully
- **THEN** the CL number SHALL be removed from the active shelves set

#### Scenario: CL deregistered when background shelve fails
- **WHEN** a background shelve fails with an error
- **THEN** the CL number SHALL be removed from the active shelves set

#### Scenario: Tracker survives concurrent access
- **WHEN** multiple pushes register/deregister CLs concurrently while status queries are in flight
- **THEN** the tracker SHALL handle all operations without data races or panics
