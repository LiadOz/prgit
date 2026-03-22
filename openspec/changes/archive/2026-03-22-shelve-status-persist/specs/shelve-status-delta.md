## MODIFIED Requirements

### shelve-status

**CHANGED**: The `GET /api/v1/repos/{group}/{name}/shelve/status/{branch}` endpoint MUST return shelve status for branches that were shelved in previous server sessions, not only in the current session. When the in-memory tracker has no entry, the endpoint MUST query the `branch_shelve_mapping` table and return `done` status if a mapping exists.
