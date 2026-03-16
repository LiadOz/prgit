## ADDED Requirements

### Requirement: YAML configuration file
The server SHALL load its configuration from a YAML file specified via a `--config` command-line flag.

#### Scenario: Start with config file
- **WHEN** the server is started with `prgit-server --config /etc/prgit/config.yaml`
- **THEN** the server SHALL parse the YAML file and configure itself accordingly

#### Scenario: Missing config file
- **WHEN** the config file path does not exist
- **THEN** the server SHALL exit with a clear error message

#### Scenario: Invalid config
- **WHEN** the config file contains invalid YAML or missing required fields
- **THEN** the server SHALL exit with a clear error message indicating what is wrong

### Requirement: Server configuration fields
The config file SHALL support the following top-level fields: `listen` (bind address) and `data_dir` (directory for repos and database).

#### Scenario: Config with listen and data_dir
- **WHEN** the config specifies `listen: "0.0.0.0:3000"` and `data_dir: "/var/lib/prgit"`
- **THEN** the server SHALL bind to `0.0.0.0:3000` and store bare repos and the SQLite database under `/var/lib/prgit`

### Requirement: Repo configuration
The config file SHALL contain a `repos` list where each entry defines a repo with `group`, `name`, `p4port`, `p4client`, `synced_branch`, `mirror_interval_secs`, and `max_changes`.

#### Scenario: Repo creates bare git repo on startup
- **WHEN** a repo is configured and its bare repo does not yet exist on disk
- **THEN** the server SHALL create a bare git repo at `{data_dir}/repos/{group}/{name}.git`

#### Scenario: Repo uses existing bare repo
- **WHEN** a repo is configured and its bare repo already exists on disk
- **THEN** the server SHALL open the existing repo without reinitializing it

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

### Requirement: Database location
The SQLite database SHALL be stored at `{data_dir}/prgit.db`.

#### Scenario: Database created on first run
- **WHEN** the server starts and no database exists at `{data_dir}/prgit.db`
- **THEN** the server SHALL create the database and initialize all required tables
