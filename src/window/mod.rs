mod handlers;
mod mirror_task;
pub(crate) mod observability;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::routing::{get, post};
use axum::Router;
use git2::Repository;
use serde::{Deserialize, Serialize};

use crate::cabinet::Database;
use crate::mirror::IntegrateStrategy;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum ShelveState {
    Queued,
    Shelving,
    Done { changelist: usize, client: String },
    Failed { error: String },
}

#[derive(Clone, Default)]
pub(crate) struct ActiveShelves {
    inner: Arc<Mutex<HashMap<String, ShelveState>>>,
}

impl ActiveShelves {
    pub fn set_queued(&self, key: &str) {
        self.inner
            .lock()
            .expect("shelve state lock poisoned")
            .insert(key.to_string(), ShelveState::Queued);
    }

    pub fn set_shelving(&self, key: &str) {
        self.inner
            .lock()
            .expect("shelve state lock poisoned")
            .insert(key.to_string(), ShelveState::Shelving);
    }

    pub fn set_done(&self, key: &str, changelist: usize, client: String) {
        self.inner
            .lock()
            .expect("shelve state lock poisoned")
            .insert(key.to_string(), ShelveState::Done { changelist, client });
    }

    pub fn set_failed(&self, key: &str, error: String) {
        self.inner
            .lock()
            .expect("shelve state lock poisoned")
            .insert(key.to_string(), ShelveState::Failed { error });
    }

    pub fn get(&self, key: &str) -> Option<ShelveState> {
        self.inner
            .lock()
            .expect("shelve state lock poisoned")
            .get(key)
            .cloned()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WindowError {
    #[error("git-http-backend not found at {0}")]
    BackendNotFound(PathBuf),
    #[error("{context}: {source}")]
    Io {
        context: String,
        source: std::io::Error,
    },
    #[error("{context}: {source}")]
    Git {
        context: String,
        source: git2::Error,
    },
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("{0}")]
    Other(String),
}

impl WindowError {
    fn io(context: impl Into<String>) -> impl FnOnce(std::io::Error) -> Self {
        let context = context.into();
        move |source| Self::Io { context, source }
    }

    fn git(context: impl Into<String>) -> impl FnOnce(git2::Error) -> Self {
        let context = context.into();
        move |source| Self::Git { context, source }
    }
}

type Result<T> = std::result::Result<T, WindowError>;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub listen: String,
    pub data_dir: PathBuf,
    pub repos: Vec<RepoConfig>,
    #[serde(default)]
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ObservabilityConfig {
    #[serde(default = "ObservabilityConfig::default_channel_capacity")]
    pub channel_capacity: usize,
    #[serde(default = "ObservabilityConfig::default_retention_days")]
    pub retention_days: u32,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 4096,
            retention_days: 30,
        }
    }
}

