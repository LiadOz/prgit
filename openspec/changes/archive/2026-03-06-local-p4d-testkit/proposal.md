## Why

The test infrastructure relies on Docker (via `testcontainers`) to run a Perforce server. Sandboxed environments like Claude Code and Cursor cannot execute Docker commands without per-command user approval, making test runs impractical. A local `p4d` backend would allow tests to run entirely in-process without container orchestration.

## What Changes

- Add a new `testkit` backend that spawns `p4d` directly in a temp directory when a cargo feature flag is enabled
- The local backend creates a temp P4 root, starts `p4d` on a random available port, sets up protections, and tears down on drop
- `P4Server::start()` selects the backend based on feature flags: `testkit-local` uses local p4d, `testkit` (existing) uses Docker
- No changes to `TestClient`, `ChangelistBuilder`, or any test code — they use `P4Server` which abstracts the backend

## Capabilities

### New Capabilities
- `local-p4d-backend`: Local p4d test server backend that spawns p4d from PATH in a temp directory, providing the same `P4Server` interface as the Docker backend

### Modified Capabilities

_(none — existing testkit behavior is unchanged when using the Docker backend)_

## Impact

- `crates/p4rs/Cargo.toml`: New `testkit-local` feature flag (no new external dependencies beyond what's already optional)
- `crates/p4rs/src/testkit.rs`: Add local p4d startup/teardown logic alongside existing Docker logic
- `Cargo.toml` (root): Dev-dependency feature update to allow `testkit-local`
- CI: No change required — Docker backend remains the default
