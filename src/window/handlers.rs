use std::io::Write;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::prelude::*;
use chrono::Utc;
use p4rs::P4;
use serde::{Deserialize, Serialize};

use crate::cabinet::Database;
use crate::shelf::Shelver;

use super::observability::{EventEmitter, ObservabilityEvent};
use super::{AppState, RepoEntry};

pub async fn health() -> StatusCode {
    StatusCode::OK
}

pub async fn shelve_status(
    State(state): State<Arc<AppState>>,
    Path((group, name, branch)): Path<(String, String, String)>,
) -> Response {
    let repo_key = format!("{group}/{name}");
    let repo_entry = match state.repos.get(&repo_key) {
        Some(entry) => entry,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let tracker_key = format!("{group}/{name}/{branch}");
    if let Some(shelve_state) = state.active_shelves.get(&tracker_key) {
        return Json(shelve_state).into_response();
    }

    // Fall back to the database for previously-completed shelves
    let db = match Database::open(&state.db_path) {
        Ok(db) => db,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let client = match db.client(repo_entry.client_id) {
        Ok(Some(c)) => c,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };

    match client.get_shelved_change_for_branch(&branch) {
        Some(changelist) => {
            let shelver = client
                .get_shelver_for_change(changelist)
                .unwrap_or_default();
            Json(super::ShelveState::Done {
                changelist,
                client: shelver,
            })
            .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
pub struct ClAliasRequest {
    shelved_cl: usize,
    alias_cl: usize,
}

#[derive(Serialize)]
pub struct ClAliasResponse {
    shelved_cl: usize,
    alias_cl: usize,
}

pub async fn create_cl_alias(
    State(state): State<Arc<AppState>>,
    Path((group, name)): Path<(String, String)>,
    req: Request<Body>,
) -> Response {
    let repo_key = format!("{group}/{name}");
    let repo_entry = match state.repos.get(&repo_key) {
        Some(entry) => entry,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let cl_repo = format!("{group}/{name}");
    let auth_user = match authenticate_push(
        req.headers(),
        &repo_entry.config.p4port,
        &state.emitter,
        &cl_repo,
    )
    .await
    {
        Ok(au) => au,
        Err(resp) => return resp,
    };

    let body_bytes = match axum::body::to_bytes(req.into_body(), 1024).await {
        Ok(b) => b,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let alias_req: ClAliasRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let db_path = state.db_path.clone();
    let client_id = repo_entry.client_id;
    let username = auth_user.username;

    let result = tokio::task::spawn_blocking(move || {
        let db = Database::open(&db_path).map_err(|e| format!("Database error: {e}"))?;
        let client = db
            .client(client_id)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| format!("Client id={client_id} not found"))?;

        let shelver = client
            .get_shelver_for_change(alias_req.shelved_cl)
            .ok_or_else(|| format!("Shelved CL {} not found", alias_req.shelved_cl))?;

        if shelver != username {
            return Err(format!(
                "Only the shelver ({shelver}) can create aliases for CL {}",
                alias_req.shelved_cl
            ));
        }

        client.set_cl_alias(alias_req.alias_cl, alias_req.shelved_cl);
        Ok(alias_req)
    })
    .await;

    match result {
        Ok(Ok(req)) => (
            StatusCode::CREATED,
            Json(ClAliasResponse {
                shelved_cl: req.shelved_cl,
                alias_cl: req.alias_cl,
            }),
        )
            .into_response(),
        Ok(Err(err)) => {
            if err.contains("not found") {
                (StatusCode::NOT_FOUND, err).into_response()
            } else if err.contains("Only the shelver") {
                (StatusCode::FORBIDDEN, err).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, err).into_response()
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Task panicked: {e}"),
        )
            .into_response(),
    }
}

pub async fn handle_git_request(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    let request_start = Instant::now();
    let path = req.uri().path().to_string();
    let git_idx = match path.find(".git/") {
        Some(idx) => idx,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let repo_key = path[1..git_idx + 4]
        .strip_suffix(".git")
        .unwrap_or(&path[1..git_idx + 4]);
    let git_path = &path[git_idx + 4..];

    let repo_entry = match state.repos.get(repo_key) {
        Some(entry) => entry,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let repo_name = format!("{}/{}", repo_entry.config.group, repo_entry.config.name);
    let method = req.method().clone();
    let is_receive_pack = git_path.contains("git-receive-pack");
    let git_service = if is_receive_pack {
        "receive-pack"
    } else {
        "upload-pack"
    };
    let query = req.uri().query().unwrap_or("").to_string();
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let auth_user = if is_receive_pack {
        match authenticate_push(
            req.headers(),
            &repo_entry.config.p4port,
            &state.emitter,
            &repo_name,
        )
        .await
        {
            Ok(au) => Some(au),
            Err(resp) => return resp,
        }
    } else {
        None
    };

    let body_bytes = match axum::body::to_bytes(req.into_body(), 512 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let request_bytes = body_bytes.len();

    let ref_updates = if is_receive_pack && method == Method::POST {
        let refs = parse_ref_updates(&body_bytes);
        let synced_ref = format!("refs/heads/{}", repo_entry.config.synced_branch);

        // Emit push events
        let username = auth_user
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_default();
        state.emitter.try_emit(ObservabilityEvent::PushReceived {
            timestamp: Utc::now(),
            user: username.clone(),
            repo: repo_name.clone(),
            payload_bytes: request_bytes,
            ref_count: refs.len(),
        });
        emit_push_events(&state.emitter, &username, &repo_name, &refs);

        if refs.iter().any(|(_, _, r)| r == &synced_ref) {
            let msg = format!(
                "Push to synced branch '{}' is not allowed",
                repo_entry.config.synced_branch
            );
            state.emitter.try_emit(ObservabilityEvent::PushRejected {
                timestamp: Utc::now(),
                user: Some(username),
                repo: repo_name.clone(),
                branch: Some(repo_entry.config.synced_branch.clone()),
                reason: "synced_branch".into(),
            });
            return git_error_response(&msg);
        }
        refs
    } else {
        Vec::new()
    };

    let repo_parent = match repo_entry.bare_repo_path.parent() {
        Some(p) => p,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let repo_dir_name = match repo_entry
        .bare_repo_path
        .file_name()
        .and_then(|f| f.to_str())
    {
        Some(n) => n,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let path_info = format!("/{repo_dir_name}{git_path}");

    let cgi_output = match spawn_cgi(
        &state.git_http_backend,
        repo_parent,
        &path_info,
        method.as_str(),
        &query,
        &content_type,
        &body_bytes,
    ) {
        Ok(output) => output,
        Err(e) => {
            log::error!("CGI proxy failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let (cgi_status, cgi_headers, cgi_body) = parse_cgi_output(&cgi_output);
    let response_bytes = cgi_output.len();

    if is_receive_pack && cgi_status.is_success() {
        if let Some(auth_user) = auth_user {
            let result = shelve_branches(&state, repo_entry, &ref_updates, auth_user).await;
            if !result.errors.is_empty() {
                let msg = format!(
                    "Push succeeded but shelving failed:\n{}",
                    result.errors.join("\n")
                );
                log::error!("{msg}");
                return git_error_response(&msg);
            }
            if !result.messages.is_empty() {
                let body = inject_sideband_messages(cgi_body, &result.messages.join("\n"));
                // Emit request.completed
                state
                    .emitter
                    .try_emit(ObservabilityEvent::RequestCompleted {
                        timestamp: Utc::now(),
                        repo: repo_name,
                        method: method.to_string(),
                        git_service: git_service.into(),
                        request_bytes,
                        response_bytes,
                        user: result.username,
                        duration_ms: request_start.elapsed().as_millis() as u64,
                    });
                return build_response(cgi_status, &cgi_headers, body);
            }
        }
    }

    // Emit request.completed for all non-early-return paths
    state
        .emitter
        .try_emit(ObservabilityEvent::RequestCompleted {
            timestamp: Utc::now(),
            repo: repo_name,
            method: method.to_string(),
            git_service: git_service.into(),
            request_bytes,
            response_bytes,
            user: None,
            duration_ms: request_start.elapsed().as_millis() as u64,
        });

    build_response(cgi_status, &cgi_headers, cgi_body.to_vec())
}

fn emit_push_events(
    emitter: &EventEmitter,
    user: &str,
    repo: &str,
    refs: &[(String, String, String)],
) {
    let zero_sha = "0000000000000000000000000000000000000000";
    let now = Utc::now();
    let mut ref_count = 0;

    for (old_sha, new_sha, refname) in refs {
        let branch = refname.strip_prefix("refs/heads/").unwrap_or(refname);
        if old_sha == zero_sha {
            emitter.try_emit(ObservabilityEvent::PushBranchCreated {
                timestamp: now,
                user: user.into(),
                repo: repo.into(),
                branch: branch.into(),
            });
        } else if new_sha == zero_sha {
            emitter.try_emit(ObservabilityEvent::PushBranchDeleted {
                timestamp: now,
                user: user.into(),
                repo: repo.into(),
                branch: branch.into(),
            });
        } else {
            emitter.try_emit(ObservabilityEvent::PushBranchUpdated {
                timestamp: now,
                user: user.into(),
                repo: repo.into(),
                branch: branch.into(),
            });
        }
        ref_count += 1;
    }

    // push.received is not emitted here — we need payload_bytes which we don't have in this function.
    // It's emitted inline in handle_git_request.
    let _ = ref_count; // used below
}

fn extract_basic_auth(headers: &HeaderMap) -> Option<(String, String)> {
    let header = headers.get("authorization")?.to_str().ok()?;
    let encoded = header.strip_prefix("Basic ")?;
    let decoded = String::from_utf8(BASE64_STANDARD.decode(encoded).ok()?).ok()?;
    let (user, pass) = decoded.split_once(':')?;
    Some((user.to_string(), pass.to_string()))
}

fn validate_p4_ticket(p4port: &str, user: &str, ticket: &str) -> bool {
    P4::new()
        .port(p4port)
        .p4_user(user)
        .password(ticket)
        .login()
        .status()
        .is_ok()
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [("WWW-Authenticate", "Basic realm=\"prgit\"")],
    )
        .into_response()
}

pub(super) struct AuthenticatedUser {
    pub p4: P4,
    pub username: String,
}

async fn authenticate_push(
    headers: &HeaderMap,
    p4port: &str,
    emitter: &EventEmitter,
    repo: &str,
) -> std::result::Result<AuthenticatedUser, Response> {
    let (user, ticket) = match extract_basic_auth(headers) {
        Some(creds) => creds,
        None => {
            emitter.try_emit(ObservabilityEvent::AuthFailed {
                timestamp: Utc::now(),
                user: None,
                repo: repo.into(),
                reason: "missing_credentials".into(),
            });
            return Err(unauthorized());
        }
    };
    let port = p4port.to_string();
    let u = user.clone();
    let t = ticket.clone();
    let valid = tokio::task::spawn_blocking(move || validate_p4_ticket(&port, &u, &t))
        .await
        .unwrap_or(false);
    if !valid {
        emitter.try_emit(ObservabilityEvent::AuthFailed {
            timestamp: Utc::now(),
            user: Some(user.clone()),
            repo: repo.into(),
            reason: "invalid_ticket".into(),
        });
        return Err((
            StatusCode::UNAUTHORIZED,
            [("WWW-Authenticate", "Basic realm=\"prgit\"")],
            "Invalid P4 credentials",
        )
            .into_response());
    }
    Ok(AuthenticatedUser {
        p4: P4::new().port(p4port).p4_user(&user).password(&ticket),
        username: user,
    })
}

fn parse_ref_updates(mut body: &[u8]) -> Vec<(String, String, String)> {
    use gix_packetline::{decode, PacketLineRef};
    let mut refs = Vec::new();
    loop {
        let Ok(decode::Stream::Complete {
            line,
            bytes_consumed,
        }) = decode::streaming(body)
        else {
            break;
        };
        let PacketLineRef::Data(payload) = line else {
            break;
        };
        let payload = payload.split(|&b| b == 0).next().unwrap_or(payload);
        if let Some((old, new, refname)) = std::str::from_utf8(payload).ok().and_then(|t| {
            let mut p = t.trim().splitn(3, ' ');
            Some((p.next()?, p.next()?, p.next()?))
        }) {
            refs.push((old.to_string(), new.to_string(), refname.to_string()));
        }
        body = &body[bytes_consumed..];
    }
    refs
}

fn spawn_cgi(
    backend: &std::path::Path,
    repo_parent: &std::path::Path,
    path_info: &str,
    method: &str,
    query: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<Vec<u8>> {
    let mut child = std::process::Command::new(backend)
        .env("GIT_PROJECT_ROOT", repo_parent)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("PATH_INFO", path_info)
        .env("REQUEST_METHOD", method)
        .env("QUERY_STRING", query)
        .env("CONTENT_TYPE", content_type)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(body);
    }

    let output = child.wait_with_output()?;

    if !output.stderr.is_empty() {
        log::debug!(
            "git-http-backend stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(output.stdout)
}

type CgiOutput<'a> = (StatusCode, Vec<(String, Vec<u8>)>, &'a [u8]);

fn parse_cgi_output(raw: &[u8]) -> CgiOutput<'_> {
    let mut headers_buf = [httparse::EMPTY_HEADER; 16];
    match httparse::parse_headers(raw, &mut headers_buf) {
        Ok(httparse::Status::Complete((body_offset, parsed_headers))) => {
            let mut status = StatusCode::OK;
            let mut headers = Vec::new();
            for h in parsed_headers {
                if h.name.eq_ignore_ascii_case("Status") {
                    status = std::str::from_utf8(h.value)
                        .ok()
                        .and_then(|v| v.split_whitespace().next())
                        .and_then(|c| c.parse::<u16>().ok())
                        .and_then(|c| StatusCode::from_u16(c).ok())
                        .unwrap_or(StatusCode::OK);
                } else {
                    headers.push((h.name.to_string(), h.value.to_vec()));
                }
            }
            (status, headers, &raw[body_offset..])
        }
        _ => (StatusCode::OK, Vec::new(), raw),
    }
}

fn build_response(status: StatusCode, headers: &[(String, Vec<u8>)], body: Vec<u8>) -> Response {
    let mut builder = Response::builder().status(status);
    for (name, value) in headers {
        builder = builder.header(name.as_str(), value.as_slice());
    }
    builder
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Build a git smart-HTTP error response that `git push` will display to the user.
fn git_error_response(message: &str) -> Response {
    let mut body = Vec::new();
    for line in message.lines() {
        let err_line = format!("ERR {line}");
        let pkt = format!("{:04x}\x03{err_line}", err_line.len() + 5);
        body.extend_from_slice(pkt.as_bytes());
    }
    body.extend_from_slice(b"0000");

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/x-git-receive-pack-result")
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Inject sideband progress messages (band=2) before the final flush packet in git protocol body.
fn inject_sideband_messages(body: &[u8], message: &str) -> Vec<u8> {
    let mut result = body.to_vec();
    let flush = b"0000";
    if let Some(pos) = result.windows(4).rposition(|w| w == flush) {
        let mut info = Vec::new();
        for line in message.lines() {
            let msg = format!("remote: {line}\n");
            let pkt = format!("{:04x}\x02{msg}", msg.len() + 5);
            info.extend_from_slice(pkt.as_bytes());
        }
        result.splice(pos..pos, info);
    }
    result
}

struct HandlerShelveResult {
    messages: Vec<String>,
    errors: Vec<String>,
    username: Option<String>,
}

async fn shelve_branches(
    state: &Arc<AppState>,
    repo_entry: &RepoEntry,
    ref_updates: &[(String, String, String)],
    auth_user: AuthenticatedUser,
) -> HandlerShelveResult {
    let zero_sha = "0000000000000000000000000000000000000000";
    let synced_ref = format!("refs/heads/{}", repo_entry.config.synced_branch);
    let branches: Vec<String> = ref_updates
        .iter()
        .filter(|(_, new_sha, refname)| new_sha != zero_sha && refname != &synced_ref)
        .filter_map(|(_, _, refname)| refname.strip_prefix("refs/heads/").map(String::from))
        .collect();

    if branches.is_empty() {
        return HandlerShelveResult {
            messages: Vec::new(),
            errors: Vec::new(),
            username: Some(auth_user.username),
        };
    }

    let db_path = state.db_path.clone();
    let client_id = repo_entry.client_id;
    let async_shelve = repo_entry.config.shelve_async();
    let username = auth_user.username;
    let user_p4 = auth_user.p4;
    let group = repo_entry.config.group.clone();
    let name = repo_entry.config.name.clone();
    let repo_name = format!("{group}/{name}");
    let emitter = state.emitter.clone();

    // Emit shelve.started for each branch
    for branch in &branches {
        emitter.try_emit(ObservabilityEvent::ShelveStarted {
            timestamp: Utc::now(),
            user: username.clone(),
            repo: repo_name.clone(),
            branch: branch.clone(),
            r#async: async_shelve,
        });
    }

    if async_shelve {
        let mut messages = Vec::new();
        for branch in &branches {
            let tracker_key = format!("{group}/{name}/{branch}");
            state.active_shelves.set_queued(&tracker_key);
            messages.push(format!("Shelving branch '{branch}' in background"));
        }

        let active = state.active_shelves.clone();
        let user_clone = username.clone();
        drop(tokio::task::spawn_blocking(move || {
            do_shelve_background(
                &db_path,
                client_id,
                &branches,
                &user_p4,
                &user_clone,
                &group,
                &name,
                &active,
                &emitter,
                async_shelve,
            );
        }));

        HandlerShelveResult {
            messages,
            errors: Vec::new(),
            username: Some(username),
        }
    } else {
        let user_clone = username.clone();
        let result = tokio::task::spawn_blocking(move || {
            do_shelve(
                &db_path,
                client_id,
                &branches,
                &user_p4,
                &user_clone,
                &emitter,
                &repo_name,
                async_shelve,
            )
        })
        .await;

        match result {
            Ok(mut r) => {
                r.username = Some(username);
                r
            }
            Err(e) => HandlerShelveResult {
                messages: Vec::new(),
                errors: vec![format!("Shelve task panicked: {e}")],
                username: Some(username),
            },
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn do_shelve(
    db_path: &str,
    client_id: u64,
    branches: &[String],
    user_p4: &P4,
    shelver_user: &str,
    emitter: &EventEmitter,
    repo: &str,
    is_async: bool,
) -> HandlerShelveResult {
    let mut messages = Vec::new();
    let mut errors = Vec::new();

    let db = match Database::open(db_path) {
        Ok(db) => db,
        Err(e) => {
            return HandlerShelveResult {
                messages,
                errors: vec![format!("Database error: {e}")],
                username: None,
            }
        }
    };
    let client = match db.client(client_id) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HandlerShelveResult {
                messages,
                errors: vec![format!("Client id={client_id} not found")],
                username: None,
            }
        }
        Err(e) => {
            return HandlerShelveResult {
                messages,
                errors: vec![format!("Database error: {e}")],
                username: None,
            }
        }
    };
    let shelver = match Shelver::new(&client) {
        Ok(s) => s,
        Err(e) => {
            return HandlerShelveResult {
                messages,
                errors: vec![format!("Shelver init error: {e}")],
                username: None,
            }
        }
    };

    for branch in branches {
        let start = Instant::now();
        match shelver.shelve(branch, user_p4, shelver_user) {
            Ok(result) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                log::info!(
                    "Shelved branch '{branch}' as CL {} on client '{}'",
                    result.changelist,
                    result.client_name
                );
                messages.push(format!(
                    "Shelved branch '{branch}' as CL {} on client '{}'",
                    result.changelist, result.client_name
                ));

                let event = if result.is_reshelve {
                    ObservabilityEvent::ShelveReshelved {
                        timestamp: Utc::now(),
                        user: shelver_user.into(),
                        repo: repo.into(),
                        branch: branch.clone(),
                        changelist: result.changelist,
                        client_name: result.client_name.clone(),
                        duration_ms,
                        file_count: result.file_count,
                        r#async: is_async,
                        commits_in_branch: result.commits_in_branch,
                    }
                } else {
                    ObservabilityEvent::ShelveCompleted {
                        timestamp: Utc::now(),
                        user: shelver_user.into(),
                        repo: repo.into(),
                        branch: branch.clone(),
                        changelist: result.changelist,
                        client_name: result.client_name.clone(),
                        duration_ms,
                        file_count: result.file_count,
                        r#async: is_async,
                        commits_in_branch: result.commits_in_branch,
                    }
                };
                emitter.try_emit(event);
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let msg = format!("Failed to shelve branch '{branch}': {e}");
                log::error!("{msg}");
                errors.push(msg);
                emitter.try_emit(ObservabilityEvent::ShelveFailed {
                    timestamp: Utc::now(),
                    user: shelver_user.into(),
                    repo: repo.into(),
                    branch: branch.clone(),
                    error: e.to_string(),
                    duration_ms,
                    r#async: is_async,
                });
            }
        }
    }
    HandlerShelveResult {
        messages,
        errors,
        username: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn do_shelve_background(
    db_path: &str,
    client_id: u64,
    branches: &[String],
    user_p4: &P4,
    shelver_user: &str,
    group: &str,
    name: &str,
    active: &super::ActiveShelves,
    emitter: &EventEmitter,
    is_async: bool,
) {
    let repo = format!("{group}/{name}");
    let db = match Database::open(db_path) {
        Ok(db) => db,
        Err(e) => {
            for branch in branches {
                let key = format!("{group}/{name}/{branch}");
                active.set_failed(&key, format!("Database error: {e}"));
                emitter.try_emit(ObservabilityEvent::ShelveFailed {
                    timestamp: Utc::now(),
                    user: shelver_user.into(),
                    repo: repo.clone(),
                    branch: branch.clone(),
                    error: format!("Database error: {e}"),
                    duration_ms: 0,
                    r#async: is_async,
                });
            }
            return;
        }
    };
    let client = match db.client(client_id) {
        Ok(Some(c)) => c,
        Ok(None) => {
            for branch in branches {
                let key = format!("{group}/{name}/{branch}");
                let err = format!("Client id={client_id} not found");
                active.set_failed(&key, err.clone());
                emitter.try_emit(ObservabilityEvent::ShelveFailed {
                    timestamp: Utc::now(),
                    user: shelver_user.into(),
                    repo: repo.clone(),
                    branch: branch.clone(),
                    error: err,
                    duration_ms: 0,
                    r#async: is_async,
                });
            }
            return;
        }
        Err(e) => {
            for branch in branches {
                let key = format!("{group}/{name}/{branch}");
                active.set_failed(&key, format!("Database error: {e}"));
                emitter.try_emit(ObservabilityEvent::ShelveFailed {
                    timestamp: Utc::now(),
                    user: shelver_user.into(),
                    repo: repo.clone(),
                    branch: branch.clone(),
                    error: format!("Database error: {e}"),
                    duration_ms: 0,
                    r#async: is_async,
                });
            }
            return;
        }
    };
    let shelver = match Shelver::new(&client) {
        Ok(s) => s,
        Err(e) => {
            for branch in branches {
                let key = format!("{group}/{name}/{branch}");
                active.set_failed(&key, format!("Shelver init error: {e}"));
                emitter.try_emit(ObservabilityEvent::ShelveFailed {
                    timestamp: Utc::now(),
                    user: shelver_user.into(),
                    repo: repo.clone(),
                    branch: branch.clone(),
                    error: format!("Shelver init error: {e}"),
                    duration_ms: 0,
                    r#async: is_async,
                });
            }
            return;
        }
    };

    for branch in branches {
        let key = format!("{group}/{name}/{branch}");
        active.set_shelving(&key);
        let start = Instant::now();
        match shelver.shelve(branch, user_p4, shelver_user) {
            Ok(result) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                log::info!(
                    "Background shelve for branch '{branch}' completed as CL {}",
                    result.changelist
                );
                active.set_done(&key, result.changelist, result.client_name.clone());
                let event = if result.is_reshelve {
                    ObservabilityEvent::ShelveReshelved {
                        timestamp: Utc::now(),
                        user: shelver_user.into(),
                        repo: repo.clone(),
                        branch: branch.clone(),
                        changelist: result.changelist,
                        client_name: result.client_name,
                        duration_ms,
                        file_count: result.file_count,
                        r#async: is_async,
                        commits_in_branch: result.commits_in_branch,
                    }
                } else {
                    ObservabilityEvent::ShelveCompleted {
                        timestamp: Utc::now(),
                        user: shelver_user.into(),
                        repo: repo.clone(),
                        branch: branch.clone(),
                        changelist: result.changelist,
                        client_name: result.client_name,
                        duration_ms,
                        file_count: result.file_count,
                        r#async: is_async,
                        commits_in_branch: result.commits_in_branch,
                    }
                };
                emitter.try_emit(event);
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                log::error!("Background shelve for branch '{branch}' failed: {e}");
                active.set_failed(&key, e.to_string());
                emitter.try_emit(ObservabilityEvent::ShelveFailed {
                    timestamp: Utc::now(),
                    user: shelver_user.into(),
                    repo: repo.clone(),
                    branch: branch.clone(),
                    error: e.to_string(),
                    duration_ms,
                    r#async: is_async,
                });
            }
        }
    }
}

// ============================================================
// Observability API endpoints
// ============================================================

#[derive(Deserialize)]
pub struct EventsQuery {
    event_type: Option<String>,
    since: Option<i64>,
    until: Option<i64>,
    repo: Option<String>,
    user: Option<String>,
    limit: Option<u32>,
}

pub async fn query_events(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventsQuery>,
) -> Response {
    let db_path = state.db_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path)?;
        let mut sql = "SELECT payload FROM events WHERE 1=1".to_string();
        let mut bindings: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref et) = params.event_type {
            sql.push_str(&format!(" AND event_type = ?{}", bindings.len() + 1));
            bindings.push(Box::new(et.clone()));
        }
        if let Some(since) = params.since {
            sql.push_str(&format!(" AND timestamp >= ?{}", bindings.len() + 1));
            bindings.push(Box::new(since));
        }
        if let Some(until) = params.until {
            sql.push_str(&format!(" AND timestamp <= ?{}", bindings.len() + 1));
            bindings.push(Box::new(until));
        }
        if let Some(ref repo) = params.repo {
            sql.push_str(&format!(" AND repo = ?{}", bindings.len() + 1));
            bindings.push(Box::new(repo.clone()));
        }
        if let Some(ref user) = params.user {
            sql.push_str(&format!(" AND user = ?{}", bindings.len() + 1));
            bindings.push(Box::new(user.clone()));
        }
        let limit = params.limit.unwrap_or(100);
        sql.push_str(&format!(
            " ORDER BY timestamp DESC LIMIT ?{}",
            bindings.len() + 1
        ));
        bindings.push(Box::new(limit));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            bindings.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows: Vec<String> = stmt
            .query_map(params_ref.as_slice(), |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Return as JSON array of raw JSON objects
        Ok::<String, rusqlite::Error>(format!("[{}]", rows.join(",")))
    })
    .await;

    match result {
        Ok(Ok(json)) => {
            (StatusCode::OK, [("content-type", "application/json")], json).into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Task error: {e}"),
        )
            .into_response(),
    }
}

pub async fn query_event_counts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventsQuery>,
) -> Response {
    let db_path = state.db_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path)?;
        let mut sql = "SELECT event_type, COUNT(*) FROM events WHERE 1=1".to_string();
        let mut bindings: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(since) = params.since {
            sql.push_str(&format!(" AND timestamp >= ?{}", bindings.len() + 1));
            bindings.push(Box::new(since));
        }
        if let Some(until) = params.until {
            sql.push_str(&format!(" AND timestamp <= ?{}", bindings.len() + 1));
            bindings.push(Box::new(until));
        }
        if let Some(ref repo) = params.repo {
            sql.push_str(&format!(" AND repo = ?{}", bindings.len() + 1));
            bindings.push(Box::new(repo.clone()));
        }
        sql.push_str(" GROUP BY event_type ORDER BY event_type");

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            bindings.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let counts: std::collections::HashMap<String, u64> = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok::<_, rusqlite::Error>(counts)
    })
    .await;

    match result {
        Ok(Ok(counts)) => Json(counts).into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Task error: {e}"),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
struct ActiveUser {
    user: String,
    push_count: u64,
    active_branches: u64,
}

pub async fn query_active_users(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventsQuery>,
) -> Response {
    let db_path = state.db_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path)?;

        let since = params.since.unwrap_or(0);
        let mut bindings: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        bindings.push(Box::new(since));

        let mut repo_filter = String::new();
        if let Some(ref repo) = params.repo {
            repo_filter = format!(" AND repo = ?{}", bindings.len() + 1);
            bindings.push(Box::new(repo.clone()));
        }

        // Get push counts per user
        let push_sql = format!(
            "SELECT user, COUNT(*) FROM events WHERE event_type = 'push.received' AND timestamp >= ?1{repo_filter} AND user IS NOT NULL GROUP BY user"
        );
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = bindings.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&push_sql)?;
        let push_counts: std::collections::HashMap<String, u64> = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Get active branches: created minus (deleted + merged)
        let created_sql = format!(
            "SELECT user, COUNT(*) FROM events WHERE event_type = 'push.branch_created' AND timestamp >= ?1{repo_filter} AND user IS NOT NULL GROUP BY user"
        );
        let mut stmt = conn.prepare(&created_sql)?;
        let created: std::collections::HashMap<String, u64> = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let closed_sql = format!(
            "SELECT user, COUNT(*) FROM events WHERE event_type IN ('push.branch_deleted', 'shelve.merged') AND timestamp >= ?1{repo_filter} AND user IS NOT NULL GROUP BY user"
        );
        let mut stmt = conn.prepare(&closed_sql)?;
        let closed: std::collections::HashMap<String, u64> = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let users: Vec<ActiveUser> = push_counts
            .into_iter()
            .map(|(user, push_count)| {
                let c = created.get(&user).copied().unwrap_or(0);
                let d = closed.get(&user).copied().unwrap_or(0);
                let active_branches = c.saturating_sub(d);
                ActiveUser { user, push_count, active_branches }
            })
            .collect();

        Ok::<_, rusqlite::Error>(users)
    })
    .await;

    match result {
        Ok(Ok(users)) => Json(users).into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {e}"),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Task error: {e}"),
        )
            .into_response(),
    }
}
