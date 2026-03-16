## Why

With async shelving, `git push` returns a CL number immediately while the actual shelve completes in the background. During this window (seconds to minutes), the CL exists but may be empty. Callers (CI systems, CLI tooling, dashboards) have no way to know if a CL's shelve is still in progress, failed, or completed. They need a queryable endpoint to check before acting on the CL.

## What Changes

- Add an HTTP endpoint (e.g., `GET /api/repos/{group}/{name}/shelve-status/{cl}`) that returns whether a given CL has a background shelve currently in progress
- Introduce in-memory tracking of active background shelves so the server can answer these queries
- Update the async shelve handler to register/deregister pending shelves in the tracker

## Capabilities

### New Capabilities
- `shelve-status`: HTTP endpoint to query the background shelve status of a given CL

### Modified Capabilities
- `push-shelve-intercept`: The async shelve handler needs to register/deregister active shelves with the tracker

## Impact

- `src/window/handlers.rs`: Async shelve path gains register/deregister calls around background completion
- `src/window/mod.rs`: New route added to the router, new shared state for tracking active shelves
- New handler function for the status endpoint
- No P4 interaction — this is purely server-side in-memory state
