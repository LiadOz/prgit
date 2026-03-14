## Context

prgit has a working library layer (`mirror`, `shelf`, `cabinet`) and a proof-of-concept CLI (`poc`) that demonstrates init, serve (mirror loop), and hook (push→shelve) workflows. The next step is an HTTP server that hosts git repos over the network, replacing the `poc` binary with a production-ready service.

The server must:
- Serve git repos via the smart HTTP protocol
- Run mirror loops in the background
- Intercept pushes to shelve branches into P4
- Authenticate pushes using P4 tickets

Existing library code requires no modifications — the server is a new binary that wraps it.

## Goals / Non-Goals

**Goals:**
- Serve git repos over HTTP with GitLab-style URL routing
- Authenticate pushes via P4 tickets (HTTP basic auth)
- Run background mirror tasks per repo
- Intercept push ref changes and shelve in-process
- Load config from a static YAML file

**Non-Goals:**
- Config hot-reload (deferred)
- Self-service repo creation via API (deferred — config-driven for now)
- Rate limiting or multi-tenancy isolation (deferred)
- Custom auth layer beyond P4 tickets (deferred)
- Replacing `git-http-backend` with a custom git protocol implementation

## Decisions

### 1. HTTP framework: axum

**Decision:** Use axum with tokio as the async runtime.

**Why:** tokio is needed for background mirror tasks. axum is tokio-native, widely adopted, and Tower middleware provides timeouts, logging, etc. for free.

**Alternatives considered:**
- actix-web: Actor model adds complexity we don't need.
- warp: Less active development, filter-based API is less intuitive.

### 2. Git protocol: proxy to git-http-backend

**Decision:** Proxy all git smart HTTP requests to the `git-http-backend` CGI program.

The server sets `GIT_PROJECT_ROOT`, `PATH_INFO`, and other CGI environment variables, spawns `git-http-backend` as a child process, and pipes the HTTP request body to stdin / response from stdout.

**Why:** git-http-backend is battle-tested and handles all pack negotiation. Reimplementing it (even with gitoxide) is significant work for no benefit at this stage.

**Alternatives considered:**
- gitoxide (`gix-protocol`): Full control but large integration effort. Could replace git-http-backend later if needed.

### 3. URL routing: GitLab-style paths

**Decision:** Route git requests using `/{group}[/{subgroup}]/{project}.git/...` paths.

The server matches incoming request paths against configured repos to find the correct bare repo on disk. The `group` and `name` fields in config define the URL path.

A request to `/engine/main.git/info/refs` maps to the repo configured as `group: "engine", name: "main"`.

Subgroups are supported via slashes in the group: `group: "engine/tools"` serves at `/engine/tools/{name}.git/...`.

### 4. Push authentication: P4 ticket via HTTP basic auth

**Decision:** Require HTTP basic auth on `git-receive-pack` (push) endpoints. The username is the P4 user, the password is the P4 ticket. The server validates the ticket by running `p4 login -s` against the configured P4 server.

For read operations (`git-upload-pack`), no auth is required.

**Why:** P4 tickets are the natural auth mechanism — no separate auth system needed. Git already supports basic auth natively. The shelver needs a P4 identity anyway, so the credentials serve double duty.

**Alternatives considered:**
- Separate ticket upload endpoint: More state to manage, no benefit.
- Server-side ticket storage (TicketStore): Unnecessary when credentials come with every request.

### 5. Push→shelve: server-intercept (no hooks)

**Decision:** The `git-receive-pack` request body contains the ref updates being pushed (old-sha, new-sha, refname). After proxying the push to `git-http-backend`, the server checks which refs were updated. For each updated ref that isn't the synced branch, it runs the shelver in-process for that branch.

**Why:** Single process, single config. No hook binary to deploy or configure. The server already has the database handle, P4 config, and repo reference in memory. The push request itself tells us exactly which branch was pushed — no need to snapshot or diff all refs.

**Alternatives considered:**
- Post-receive hook: Works but requires a separate binary with its own config/DB access.
- Hybrid (hook notifies server): Unnecessary indirection.

### 6. Mirror scheduler: one tokio task per repo

**Decision:** On startup, the server spawns a `tokio::spawn` task per configured repo. Each task runs an infinite loop: call `Mirror.run()`, sleep for the configured interval, repeat.

The tasks are spawned after initial config load. If a mirror iteration fails, it logs the error and continues (does not crash the server).

### 7. Config format: static YAML

**Decision:** A single YAML config file defines the server listen address, data directory (where bare repos and the SQLite DB live), and a list of repos with their P4 client mapping and mirror settings.

```yaml
listen: "0.0.0.0:3000"
data_dir: "/var/lib/prgit"

repos:
  - group: "engine"
    name: "main"
    p4port: "perforce:1666"
    p4client: "engine-main"
    synced_branch: "main"
    mirror_interval_secs: 60
    max_changes: 100
```

The bare git repo for each entry is created at `{data_dir}/repos/{group}/{name}.git` if it doesn't exist. The SQLite database lives at `{data_dir}/prgit.db`.

### 8. Binary structure

**Decision:** New binary `prgit-server` in `src/bin/server.rs` (or `src/bin/server/main.rs` if it grows). Takes a `--config` flag pointing to the YAML file.

```
prgit-server --config /etc/prgit/config.yaml
```

## Risks / Trade-offs

- **[Concurrent pushes to same branch]** Two users pushing the same branch simultaneously could race the shelver. → Shelving is idempotent (reshelve overwrites), so the last push wins. Acceptable for Phase 1.
- **[git-http-backend availability]** The server depends on `git-http-backend` being in PATH. → Fail fast on startup if not found. Document the requirement.
- **[Mirror task failure]** A persistent P4 outage causes mirror tasks to log errors in a loop. → Log at warn level, use exponential backoff on consecutive failures.
- **[Blocking P4 calls in async context]** Mirror and shelver use synchronous P4 commands. → Run them via `tokio::task::spawn_blocking` to avoid blocking the async runtime.