impl ObservabilityConfig {
    fn default_channel_capacity() -> usize {
        4096
    }
    fn default_retention_days() -> u32 {
        30
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ShelveSettings {
    #[serde(default, rename = "async")]
    pub r#async: bool,
    #[serde(default)]
    pub description: crate::shelf::ShelveDescriptionMode,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoConfig {
    pub group: String,
    pub name: String,
    pub p4port: String,
    pub p4client: String,
    pub synced_branch: String,
    pub mirror_interval_secs: u64,
    pub max_changes: usize,
    #[serde(default)]
    pub shelve: Option<ShelveSettings>,
}

impl RepoConfig {
    pub fn shelve_async(&self) -> bool {
        self.shelve.as_ref().is_some_and(|s| s.r#async)
    }

    pub fn shelve_description_mode(&self) -> crate::shelf::ShelveDescriptionMode {
        self.shelve
            .as_ref()
            .map(|s| s.description)
            .unwrap_or_default()
    }

    fn url_path(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }

    fn bare_repo_path(&self, data_dir: &std::path::Path) -> PathBuf {
        data_dir
            .join("repos")
            .join(&self.group)
            .join(format!("{}.git", self.name))
    }
}

pub(crate) struct RepoEntry {
    pub config: RepoConfig,
    pub bare_repo_path: PathBuf,
    pub client_id: u64,
}

pub(crate) struct AppState {
    pub repos: HashMap<String, RepoEntry>,
    pub db_path: String,
    pub git_http_backend: PathBuf,
    pub active_shelves: ActiveShelves,
    pub emitter: observability::EventEmitter,
}

fn to_str(path: &std::path::Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| WindowError::Other(format!("Non-UTF8 path: {}", path.display())))
}

fn find_git_http_backend() -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["--exec-path"])
        .output()
        .map_err(WindowError::io("Failed to run git --exec-path"))?;
    let exec_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let backend = PathBuf::from(&exec_path).join("git-http-backend");
    if !backend.exists() {
        return Err(WindowError::BackendNotFound(backend));
    }
    log::info!("Found git-http-backend at {}", backend.display());
    Ok(backend)
}

fn init_repos(config: &ServerConfig, db: &Database) -> Result<HashMap<String, RepoEntry>> {
    let mut repos = HashMap::new();
    for repo_config in &config.repos {
        let bare_path = repo_config.bare_repo_path(&config.data_dir);

        let repo = if bare_path.exists() {
            let repo = Repository::open_bare(&bare_path).map_err(WindowError::git(format!(
                "Failed to open repo at {}",
                bare_path.display()
            )))?;
            log::info!("Opened existing repo: {}", bare_path.display());
            repo
        } else {
            let parent = bare_path.parent().ok_or_else(|| {
                WindowError::Other(format!("Invalid bare repo path: {}", bare_path.display()))
            })?;
            std::fs::create_dir_all(parent)
                .map_err(WindowError::io("Failed to create repo directory"))?;
            let repo = Repository::init_bare(&bare_path).map_err(WindowError::git(format!(
                "Failed to init bare repo at {}",
                bare_path.display()
            )))?;
            repo.config()
                .map_err(WindowError::git("Failed to open repo config"))?
                .set_bool("http.receivepack", true)
                .map_err(WindowError::git("Failed to set http.receivepack"))?;
            log::info!("Created bare repo: {}", bare_path.display());
            repo
        };

        // Ensure HEAD points to the synced branch so mirror commits land there
        let synced_head = format!("refs/heads/{}", repo_config.synced_branch);
        repo.set_head(&synced_head)
            .map_err(WindowError::git("Failed to set HEAD to synced branch"))?;

        let client_id = match db.client_by_name(&repo_config.p4client)? {
            Some(client) => {
                log::info!(
                    "Using existing client '{}' (id={})",
                    repo_config.p4client,
                    client.client_id
                );
                client.client_id
            }
            None => {
                let id =
                    db.create_prgit_client(&repo_config.p4client, "p4", &repo_config.p4port, "")?;
                db.create_prgit_repo(
                    id,
                    to_str(&bare_path)?,
                    &repo_config.synced_branch,
                    IntegrateStrategy::MergeOurs,
                    Some(repo_config.max_changes),
                )?;
                let shelve_root = config.data_dir.join("shelve_clients");
                std::fs::create_dir_all(&shelve_root).ok();
                db.create_shelve_config(id, to_str(&shelve_root)?)?;
                log::info!("Created client '{}' (id={id})", repo_config.p4client);
                id
            }
        };

        repos.insert(
            repo_config.url_path(),
            RepoEntry {
                config: repo_config.clone(),
                bare_repo_path: bare_path,
                client_id,
            },
        );
    }
    Ok(repos)
}

pub fn build_app(config: &ServerConfig) -> Result<(Router, observability::EventEmitter)> {
    let git_http_backend = find_git_http_backend()?;

    std::fs::create_dir_all(&config.data_dir).map_err(WindowError::io(format!(
        "Failed to create data_dir {}",
        config.data_dir.display()
    )))?;

    let db_path = config.data_dir.join("prgit.db");
    let db_path_str = to_str(&db_path)?;
    let db = Database::open(db_path_str)?;

    let repos = init_repos(config, &db)?;

    let (tx, rx) = tokio::sync::mpsc::channel(config.observability.channel_capacity);
    let emitter = observability::EventEmitter::new(tx);
    observability::spawn_collector(
        rx,
        db_path_str.to_string(),
        config.observability.retention_days,
    );

    let emitter_clone = emitter.clone();
    let state = Arc::new(AppState {
        repos,
        db_path: db_path_str.to_string(),
        git_http_backend,
        active_shelves: ActiveShelves::default(),
        emitter,
    });

    let router = Router::new()
        .route("/api/health", get(handlers::health))
        .route("/api/v1/events", get(handlers::query_events))
        .route("/api/v1/events/counts", get(handlers::query_event_counts))
        .route("/api/v1/events/users", get(handlers::query_active_users))
        .route(
            "/api/v1/repos/{group}/{name}/shelve/status/{branch}",
            get(handlers::shelve_status),
        )
        .route(
            "/api/v1/repos/{group}/{name}/shelve/cl-alias",
            post(handlers::create_cl_alias),
        )
        .fallback(handlers::handle_git_request)
        .with_state(state);

    Ok((router, emitter_clone))
}

pub fn spawn_mirror_tasks(config: &ServerConfig, emitter: &observability::EventEmitter) {
    mirror_task::spawn_all(config, emitter);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_without_shelve_section() {
        let yaml = r#"
listen: "127.0.0.1:8080"
data_dir: "/tmp/test"
repos:
  - group: depot
    name: main
    p4port: "localhost:1666"
    p4client: test
    synced_branch: master
    mirror_interval_secs: 30
    max_changes: 100
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.repos[0].shelve_async());
        assert!(config.repos[0].shelve.is_none());
    }

