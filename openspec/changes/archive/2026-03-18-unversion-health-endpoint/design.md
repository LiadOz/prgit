## Context

The health endpoint was moved to `/api/v1/health` as part of the API versioning change. Health checks are infrastructure concerns, not versioned API surface — they should stay at `/api/health`.

## Goals / Non-Goals

**Goals:**
- Move health endpoint back to `/api/health`

**Non-Goals:**
- Changing any other endpoints

## Decisions

Single route change in `build_app()`. No architectural decisions needed.

## Risks / Trade-offs

None — this is a trivial route change.
