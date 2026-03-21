## ADDED Requirements

### Requirement: Event taxonomy
The system SHALL define a structured event type enum covering 5 categories: request, push, shelve, mirror, and auth. Each event variant SHALL carry typed fields specific to that event.

#### Scenario: All event categories representable
- **WHEN** an observable action occurs in any of the 5 categories
- **THEN** it SHALL be representable as a variant of the event enum with all required fields populated

### Requirement: Request events
The system SHALL emit a `request.completed` event for every git HTTP request (both reads and writes) after the response is produced. The event SHALL include: repo (group/name), timestamp, HTTP method, git_service (upload-pack or receive-pack), request_bytes, response_bytes, authenticated user (if any), and duration_ms.

#### Scenario: Clone request emits event
- **WHEN** a client completes a `POST git-upload-pack` request
- **THEN** a `request.completed` event SHALL be emitted with git_service="upload-pack", request_bytes set to the client's want/have negotiation size, and response_bytes set to the packfile size

#### Scenario: Push request emits event
- **WHEN** a client completes a `POST git-receive-pack` request
- **THEN** a `request.completed` event SHALL be emitted with git_service="receive-pack", the authenticated user populated, and request_bytes set to the push payload size

#### Scenario: Info/refs discovery emits event
- **WHEN** a client sends a `GET info/refs` request
- **THEN** a `request.completed` event SHALL be emitted with the appropriate git_service and response_bytes

### Requirement: Push events
The system SHALL emit push events when ref updates are parsed from a receive-pack request. Events SHALL include: user, repo, branch, and timestamp.

#### Scenario: Branch creation detected
- **WHEN** a ref update has old_sha of all zeros
- **THEN** a `push.branch_created` event SHALL be emitted for that branch

#### Scenario: Branch update detected
- **WHEN** a ref update has both old_sha and new_sha as nonzero values
- **THEN** a `push.branch_updated` event SHALL be emitted for that branch

#### Scenario: Branch deletion detected
- **WHEN** a ref update has new_sha of all zeros
- **THEN** a `push.branch_deleted` event SHALL be emitted for that branch

#### Scenario: Push rejected
- **WHEN** a push is rejected (synced branch protection, auth failure)
- **THEN** a `push.rejected` event SHALL be emitted with the reason

#### Scenario: Push received with payload size
- **WHEN** a receive-pack request is processed
- **THEN** a `push.received` event SHALL be emitted with payload_bytes and ref_count

### Requirement: Shelve events
The system SHALL emit shelve lifecycle events. Each event SHALL include user, repo, and branch at minimum.

#### Scenario: Shelve started
- **WHEN** the shelver begins processing a branch
- **THEN** a `shelve.started` event SHALL be emitted with the async flag indicating the shelve mode

#### Scenario: Shelve completed (first shelve)
- **WHEN** a branch is shelved for the first time (no existing shelve for that branch)
- **THEN** a `shelve.completed` event SHALL be emitted with changelist, client_name, duration_ms, file_count, async flag, and commits_in_branch count

#### Scenario: Shelve reshelved (subsequent shelve)
- **WHEN** a branch is shelved and an existing shelve changelist already exists for that branch
- **THEN** a `shelve.reshelved` event SHALL be emitted with the same fields as shelve.completed

#### Scenario: Shelve failed
- **WHEN** the shelver fails for a branch
- **THEN** a `shelve.failed` event SHALL be emitted with the error message, duration_ms, and async flag

#### Scenario: Shelve merged
- **WHEN** the mirror processes a P4 change whose old_change maps to a shelved branch via branch_shelve_mapping
- **THEN** a `shelve.merged` event SHALL be emitted with repo, branch, shelved_cl, submitted_cl, and shelver_user

### Requirement: Mirror events
The system SHALL emit mirror lifecycle events per repo and per change.

#### Scenario: Mirror cycle started
- **WHEN** a mirror iteration begins
- **THEN** a `mirror.cycle_started` event SHALL be emitted with repo and last_sync_change

#### Scenario: Mirror cycle completed
- **WHEN** a mirror iteration completes successfully
- **THEN** a `mirror.cycle_completed` event SHALL be emitted with repo, changes_synced count, new_last_sync, and duration_ms

#### Scenario: Mirror cycle failed
- **WHEN** a mirror iteration fails
- **THEN** a `mirror.cycle_failed` event SHALL be emitted with repo, error message, and duration_ms

#### Scenario: Individual change committed
- **WHEN** a single P4 change is mirrored to a git commit
- **THEN** a `mirror.change_committed` event SHALL be emitted with repo, p4_change, commit_hash, user, file_count, duration_ms, merge_parent (if MergeOurs was used), and merge_strategy

### Requirement: Auth events
The system SHALL emit an event when authentication fails.

#### Scenario: Auth failure recorded
- **WHEN** a push request fails P4 ticket validation
- **THEN** an `auth.failed` event SHALL be emitted with the attempted username, repo, and reason

### Requirement: Commits in branch count
The `shelve.completed` and `shelve.reshelved` events SHALL include a `commits_in_branch` field representing the number of commits between the merge base and the branch tip.

#### Scenario: Single commit branch
- **WHEN** a branch has exactly one commit ahead of the merge base
- **THEN** commits_in_branch SHALL be 1

#### Scenario: Multi-commit branch
- **WHEN** a branch has 5 commits ahead of the merge base
- **THEN** commits_in_branch SHALL be 5

### Requirement: All events carry common fields
Every event SHALL include a timestamp (UTC) and an event_type string identifier.

#### Scenario: Event serialization includes common fields
- **WHEN** any event is emitted
- **THEN** it SHALL include at minimum timestamp and event_type fields
