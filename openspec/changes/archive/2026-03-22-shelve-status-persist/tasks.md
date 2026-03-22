## 1. DB fallback in shelve_status handler

- [x] 1.1 In `shelve_status` handler (`src/window/handlers.rs`), when `ActiveShelves` returns `None`, open the DB, look up the repo's `client_id`, and call `get_shelved_change_for_branch`. If found, return `ShelveState::Done` with the changelist and shelver user. Otherwise return 404.
- [x] 1.2 Add integration test: shelve a branch, clear `ActiveShelves`, query status endpoint — should return `done` with correct changelist.
