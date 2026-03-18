## Why

The health endpoint is used by load balancers and monitoring tools that shouldn't need to know about API versions. Moving it under `/api/v1/` was an oversight — health checks are infrastructure, not versioned API surface.

## What Changes

- Move health endpoint from `/api/v1/health` back to `/api/health`

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

_(none — no existing spec for health endpoint)_

## Impact

- `src/window/mod.rs` — Change route from `/api/v1/health` to `/api/health`
- `tests/server_tests.rs` — Update test URI
