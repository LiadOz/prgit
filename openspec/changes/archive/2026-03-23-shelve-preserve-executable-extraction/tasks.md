## 1. Fix extraction

- [x] 1.1 Set `0o755` permissions on extracted files when git tree entry has `BlobExecutable` filemode

## 2. Tests

- [x] 2.1 E2E test: executable file (text+x in P4, 100755 in git) edited in git, shelved through full Shelver path — executable preserved (`test_shelve_preserves_executable_through_full_path`)
- [x] 2.2 Verify test fails without the fix (regression guard) — confirmed: `executable: false` without fix
