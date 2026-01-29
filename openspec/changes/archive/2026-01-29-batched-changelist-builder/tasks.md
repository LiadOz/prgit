## 1. Core Types

- [x] 1.1 Create `crates/p4rs/src/changelist.rs` module
- [x] 1.2 Add `PendingOp` enum with `Add`, `Edit`, `Delete`, `Move` variants
- [x] 1.3 Add `ChangelistBuilder` struct with `p4`, `changelist`, `root`, `pending`, `immediate` fields
- [x] 1.4 Export `ChangelistBuilder` from `lib.rs`

## 2. Builder Construction

- [x] 2.1 Implement `ChangelistBuilder::new(&p4, root, description)` that creates changelist
- [x] 2.2 Implement `.immediate()` method to enable immediate mode

## 3. File Operations (Library - P4 only)

- [x] 3.1 Add `determine_file_type(path) -> Result<FileType, P4Error>` helper (extract from `ShelveClient`)
- [x] 3.2 Implement `add(path)` - auto-detect type from disk, error if file missing
- [x] 3.3 Implement `add_with_type(path, file_type)` - explicit type, no detection
- [x] 3.4 Implement `edit(path)` - auto-detect type from disk, error if file missing
- [x] 3.5 Implement `edit_with_type(path, file_type)` - explicit type, no detection
- [x] 3.6 Implement `delete(path)` - no file type needed
- [x] 3.7 Implement `move_file(from, to)` - auto-detect type from source file
- [x] 3.8 Implement `move_file_with_type(from, to, file_type)` - explicit type, no detection

## 4. Flush Logic

- [x] 4.1 Implement `flush()` method that groups and executes pending operations
- [x] 4.2 Group adds by file type, execute batched `p4 add` commands
- [x] 4.3 Group edits by file type, execute batched `p4 edit` commands
- [x] 4.4 Execute batched `p4 delete` command for all deletes
- [x] 4.5 Handle moves: edit source, then move (can't batch moves)
- [x] 4.6 Clear pending vec after successful flush

## 5. Submit

- [x] 5.1 Implement `submit()` that calls `flush()` then `p4 submit`
- [x] 5.2 Return `SubmitResult` from submit

## 6. Testkit Update

- [x] 6.1 Remove `ChangelistBuilder` implementation from `testkit.rs`
- [x] 6.2 Add `ChangelistBuilderExt` trait with content-writing methods
- [x] 6.3 Implement `add_file(path, content)` - writes file, calls `add()`
- [x] 6.4 Implement `edit_file(path, content)` - calls `edit()`, writes file
- [x] 6.5 Implement `delete_file(path)` - delegates to `delete()`
- [x] 6.6 Implement `move_file(from, to)` with optional content parameter
- [x] 6.7 Update `TestClient::changelist()` to return library's builder
- [x] 6.8 Verify existing tests pass with batched mode

## 7. Tests

- [x] 7.1 Add unit tests for `PendingOp` grouping logic
- [x] 7.2 Add integration test for batched add (multiple files, one command)
- [x] 7.3 Add integration test for batched edit
- [x] 7.4 Add integration test for batched delete
- [x] 7.5 Add integration test for mixed operations
- [x] 7.6 Add integration test for immediate mode
- [x] 7.7 Add test for file type auto-detection (text, executable, symlink)
- [x] 7.8 Add test for explicit file type override with `_with_type()` methods
- [x] 7.9 Add test for error when `add()` called on non-existent file
