## 1. Skip .git path components during mirroring

- [x] 1.1 In `Mirror::create_commit` (`src/mirror/mirror.rs`), add a check after computing `path_in_repo` that skips files where any path component equals `.git`. Use `std::path::Path::components()`. Log a warning with the full depot path.
- [x] 1.2 Add a test in `tests/mirror_tests.rs` that submits a file with `.git` as a path component (e.g. `submod/.git`) and verifies the mirror completes without error and the `.git` file is absent from the git tree.
- [x] 1.3 Add a test that other files in the same change are still mirrored when one file is skipped.

## 2. Observability

- [x] 2.1 Add `MirrorFileSkipped { repo, change, depot_path, reason }` variant to `ObservabilityEvent` in `src/window/observability.rs`.
- [x] 2.2 Thread the emitter (or a callback) into the mirror so it can emit the event when a file is skipped. Alternatively, return skipped file info from `create_commit` and let the caller emit.
