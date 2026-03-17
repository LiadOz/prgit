## 1. Database Schema

- [x] 1.1 Add `shelver_user TEXT NOT NULL DEFAULT ''` column to `branch_shelve_mapping` table in `cabinet/tables.rs`
- [x] 1.2 Create `shelve_cl_alias` table in `cabinet/tables.rs` with `(prgit_client_id, alias_cl, shelved_change)` and UNIQUE constraint on `(prgit_client_id, shelved_change)`

## 2. Cabinet Methods

- [x] 2.1 Update `set_shelved_change_for_branch` to accept and store `shelver_user` parameter
- [x] 2.2 Add `get_shelver_for_change` method to look up the shelver username for a shelved CL
- [x] 2.3 Add `set_cl_alias` method to insert/replace a CL alias mapping
- [x] 2.4 Add `get_shelved_change_for_alias` method to resolve an alias CL to the original shelved CL
- [x] 2.5 Update `get_branch_for_change` to fall back to alias resolution when no direct mapping exists
- [x] 2.6 Add unit tests for all new and modified cabinet methods

## 3. Shelver Integration

- [x] 3.1 Update `Shelver::shelve` and `Shelver::prepare_shelve` to pass the shelver username to `set_shelved_change_for_branch`
- [x] 3.2 Update `shelve_branches` handler calls to propagate the P4 username from auth

## 4. Window Endpoint

- [x] 4.1 Add `create_cl_alias` handler in `window/handlers.rs` with JSON request/response types
- [x] 4.2 Implement shelver authorization check in the handler (compare authenticated user with stored shelver)
- [x] 4.3 Register the POST route at `/api/repos/{group}/{name}/cl-alias` in `build_app`

## 5. Tests

- [x] 5.1 Add integration tests for alias-aware `get_branch_for_change` (direct mapping, alias fallback, no mapping)
- [x] 5.2 Add tests for `get_shelver_for_change` and `set_cl_alias`
- [x] 5.3 Add tests for shelver username storage in shelve flow
- [x] 5.4 Verify all existing tests still pass
