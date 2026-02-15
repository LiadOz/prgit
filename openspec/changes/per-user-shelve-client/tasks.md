## 1. Update database schema

- [ ] 1.1 Remove `max_clients` and `timeout_secs` columns from `shelve_config` table schema
- [ ] 1.2 Remove `shelve_clients` table schema entirely
- [ ] 1.3 Update `Database::create_shelve_config()` to only take `clients_root`
- [ ] 1.4 Update `ShelveConfig` struct to only have `prgit_client_id` and `clients_root`

## 2. Remove pool management from PrgitClient

- [ ] 2.1 Remove `get_available_shelve_client()` method
- [ ] 2.2 Remove `get_timed_out_shelve_client()` method
- [ ] 2.3 Remove `count_shelve_clients()` method
- [ ] 2.4 Remove `acquire_shelve_client()` method
- [ ] 2.5 Remove `release_shelve_client()` method
- [ ] 2.6 Remove `register_shelve_client()` method

## 3. Replace ClientPool with simple function

- [ ] 3.1 Create `get_shelve_client(prgit_client, user_p4: &P4) -> Result<ShelveClient>` function
- [ ] 3.2 Extract user_id from user_p4 (p4 info or similar)
- [ ] 3.3 Implement client name derivation: `{base_client}-{user_id}-shelve`
- [ ] 3.4 Implement P4 client existence check
- [ ] 3.5 Implement P4 client creation (copy view from base client, Host field empty)
- [ ] 3.6 Implement local directory creation
- [ ] 3.7 Add `fs2` crate dependency for file locking
- [ ] 3.8 Implement flock-based locking (`{client_root}/.prgit.lock` with try_lock_exclusive)
- [ ] 3.9 Store lock file handle in returned struct (keeps lock held)
- [ ] 3.10 Return ShelveClient::new() (lock auto-releases when struct is dropped)

## 4. Remove old pool code

- [ ] 4.1 Remove `ClientPool` struct
- [ ] 4.2 Remove `ClientLease` struct
- [ ] 4.3 Remove `ClientLeaseType` enum
- [ ] 4.4 Remove `ClientPoolError` variants that are no longer needed

## 5. Update Shelver

- [ ] 5.1 Update `Shelver::new()` to not create ClientPool
- [ ] 5.2 Update `Shelver::shelve()` to take `user_p4: &P4` parameter
- [ ] 5.3 Update `Shelver::shelve()` to call `get_shelve_client(user_p4)`
- [ ] 5.4 Remove pool field from Shelver struct

## 6. Update tests

- [ ] 6.1 Remove ClientPool tests
- [ ] 6.2 Add tests for `get_shelve_client()` function
- [ ] 6.3 Add test for lock file - concurrent access returns ClientBusy error
- [ ] 6.4 Add test for lock file cleanup on drop
- [ ] 6.5 Update Shelver tests to pass user_p4
- [ ] 6.6 Update any integration tests

## 7. Verification

- [ ] 7.1 Run all shelf module tests
- [ ] 7.2 Run full test suite
