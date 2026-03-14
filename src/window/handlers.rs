use std::io::Write;
use std::process::Stdio;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::prelude::*;
use p4rs::P4;

use crate::cabinet::Database;
use crate::shelf::Shelver;

use super::{AppState, RepoEntry};

pub async fn health() -> StatusCode {
    StatusCode::OK
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

    let user_p4 = if is_receive_pack {
        match authenticate_push(req.headers(), &repo_entry.config.p4port).await {
            Ok(p4) => Some(p4),
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
            return (
                StatusCode::FORBIDDEN,
                format!(
                    "Push to synced branch '{}' is not allowed",
                    repo_entry.config.synced_branch
                ),
            )
                .into_response();
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

    let response = parse_cgi_response(&cgi_output);

    if is_receive_pack && response.status().is_success() {
        if let Some(user_p4) = user_p4 {
            shelve_branches(&state, repo_entry, &ref_updates, user_p4);
        }
    }

    response
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

async fn authenticate_push(headers: &HeaderMap, p4port: &str) -> std::result::Result<P4, Response> {
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
    Ok(P4::new().port(p4port).p4_user(&user).password(&ticket))
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

fn parse_cgi_response(raw: &[u8]) -> Response {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    match httparse::parse_headers(raw, &mut headers) {
        Ok(httparse::Status::Complete((body_offset, parsed_headers))) => {
            let mut status = StatusCode::OK;
            let mut builder = Response::builder();
            for h in parsed_headers {
                if h.name.eq_ignore_ascii_case("Status") {
                    status = std::str::from_utf8(h.value)
                        .ok()
                        .and_then(|v| v.split_whitespace().next())
                        .and_then(|c| c.parse::<u16>().ok())
                        .and_then(|c| StatusCode::from_u16(c).ok())
                        .unwrap_or(StatusCode::OK);
                } else {
                    builder = builder.header(h.name, h.value);
                }
            }
            match builder
                .status(status)
                .body(Body::from(raw[body_offset..].to_vec()))
            {
                Ok(resp) => resp,
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        _ => (StatusCode::OK, raw.to_vec()).into_response(),
    }
}

fn shelve_branches(
    state: &Arc<AppState>,
    repo_entry: &RepoEntry,
    ref_updates: &[(String, String, String)],
    user_p4: P4,
) {
    let zero_sha = "0000000000000000000000000000000000000000";
    let synced_ref = format!("refs/heads/{}", repo_entry.config.synced_branch);
    let branches: Vec<String> = ref_updates
        .iter()
        .filter(|(_, new_sha, refname)| new_sha != zero_sha && refname != &synced_ref)
        .filter_map(|(_, _, refname)| refname.strip_prefix("refs/heads/").map(String::from))
        .collect();

    if branches.is_empty() {
        return;
    }

    let db_path = state.db_path.clone();
    let client_id = repo_entry.client_id;
    let p4client = repo_entry.config.p4client.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = do_shelve(&db_path, client_id, &branches, &user_p4) {
            log::error!("Shelving failed for client '{p4client}': {e}");
        }
    });
}

fn do_shelve(
    db_path: &str,
    client_id: u64,
    branches: &[String],
    user_p4: &P4,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let db = Database::open(db_path)?;
    let client = db
        .client(client_id)?
        .ok_or_else(|| format!("Client id={client_id} not found"))?;
    let shelver = Shelver::new(&client)?;
    for branch in branches {
        match shelver.shelve(branch, user_p4) {
            Ok(cl) => log::info!("Shelved branch '{branch}' as CL {cl}"),
            Err(e) => log::error!("Failed to shelve branch '{branch}': {e}"),
        }
    }
    Ok(())
}
