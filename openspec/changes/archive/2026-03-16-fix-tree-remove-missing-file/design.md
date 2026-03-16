## Context

`CommitBuilder` in `src/mirror/commit_builder.rs` wraps git2's `TreeUpdateBuilder` to construct git commits during P4-to-git mirroring. Its `remove()` method directly delegates to `TreeUpdateBuilder::remove()`, which panics/errors if the target path doesn't exist in the base tree.

P4 can produce Delete actions for files that aren't in the git tree — double-deletes (P4 allows #N delete after #N-1 delete via re-add cycles) and deletes on files that arrived via skipped Branch/Integrate actions. The mirror must tolerate these gracefully.

## Goals / Non-Goals

**Goals:**
- `CommitBuilder::remove()` silently tolerates paths that don't exist in the git tree
- Missing-file removes produce a warning log for observability
- No change to the public API signature

**Non-Goals:**
- Validating P4 action sequences upstream of CommitBuilder
- Deduplicating repeated removes of the same path (harmless — the second check simply won't find it)

## Decisions

### Deferred removal with existence check

**Decision:** Collect removes in a `pending_removes: Vec<String>` field. In `build_tree()`, resolve the base tree first, then iterate `pending_removes` and only call `TreeUpdateBuilder::remove()` for paths that exist in the base tree (checked via `tree.get_path()`).

**Rationale:** The base tree is already resolved in `build_tree()`, so checking existence there is natural and zero-cost — `get_path()` is a tree lookup. Checking at `remove()` call-time would require resolving the base tree eagerly, changing the builder's lifecycle.

**Alternative considered:** Catch the error from `create_updated()` and retry without the offending path. Rejected — git2 doesn't provide granular error recovery, and retrying tree builds is wasteful.

## Risks / Trade-offs

- **[Ordering of upsert+remove on same path]** → If the same path is both upserted and in `pending_removes`, the remove check runs against the *base* tree (before upserts). This is correct: `TreeUpdateBuilder` applies upserts and removes together in `create_updated()`, and a remove after upsert on the same path would be a P4 action sequence bug, not something we need to handle here.
- **[Warning noise]** → Double-deletes will log warnings. This is intentional — operators should know when P4 history has these anomalies. If it becomes noisy, log level can be adjusted later.
