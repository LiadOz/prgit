## ADDED Requirements

### Requirement: Async event collection via bounded channel
The system SHALL use a bounded `tokio::sync::mpsc` channel to decouple event emission from storage. Emitters SHALL use `try_send` which never blocks. If the channel is full, the event SHALL be dropped and a warning logged.

#### Scenario: Event emitted without blocking
- **WHEN** a handler emits an event via the channel
- **THEN** the `try_send` call SHALL return immediately regardless of collector backpressure

#### Scenario: Channel full drops event
- **WHEN** the event channel buffer is full
- **THEN** the event SHALL be silently dropped and a warning SHALL be logged (not returned to the caller)

### Requirement: SQLite event table
The system SHALL store events in a SQLite table within the existing prgit database. The table SHALL have columns for: id (autoincrement), event_type (text), timestamp (integer, unix epoch ms), and payload (text, JSON-serialized event data).

#### Scenario: Event persisted to database
- **WHEN** the collector task receives an event from the channel
- **THEN** it SHALL insert a row into the events table with the event type, timestamp, and JSON payload

#### Scenario: Database write failure does not crash
- **WHEN** the SQLite write fails (e.g., disk full)
- **THEN** the collector SHALL log the error and continue processing the next event

### Requirement: Collector background task
The system SHALL spawn a single background task that reads from the event channel and writes to SQLite. This task SHALL run for the lifetime of the server.

#### Scenario: Collector starts on server startup
- **WHEN** the server starts
- **THEN** a collector task SHALL be spawned that begins draining the event channel

#### Scenario: Collector processes events in order
- **WHEN** multiple events are sent to the channel
- **THEN** the collector SHALL write them to SQLite in the order they were received

### Requirement: Event retention policy
The system SHALL periodically prune events older than a configurable retention period (default: 30 days). Pruning SHALL run within the collector task on a timer, not on every write.

#### Scenario: Old events pruned
- **WHEN** the retention timer fires
- **THEN** the collector SHALL delete all events with timestamp older than the retention period

#### Scenario: Retention does not block collection
- **WHEN** pruning runs
- **THEN** it SHALL not block event writes for more than the duration of the DELETE query

### Requirement: Collection errors never propagate
No error from the event collection system (channel send, SQLite write, pruning) SHALL propagate to request handlers, the shelver, or the mirror. All errors SHALL be logged at warn or error level only.

#### Scenario: Collector crash does not affect server
- **WHEN** the collector task panics
- **THEN** the server SHALL continue serving requests; event emission calls SHALL fail silently (channel closed, try_send returns Err)
