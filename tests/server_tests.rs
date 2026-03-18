use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Once, OnceLock};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::prelude::*;
use p4rs::testkit::SERVER;
use prgit::window::{RepoConfig, ServerConfig};
use tempfile::TempDir;
use test_log::test;
use tower::ServiceExt;

static SERVER_INIT: Once = Once::new();
static SECURITY_SETUP: Once = Once::new();
static ADMIN_TICKET: OnceLock<String> = OnceLock::new();

/// Ensure SERVER is initialized outside the tokio runtime.
/// The Docker testkit uses `block_on` internally which panics inside async tests.
fn ensure_server() {
    SERVER_INIT.call_once(|| {
        std::thread::spawn(|| {
            let _ = SERVER.port;
        })
        .join()
        .expect("Failed to initialize P4 test server");
    });
}

/// P4 port string for the test server.
fn p4port() -> String {
    ensure_server();
    format!("localhost:{}", SERVER.port)
}

/// Run a raw p4 command, returning (stdout, stderr, success).
fn p4_cmd(args: &[&str]) -> (String, String, bool) {
    let output = Command::new("p4")
        .args(["-p", &p4port()])
        .args(args)
        .output()
        .expect("Failed to run p4");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

/// Run a raw p4 command with stdin.
fn p4_cmd_stdin(args: &[&str], input: &[u8]) -> (String, String, bool) {
    let mut child = Command::new("p4")
        .args(["-p", &p4port()])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn p4");
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input);
    }
    let output = child.wait_with_output().expect("p4 command failed");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

/// Set up P4 security level 1 (admin password + ticket).
/// Only runs once per test binary.
fn setup_security() {
    SECURITY_SETUP.call_once(|| {
        let (out, err, ok) = p4_cmd(&["-u", "admin", "configure", "set", "security=1"]);
        log::info!("configure set security=1: ok={ok} out={out} err={err}");

        let (out, err, ok) = p4_cmd_stdin(
            &["-u", "admin", "passwd"],
            b"admin123\nadmin123\n",
        );
        log::info!("admin passwd: ok={ok} out={out} err={err}");

        let ticket = get_ticket("admin", "admin123");
        log::info!("admin ticket: {ticket}");
        ADMIN_TICKET.set(ticket).expect("admin ticket already set");
    });
}

/// Create a P4 user with a UUID-based name and return (username, ticket).
fn create_test_user() -> (String, String) {
    setup_security();
    let admin_ticket = ADMIN_TICKET.get().expect("admin ticket not set");

    let user = format!("u-{}", uuid::Uuid::new_v4());
    let pass = format!("P-{}!", uuid::Uuid::new_v4());

    let spec = format!("User: {user}\nEmail: {user}@test.com\nFullName: Test {user}\n");
    let (out, err, ok) = p4_cmd_stdin(
        &["-u", "admin", "-P", admin_ticket, "user", "-f", "-i"],
        spec.as_bytes(),
    );
    log::info!("create user {user}: ok={ok} out={out} err={err}");

    let passwd_input = format!("{pass}\n{pass}\n");
    let (out, err, ok) = p4_cmd_stdin(
        &["-u", "admin", "-P", admin_ticket, "passwd", &user],
        passwd_input.as_bytes(),
    );
    log::info!("passwd {user}: ok={ok} out={out} err={err}");

    let ticket = get_ticket(&user, &pass);
    (user, ticket)
}

/// Get a login ticket for a user. Panics with diagnostics on failure.
fn get_ticket(user: &str, password: &str) -> String {
    let (out, err, ok) = p4_cmd_stdin(
        &["-u", user, "login", "-p"],
        format!("{password}\n").as_bytes(),
    );
    assert!(
        ok,
        "p4 login -p failed for {user}: stdout={out} stderr={err}"
    );
    // `p4 login -p` output includes prompts; the ticket is the last line
    out.lines()
        .last()
        .expect("empty login output")
        .trim()
        .to_string()
}

struct TestServer {
    data_dir: TempDir,
    config: ServerConfig,
}

impl TestServer {
    fn new() -> Self {
        let data_dir = TempDir::new().expect("Failed to create temp dir");
        let config = ServerConfig {
            listen: "127.0.0.1:0".to_string(),
            data_dir: data_dir.path().to_path_buf(),
            repos: vec![RepoConfig {
                group: "depot".to_string(),
                name: "main".to_string(),
                p4port: p4port(),
                p4client: format!("prgit-test-{}", uuid::Uuid::new_v4()),
                synced_branch: "main".to_string(),
                mirror_interval_secs: 3600,
                max_changes: 100,
                shelve: None,
            }],
        };
        Self { data_dir, config }
    }

    fn with_security() -> Self {
        setup_security();
        Self::new()
    }

    fn app(&self) -> axum::Router {
        prgit::window::build_app(&self.config).expect("Failed to build app")
    }

