## 1. Database cleanup method

- [x] 1.1 Add `clear_shelved_change_for_branch(branch)` method to PrgitClient that DELETEs from branch_shelve_mapping

## 2. Mirror task cleanup

- [x] 2.1 After building ShelveMergedInfo in mirror_task.rs, call `p4 shelve -d` on the shelved CL
- [x] 2.2 Call `clear_shelved_change_for_branch` to remove the mapping
- [x] 2.3 Log success at info level, failure at warn level — never block mirroring

## 3. Tests

- [x] 3.1 Unit test: `clear_shelved_change_for_branch` removes the mapping and returns None on subsequent lookup
- [x] 3.2 Unit test: `clear_shelved_change_for_branch` on nonexistent branch is a no-op
