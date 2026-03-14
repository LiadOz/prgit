## Why

prgit currently runs as CLI tools (`prgit` binary for one-shot mirror, `poc` for init/serve/hook). There's no HTTP server that can serve git repos to developers over the network. To enable `git clone`/`push`/`pull` workflows against prgit-managed repos, we need a server that hosts git repositories, proxies the git protocol, runs mirror loops, and intercepts pushes to trigger P4 shelving.

## What Changes

- Add an HTTP server (axum) that serves git repositories over the smart HTTP protocol
- Proxy git clone/fetch/push to `git-http-backend`, with the server routing requests to the correct bare repo based on GitLab-style URL paths
- Authenticate pushes using HTTP basic auth where username=P4 user, password=P4 ticket, validated against the P4 server in real time
- Allow anonymous access for read operations (clone/fetch)
- Run one background mirror task per repo, polling P4 periodically and syncing changes to git
- Intercept push results (ref changes) after proxying to `git-http-backend` and run the shelver in-process to create/update P4 shelved changelists
- Load repo configuration from a static YAML config file
- Add a health check endpoint

## Capabilities

### New Capabilities
- `git-http-serving`: Proxy git smart HTTP protocol to `git-http-backend`, routing requests via GitLab-style URLs `/{group}[/{subgroup}]/{project}.git`
- `push-auth`: Authenticate git push operations using P4 tickets via HTTP basic auth, validated against the P4 server
- `push-shelve-intercept`: Detect ref changes after a push is proxied and run the shelver in-process (replacing the post-receive hook)
- `mirror-scheduler`: Background task per repo that runs the mirror loop on a configurable interval
- `server-config`: YAML-based static configuration for server listen address, data directory, and repo definitions

### Modified Capabilities

_(none)_

## Impact

- New binary: `prgit-server`
- New dependencies: `axum`, `tokio`, `tower`, `serde_yaml`, `hyper`
- `Cargo.toml`: new binary target and dependencies
- Existing library code (`mirror`, `shelf`, `cabinet`) used as-is — no modifications needed
- `poc` binary becomes redundant once server is complete (its `serve` and `hook` subcommands are replaced)
