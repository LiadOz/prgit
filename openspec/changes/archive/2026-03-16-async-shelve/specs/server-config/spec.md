## ADDED Requirements

### Requirement: Shelve configuration section
Each repo entry in the config file SHALL support an optional `shelve` section for shelve-related settings. When the `shelve` section is omitted, all shelve settings SHALL use their defaults.

#### Scenario: Config with shelve section
- **WHEN** a repo config includes `shelve: { async: true }`
- **THEN** the server SHALL enable async shelving for that repo

#### Scenario: Config without shelve section
- **WHEN** a repo config omits the `shelve` section entirely
- **THEN** the server SHALL use default shelve settings (async disabled)

### Requirement: Shelve async setting
The `shelve` section SHALL support an `async` boolean field that defaults to `false`. When `true`, the server SHALL use background shelving for pushes to that repo.

#### Scenario: Async shelve enabled
- **WHEN** a repo is configured with `shelve.async: true`
- **THEN** the push handler SHALL use the two-phase shelve flow, returning the changelist number immediately

#### Scenario: Async shelve disabled (default)
- **WHEN** a repo is configured with `shelve.async: false` or the field is omitted
- **THEN** the push handler SHALL use the synchronous shelve flow, waiting for the shelve to complete before responding
