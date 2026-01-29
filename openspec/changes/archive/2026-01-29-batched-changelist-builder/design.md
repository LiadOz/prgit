## Context

The existing `ChangelistBuilder` in testkit executes P4 commands immediately for each operation. The `ShelveClient` in prgit demonstrates the efficient pattern - collecting operations and executing batched commands grouped by action and file type. This design moves that pattern into p4rs as a first-class API.

## Goals

1. Provide an ergonomic builder API that's efficient by default
2. Minimize P4 command invocations through batching
3. Maintain backward compatibility with testkit usage
4. Support immediate mode for cases that need it

## Non-Goals

1. Replacing `ShelveClient` - it has shelve-specific logic beyond just batching
2. Transaction/rollback semantics - P4 doesn't support this well
3. Automatic conflict detection - that's P4's job at submit time

## Design Decisions

### 1. Struct Layout

```rust
pub struct ChangelistBuilder<'p> {
    p4: &'p P4,
    changelist: usize,
    root: PathBuf,
    pending: Vec<PendingOp>,
    immediate: bool,
}

enum PendingOp {
    Add { path: String, file_type: FileType },
    Edit { path: String, file_type: FileType },
    Delete { path: String },
    Move { from: String, to: String, file_type: Option<FileType> },
}
```

**Rationale**: Store operations as an enum to preserve operation order while enabling grouping during flush. The `root` path is needed to resolve relative paths and write file content.

### 2. File Type Detection

Auto-detect file type when the operation is queued (not during flush):
- `add(path)`, `edit(path)`, `move_file(from, to)` - detect from filesystem
- `add_with_type(path, ft)`, `edit_with_type(path, ft)`, `move_file_with_type(from, to, ft)` - use explicit type

Detection logic (same as `ShelveClient::determine_file_type()`):
1. Check symlink first (`is_symlink()`) → `FileType::symlink()`
2. Check executable bit (`mode & 0o111`) → `FileType::text().executable()`
3. Default → `FileType::text()`

**When detection fails** (file doesn't exist):
- `add()` / `edit()` without explicit type returns error
- Use `_with_type()` variant when file won't exist at queue time

This allows:
- Grouping by file type during flush
- Early error if file doesn't exist (catches bugs)
- Explicit override when needed (binary files, pre-creation scenarios)

### 3. Flush Ordering

Execute operations in this order during `flush()`:
1. **Edits first** - must open files before moving them
2. **Moves** - depends on edit being done
3. **Adds** - independent
4. **Deletes** - independent

Within each category, group by file type to minimize commands.

### 4. Immediate Mode

Enabled via `.immediate()` builder method (opt-in):

```rust
ChangelistBuilder::new(&p4, root, "desc")
    .immediate()
    .add_file("a.txt", content)  // executes p4 add now
    .add_file("b.txt", content)  // executes p4 add now
    .submit();
```

When enabled, each operation method:
1. Detects file type
2. Writes content (if applicable)
3. Executes P4 command immediately
4. Does NOT add to `pending` vec

Default is batched (efficient). Immediate is opt-in for rare cases that need it.

### 5. Builder Ownership

The builder takes `&P4` (borrowed) rather than owning it. This matches the current testkit pattern and allows the P4 instance to be reused.

The builder does NOT implement `Drop` to auto-submit or auto-revert. Explicit `submit()` or `flush()` is required. If dropped with pending operations, they are silently discarded (user's responsibility).

### 6. Testkit Integration

The library's `ChangelistBuilder` handles P4 operations only (paths, not content). Testkit adds an extension trait for convenience methods that write content:

```rust
// Library: P4 operations only
impl ChangelistBuilder {
    pub fn add(&mut self, path: &str) -> &mut Self { ... }
    pub fn edit(&mut self, path: &str) -> &mut Self { ... }
    pub fn delete(&mut self, path: &str) -> &mut Self { ... }
}

// Testkit extension: write content + P4 operation
pub trait ChangelistBuilderExt {
    fn add_file(&mut self, path: &str, content: impl AsRef<[u8]>) -> &mut Self;
    fn edit_file(&mut self, path: &str, content: impl AsRef<[u8]>) -> &mut Self;
}

impl ChangelistBuilderExt for ChangelistBuilder<'_> {
    fn add_file(&mut self, path: &str, content: impl AsRef<[u8]>) -> &mut Self {
        self.write_file(path, content);  // helper or inline
        self.add(path)
    }
}
```

This keeps the library focused on P4, testkit adds file I/O convenience.

### 7. Error Handling

- `flush()` returns `Result<(), P4Error>` - stops on first error
- `submit()` returns `Result<SubmitResult, P4Error>`
- Individual operations in batched mode don't return results (errors surface at flush)
- In immediate mode, each operation returns `Result<Self, P4Error>` for chaining

## Alternatives Considered

### A. Generic over storage path
Could take `AsRef<Path>` for root instead of `PathBuf`. Decided against - simpler to just require `PathBuf`, users can call `.into()`.

### B. Async batching
Could collect operations and execute in parallel. Decided against - P4 server handles parallelism poorly, serial execution is more reliable.

### C. Auto-flush on drop
Could flush pending operations when builder is dropped. Decided against - implicit behavior is surprising, explicit `flush()` or `submit()` is clearer.
