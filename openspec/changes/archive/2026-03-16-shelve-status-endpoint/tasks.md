## 1. Active Shelves Tracker

- [x] 1.1 Create `ActiveShelves` struct wrapping `Arc<Mutex<HashSet<usize>>>` with `insert(cl)`, `remove(cl)`, and `contains(cl)` methods
- [x] 1.2 Add `active_shelves: ActiveShelves` field to `AppState` and initialize in `build_app()`

## 2. Handler Integration

- [x] 2.1 In async path of `shelve_branches()`, insert each prepared CL into `active_shelves` before spawning background task
- [x] 2.2 Clone `ActiveShelves` handle into background closure; remove each CL in `complete_pending_shelves()` after completion (success or failure)

## 3. Status Endpoint

- [x] 3.1 Add `shelve_status` handler function: extract repo path + CL from URL, check `active_shelves.contains(cl)`, return JSON `{ "active": bool }`
- [x] 3.2 Add route `/api/repos/{group}/{name}/shelve-status/{cl}` in `build_app()`
- [x] 3.3 Handle error cases: 404 for unknown repo, 400 for non-numeric CL

## 4. Testing

- [x] 4.1 Unit test: `ActiveShelves` insert/remove/contains behavior
- [x] 4.2 Integration test: status endpoint returns `{ "active": false }` for unknown CL
- [x] 4.3 Integration test: status endpoint returns 404 for non-existent repo
