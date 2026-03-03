use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};
use git2::Repository;
use p4rs::P4;
use prgit::cabinet::{Database, PrgitClient};
use prgit::mirror::{IntegrateStrategy, Mirror};
use prgit::shelf::Shelver;

#[derive(Parser)]
#[command(name = "poc")]
#[command(about = "prgit POC - P4/Git bridge server")]
struct Cli {
    #[arg(long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize: create bare repo, mirror P4, install post-receive hook
    Init {
        #[arg(long)]
        port: String,

        #[arg(long)]
        client: String,

        #[arg(long)]
        repo_path: PathBuf,

        #[arg(long, default_value = "master")]
        synced_branch: String,

        #[arg(long, default_value = "100")]
        max_changes: usize,

        #[arg(long, default_value = "p4")]
        p4_path: String,

        /// P4 user. If omitted, uses P4USER env var or P4 default
        #[arg(long)]
        p4_user: Option<String>,

        /// Directory for per-user shelve client workspaces.
        /// Defaults to <repo_path>/shelve_clients
        #[arg(long)]
        shelve_clients_root: Option<PathBuf>,
    },
    /// Run mirror daemon: polls P4 for new submitted changes on an interval
    Serve {
        #[arg(long)]
        repo_path: PathBuf,

        #[arg(long)]
        client: String,

        /// Mirror poll interval in seconds
        #[arg(long, default_value = "60")]
        interval_secs: u64,
    },
    /// Handle git post-receive hook (invoked by git, reads refs from stdin)
    Hook {
        #[arg(long)]
        repo_path: PathBuf,

        #[arg(long)]
        client: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&cli.log_level))
        .init();

    match cli.command {
        Commands::Init {
            port,
            client,
            repo_path,
            synced_branch,
            max_changes,
            p4_path,
            shelve_clients_root,
            p4_user,
        } => {
            let p4_user = p4_user
                .or_else(|| std::env::var("P4USER").ok())
                .unwrap_or_default();
            cmd_init(
                &port,
                &client,
                &repo_path,
                &synced_branch,
                max_changes,
                &p4_path,
                &p4_user,
                shelve_clients_root.as_deref(),
            )
        }
        Commands::Serve {
            repo_path,
            client,
            interval_secs,
        } => cmd_serve(&repo_path, &client, interval_secs),
        Commands::Hook { repo_path, client } => cmd_hook(&repo_path, &client),
    }
}

fn cmd_init(
    port: &str,
    client: &str,
    repo_path: &Path,
    synced_branch: &str,
    max_changes: usize,
    p4_path: &str,
    p4_user: &str,
    shelve_clients_root: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create/open bare git repo
    let repo = if repo_path.exists() {
        Repository::open_bare(repo_path)?
    } else {
        Repository::init_bare(repo_path)?
    };
    // Canonicalize after creation so the DB always has an absolute path
    let repo_path = repo_path.canonicalize()?;

    eprintln!("Initializing prgit repo at {}", repo_path.display());

    // 2. Open database
    let db_path = repo_path.join("mirror.db");
    let db = Database::open(db_path.to_str().ok_or("Invalid db path")?)?;

    // 3. Create or find prgit client
    let client_data = match db.client_by_name(client)? {
        Some(cd) => cd,
        None => {
            let id = db.create_prgit_client(client, p4_path, port, p4_user)?;
            db.create_prgit_repo(
                id,
                repo_path.to_str().ok_or("Invalid repo path")?,
                synced_branch,
                IntegrateStrategy::MergeOurs,
                Some(max_changes),
            )?;
            db.client(id)?.ok_or("Failed to create client")?
        }
    };

    // 4. Set up shelve config
    let clients_root = shelve_clients_root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| repo_path.join("shelve_clients"));
    std::fs::create_dir_all(&clients_root)?;

    if client_data.shelve_config().is_none() {
        db.create_shelve_config(
            client_data.client_id,
            clients_root.to_str().ok_or("Invalid clients root path")?,
        )?;
    }

    // 5. Run initial mirror
    eprintln!("Running initial P4 mirror...");
    eprintln!("  P4 port:   {}", port);
    eprintln!("  P4 client: {}", client);
    let p4 = make_p4(&client_data);
    let mut mirror = Mirror::new(p4, repo, client_data);
    mirror.run()?;
    let last_change = mirror.last_sync_change();
    eprintln!("Mirror complete. Last synced change: {}", last_change);
    if last_change == 0 {
        eprintln!("  WARNING: No changes were mirrored. Verify the P4 client '{}' has submitted changes.", client);
    }

    // 6. Sync commit-to-change mappings (mirror stores them in git notes,
    //    shelver looks them up in the DB — bridge the gap here)
    let repo = Repository::open_bare(&repo_path)?;
    let mapping_client = db.client_by_name(client)?.ok_or("Client not found after init")?;
    let mapped = sync_commit_mappings(&repo, &mapping_client)?;
    eprintln!("Synced {} commit-to-change mappings", mapped);

    // 7. Install post-receive hook
    install_hook(&repo_path, client)?;
    eprintln!("Post-receive hook installed.");

    eprintln!();
    eprintln!("Init complete!");
    eprintln!("  Repo:     {}", repo_path.display());
    eprintln!("  Client:   {}", client);
    eprintln!("  Shelve:   {}", clients_root.display());
    eprintln!("  Hook:     {}", repo_path.join("hooks/post-receive").display());
    eprintln!();
    eprintln!("Users can now clone:");
    eprintln!("  git clone {}", repo_path.display());
    eprintln!();
    eprintln!("Start the mirror daemon:");
    eprintln!("  poc serve --repo-path {} --client {}", repo_path.display(), client);

    Ok(())
}

