## 1. Update database schema

- [x] 1.1 Remove `max_clients` and `timeout_secs` columns from `shelve_config` table schema
- [x] 1.2 Remove `shelve_clients` table schema entirely
- [x] 1.3 Update `Database::create_shelve_config()` to only take `clients_root`
- [x] 1.4 Update `ShelveConfig` struct to only have `prgit_client_id` and `clients_root`

## 2. Remove pool management from PrgitClient

- [x] 2.1 Remove `get_available_shelve_client()` method
- [x] 2.2 Remove `get_timed_out_shelve_client()` method
- [x] 2.3 Remove `count_shelve_clients()` method
- [x] 2.4 Remove `acquire_shelve_client()` method
- [x] 2.5 Remove `release_shelve_client()` method
- [x] 2.6 Remove `register_shelve_client()` method

## 3. Replace ClientPool with simple function

- [x] 3.1 Create `get_shelve_client(prgit_client, user_p4: &P4) -> Result<ShelveClient>` function
- [x] 3.2 Extract user_id from user_p4 (p4 info or similar)
- [x] 3.3 Implement client name derivation: `{base_client}-{user_id}-shelve`
- [x] 3.4 Implement P4 client existence check
- [x] 3.5 Implement P4 client creation (copy view from base client, Host field empty)
- [x] 3.6 Implement local directory creation
- [x] 3.7 Add `libc` crate dependency for file locking (used instead of fs2)
- [x] 3.8 Implement flock-based locking (`{client_root}/.prgit.lock` with libc::flock LOCK_EX|LOCK_NB)
- [x] 3.9 Store lock file handle in returned struct (keeps lock held)
- [x] 3.10 Return ShelveClient::new() (lock auto-releases when struct is dropped)

## 4. Remove old pool code

- [x] 4.1 Remove `ClientPool` struct
- [x] 4.2 Remove `ClientLease` struct
- [x] 4.3 Remove `ClientLeaseType` enum
- [x] 4.4 Remove `ClientPoolError` variants that are no longer needed

## 5. Update Shelver

- [x] 5.1 Update `Shelver::new()` to not create ClientPool
- [x] 5.2 Update `Shelver::shelve()` to take `user_p4: &P4` parameter
- [x] 5.3 Update `Shelver::shelve()` to call `get_shelve_client(user_p4)`
- [x] 5.4 Remove pool field from Shelver struct

## 6. Update tests

- [x] 6.1 Remove ClientPool tests
- [x] 6.2 Add tests for `get_shelve_client()` function
- [x] 6.3 Add test for lock file - concurrent access returns ClientBusy error
- [x] 6.4 Add test for lock file cleanup on drop
- [x] 6.5 Update Shelver tests to pass user_p4
- [x] 6.6 Update any integration tests

## 7. Verification

- [x] 7.1 Run all shelf module tests
- [x] 7.2 Run full test suite
