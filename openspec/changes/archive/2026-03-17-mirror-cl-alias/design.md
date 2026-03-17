## Context

prgit mirrors P4 changelists to git commits. When a user pushes a git branch, prgit shelves it as a P4 CL and records the mapping `(branch → shelved_cl)` in `branch_shelve_mapping`. During mirroring, when a submitted CL matches a shelved CL, the mirror adds the git branch as a merge parent (merge-ours strategy).

Some users unshelve the change to their own workspace, move files to a new CL, and submit that instead. The mirror has no way to connect this new CL back to the original branch, breaking the merge integration.

## Goals / Non-Goals

**Goals:**
- Allow the shelving user to declare "CL X is the same change as my shelved CL Y"
- Mirror uses alias to resolve the branch for merge integration
- Authorization: only the original shelver can create aliases for their CLs

**Non-Goals:**
- Many-to-one or many-to-many alias mappings
- Automatic detection of unshelved-and-resubmitted CLs
- Alias deletion endpoint (aliases are replaced by new ones or become irrelevant after submission)

## Decisions

### 1. Store shelver username in `branch_shelve_mapping`

Currently the table stores `(prgit_client_id, branch, shelved_change)`. We add a `shelver_user TEXT NOT NULL` column to record which P4 user created the shelve. This is set during the shelve flow from the authenticated user's identity.

**Why not query P4 for CL ownership?** The shelve client name is derived from the base client, not the user's personal client. The P4 CL owner is the shelve-client's user, not necessarily the pushing user. Storing it at shelve time is authoritative.

**Migration:** New column with default `''` for existing rows. Existing rows won't support alias auth, but this is acceptable since alias is a new feature.

### 2. New `shelve_cl_alias` table

```sql
CREATE TABLE IF NOT EXISTS shelve_cl_alias (
    prgit_client_id INTEGER NOT NULL REFERENCES prgit_clients(id),
    alias_cl INTEGER NOT NULL,
    shelved_change INTEGER NOT NULL,
    PRIMARY KEY (prgit_client_id, alias_cl),
    UNIQUE (prgit_client_id, shelved_change)
);
```

The `UNIQUE` on `(prgit_client_id, shelved_change)` enforces one-to-one: each shelved CL has at most one alias, and each alias maps to at most one shelved CL. Using `INSERT OR REPLACE` handles the replacement semantics.

**Why a separate table vs. adding to `branch_shelve_mapping`?** The alias is a different relationship — it maps between two CLs, not between a branch and a CL. Keeping them separate avoids complicating the existing mapping logic.

### 3. POST endpoint on the window server

Route: `POST /api/repos/{group}/{name}/cl-alias`

Body: `{"shelved_cl": <number>, "alias_cl": <number>}`

Auth: Reuse `authenticate_push` (HTTP Basic with P4 ticket). The handler extracts the username from auth, looks up the shelver for the `shelved_cl` in `branch_shelve_mapping`, and compares.

Response codes: 201 (created), 400 (bad request), 401 (unauthorized), 403 (not the shelver), 404 (repo or shelved CL not found).

### 4. Alias-aware `get_branch_for_change`

The current `get_branch_for_change` queries `branch_shelve_mapping` by `shelved_change`. The new logic:
1. Query `branch_shelve_mapping` directly (existing behavior)
2. If no result, query `shelve_cl_alias` to resolve `alias_cl → shelved_change`
3. If alias found, query `branch_shelve_mapping` with the resolved `shelved_change`

This is a single method change in `PrgitClient`. The `MirrorData::get_related_branch` implementation calls this, so no changes to the mirror itself.

## Risks / Trade-offs

- **[Stale aliases]** → Aliases persist after the CL is submitted and mirrored. This is harmless — the alias lookup only matters at mirror time for the specific CL. No cleanup needed.
- **[Race condition: alias created after mirror runs]** → If the user submits the new CL and the mirror processes it before the alias is created, the merge integration is missed for that run. The alias will be used on the next mirror iteration if the CL is still relevant. This is acceptable given the user must manually create the alias anyway.
- **[Schema migration]** → Adding a column to `branch_shelve_mapping` and a new table. SQLite `ALTER TABLE ADD COLUMN` with a default handles this. The `CREATE TABLE IF NOT EXISTS` for the new table is safe.
