## ADDED Requirements

### Requirement: Local p4d server startup
When the `testkit-local` feature is enabled, `P4Server::start()` SHALL spawn a local `p4d` process from PATH using a temporary directory as the server root and a dynamically assigned port.

#### Scenario: Successful local server start
- **WHEN** `testkit-local` feature is enabled and `p4d` is available in PATH
- **THEN** `P4Server::start()` SHALL create a temp directory, start `p4d -r <tempdir> -p localhost:<port>`, wait for the server to become ready, and return a `P4Server` with the assigned port

#### Scenario: p4d not found in PATH
- **WHEN** `testkit-local` feature is enabled and `p4d` is not available in PATH
- **THEN** `P4Server::start()` SHALL panic with a message indicating that `p4d` was not found

#### Scenario: External server override
- **WHEN** `P4RS_TEST_PORT` environment variable is set
- **THEN** `P4Server::start()` SHALL use the external server regardless of which feature is enabled (same behavior as existing)

### Requirement: Local p4d server cleanup
The local p4d process and its temp directory SHALL be cleaned up when the `P4Server` is dropped or the test process exits.

#### Scenario: Normal drop cleanup
- **WHEN** a `P4Server` using the local backend is dropped
- **THEN** the `p4d` child process SHALL be killed and the temp directory SHALL be removed

#### Scenario: Process exit cleanup
- **WHEN** the test process exits (including abnormal exit)
- **THEN** the `p4d` process SHALL be killed via the registered atexit handler

### Requirement: Admin and protection setup
After starting the local `p4d` server, the system SHALL configure the same admin user and protections as the Docker backend.

#### Scenario: Protections configured on local server
- **WHEN** the local `p4d` server has started and is ready
- **THEN** `setup_protections()` SHALL be called to configure admin super access and default write access, using the same `ADMIN_USER` and `ADMIN_PASSWORD` constants

### Requirement: Feature flag does not require Docker dependency
When `testkit-local` is enabled without `testkit`, the `testcontainers` crate SHALL NOT be a required dependency.

#### Scenario: Compile without testcontainers
- **WHEN** `testkit-local` feature is enabled and `testkit` feature is not enabled
- **THEN** the crate SHALL compile without the `testcontainers` dependency

### Requirement: API compatibility
The `P4Server` returned by the local backend SHALL expose the same public API as the Docker backend: `port`, `p4()`, `admin_p4()`, `test_client()`.

#### Scenario: Test code unchanged
- **WHEN** tests use `SERVER.p4()`, `SERVER.test_client()`, or `SERVER.admin_p4()`
- **THEN** these methods SHALL work identically regardless of which backend is active
