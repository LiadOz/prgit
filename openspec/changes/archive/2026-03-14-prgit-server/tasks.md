## 1. Project Setup

- [x] 1.1 Add dependencies to root `Cargo.toml`: `axum`, `tokio` (full features), `tower`, `tower-http`, `serde_yaml`, `hyper`
- [x] 1.2 Create `src/bin/server.rs` with `--config` clap argument and tokio main
- [x] 1.3 Define config structs: `ServerConfig` (listen, data_dir, repos), `RepoConfig` (group, name, p4port, p4client, synced_branch, mirror_interval_secs, max_changes) with serde Deserialize
- [x] 1.4 Parse YAML config file, exit with clear error on missing file or invalid YAML

## 2. Server Startup & Repo Initialization

- [x] 2.1 On startup, validate `git-http-backend` is in PATH — exit with error if not found
- [x] 2.2 For each configured repo, create bare git repo at `{data_dir}/repos/{group}/{name}.git` if it doesn't exist, or open existing
- [x] 2.3 Open or create SQLite database at `{data_dir}/prgit.db`, initialize tables
- [x] 2.4 For each configured repo, create or retrieve the `PrgitClient` entry in the database

## 3. HTTP Routing

- [x] 3.1 Build axum router with wildcard path matching for `/{path:.*}.git/{git_path:.*}` patterns
- [x] 3.2 Implement repo resolution: extract group + name from URL path, match against configured repos, return 404 if not found
- [x] 3.3 Add `GET /api/health` endpoint returning 200

## 4. Git HTTP Backend Proxy

- [x] 4.1 Implement CGI proxy: spawn `git-http-backend` with correct env vars (`GIT_PROJECT_ROOT`, `GIT_HTTP_EXPORT_ALL`, `PATH_INFO`, `REQUEST_METHOD`, `QUERY_STRING`, `CONTENT_TYPE`)
- [x] 4.2 Pipe request body to stdin, read CGI response from stdout, parse CGI headers, return as HTTP response
- [x] 4.3 Wire up `GET /{repo}/info/refs` and `POST /{repo}/git-upload-pack` to the proxy (anonymous, no auth)
- [x] 4.4 Wire up `POST /{repo}/git-receive-pack` to the proxy (with auth, see next section)

## 5. Push Authentication

- [x] 5.1 Extract HTTP basic auth credentials (P4 user + ticket) from the Authorization header on `git-receive-pack` requests
- [x] 5.2 Return 401 with `WWW-Authenticate: Basic` if no auth header present on push
- [x] 5.3 Validate P4 ticket by running `p4 login -s` with the extracted credentials against the repo's configured `p4port`
- [x] 5.4 Return 401 if ticket validation fails

## 6. Synced Branch Protection

- [x] 6.1 Before proxying a `git-receive-pack` request, parse the incoming ref updates from the request to determine target refs
- [x] 6.2 Reject the push with an error if any ref targets the synced branch

## 7. Push→Shelve Intercept

- [x] 7.1 After a successful `git-receive-pack` proxy, identify which non-delete refs were updated from the parsed ref updates
- [x] 7.2 For each updated branch, run `Shelver.shelve(branch, user_p4)` via `spawn_blocking` using the authenticated P4 identity
- [x] 7.3 Log shelve errors without failing the push response

## 8. Mirror Scheduler

- [x] 8.1 On startup, spawn one `tokio::spawn` background task per configured repo
- [x] 8.2 Each task runs `Mirror.run()` via `spawn_blocking`, then sleeps for the configured interval, in a loop
- [x] 8.3 Log mirror errors and continue to next iteration (don't crash the task)

## 9. Verification

- [x] 9.1 ~~Manual test~~ Integration test: `test_git_clone_via_server` — clone via HTTP server
- [x] 9.2 ~~Manual test~~ Integration test: `test_push_to_feature_branch_not_rejected_as_forbidden` — push feature branch
- [x] 9.3 ~~Manual test~~ Integration test: `test_push_to_synced_branch_rejected` — synced branch rejected (403)
- [x] 9.4 Covered by `tests/mirror_tests.rs` (16 tests)
- [x] 9.5 ~~Manual test~~ Integration test: `test_receive_pack_post_without_auth_returns_401` — unauthenticated push returns 401
