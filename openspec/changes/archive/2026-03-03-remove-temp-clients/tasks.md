## 1. Add PoolExhausted error

- [ ] 1.1 Add `PoolExhausted` variant to `ClientPoolError` enum
- [ ] 1.2 Add descriptive error message for the new variant

## 2. Remove temporary client logic

- [ ] 2.1 Remove `ClientLeaseType` enum entirely
- [ ] 2.2 Remove `lease_type` field from `ClientLease`
- [ ] 2.3 Remove `ClientLease::new_temporary()` constructor
- [ ] 2.4 Remove `create_temporary_client()` method from `ClientPool`
- [ ] 2.5 Simplify `Drop` impl for `ClientLease` (always release to pool)

## 3. Update acquire() to fail fast

- [ ] 3.1 Replace `create_temporary_client()` call with `Err(ClientPoolError::PoolExhausted)`

## 4. Update tests

- [ ] 4.1 Remove `test_acquire_creates_temporary_when_max_reached`
- [ ] 4.2 Remove `test_temporary_client_deleted_on_drop`
- [ ] 4.3 Add `test_acquire_returns_error_when_pool_exhausted`

## 5. Verification

- [ ] 5.1 Run client_pool tests
- [ ] 5.2 Run full test suite
