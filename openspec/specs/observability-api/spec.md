# observability-api Specification

## Purpose
TBD - created by archiving change window-observability. Update Purpose after archive.
## Requirements
### Requirement: Query events endpoint
The server SHALL expose a `GET /api/v1/events` endpoint that returns stored events as a JSON array. The endpoint SHALL support query parameters for filtering by event_type, time range (since/until as unix epoch ms), repo, and user. Results SHALL be ordered by timestamp descending with a default limit of 100.

#### Scenario: Query all events
- **WHEN** a client sends `GET /api/v1/events`
- **THEN** the server SHALL return the most recent 100 events as a JSON array

#### Scenario: Filter by event type
- **WHEN** a client sends `GET /api/v1/events?event_type=shelve.completed`
- **THEN** the server SHALL return only events of that type

#### Scenario: Filter by time range
- **WHEN** a client sends `GET /api/v1/events?since=1710000000000&until=1710100000000`
- **THEN** the server SHALL return only events within that time range

#### Scenario: Filter by repo
- **WHEN** a client sends `GET /api/v1/events?repo=depot/main`
- **THEN** the server SHALL return only events for that repo

#### Scenario: Custom limit
- **WHEN** a client sends `GET /api/v1/events?limit=500`
- **THEN** the server SHALL return up to 500 events

### Requirement: Event counts endpoint
The server SHALL expose a `GET /api/v1/events/counts` endpoint that returns aggregated counts of events grouped by event_type. The endpoint SHALL support the same time range and repo filters as the events endpoint.

#### Scenario: Get event counts
- **WHEN** a client sends `GET /api/v1/events/counts`
- **THEN** the server SHALL return a JSON object mapping event_type to count (e.g., `{"shelve.completed": 42, "push.received": 150}`)

#### Scenario: Counts filtered by time range
- **WHEN** a client sends `GET /api/v1/events/counts?since=1710000000000`
- **THEN** the server SHALL return counts only for events after that timestamp

### Requirement: Active users endpoint
The server SHALL expose a `GET /api/v1/events/users` endpoint that returns distinct users who have pushed within a given time range, with their push count and active branch count.

#### Scenario: List active users
- **WHEN** a client sends `GET /api/v1/events/users?since=1710000000000`
- **THEN** the server SHALL return a JSON array of objects with user, push_count, and active_branches (branches created but not yet merged/deleted in that window)

