## ADDED Requirements

### Requirement: P4Message captures server message metadata

The system SHALL provide a `P4Message` struct that captures P4 server message metadata including severity level, generic code, message ID, and human-readable text.

#### Scenario: Message from warning response
- **WHEN** P4 returns a JSON object with `{"data": "...", "severity": 2, "generic": 17}`
- **THEN** a `P4Message` is created with `severity=2`, `generic=17`, and `data` containing the message text

#### Scenario: Classify message as warning
- **WHEN** a `P4Message` has severity equal to 2
- **THEN** `is_warning()` returns true and `is_error()` returns false

#### Scenario: Classify message as error
- **WHEN** a `P4Message` has severity greater than or equal to 3
- **THEN** `is_error()` returns true and `is_warning()` returns false

### Requirement: P4Output wraps results and warnings

The system SHALL provide a `P4Output<T>` struct that contains a `results: Vec<T>` field for successful operation results and a `warnings: Vec<P4Message>` field for warning messages.

#### Scenario: Command returns multiple results with no warnings
- **WHEN** a P4 command returns 5 JSON result objects and no warning messages
- **THEN** `P4Output.results` contains 5 parsed items and `P4Output.warnings` is empty

#### Scenario: Command returns results with warnings
- **WHEN** a P4 command returns 3 success results and 2 warning messages (severity=2)
- **THEN** `P4Output.results` contains 3 items and `P4Output.warnings` contains 2 `P4Message` entries

#### Scenario: Command returns no results
- **WHEN** a P4 command returns zero result objects
- **THEN** `P4Output.results` is empty and `is_empty()` returns true

### Requirement: P4Output provides single-result accessor

The system SHALL provide a `single()` method on `P4Output<T>` that returns the single result or an error if there are zero or more than one results.

#### Scenario: Extract single result
- **WHEN** `P4Output.results` contains exactly one item
- **THEN** `single()` returns `Ok(item)`

#### Scenario: Error on empty results
- **WHEN** `P4Output.results` is empty
- **THEN** `single()` returns `Err(P4Error)`

#### Scenario: Error on multiple results
- **WHEN** `P4Output.results` contains more than one item
- **THEN** `single()` returns `Err(P4Error)`

### Requirement: P4Output is iterable

The system SHALL implement `IntoIterator` for `P4Output<T>` to allow direct iteration over results.

#### Scenario: Iterate over results
- **WHEN** user writes `for item in p4_output`
- **THEN** the loop iterates over all items in `results`
