## Why

The mirror relies on the `branch_shelve_mapping` to associate a submitted P4 changelist with the git branch it originated from, enabling merge-ours integration. This mapping is keyed by the shelved CL that prgit created. However, some users unshelve the change to their own client workspace, move the files to a different CL, and submit that CL instead. When this happens, the mirror sees a submitted CL it has no branch mapping for, so the merge integration is lost. Users need a way to tell prgit "CL X is really the same change as shelved CL Y" so the mirror can resolve the branch correctly.

## What Changes

- Add a new `shelve_cl_alias` database table that maps an alias CL to an original shelved CL (one-to-one).
- Add a new POST endpoint on the window server (`/api/repos/{group}/{name}/cl-alias`) that allows a user to create an alias mapping.
- The endpoint requires the caller to authenticate as the user who originally shelved the CL (the shelver).
- The mirror's related-branch resolution (`get_related_branch`) falls through to the alias table when no direct mapping exists: alias CL → original shelved CL → branch.

## Capabilities

### New Capabilities
- `cl-alias`: CL alias creation, storage, authorization, and resolution for the mirror's branch lookup.

### Modified Capabilities

## Impact

- **Database**: New `shelve_cl_alias` table in `cabinet/tables.rs`.
- **Cabinet**: New methods on `PrgitClient` for alias CRUD and alias-aware branch lookup.
- **Window handlers**: New POST route with auth validation (reuses existing P4 ticket auth).
- **Window router**: New route registration in `build_app`.
- **Mirror**: `get_related_branch` implementation in `PrgitClient` updated to check aliases as a fallback.
