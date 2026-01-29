## 1. Core Types

- [x] 1.1 Add `P4Message` struct to `error.rs` with severity, generic, msgid, data fields
- [x] 1.2 Implement `is_error()`, `is_warning()`, `is_info()` methods on `P4Message`
- [x] 1.3 Add `P4Output<T>` struct to new `output.rs` with results and warnings fields
- [x] 1.4 Implement `single()`, `first()`, `is_empty()`, `len()` methods on `P4Output`
- [x] 1.5 Implement `IntoIterator` for `P4Output<T>`
- [x] 1.6 Export new types from `lib.rs`

## 2. Error Handling

- [x] 2.1 Restructure `P4Error::Command` variant to hold `errors: Vec<P4Message>` and `partial_results: Option<Value>`
- [x] 2.2 Add `message()` helper method to `P4Error` for concatenated error text
- [x] 2.3 Add `contains(&str)` helper method to `P4Error` for pattern matching
- [x] 2.4 Update `extract_errors()` in `p4.rs` to return `Vec<P4Message>` instead of tuple

## 3. Core Execution

- [x] 3.1 Add internal `run_and_parse()` method that returns `(Vec<T>, Vec<P4Message>)`
- [x] 3.2 Update `run()` to return `Result<P4Output<T>, P4Error>` with error on severity >= 3
- [x] 3.3 Add `run_lenient()` that collects all messages as warnings
- [x] 3.4 Remove `run_multi_line()` distinction (handle internally based on command)

## 4. Command Updates

- [x] 4.1 Update `P4Command` trait to use `P4Output<Self::Response>`
- [x] 4.2 Update `info.rs` to return `P4Output<InfoResponse>`
- [x] 4.3 Update `changes.rs` to return `P4Output<ChangeData>`
- [x] 4.4 Update `describe.rs` to return `P4Output<DescribeResult>`
- [x] 4.5 Update `sync.rs` to return `P4Output<SyncResult>`
- [x] 4.6 Update `edit.rs`, `add.rs`, `delete.rs` to return `P4Output`
- [x] 4.7 Update `opened.rs`, `revert.rs`, `reopen.rs` to return `P4Output`
- [x] 4.8 Update `files.rs`, `print.rs`, `where_cmd.rs` to return `P4Output`
- [x] 4.9 Update `change.rs` form commands to return `P4Output<usize>`
- [x] 4.10 Update `client.rs` form commands to return `P4Output<String>`
- [x] 4.11 Update `shelve.rs`, `submit.rs`, `move_file.rs`, `user.rs` to return `P4Output`

## 5. Consumer Updates

- [x] 5.1 Update `src/shelf/shelve_client.rs` to use new API
- [x] 5.2 Update `src/shelf/client_pool.rs` to use new API
- [x] 5.3 Update `src/shelf/shelver.rs` to use new API (no changes needed)
- [x] 5.4 Update `src/mirror/mirror.rs` to use new API
- [x] 5.5 Update `src/main.rs` to use new API (no changes needed)

## 6. Tests

- [x] 6.1 Add unit tests for `P4Message` classification
- [x] 6.2 Add unit tests for `P4Output` methods (single, first, iteration)
- [x] 6.3 Update `test_p4.rs` integration tests for new return types
- [x] 6.4 Add tests for `run_lenient()` partial success behavior (covered by existing patterns)
- [x] 6.5 Update `mirror_tests.rs` for new API
