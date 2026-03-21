## Purpose
Git smart HTTP protocol serving via git-http-backend CGI proxy, with URL routing, health checks, and request observability.
## Requirements
### Requirement: Smart HTTP protocol endpoints
The server SHALL handle git smart HTTP protocol requests by proxying them to `git-http-backend`.

#### Scenario: Clone discovery
- **WHEN** a client sends `GET /{group}/{project}.git/info/refs?service=git-upload-pack`
- **THEN** the server SHALL proxy the request to `git-http-backend` with the correct `GIT_PROJECT_ROOT` and `PATH_INFO`, and return the response

#### Scenario: Clone/fetch data transfer
- **WHEN** a client sends `POST /{group}/{project}.git/git-upload-pack`
- **THEN** the server SHALL proxy the request body to `git-http-backend` and stream the response back

#### Scenario: Push discovery
- **WHEN** a client sends `GET /{group}/{project}.git/info/refs?service=git-receive-pack`
- **THEN** the server SHALL proxy the request to `git-http-backend` and return the response

#### Scenario: Push data transfer
- **WHEN** a client sends `POST /{group}/{project}.git/git-receive-pack`
- **THEN** the server SHALL proxy the request body to `git-http-backend` and stream the response back

### Requirement: GitLab-style URL routing
The server SHALL route git requests using `/{group}[/{subgroup}]/{project}.git` URL paths, mapping them to bare repos on disk.

#### Scenario: Simple group path
- **WHEN** a request arrives for `/engine/main.git/info/refs`
- **THEN** the server SHALL resolve it to the repo configured with `group: "engine"` and `name: "main"`

#### Scenario: Nested subgroup path
- **WHEN** a request arrives for `/engine/tools/build-system.git/info/refs`
- **THEN** the server SHALL resolve it to the repo configured with `group: "engine/tools"` and `name: "build-system"`

#### Scenario: Unknown repo
- **WHEN** a request arrives for a repo path that matches no configured repo
- **THEN** the server SHALL return HTTP 404

### Requirement: git-http-backend CGI invocation
The server SHALL spawn `git-http-backend` as a CGI process, setting the required environment variables (`GIT_PROJECT_ROOT`, `GIT_HTTP_EXPORT_ALL`, `PATH_INFO`, `REQUEST_METHOD`, `QUERY_STRING`, `CONTENT_TYPE`), piping the request body to stdin and reading the response from stdout.

#### Scenario: CGI environment setup
- **WHEN** proxying a git request for a resolved repo
- **THEN** the server SHALL set `GIT_PROJECT_ROOT` to the bare repo's parent directory, `PATH_INFO` to `/{repo-dir-name}/{git-path}`, and `GIT_HTTP_EXPORT_ALL` to `1`

#### Scenario: git-http-backend not found
- **WHEN** the server starts and `git-http-backend` is not available in PATH
- **THEN** the server SHALL fail to start with a clear error message

### Requirement: Health check endpoint
The server SHALL expose a `GET /api/health` endpoint that returns HTTP 200.

#### Scenario: Health check
- **WHEN** a client sends `GET /api/health`
- **THEN** the server SHALL return HTTP 200

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

