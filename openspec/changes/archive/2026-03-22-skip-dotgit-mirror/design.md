## Context

The mirror loop in `Mirror::create_commit` iterates over all files in a P4 change and creates corresponding git tree entries via `CommitBuilder`. libgit2 rejects `.git` as a tree entry name (security hardening against repo injection attacks). If any file in a P4 change has `.git` as a path component, the entire mirror cycle fails.

This is a real issue — git submodules and worktrees store a `.git` file (not directory) that points to the actual git objects location. If someone submits such a repo into P4, the `.git` file ends up in the depot.

## Goals / Non-Goals

**Goals:**
- Mirror continues processing when encountering `.git` path components
- Operator visibility into skipped files via logs and observability events
- No data loss — all other files in the same change are mirrored normally

**Non-Goals:**
- Rewriting or transforming `.git` files into something git-safe
- Filtering other git-reserved names (e.g. `.gitmodules`, `.gitattributes`) — those are valid git tree entries
- Filtering at the P4 depot level (not our responsibility)

## Decisions

**Filter location: inside `create_commit`, before `builder.upsert`/`builder.remove`**

The filter goes in the file iteration loop at line 174 of `mirror.rs`. This is the narrowest point — we skip the individual file entry without affecting the rest of the change. A `continue` after logging is sufficient.

**Check all path components, not just the filename**

A `.git` component can appear anywhere in the path (e.g. `foo/.git/config`). Use `Path::components()` to check each segment.

**Log + emit event, don't error**

A warning log for immediate visibility, plus a `mirror.file_skipped` observability event for historical tracking. The mirror should never fail due to uningested P4 content.

## Risks / Trade-offs

- **Skipped files are silently absent from git** — mitigated by warning logs and the observability event. Operators can search for skipped files and clean up the depot.
- **Future git-reserved names** — if libgit2 adds more restrictions, we'll hit the same crash. Acceptable risk; we fix them as they appear rather than speculatively filtering.
