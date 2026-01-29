## Context

P4's CLI output varies by command: single JSON objects, multiple JSON lines, or plain text for form inputs. The current p4rs implementation handles this inconsistently - some commands return `Vec<T>`, others return `T`, and warnings/partial failures are either lost or cause full failures. P4Python provides a battle-tested model: always return lists, separate warnings from results, and provide exception level control.

Current pain points:
- `run()` vs `run_multi_line()` distinction leaks implementation details
- Warnings (severity=2) are either swallowed or treated as errors
- Partial success (8 files sync, 2 fail) causes complete failure
- No access to P4 message metadata (severity, generic code, msgid)
- Form input commands parse text output manually with fragile string splitting

## Goals / Non-Goals

**Goals:**
- Unified return type (`P4Output<T>`) for all commands
- Preserve warnings separately from results
- Support both strict (fail on errors) and lenient (partial success) modes
- Capture P4 message metadata for debugging and specific error handling
- Maintain backward compatibility with existing command builder pattern

**Non-Goals:**
- Changing form input text parsing (P4 doesn't support JSON for `-i` commands)
- Supporting P4Python's `exception_level` exactly (we use Rust's Result type instead)
- Async/parallel command execution
- Caching or connection pooling

## Decisions

### Decision 1: Always return `P4Output<T>` wrapper

All commands return `Result<P4Output<T>, P4Error>` instead of `Result<Vec<T>, P4Error>` or `Result<T, P4Error>`.

**Rationale**: Consistency over convenience. Callers can use `.single()` or `.first()` helpers when they know there's one result. This matches P4Python's approach where even `info` returns a list.

**Alternatives considered**:
- Keep per-command return types (current approach) - rejected because it requires callers to know command semantics
- Return `(Vec<T>, Vec<Warning>)` tuple - rejected because it's less ergonomic than a named struct

### Decision 2: Severity-based message classification

Messages are classified by P4's severity field:
- 0-1: Info (filtered out, not exposed to callers)
- 2: Warning (collected in `P4Output.warnings`)
- 3+: Error (causes `Err(P4Error::Command {...})`)

**Rationale**: Matches P4Python's classification and P4's documented behavior.

### Decision 3: Two run modes - strict and lenient

- `run()`: Fails if any severity >= 3 messages occur
- `run_lenient()`: Collects errors as warnings, only fails on fatal issues (connection, parse)

**Rationale**: Different use cases need different behavior. Sync operations often want lenient mode (get what you can), while change creation wants strict mode (fail if something's wrong).

### Decision 4: Error type restructure

`P4Error::Command` contains:
- `errors: Vec<P4Message>` - all error messages
- `partial_results: Option<serde_json::Value>` - raw JSON of any successful results

**Rationale**: Allows callers to extract partial successes from failures if needed. The raw JSON avoids needing to parameterize the error type.

## Risks / Trade-offs

**Breaking change** → All p4rs consumers need updates. Mitigated by prgit being the only consumer and controlled migration.

**Slight API verbosity** → `p4.info().run()?.single()?` instead of `p4.info().run()?`. Mitigated by helper methods and clearer semantics.

**Partial results as raw JSON** → Type safety lost for partial results in error case. Accepted because this is an edge case and avoids complex generics.
