use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use prgit::window::ServerConfig;

#[derive(Parser)]
#[command(name = "prgit-server")]
struct Cli {
    #[arg(long)]
    config: PathBuf,
}

fn load_config(path: &std::path::Path) -> Result<ServerConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file {}", path.display()))?;
    serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse config file {}", path.display()))
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config(&cli.config)?;
    log::info!(
        "Loaded config: listen={}, data_dir={}, {} repo(s)",
        config.listen,
        config.data_dir.display(),
        config.repos.len()
    );

    let app = prgit::window::build_app(&config)?;
    prgit::window::spawn_mirror_tasks(&config);

    let listener = tokio::net::TcpListener::bind(&config.listen)
        .await
        .with_context(|| format!("Failed to bind to {}", config.listen))?;
    log::info!("Server listening on {}", config.listen);
    axum::serve(listener, app).await.context("Server error")?;
    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(e) = run().await {
        log::error!("{e:#}");
        std::process::exit(1);
    }
}