    #[test]
    fn test_config_with_shelve_async_true() {
        let yaml = r#"
listen: "127.0.0.1:8080"
data_dir: "/tmp/test"
repos:
  - group: depot
    name: main
    p4port: "localhost:1666"
    p4client: test
    synced_branch: master
    mirror_interval_secs: 30
    max_changes: 100
    shelve:
      async: true
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.repos[0].shelve_async());
    }

    #[test]
    fn test_config_with_shelve_async_false() {
        let yaml = r#"
listen: "127.0.0.1:8080"
data_dir: "/tmp/test"
repos:
  - group: depot
    name: main
    p4port: "localhost:1666"
    p4client: test
    synced_branch: master
    mirror_interval_secs: 30
    max_changes: 100
    shelve:
      async: false
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.repos[0].shelve_async());
    }

    #[test]
    fn test_config_with_empty_shelve_section() {
        let yaml = r#"
listen: "127.0.0.1:8080"
data_dir: "/tmp/test"
repos:
  - group: depot
    name: main
    p4port: "localhost:1666"
    p4client: test
    synced_branch: master
    mirror_interval_secs: 30
    max_changes: 100
    shelve: {}
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.repos[0].shelve_async());
    }

    #[test]
    fn test_config_shelve_description_defaults_to_update() {
        let yaml = r#"
listen: "127.0.0.1:8080"
data_dir: "/tmp/test"
repos:
  - group: depot
    name: main
    p4port: "localhost:1666"
    p4client: test
    synced_branch: master
    mirror_interval_secs: 30
    max_changes: 100
    shelve:
      async: true
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.repos[0].shelve_description_mode(),
            crate::shelf::ShelveDescriptionMode::Update
        );
    }

    #[test]
    fn test_config_shelve_description_keep_original() {
        let yaml = r#"
listen: "127.0.0.1:8080"
data_dir: "/tmp/test"
repos:
  - group: depot
    name: main
    p4port: "localhost:1666"
    p4client: test
    synced_branch: master
    mirror_interval_secs: 30
    max_changes: 100
    shelve:
      description: keep_original
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.repos[0].shelve_description_mode(),
            crate::shelf::ShelveDescriptionMode::KeepOriginal
        );
    }

    #[test]
    fn test_config_shelve_description_update_explicit() {
        let yaml = r#"
listen: "127.0.0.1:8080"
data_dir: "/tmp/test"
repos:
  - group: depot
    name: main
    p4port: "localhost:1666"
    p4client: test
    synced_branch: master
    mirror_interval_secs: 30
    max_changes: 100
    shelve:
      description: update
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.repos[0].shelve_description_mode(),
            crate::shelf::ShelveDescriptionMode::Update
        );
    }

    #[test]
    fn test_config_no_shelve_section_defaults_description_to_update() {
        let yaml = r#"
listen: "127.0.0.1:8080"
data_dir: "/tmp/test"
repos:
  - group: depot
    name: main
    p4port: "localhost:1666"
    p4client: test
    synced_branch: master
    mirror_interval_secs: 30
    max_changes: 100
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.repos[0].shelve_description_mode(),
            crate::shelf::ShelveDescriptionMode::Update
        );
    }

    #[test]
    fn test_active_shelves_state_transitions() {
        let tracker = ActiveShelves::default();
        assert!(tracker.get("depot/main/feature").is_none());

        // queued → shelving → done
        tracker.set_queued("depot/main/feature");
        assert!(matches!(
            tracker.get("depot/main/feature"),
            Some(ShelveState::Queued)
        ));

        tracker.set_shelving("depot/main/feature");
        assert!(matches!(
            tracker.get("depot/main/feature"),
            Some(ShelveState::Shelving)
        ));

        tracker.set_done("depot/main/feature", 12345, "client-1".to_string());
        match tracker.get("depot/main/feature") {
            Some(ShelveState::Done { changelist, client }) => {
                assert_eq!(changelist, 12345);
                assert_eq!(client, "client-1");
            }
            other => panic!("Expected Done, got {other:?}"),
        }

        // queued → shelving → failed
        tracker.set_queued("depot/main/bugfix");
        tracker.set_shelving("depot/main/bugfix");
        tracker.set_failed("depot/main/bugfix", "P4 error".to_string());
        match tracker.get("depot/main/bugfix") {
            Some(ShelveState::Failed { error }) => assert_eq!(error, "P4 error"),
            other => panic!("Expected Failed, got {other:?}"),
        }
    }
}
