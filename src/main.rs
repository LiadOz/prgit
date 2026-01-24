use std::path::PathBuf;

use clap::Parser;
use git2::Repository;
use prgit::cabinet::Database;
use prgit::mirror::{IntegrateStrategy, Mirror};

#[derive(Parser)]
#[command(name = "prgit")]
#[command(about = "Mirror Perforce to Git")]
struct Args {
    #[arg(long, default_value = "localhost:1666")]
    port: String,

    #[arg(long)]
    client: String,

    #[arg(long, default_value = "test_repo")]
    repo_path: PathBuf,

    #[arg(long, default_value = "100")]
    max_changes: usize,

    #[arg(long, default_value = "info")]
    log_level: String,

    #[arg(long, default_value = "p4")]
    p4_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&args.log_level))
        .init();

    log::info!("Initializing bare repo at {:?}", args.repo_path);
    let repo = if args.repo_path.exists() {
        Repository::open_bare(&args.repo_path)?
    } else {
        Repository::init_bare(&args.repo_path)?
    };

    let db_path = args.repo_path.join("mirror.db");
    let db = Database::open(db_path.to_str().expect("Invalid db path"))?;

    let client_data = match db.client_by_name(&args.client)? {
        Some(cd) => cd,
        None => {
            let id = db.create_prgit_client(&args.client, &args.p4_path, &args.port, "")?;
            db.create_prgit_repo(id, args.repo_path.to_str().expect("Invalid repo path"), IntegrateStrategy::MergeOurs, Some(args.max_changes))?;
            db.client(id)?.expect("just created")
        }
    };

    let p4 = client_data.p4();
    log::info!(
        "Starting mirror for client {} from {}",
        args.client,
        args.port
    );
    let mut mirror = Mirror::new(p4, repo, client_data);
    mirror.run()?;

    log::info!("Mirror complete");
    Ok(())
}
