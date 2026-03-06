## Context

Tests currently use `testcontainers` to spin up a Docker container running `p4d`. This works well in standard development but fails in sandboxed environments (Claude Code, Cursor) where Docker commands require per-command user approval. The existing `P4RS_TEST_PORT` escape hatch requires a manually running server, which defeats automated testing.

The `p4d` binary is lightweight and can run directly on the host — the Docker image itself just downloads `p4d` and runs it. There's no actual need for containerization beyond isolation, which a temp directory provides equally well.

## Goals / Non-Goals

**Goals:**
- Allow `cargo test` to run without Docker when `p4d` is available in PATH
- Provide the same `P4Server` API so no test code changes are needed
- Clean up temp p4d processes and directories reliably on drop and on process exit

**Non-Goals:**
- Replacing the Docker backend — it remains the default
- Supporting p4d configuration beyond what the Docker image provides (basic server, admin user, protections)
- Auto-installing p4d

## Decisions

### 1. Feature flag selection strategy

**Decision:** Use a cargo feature `testkit-local` that replaces the Docker backend at compile time.

When `testkit-local` is enabled, `P4Server::start()` spawns a local `p4d` process instead of using testcontainers. The `testkit` feature continues to use Docker. Both features enable the shared test utilities (`TestClient`, `ChangelistBuilder`, etc.).

**Alternatives considered:**
- Runtime env var toggle: Simpler but would require both `testcontainers` and local code compiled in. The whole point is to avoid the Docker dependency.
- Separate test binary: Too much duplication.

### 2. Local p4d lifecycle

**Decision:** Spawn `p4d` with `-r <tempdir> -p localhost:0` (or a random port) as a child process. Store the `Child` handle in `P4Server` and kill it on `Drop`. Also register an `atexit` handler (matching the existing Docker cleanup pattern) for abnormal exits.

To find the actual port when using `-p localhost:0`, we'll first bind a `TcpListener` to port 0, record the assigned port, drop the listener, then pass that port to `p4d`. There's a small TOCTOU window but it's acceptable for tests.

**Alternatives considered:**
- Using `-p localhost:0` and parsing p4d output for the port: p4d doesn't reliably print the bound port in a parseable way.

### 3. Shared testkit code structure

**Decision:** Use `#[cfg(feature = "testkit-local")]` and `#[cfg(not(feature = "testkit-local"))]` blocks within `testkit.rs` to swap the `P4Server` implementation. All shared code (`TestClient`, `ChangelistBuilder` helpers, `SERVER` static, `ADMIN_USER`, etc.) remains unconditional under the `testkit` feature.

The `testkit-local` feature implies `testkit` minus the `testcontainers` dependency — it brings in `tempfile`, `uuid`, `libc` but not `testcontainers`.

### 4. Admin user setup

**Decision:** After starting local `p4d`, run the same `setup_protections()` that the Docker path uses. The Docker image doesn't do any special user setup — it's all done in `P4Server::start()` after the container is running. So the local path just needs to start `p4d` and call the same setup code.

## Risks / Trade-offs

- **[TOCTOU port race]** → Acceptable for test infrastructure; if port is taken, test fails fast with a clear error.
- **[p4d not in PATH]** → `P4Server::start()` will panic with a clear message ("p4d not found in PATH — install Perforce or use the testkit feature with Docker").
- **[Zombie p4d processes]** → Mitigated by Drop impl + atexit handler. Same pattern as existing Docker cleanup.
- **[Platform differences]** → `p4d` is available for Linux and macOS. Windows is not a target for this project.