fn cmd_serve(
    repo_path: &Path,
    client: &str,
    interval_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!(
        "Starting mirror daemon for {} (interval: {}s)",
        repo_path.display(),
        interval_secs
    );

    let db_path = repo_path.join("mirror.db");
    let db = Database::open(db_path.to_str().ok_or("Invalid db path")?)?;

    let client_data = db.client_by_name(client)?.ok_or("Client not found")?;
    let p4 = make_p4(&client_data);
    let repo = Repository::open_bare(repo_path)?;
    let mut mirror = Mirror::new(p4, repo, client_data);

    loop {
        let before = mirror.last_sync_change();
        eprintln!("[serve] Polling P4 for changes since {}...", before);

        match mirror.run() {
            Ok(()) => {
                let after = mirror.last_sync_change();
                if after > before {
                    eprintln!("[serve] Synced changes {}-{}", before + 1, after);
                    let repo = Repository::open_bare(repo_path)?;
                    let mapping_client =
                        db.client_by_name(client)?.ok_or("Client not found")?;
                    let mapped = sync_commit_mappings(&repo, &mapping_client)?;
                    eprintln!("[serve] Mapped {} new commits", mapped);
                } else {
                    eprintln!("[serve] No new changes.");
                }
            }
            Err(e) => {
                eprintln!("[serve] Mirror error: {}", e);
            }
        }

        eprintln!("[serve] Sleeping {}s...", interval_secs);
        thread::sleep(Duration::from_secs(interval_secs));
    }
}

fn cmd_hook(
    repo_path: &Path,
    client: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = repo_path.join("mirror.db");
    let db = Database::open(db_path.to_str().ok_or("Invalid db path")?)?;
    let client_data = db.client_by_name(client)?.ok_or("Client not found")?;
    let user_p4 = make_p4(&client_data);

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            continue;
        }

        let new_sha = parts[1];
        let refname = parts[2];

        // Skip branch deletes (all-zero SHA)
        if new_sha.chars().all(|c| c == '0') {
            continue;
        }

        let branch = match refname.strip_prefix("refs/heads/") {
            Some(b) => b,
            None => continue,
        };

        // Don't shelve pushes to the synced branch
        if branch == client_data.git_config.synced_branch {
            eprintln!(
                "prgit: push to '{}' is managed by mirror, skipping shelve",
                branch
            );
            continue;
        }

        eprintln!("prgit: shelving branch '{}'...", branch);

        let shelver = Shelver::new(&client_data)?;
        match shelver.shelve(branch, &user_p4) {
            Ok(cl) => {
                eprintln!("prgit: === Shelved as changelist {} ===", cl);
            }
            Err(e) => {
                eprintln!("prgit: shelve failed: {}", e);
            }
        }
    }

    Ok(())
}

/// Walk git notes (refs/notes/p4) and populate commit_change_mapping in the DB.
/// The mirror writes P4 metadata to git notes; the shelver reads mappings from
/// the DB. This function bridges the two.
fn sync_commit_mappings(
    repo: &Repository,
    client: &prgit::cabinet::PrgitClient,
) -> Result<usize, Box<dyn std::error::Error>> {
    let notes = match repo.notes(Some("refs/notes/p4")) {
        Ok(n) => n,
        Err(_) => return Ok(0),
    };

    let mut count = 0;
    for note_item in notes {
        let (note_oid, commit_oid) = note_item?;
        let note_blob = repo.find_blob(note_oid)?;
        let note_content = std::str::from_utf8(note_blob.content())?;

        for line in note_content.lines() {
            if let Some(change_str) = line.strip_prefix("P4-Change: ") {
                if let Ok(change) = change_str.trim().parse::<usize>() {
                    let commit_hash = commit_oid.to_string();
                    // Skip if already mapped
                    if client.get_change_for_commit(&commit_hash).is_none() {
                        client.map_commit_to_change(&commit_hash, change);
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

/// Build a P4 instance from a PrgitClient config.
/// Unlike client_data.p4(), this skips `-u` when p4user is empty,
/// letting P4 fall back to P4USER env var or OS username.
fn make_p4(client: &PrgitClient) -> P4 {
    let mut p4 = P4::new()
        .p4_path(&client.p4_config.p4_path)
        .port(&client.p4_config.p4port)
        .client_name(&client.p4_config.client_name);
    if !client.p4_config.p4user.is_empty() {
        p4 = p4.p4_user(&client.p4_config.p4user);
    }
    p4
}

fn install_hook(
    repo_path: &Path,
    client: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let hooks_dir = repo_path.join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join("post-receive");
    let poc_binary = std::env::current_exe()?;
    let canonical_repo = repo_path.canonicalize()?;

    let hook_content = format!(
        r#"#!/bin/bash
"{}" hook --repo-path "{}" --client "{}"
"#,
        poc_binary.display(),
        canonical_repo.display(),
        client,
    );

    std::fs::write(&hook_path, &hook_content)?;

    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))?;

    Ok(())
}
