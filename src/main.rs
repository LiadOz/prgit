use clap::Parser;
use git2::Repository;
use p4rs::P4;
use prgit::db::Database;
use prgit::mirror::{IntegrateStrategy, Mirror};
use std::path::PathBuf;

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

    let p4 = P4::new().port(&args.port);
    let db_path = args.repo_path.join("mirror.db"); // TODO: should probably have this in a central directory or something
    let db = Database::open(db_path.to_str().expect("Invalid db path"))?;
    let mirror_data = db.mirror_data(
        args.client.clone(),
        IntegrateStrategy::MergeOurs,
        Some(args.max_changes),
    );

    log::info!(
        "Starting mirror for client {} from {}",
        args.client,
        args.port
    );
    let mut mirror = Mirror::new(p4, repo, mirror_data);
    mirror.run()?;

    log::info!("Mirror complete");
    Ok(())
}
