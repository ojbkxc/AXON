mod app;
mod config_watcher;
mod handlers;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use axon_core::AxonConfig;

use crate::app::AppState;

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

    let log_level = cli
        .log_level
        .clone()
        .or_else(|| std::env::var("AXON_LOG_LEVEL").ok())
        .unwrap_or_else(|| "info".into());

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level)),
        )
        .init();

    tracing::info!("AXON v{} starting", env!("CARGO_PKG_VERSION"));

    let config_path = PathBuf::from(&cli.config);
    let mut config = if config_path.exists() {
        AxonConfig::from_file(&cli.config)?
    } else {
        tracing::warn!("config file '{}' not found, using defaults", cli.config);
        AxonConfig::default()
    };

    if let Some(addr) = &cli.addr {
        config.server.addr = addr.clone();
    }

    config.validate()?;

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

fn build_router(state: Arc<AppState>) -> axum::Router {
    use axum::routing::{delete, get, post};

    let api = axum::Router::new()
        .route(
            "/v1/chat/completions",
            post(handlers::chat::chat_completions),
        )
        .route("/v1/models", get(handlers::chat::list_models))
        .route("/v1/agents", get(handlers::agents::list_agents))
        .route("/v1/agents/:id", get(handlers::agents::get_agent))
        .route(
            "/v1/agents/:id/invoke",
            post(handlers::agents::invoke_agent),
        )
        .route(
            "/v1/conversations",
            post(handlers::conversations::create_conversation),
        )
        .route(
            "/v1/conversations",
            get(handlers::conversations::list_conversations),
        )
        .route(
            "/v1/conversations/:id",
            get(handlers::conversations::get_conversation),
        )
        .route(
            "/v1/conversations/:id",
            delete(handlers::conversations::delete_conversation),
        )
        .route(
            "/v1/conversations/:id/messages",
            get(handlers::conversations::get_messages),
        )
        .route("/v1/tools", get(handlers::system::list_tools))
        .route("/v1/usage", get(handlers::system::usage_stats));

    let system = axum::Router::new()
        .route("/healthz", get(handlers::system::healthz))
        .route("/readyz", get(handlers::system::readyz))
        .route("/status", get(handlers::system::status))
        .route("/metrics", get(handlers::system::metrics));

    axum::Router::new()
        .merge(api)
        .merge(system)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
