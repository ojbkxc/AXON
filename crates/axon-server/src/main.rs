use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

use axon_core::AxonConfig;
use axon_server::{app::AppState, build_router, config_watcher};

#[derive(Parser, Debug)]
#[command(
    name = "axon",
    version,
    about = "AXON — mobile multi-agent + AI gateway"
)]
struct Cli {
    #[arg(
        long,
        default_value = "config.yaml",
        help = "Path to config file (yaml/json)"
    )]
    config: String,
    #[arg(long, help = "Override listen address")]
    addr: Option<String>,
    #[arg(long, help = "Override log level (trace/debug/info/warn/error)")]
    log_level: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config_path = PathBuf::from(&cli.config);
    let mut config = if config_path.exists() {
        AxonConfig::from_file(&cli.config)?
    } else {
        eprintln!("config file '{}' not found, using defaults", cli.config);
        AxonConfig::default()
    };

    if let Some(addr) = &cli.addr {
        config.server.addr = addr.clone();
    }

    config.validate()?;

    let log_level = cli
        .log_level
        .clone()
        .or_else(|| std::env::var("AXON_LOG_LEVEL").ok())
        .unwrap_or_else(|| config.observability.log_level.clone());
    let log_format = std::env::var("AXON_LOG_FORMAT")
        .ok()
        .unwrap_or_else(|| config.observability.log_format.clone());

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level));
    if log_format.eq_ignore_ascii_case("json") {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "AXON starting");

    let working_dir = std::env::current_dir()?;
    let state = AppState::new(config, working_dir)?;

    let config_path_for_watcher = config_path.clone();
    let state_for_watcher = state.clone();
    if config_path.exists() {
        if let Err(e) = config_watcher::spawn_watcher(state_for_watcher, config_path_for_watcher) {
            tracing::warn!("failed to start config watcher: {e}");
        }
    }

    let addr: SocketAddr = state
        .current_config()
        .server
        .addr
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid addr: {e}"))?;

    let router = build_router(state.clone());

    tracing::info!("listening on http://{addr}");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
