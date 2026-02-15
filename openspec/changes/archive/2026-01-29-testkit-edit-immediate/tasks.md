## 1. Implementation

- [x] 1.1 Modify `edit_file` in testkit to call P4 edit directly before writing content
- [x] 1.2 Modify `edit_file_with_opts` in testkit to call P4 edit directly before writing content

## 2. Verification

- [x] 2.1 Run `test_shelve_correct_base_revision` to verify fix (blocked by separate ShelveClient bug)
- [x] 2.2 Run full test suite to ensure no regressions (blocked by separate ShelveClient bug)
