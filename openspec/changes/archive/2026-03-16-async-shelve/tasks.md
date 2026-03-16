## 1. ShelveClient refactor

- [x] 1.1 Add `Clone, Copy` derives to `FileAction` in `shelve_client.rs`
- [x] 1.2 Extract `create_or_reuse_changelist()` method from `ShelveClient::run()`
- [x] 1.3 Extract `shelve_changelist()` method from `ShelveClient::run()`
- [x] 1.4 Rewrite `run()` to call `create_or_reuse_changelist()` then `shelve_changelist()`

## 2. Shelver prepare_shelve

- [x] 2.1 Add `PendingShelve` struct to `shelver.rs` owning `ShelveClientHandle`, `TempDir`, `Vec<ChangedFile>`, `base_change`, `changelist`
- [x] 2.2 Implement `PendingShelve::complete()` that calls `shelve_changelist()` and consumes self
- [x] 2.3 Add `Shelver::prepare_shelve()` method returning `(ShelveResult, PendingShelve)`
- [x] 2.4 Export `PendingShelve` from `shelf/mod.rs`

## 3. Server config

- [x] 3.1 Add `ShelveSettings` struct with `r#async: bool` field (serde default false) in `window/mod.rs`
- [x] 3.2 Add `shelve: Option<ShelveSettings>` to `RepoConfig` with `#[serde(default)]`
- [x] 3.3 Add `RepoConfig::shelve_async()` helper method returning the effective async setting

## 4. Handler async path

- [x] 4.1 Add `do_prepare_shelve()` function that calls `shelver.prepare_shelve()` per branch, returns `(HandlerShelveResult, Vec<(String, PendingShelve)>)`
- [x] 4.2 Add `complete_pending_shelves()` function that calls `complete()` on each pending shelve with error logging
- [x] 4.3 Modify `shelve_branches()` to branch on `config.shelve_async()`: sync path uses existing `do_shelve()`, async path uses `do_prepare_shelve()` + fire-and-forget `spawn_blocking`
- [x] 4.4 Update sideband message format for async mode: "Shelving branch '{branch}' as CL {cl} on client '{client}' (in background)"

## 5. Verification

- [x] 5.1 Verify existing shelve tests still pass (`cargo test`)
- [x] 5.2 Verify config parsing works with and without the `shelve` section
