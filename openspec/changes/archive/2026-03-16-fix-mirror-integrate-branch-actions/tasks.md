## 1. Fix

- [x] 1.1 In `src/mirror/mirror.rs` `create_commit`, add `FileAction::Branch` and `FileAction::Integrate` to the upsert match arm alongside `Add`/`Edit`/`MoveAdd`

## 2. Validation

- [x] 2.1 Run the existing mirror test suite to ensure no regressions