    fn repo_url_prefix(&self) -> String {
        format!(
            "/{}/{}.git",
            self.config.repos[0].group, self.config.repos[0].name
        )
    }

    fn basic_auth_header(user: &str, pass: &str) -> String {
        format!(
            "Basic {}",
            BASE64_STANDARD.encode(format!("{user}:{pass}"))
        )
    }
}

// ============================================================
// Health endpoint
// ============================================================

#[test(tokio::test)]
async fn test_health_endpoint() {
    let server = TestServer::new();
    let app = server.app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================
// Repo resolution / 404
// ============================================================

#[test(tokio::test)]
async fn test_unknown_repo_returns_404() {
    let server = TestServer::new();
    let app = server.app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/no/such.git/info/refs?service=git-upload-pack")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test(tokio::test)]
async fn test_no_git_suffix_returns_404() {
    let server = TestServer::new();
    let app = server.app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/some/random/path")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================
// Git info/refs (anonymous read)
// ============================================================

#[test(tokio::test)]
async fn test_info_refs_returns_success() {
    let server = TestServer::new();
    let app = server.app();
    let uri = format!(
        "{}/info/refs?service=git-upload-pack",
        server.repo_url_prefix()
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri(&uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    // Even an empty repo should return 200 from git-http-backend
    assert!(
        response.status().is_success() || response.status() == StatusCode::FORBIDDEN,
        "Expected success or 403 for empty repo, got {}",
        response.status()
    );
}

// ============================================================
// Push authentication (git-receive-pack)
// ============================================================

#[test(tokio::test)]
async fn test_receive_pack_post_without_auth_returns_401() {
    let server = TestServer::new();
    let app = server.app();
    // POST to git-receive-pack (not info/refs) triggers auth check
    let uri = format!("{}/git-receive-pack", server.repo_url_prefix());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .header("content-type", "application/x-git-receive-pack-request")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(
        response.headers().get("www-authenticate").is_some(),
        "Expected WWW-Authenticate header"
    );
}

#[test(tokio::test)]
async fn test_receive_pack_with_bad_credentials_returns_401() {
    let server = TestServer::with_security();
    let app = server.app();
    let uri = format!("{}/git-receive-pack", server.repo_url_prefix());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .header(
                    "authorization",
                    TestServer::basic_auth_header("nobody", "badpass"),
                )
                .header("content-type", "application/x-git-receive-pack-request")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================
// Synced branch protection
// ============================================================

#[test(tokio::test)]
async fn test_push_to_synced_branch_rejected() {
    let server = TestServer::with_security();
    let (user, ticket) = create_test_user();
    let app = server.app();

    // Construct a pkt-line payload that targets refs/heads/main (the synced branch)
    let ref_update = format!(
        "0000000000000000000000000000000000000000 {} refs/heads/main",
        "a".repeat(40)
    );
    let pkt_line = format!("{:04x}{}\n", ref_update.len() + 5, ref_update);

    let uri = format!("{}/git-receive-pack", server.repo_url_prefix());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .header(
                    "authorization",
                    TestServer::basic_auth_header(&user, &ticket),
                )
                .header("content-type", "application/x-git-receive-pack-request")
                .body(Body::from(pkt_line))
                .expect("request"),
        )
        .await
        .expect("response");
    // Now returns 200 with a git protocol error packet instead of 403
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let body_str = String::from_utf8_lossy(&body);
    assert!(
        body_str.contains("Push to synced branch"),
        "Expected git error message about synced branch, got: {body_str}"
    );
}

#[test(tokio::test)]
async fn test_push_to_feature_branch_not_rejected_as_forbidden() {
    let server = TestServer::with_security();
    let (user, ticket) = create_test_user();
    let app = server.app();

    // Construct a pkt-line payload targeting a feature branch
    let ref_update = format!(
        "0000000000000000000000000000000000000000 {} refs/heads/feature-x",
        "b".repeat(40)
    );
    let pkt_line = format!("{:04x}{}\n", ref_update.len() + 5, ref_update);

    let uri = format!("{}/git-receive-pack", server.repo_url_prefix());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .header(
                    "authorization",
                    TestServer::basic_auth_header(&user, &ticket),
                )
                .header("content-type", "application/x-git-receive-pack-request")
                .body(Body::from(pkt_line))
                .expect("request"),
        )
        .await
        .expect("response");
    // Should not contain synced-branch rejection message
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let body_str = String::from_utf8_lossy(&body);
    assert!(
        !body_str.contains("Push to synced branch"),
        "Push to feature branch should not be rejected as synced branch"
    );
}

// ============================================================
// Multi-user: different users get independent auth
// ============================================================

#[test(tokio::test)]
async fn test_multiple_users_auth_independently() {
    let server = TestServer::with_security();
    let (user_a, ticket_a) = create_test_user();
    let (user_b, ticket_b) = create_test_user();
    let uri = format!("{}/git-receive-pack", server.repo_url_prefix());

    // User A authenticates
    let ref_update = format!(
        "0000000000000000000000000000000000000000 {} refs/heads/branch-a",
        "c".repeat(40)
    );
    let pkt_line = format!("{:04x}{}\n", ref_update.len() + 5, ref_update);

    let app = server.app();
    let resp_a = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .header(
                    "authorization",
                    TestServer::basic_auth_header(&user_a, &ticket_a),
                )
                .header("content-type", "application/x-git-receive-pack-request")
                .body(Body::from(pkt_line.clone()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_ne!(
        resp_a.status(),
        StatusCode::UNAUTHORIZED,
        "User A should authenticate successfully"
    );

    // User B authenticates
    let ref_update_b = format!(
        "0000000000000000000000000000000000000000 {} refs/heads/branch-b",
        "d".repeat(40)
    );
    let pkt_line_b = format!("{:04x}{}\n", ref_update_b.len() + 5, ref_update_b);

    let app = server.app();
    let resp_b = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .header(
                    "authorization",
                    TestServer::basic_auth_header(&user_b, &ticket_b),
                )
                .header("content-type", "application/x-git-receive-pack-request")
                .body(Body::from(pkt_line_b))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_ne!(
        resp_b.status(),
        StatusCode::UNAUTHORIZED,
        "User B should authenticate successfully"
    );

    // Cross-check: A's ticket doesn't work for B
    let app = server.app();
    let cross_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .header(
                    "authorization",
                    TestServer::basic_auth_header(&user_b, &ticket_a),
                )
                .header("content-type", "application/x-git-receive-pack-request")
                .body(Body::from(pkt_line))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(
        cross_resp.status(),
        StatusCode::UNAUTHORIZED,
        "User A's ticket should not work for User B"
    );
}

// ============================================================
// Full git clone test (real HTTP server)
// ============================================================

#[test(tokio::test)]
async fn test_git_clone_via_server() {
    let server = TestServer::new();

    // build_app creates the bare repo — call it first
    let app = server.app();

    let bare_repo_path = server
        .data_dir
        .path()
        .join("repos")
        .join(&server.config.repos[0].group)
        .join(format!("{}.git", server.config.repos[0].name));

    // Seed the bare repo with a commit via a temp working copy
    {
        let work_dir = TempDir::new().expect("work dir");
        let repo = git2::Repository::init(work_dir.path()).expect("init");

        // Add origin pointing at bare repo
        repo.remote("origin", bare_repo_path.to_str().expect("path"))
            .expect("remote");

        std::fs::write(work_dir.path().join("hello.txt"), "world").expect("write");
        let mut index = repo.index().expect("index");
        index
            .add_path(std::path::Path::new("hello.txt"))
            .expect("add");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let sig = git2::Signature::now("test", "test@test.com").expect("sig");
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .expect("commit");

        let mut remote = repo.find_remote("origin").expect("find remote");
        remote
            .push(&["refs/heads/master:refs/heads/main"], None)
            .expect("push to bare");
    }

    // Point bare repo HEAD to main
    let bare_repo = git2::Repository::open_bare(&bare_repo_path).expect("open bare");
    bare_repo
        .set_head("refs/heads/main")
        .expect("set HEAD to main");

    // Start real HTTP server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");

    let shutdown = tokio_util::sync::CancellationToken::new();
    let shutdown_signal = shutdown.clone();
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move { shutdown_signal.cancelled().await })
            .await
            .expect("serve");
    });

    // Clone via the HTTP server (use tokio Command to avoid blocking the runtime)
    let clone_dest = TempDir::new().expect("clone dest");
    let url = format!(
        "http://{}/{}/{}.git",
        addr, server.config.repos[0].group, server.config.repos[0].name
    );
    let output = tokio::process::Command::new("git")
        .args(["clone", &url, clone_dest.path().to_str().expect("path")])
        .output()
        .await
        .expect("git clone");

    shutdown.cancel();
    let _ = server_handle.await;

    assert!(
        output.status.success(),
        "git clone failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the cloned content
    let cloned_file = clone_dest.path().join("hello.txt");
    assert!(cloned_file.exists(), "hello.txt should exist in clone");
    assert_eq!(
        std::fs::read_to_string(&cloned_file).expect("read"),
        "world"
    );
}

// ============================================================
// Shelve status endpoint
// ============================================================

#[test(tokio::test)]
async fn test_shelve_status_unknown_branch_returns_404() {
    let server = TestServer::new();
    let app = server.app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/repos/depot/main/shelve/status/nonexistent-branch")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test(tokio::test)]
async fn test_shelve_status_nonexistent_repo_returns_404() {
    let server = TestServer::new();
    let app = server.app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/repos/depot/nonexistent/shelve/status/feature")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
