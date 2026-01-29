## ADDED Requirements

### Requirement: Commands return P4Output wrapper

All P4 command implementations SHALL return `Result<P4Output<T>, P4Error>` where T is the command-specific result type.

#### Scenario: Info command returns wrapped output
- **WHEN** user calls `p4.info().run()`
- **THEN** the return type is `Result<P4Output<InfoResponse>, P4Error>`

#### Scenario: Changes command returns wrapped output
- **WHEN** user calls `p4.changes(&files).run()`
- **THEN** the return type is `Result<P4Output<ChangeData>, P4Error>`

#### Scenario: Form commands return wrapped output
- **WHEN** user calls `p4.change().set(&spec).run()`
- **THEN** the return type is `Result<P4Output<usize>, P4Error>`

### Requirement: Strict mode fails on errors

The `run()` method SHALL return an error when any P4 message with severity >= 3 is encountered.

#### Scenario: Single error causes failure
- **WHEN** P4 returns one error message (severity=3) and zero results
- **THEN** `run()` returns `Err(P4Error::Command { errors, .. })`

#### Scenario: Mixed results and errors causes failure
- **WHEN** P4 returns 2 success results and 1 error message (severity=3)
- **THEN** `run()` returns `Err(P4Error::Command { errors, partial_results })` with partial_results containing the 2 successes

#### Scenario: Warnings do not cause failure in strict mode
- **WHEN** P4 returns 3 success results and 2 warning messages (severity=2)
- **THEN** `run()` returns `Ok(P4Output)` with results and warnings populated

### Requirement: Lenient mode tolerates errors

The `run_lenient()` method SHALL collect error messages as warnings and only fail on fatal issues (connection, JSON parse).

#### Scenario: Errors collected as warnings
- **WHEN** P4 returns 2 success results and 1 error message (severity=3)
- **THEN** `run_lenient()` returns `Ok(P4Output)` with 2 results and 1 item in warnings

#### Scenario: Connection failure still fails
- **WHEN** P4 cannot connect to the server
- **THEN** `run_lenient()` returns `Err(P4Error::Connection)`

#### Scenario: Parse failure still fails
- **WHEN** P4 returns invalid JSON that cannot be parsed
- **THEN** `run_lenient()` returns `Err(P4Error::Json(..))`

### Requirement: P4Error contains structured error information

The `P4Error::Command` variant SHALL contain a `Vec<P4Message>` of error messages and an optional raw JSON value for partial results.

#### Scenario: Multiple errors captured
- **WHEN** P4 returns 3 error messages for different files
- **THEN** `P4Error::Command.errors` contains 3 `P4Message` entries

#### Scenario: Partial results preserved
- **WHEN** P4 returns 5 success results and 2 error messages
- **THEN** `P4Error::Command.partial_results` contains the raw JSON of the 5 successes

#### Scenario: Check for specific error
- **WHEN** user calls `error.contains("no such file")`
- **THEN** returns true if any error message data contains that substring
