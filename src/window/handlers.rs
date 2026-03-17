use std::io::Write;
use std::process::Stdio;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::prelude::*;
use p4rs::P4;
use serde::{Deserialize, Serialize};

use crate::cabinet::Database;
use crate::shelf::{PendingShelve, Shelver};

use super::{ActiveShelves, AppState, RepoEntry};

pub async fn health() -> StatusCode {
    StatusCode::OK
}

#[derive(Serialize)]
pub struct ShelveStatusResponse {
    active: bool,
}

pub async fn shelve_status(
    State(state): State<Arc<AppState>>,
    Path((group, name, cl_str)): Path<(String, String, String)>,
) -> Response {
    let cl: usize = match cl_str.parse() {
        Ok(n) => n,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let repo_key = format!("{group}/{name}");
    if !state.repos.contains_key(&repo_key) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let active = state.active_shelves.contains(cl);
    Json(ShelveStatusResponse { active }).into_response()
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

    let auth_user = match authenticate_push(req.headers(), &repo_entry.config.p4port).await {
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

    let method = req.method().clone();
    let is_receive_pack = git_path.contains("git-receive-pack");
    let query = req.uri().query().unwrap_or("").to_string();
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let auth_user = if is_receive_pack {
        match authenticate_push(req.headers(), &repo_entry.config.p4port).await {
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

    let ref_updates = if is_receive_pack && method == Method::POST {
        let refs = parse_ref_updates(&body_bytes);
        let synced_ref = format!("refs/heads/{}", repo_entry.config.synced_branch);
        if refs.iter().any(|(_, _, r)| r == &synced_ref) {
            let msg = format!(
                "Push to synced branch '{}' is not allowed",
                repo_entry.config.synced_branch
            );
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
    let repo_dir_name = match repo_entry.bare_repo_path.file_name().and_then(|f| f.to_str()) {
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

    if is_receive_pack && cgi_status.is_success() {
        if let Some(auth_user) = auth_user {
            let result = shelve_branches(&state, repo_entry, &ref_updates, auth_user).await;
            if !result.errors.is_empty() {
                let msg = format!("Push succeeded but shelving failed:\n{}", result.errors.join("\n"));
                log::error!("{msg}");
                return git_error_response(&msg);
            }
            if !result.shelved.is_empty() {
                let is_async = repo_entry.config.shelve_async();
                let lines: Vec<String> = result
                    .shelved
                    .iter()
                    .map(|(branch, cl, client)| {
                        if is_async {
                            format!("Shelving branch '{branch}' as CL {cl} on client '{client}' (in background)")
                        } else {
                            format!("Shelved branch '{branch}' as CL {cl} on client '{client}'")
                        }
                    })
                    .collect();
                let body = inject_sideband_messages(&cgi_body, &lines.join("\n"));
                return build_response(cgi_status, &cgi_headers, body);
            }
        }
    }

    build_response(cgi_status, &cgi_headers, cgi_body.to_vec())
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

async fn authenticate_push(headers: &HeaderMap, p4port: &str) -> std::result::Result<AuthenticatedUser, Response> {
    let (user, ticket) = extract_basic_auth(headers).ok_or_else(unauthorized)?;
    let port = p4port.to_string();
    let u = user.clone();
    let t = ticket.clone();
    let valid = tokio::task::spawn_blocking(move || validate_p4_ticket(&port, &u, &t))
        .await
        .unwrap_or(false);
    if !valid {
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
        let Ok(decode::Stream::Complete { line, bytes_consumed }) =
            decode::streaming(body) else { break };
        let PacketLineRef::Data(payload) = line else { break };
        let payload = payload.split(|&b| b == 0).next().unwrap_or(payload);
        if let Some((old, new, refname)) = std::str::from_utf8(payload)
            .ok()
            .and_then(|t| {
                let mut p = t.trim().splitn(3, ' ');
                Some((p.next()?, p.next()?, p.next()?))
            })
        {
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

fn parse_cgi_output(raw: &[u8]) -> (StatusCode, Vec<(String, Vec<u8>)>, &[u8]) {
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
    shelved: Vec<(String, usize, String)>,
    errors: Vec<String>,
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
        return HandlerShelveResult { shelved: Vec::new(), errors: Vec::new() };
    }

    let db_path = state.db_path.clone();
    let client_id = repo_entry.client_id;
    let async_shelve = repo_entry.config.shelve_async();
    let username = auth_user.username;
    let user_p4 = auth_user.p4;

    if async_shelve {
        let u = username.clone();
        let result = tokio::task::spawn_blocking(move || {
            do_prepare_shelve(&db_path, client_id, &branches, &user_p4, &u)
        })
        .await;

        match result {
            Ok((handler_result, pending)) => {
                if !pending.is_empty() {
                    let active = state.active_shelves.clone();
                    for (_, cl, _) in &handler_result.shelved {
                        active.insert(*cl);
                    }
                    let _ = tokio::task::spawn_blocking(move || {
                        complete_pending_shelves(pending, &active);
                    });
                }
                handler_result
            }
            Err(e) => HandlerShelveResult {
                shelved: Vec::new(),
                errors: vec![format!("Shelve task panicked: {e}")],
            },
        }
    } else {
        let result = tokio::task::spawn_blocking(move || {
            do_shelve(&db_path, client_id, &branches, &user_p4, &username)
        })
        .await;

        match result {
            Ok(r) => r,
            Err(e) => HandlerShelveResult {
                shelved: Vec::new(),
                errors: vec![format!("Shelve task panicked: {e}")],
            },
        }
    }
}

fn do_shelve(
    db_path: &str,
    client_id: u64,
    branches: &[String],
    user_p4: &P4,
    shelver_user: &str,
) -> HandlerShelveResult {
    let mut shelved = Vec::new();
    let mut errors = Vec::new();

    let db = match Database::open(db_path) {
        Ok(db) => db,
        Err(e) => return HandlerShelveResult { shelved, errors: vec![format!("Database error: {e}")] },
    };
    let client = match db.client(client_id) {
        Ok(Some(c)) => c,
        Ok(None) => return HandlerShelveResult { shelved, errors: vec![format!("Client id={client_id} not found")] },
        Err(e) => return HandlerShelveResult { shelved, errors: vec![format!("Database error: {e}")] },
    };
    let shelver = match Shelver::new(&client) {
        Ok(s) => s,
        Err(e) => return HandlerShelveResult { shelved, errors: vec![format!("Shelver init error: {e}")] },
    };

    for branch in branches {
        match shelver.shelve(branch, user_p4, shelver_user) {
            Ok(result) => {
                log::info!("Shelved branch '{branch}' as CL {} on client '{}'", result.changelist, result.client_name);
                shelved.push((branch.clone(), result.changelist, result.client_name));
            }
            Err(e) => {
                let msg = format!("Failed to shelve branch '{branch}': {e}");
                log::error!("{msg}");
                errors.push(msg);
            }
        }
    }
    HandlerShelveResult { shelved, errors }
}

fn do_prepare_shelve(
    db_path: &str,
    client_id: u64,
    branches: &[String],
    user_p4: &P4,
    shelver_user: &str,
) -> (HandlerShelveResult, Vec<(String, PendingShelve)>) {
    let mut shelved = Vec::new();
    let mut pending = Vec::new();
    let mut errors = Vec::new();

    let db = match Database::open(db_path) {
        Ok(db) => db,
        Err(e) => return (HandlerShelveResult { shelved, errors: vec![format!("Database error: {e}")] }, pending),
    };
    let client = match db.client(client_id) {
        Ok(Some(c)) => c,
        Ok(None) => return (HandlerShelveResult { shelved, errors: vec![format!("Client id={client_id} not found")] }, pending),
        Err(e) => return (HandlerShelveResult { shelved, errors: vec![format!("Database error: {e}")] }, pending),
    };
    let shelver = match Shelver::new(&client) {
        Ok(s) => s,
        Err(e) => return (HandlerShelveResult { shelved, errors: vec![format!("Shelver init error: {e}")] }, pending),
    };

    for branch in branches {
        match shelver.prepare_shelve(branch, user_p4, shelver_user) {
            Ok((result, pend)) => {
                log::info!("Prepared shelve for branch '{branch}' as CL {} (async)", result.changelist);
                shelved.push((branch.clone(), result.changelist, result.client_name));
                pending.push((branch.clone(), pend));
            }
            Err(e) => {
                let msg = format!("Failed to prepare shelve for branch '{branch}': {e}");
                log::error!("{msg}");
                errors.push(msg);
            }
        }
    }
    (HandlerShelveResult { shelved, errors }, pending)
}

fn complete_pending_shelves(pending: Vec<(String, PendingShelve)>, active: &ActiveShelves) {
    for (branch, pend) in pending {
        let cl = pend.changelist();
        let result = pend.complete();
        active.remove(cl);
        match result {
            Ok(()) => log::info!("Background shelve for branch '{branch}' completed (CL {cl})"),
            Err(e) => log::error!("Background shelve for branch '{branch}' failed: {e}"),
        }
    }
}
