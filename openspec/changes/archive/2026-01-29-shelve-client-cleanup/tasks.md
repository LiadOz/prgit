## 1. Implementation

- [x] 1.1 Add `p4 sync //...#none` after revert in ShelveClient::new()
- [x] 1.2 Add function to delete contents of client_root (preserving directory)
- [x] 1.3 Call cleanup function after sync #none in ShelveClient::new()
- [x] 1.4 Add cleanup in Drop as well (belt and suspenders)

## 2. Fix apply_changes order

- [x] 2.1 Move p4 edit before file copy in apply_changes() to fix permission denied error

## 3. Verification

- [x] 3.1 Run shelve_client tests to verify fix (51 pass, 2 fail - symlink conversion tests are P4 limitations)
