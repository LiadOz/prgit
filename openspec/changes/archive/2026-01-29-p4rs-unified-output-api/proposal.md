## Why

P4's CLI output is inconsistent - single vs multi-line JSON, errors mixed with results, warnings silently dropped. The current p4rs API loses information (warnings), fails entirely on partial success, and has no unified way to handle P4 messages. Following P4Python's proven approach will give us a robust, predictable API.

## What Changes

- **BREAKING**: All commands return `P4Output<T>` instead of `Vec<T>` or `T`
- Add `P4Message` struct to capture P4 server messages with severity, generic code, and msgid
- Add `P4Output<T>` wrapper that holds results + warnings separately
- Add `run_lenient()` method for partial-failure tolerance
- Restructure `P4Error::Command` to include partial results and multiple error messages
- Form commands (`change -i`, `client -i`) also wrapped in `P4Output` for consistency

## Capabilities

### New Capabilities
- `p4-output-types`: New `P4Output<T>` and `P4Message` types that separate results from warnings
- `p4-error-handling`: Improved error handling with `run()` vs `run_lenient()` modes and structured error messages

### Modified Capabilities

## Impact

- `crates/p4rs/src/error.rs`: Restructure `P4Error` enum, add `P4Message`
- `crates/p4rs/src/p4.rs`: Change `run()`, `run_multi_line()` to return `P4Output`
- `crates/p4rs/src/commands/*.rs`: All command `run()` methods return `P4Output<T>`
- `crates/p4rs/src/lib.rs`: Export new types
- `src/shelf/*.rs`: Update prgit to use new API
- `src/mirror/*.rs`: Update prgit to use new API
- `tests/`: Update all tests for new return types
