pub mod app;
pub mod config_watcher;
pub mod handlers;

use std::sync::Arc;

use axum::Router;

use crate::app::AppState;

pub fn build_router(state: Arc<AppState>) -> Router {
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

    let ui = axum::Router::new()
        .route("/ui", get(handlers::system::ui_index))
        .route("/ui/", get(handlers::system::ui_index))
        .route("/ui/*path", get(handlers::system::ui_asset));

    axum::Router::new()
        .merge(api)
        .merge(system)
        .merge(ui)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state)
}
