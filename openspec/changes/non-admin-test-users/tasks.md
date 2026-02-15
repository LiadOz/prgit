## 1. Implement protect command

- [x] 1.1 Create `crates/p4rs/src/commands/protect.rs`
- [x] 1.2 Define `ProtectionTable` struct with Serialize/Deserialize
- [x] 1.3 Define `Protection` struct (access, kind, name, host, path)
- [x] 1.4 Define `AccessLevel` enum (super, admin, write, read, list, etc.)
- [x] 1.5 Define `ProtectionKind` enum (user, group)
- [x] 1.6 Implement `Protect` command builder with `get()` and `set()` methods
- [x] 1.7 Implement get: `p4 protect -o` parsing
- [x] 1.8 Implement set: `p4 protect -i` with formatted input
- [x] 1.9 Add protect module to `commands/mod.rs`
- [x] 1.10 Add `protect()` method to `P4` struct

## 2. Add PermissionDenied error

- [x] 2.1 Add `PermissionDenied(String)` variant to `P4Error` enum
- [x] 2.2 Auto-detect permission errors in `P4Error::command()` constructor

## 3. Update testkit with admin support

- [x] 3.1 Add `ADMIN_USER` and `ADMIN_PASSWORD` constants
- [x] 3.2 Add `admin_p4(p4: &P4) -> P4` function
- [x] 3.3 Update `P4Server::start()` to setup protections after server is ready
- [x] 3.4 Create protection table with super for admin, write for all users

## 4. Add tests

- [x] 4.1 Add test for `protect().get()` command
- [x] 4.2 Add test for `protect().set()` command
- [x] 4.3 Add test that non-admin user cannot modify protections
- [x] 4.4 Add test that admin_p4() can perform admin operations
- [x] 4.5 Add test for PermissionDenied error detection
- [x] 4.6 Verify existing tests still pass with new protection setup

## 5. Verification

- [x] 5.1 Run `cargo test -p p4rs`
- [x] 5.2 Run `cargo clippy -p p4rs`
- [x] 5.3 Verify all tests run as non-admin by default
