# Why prgit and not other tools

## What prgit is not

**Not a migration tool.** It doesn't move your history out of Perforce.

**Not a way to bypass Perforce.** You can't submit, rewrite history, or restructure the depot from git. You can only introduce changes — via shelved changelists that go through your existing P4 workflows.

**Not hiding Perforce from you.** Users still have P4 accounts and interact with P4 for review and submission. prgit lowers the bar — most developers don't need to learn client specs, pending changelists, or `p4 reconcile` — but it doesn't eliminate P4 from the picture.

## Existing tools

### git-p4

Client-side tool bundled with git. Each developer runs their own isolated git clone — no shared history, no seeing other people's branches. It can submit directly to P4, bypassing review workflows. Every developer needs the p4 CLI and client access configured locally.

prgit is server-side. One shared git repo, everyone sees each other's branches, and changes go through shelve — not direct submit.

### Helix Core Git Connector

Perforce's official solution, but it works the other direction: it mirrors git repos into P4 using graph depots, a separate depot type from the classic depots your existing workflows use. It's designed for teams that already use git and want to consolidate into P4, not for teams with an existing P4 depot that want a git interface.

prgit works with your existing classic depot as-is.

### p4-fusion, git-p4-bridge

One-way sync tools (P4 to git). No write-back path.

prgit is bidirectional — mirror in, shelve out.
