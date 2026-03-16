## 1. Core Implementation

- [x] 1.1 Add `pending_removes: Vec<String>` field to `CommitBuilder` struct
- [x] 1.2 Update `CommitBuilder::new()` and `CommitBuilder::from_head()` to initialize `pending_removes` as empty vec
- [x] 1.3 Change `remove()` method to push path onto `pending_removes` instead of calling `tree_builder.remove()`
- [x] 1.4 In `build_tree()`, iterate `pending_removes` and call `tree_builder.remove()` only for paths where `base_tree.get_path()` succeeds
- [x] 1.5 Log warning for skipped removes: `Skipping remove of '{path}': not in tree`

## 2. Testing

- [x] 2.1 Add test: remove of an existing file produces correct tree (file absent)
- [x] 2.2 Add test: remove of a non-existent file succeeds without error and logs warning
- [x] 2.3 Add test: double-remove of same file succeeds (first removes, second skips)
