## ADDED Requirements

### Requirement: Background mirror task per repo
The server SHALL spawn one background task per configured repo that periodically runs the mirror to sync P4 changes to git.

#### Scenario: Mirror runs on interval
- **WHEN** a repo is configured with `mirror_interval_secs: 60`
- **THEN** the server SHALL run `Mirror.run()` for that repo approximately every 60 seconds

#### Scenario: Mirror tasks start on server startup
- **WHEN** the server starts with configured repos
- **THEN** the server SHALL spawn a mirror task for each repo and run the first mirror iteration immediately

### Requirement: Mirror failure resilience
A mirror task failure SHALL NOT crash the server or affect other repos.

#### Scenario: Mirror iteration fails
- **WHEN** a mirror iteration fails (e.g., P4 server unreachable)
- **THEN** the server SHALL log the error and continue scheduling the next iteration

### Requirement: Blocking P4 calls run off async runtime
Mirror operations (which use synchronous P4 commands) SHALL run via `spawn_blocking` to avoid blocking the tokio async runtime.

#### Scenario: Mirror does not block async tasks
- **WHEN** a mirror iteration runs a slow P4 query
- **THEN** HTTP request handling and other mirror tasks SHALL continue to operate without blocking
