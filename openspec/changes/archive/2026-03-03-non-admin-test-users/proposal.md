## Why

Currently all tests run with implicit super user access because the test p4d server has no protection table configured. This means:

1. Tests don't catch permission-related bugs
2. Code that should fail for non-admin users passes in tests
3. No way to test admin-only operations separately from regular operations

Running tests as non-admin users better simulates real-world usage and catches permission issues early.

## What Changes

- Setup protection table when test server starts
- Create admin user with known credentials
- Test clients run as regular users (write access only)
- Add `admin_p4()` function to get admin access when needed
- Implement `p4 protect` command to manage protection table
- Add `PermissionDenied` error variant for clearer error handling

## Capabilities

### New Capabilities

- `p4-protect-command`: Get and set P4 protection table
- `testkit-admin-access`: Function to obtain admin P4 access from any P4 object

### Modified Capabilities

- `testkit-server-setup`: Server startup now configures protections

## Impact

- `crates/p4rs/src/commands/protect.rs` - New file for protect command
- `crates/p4rs/src/commands/mod.rs` - Export protect module
- `crates/p4rs/src/p4.rs` - Add protect() method
- `crates/p4rs/src/testkit.rs` - Setup protections, add admin_p4()
- `crates/p4rs/src/error.rs` - Add PermissionDenied variant
- `crates/p4rs/tests/test_p4.rs` - Add non-admin and permission tests
