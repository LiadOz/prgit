## 1. Feature Flag Setup

- [x] 1.1 Add `testkit-local` feature to `crates/p4rs/Cargo.toml` that enables `tempfile`, `uuid`, `libc` but NOT `testcontainers`
- [x] 1.2 Update root `Cargo.toml` dev-dependency to allow enabling `testkit-local` feature

## 2. Local P4Server Implementation

- [x] 2.1 Add port allocation helper: bind `TcpListener` to port 0, extract assigned port, drop listener
- [x] 2.2 Add local `p4d` startup logic: create temp dir, spawn `p4d -r <tempdir> -p localhost:<port>`, wait for ready using `wait_for_p4_ready()`
- [x] 2.3 Store `Child` process handle and `TempDir` in `P4Server` (behind `#[cfg(feature = "testkit-local")]`)
- [x] 2.4 Implement `Drop` for local backend: kill child process
- [x] 2.5 Register atexit handler to kill p4d process on abnormal exit (matching existing Docker cleanup pattern)

## 3. Conditional Compilation

- [x] 3.1 Gate Docker-specific code (`testcontainers` imports, container field, Docker cleanup) behind `#[cfg(all(feature = "testkit", not(feature = "testkit-local")))]`
- [x] 3.2 Gate local p4d code behind `#[cfg(feature = "testkit-local")]`
- [x] 3.3 Ensure shared code (`TestClient`, `ChangelistBuilder` helpers, `ADMIN_USER`, `setup_protections`, `wait_for_p4_ready`, `SERVER` static) remains available under both feature flags

## 4. Verification

- [x] 4.1 Run `cargo test -p p4rs --features testkit-local` and confirm tests pass with local p4d
- [x] 4.2 Run `cargo test --features testkit-local` (root crate) and confirm mirror tests pass
- [x] 4.3 Verify `cargo test -p p4rs --features testkit` still works with Docker backend
