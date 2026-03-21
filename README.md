# prgit

A bidirectional bridge between Perforce and Git. P4 submitted changes mirror into git commits; git branch pushes shelve back into P4.

## The problem

Your team uses Perforce. It handles large repos, binary assets, and fine-grained access control well. But developers want to use git.

Not because git is better at version control — but because git's authoring experience is better. Local branches are instant. You can stage, stash, rebase, and iterate without touching a server. You can work offline. And the entire modern tooling ecosystem — IDEs, code review, CI, AI assistants — assumes git.

## What prgit does

prgit gives your team git for the authoring loop while keeping Perforce as the system of record. You write code in git, push a branch, and it becomes a shelved changelist in P4 that goes through your normal review and submit process. No migration, no new workflows on the P4 side, no special commands to learn. Just git.

Most users only need to know the submission side of P4 — how to review a shelved changelist and submit it. You don't need to understand P4 branches, client mappings, streams, or integration. Those are concerns for the people setting up the bridge and the integration specialists, not for the developer writing code.

## The prgit flow

You use git commands to do P4 things:

- `git clone` — get the repo (mirrored from P4 submitted changes)
- `git pull` — get latest (new P4 submissions mirrored to git)
- `git push` — shelve your branch in P4
- Review and submit happen in P4, through your existing process

Multiple users share one git repo. Everyone sees each other's branches. Shelved changelists are tied to branches — push again and the shelve updates.

## Prerequisites

- **Rust** (stable toolchain) — to build prgit
- **p4 CLI** — prgit calls `p4` as a subprocess
- **git** — must include `git-http-backend` (ships with standard git installs)
- **A running Perforce server** — with a client workspace configured for the depot path you want to mirror

## Quick start

Build:

```bash
cargo build --release
```

This produces `prgit-server` in `target/release/`.

Create a config file (e.g. `prgit-server.yaml`):

```yaml
listen: "127.0.0.1:8080"
data_dir: "/var/lib/prgit"
repos:
  - group: depot
    name: main
    p4port: "ssl:perforce.example.com:1666"
    p4client: prgit-mirror
    synced_branch: master
    mirror_interval_secs: 30
    max_changes: 100
```

Run:

```bash
RUST_LOG=info ./target/release/prgit-server --config prgit-server.yaml
```

On startup the server will:
1. Create bare git repos under `data_dir/repos/` if they don't exist
2. Start a background mirror task for each repo (polling P4 on the configured interval)
3. Listen for git HTTP requests and API calls

Clone the mirrored repo:

```bash
git clone http://localhost:8080/depot/main.git
```

## Configuration

### Server config

| Field | Description |
|-------|-------------|
| `listen` | Address and port to bind (e.g. `"0.0.0.0:8080"`) |
| `data_dir` | Directory for git repos, SQLite database, and shelve workspaces |
| `repos` | Array of repositories to mirror |

### Repo config

| Field | Description |
|-------|-------------|
| `group` | URL path prefix — maps to `/{group}/{name}.git` |
| `name` | Repository name |
| `p4port` | Perforce server address |
| `p4client` | P4 client workspace name (must be configured on the server host) |
| `synced_branch` | Git branch that mirrors P4 (usually `master` or `main`) |
| `mirror_interval_secs` | How often to poll P4 for new changes |
| `max_changes` | Maximum number of P4 changes to process per mirror cycle |
| `shelve.async` | If `true`, pushes return immediately and shelve in the background (default: `false`) |

### Data directory layout

```
data_dir/
├── prgit.db              # SQLite database (mappings, metadata)
├── repos/
│   └── {group}/
│       └── {name}.git    # Bare git repo
└── shelve_clients/       # Temporary P4 workspaces for shelving
```

## Usage

### Pushing changes (shelving to P4)

Create a branch, make changes, and push:

```bash
git checkout -b my-feature
# ... make changes ...
git commit -am "implement feature"
git push origin my-feature
```

Push requires authentication. Git will prompt for credentials:
- **Username**: your P4 username
- **Password**: a P4 ticket (obtain one with `p4 login`)

prgit validates the ticket against P4 in real time — it doesn't store credentials.

On push, prgit diffs your branch against the synced branch, finds the corresponding P4 change at the merge base, and creates (or updates) a shelved changelist containing your changes. Push the same branch again to update the shelf.

### Checking shelve status

If the server is configured with `shelve.async: true`, pushes return immediately. Poll for the result:

```
GET /api/v1/repos/{group}/{name}/shelve/status/{branch}
```

Response:

```json
{"state": "done", "changelist": 12345, "client": "prgit-shelve-jsmith"}
```

Possible states: `queued`, `shelving`, `done`, `failed`.

## API

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/api/health` | GET | No | Returns 200 OK |
| `/api/v1/repos/{group}/{name}/shelve/status/{branch}` | GET | No | Shelve status for a branch |
| `/api/v1/repos/{group}/{name}/shelve/cl-alias` | POST | Yes | Register an alternate CL for mirror branch resolution |
| `/{group}/{name}.git/*` | GET | No | Git clone/fetch (anonymous) |
| `/{group}/{name}.git/*` | POST | Yes | Git push (requires P4 credentials) |

## Architecture

prgit is a single Rust binary with four modules:

- **mirror** — polls P4 for submitted changes and creates corresponding git commits on the synced branch
- **shelf** — diffs a pushed git branch against the synced branch and creates/updates a shelved P4 changelist
- **window** — HTTP server that proxies git smart protocol via `git-http-backend`, intercepts pushes to trigger shelving, and serves the REST API
- **cabinet** — SQLite persistence layer mapping commits to changelists, branches to shelves, and tracking client configuration

The **p4rs** crate (`crates/p4rs/`) is a standalone Rust wrapper around the `p4` CLI with typed commands and JSON output parsing.

## Development

```bash
cargo make test     # run tests
cargo make ci       # fmt + lint + test + coverage + audit
cargo make cov      # LLVM code coverage report
```

Tests use a local `p4d` instance. Enable with the `testkit-local` feature:

```bash
cargo test --features testkit-local
```

## Further reading

- [Why prgit and not other tools](docs/why-prgit.md) — comparison to git-p4, Helix Git Connector, and other alternatives
