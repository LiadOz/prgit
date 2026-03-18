# async-shelve Specification

## Purpose
Background shelving mode that runs the entire shelve operation in a background task, enabling non-blocking git push responses on slow P4 servers.

## Requirements

### Requirement: Background shelve operation
The shelver SHALL support an async mode where the entire shelve operation runs in the background. When async shelving is enabled, the push handler SHALL register the branch in the active shelves tracker and spawn a background task that calls `shelve()` directly.

#### Scenario: Async shelve runs entirely in background
- **WHEN** a push is received for branch `feature-xyz` and async shelving is enabled
- **THEN** the system SHALL register the branch as queued in the active shelves tracker and spawn a background task that calls `shelve("feature-xyz", user_p4, shelver_user)` — no P4 interaction occurs before the push response is sent

#### Scenario: Background task updates tracker on completion
- **WHEN** the background shelve task completes successfully for branch `feature-xyz`
- **THEN** the system SHALL update the tracker entry to done state with the resulting CL number and client name

#### Scenario: Background task updates tracker on failure
- **WHEN** the background shelve task fails for branch `feature-xyz`
- **THEN** the system SHALL update the tracker entry to failed state with the error message

### Requirement: Background shelve failure is non-fatal
When a background shelve fails, the failure SHALL be logged but SHALL NOT affect the already-completed git push response.

#### Scenario: Background shelve fails
- **WHEN** the background shelve task encounters a P4 error
- **THEN** the server SHALL log the error at error level and the tracker entry SHALL be set to failed state
