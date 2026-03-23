## 1. Fix FileType Display

- [x] 1.1 Fix `FileType::Display` to combine modifiers after a single `+` (e.g. `text+xC` not `text+x+C`)

## 2. Fix shelve edit path

- [x] 2.1 After `p4 edit`, query the opened file's depot type (via `p4 opened`)
- [x] 2.2 Compute the effective type: start from depot type, toggle only executable bit and base type based on git working copy
- [x] 2.3 Only call `p4 reopen` if the effective type differs from the depot type

## 3. Tests

- [x] 3.1 Test: edit `text+Cx` file without permission change preserves `text+Cx` (`test_edit_preserves_p4_file_type_modifiers`)
- [x] 3.2 Test: edit `text+C` file and add executable preserves `text+Cx` (`test_edit_adding_executable_preserves_other_modifiers`)
- [x] 3.3 Convert bug-documenting test to passing assertion after fix
- [x] 3.4 Test: edit `text+kx` file and remove executable preserves `+k` modifier (`test_edit_removing_executable_preserves_other_modifiers`)
