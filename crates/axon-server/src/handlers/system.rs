use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;

use axon_protocol::{HealthResponse, StatusResponse, ToolInfoResponse, UsageStatsResponse};

use crate::app::AppState;

pub async fn healthz() -> Response {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
    .into_response()
}

pub async fn readyz(State(state): State<Arc<AppState>>) -> Response {
    let config = state.current_config();
    let ready = !config.server.addr.is_empty();
    let status = if ready { "ready" } else { "not ready" };
    Json(HealthResponse {
        status: status.into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
    .into_response()
}

pub async fn status(State(state): State<Arc<AppState>>) -> Response {
    let config = state.current_config();
    let uptime = state.start_time.elapsed().as_secs();
    Json(StatusResponse {
        status: "running".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: uptime,
        models: config.models.len(),
        agents: config.agents.len(),
        routes: config.routes.len(),
    })
    .into_response()
}

pub async fn metrics(State(state): State<Arc<AppState>>) -> Response {
    let stats = match state.store.get_usage_stats() {
        Ok(s) => s,
        Err(_) => {
            return "# axon metrics unavailable\n".to_string().into_response();
        }
    };

    let mut buf = String::new();
    buf.push_str("# HELP axon_requests_total Total number of requests\n");
    buf.push_str("# TYPE axon_requests_total counter\n");
    buf.push_str(&format!("axon_requests_total {}\n", stats.total_requests));
    buf.push_str("# HELP axon_tokens_total Total tokens used\n");
    buf.push_str("# TYPE axon_tokens_total counter\n");
    buf.push_str(&format!("axon_tokens_total {}\n", stats.total_tokens));
    buf.push_str("# HELP axon_duration_ms_total Total duration in ms\n");
    buf.push_str("# TYPE axon_duration_ms_total counter\n");
    buf.push_str(&format!(
        "axon_duration_ms_total {}\n",
        stats.total_duration_ms
    ));
    for m in &stats.by_model {
        buf.push_str(&format!(
            "axon_model_requests{{model=\"{}\"}} {}\n",
            m.model, m.requests
        ));
        buf.push_str(&format!(
            "axon_model_tokens{{model=\"{}\"}} {}\n",
            m.model, m.tokens
        ));
    }
    buf.into_response()
}

pub async fn usage_stats(State(state): State<Arc<AppState>>) -> Response {
    match state.store.get_usage_stats() {
        Ok(stats) => Json(UsageStatsResponse::from_stats(&stats)).into_response(),
        Err(e) => {
            let err = axon_protocol::ErrorResponse {
                error: axon_protocol::ErrorDetail {
                    message: e.to_string(),
                    code: None,
                },
            };
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(err),
            )
                .into_response()
        }
    }
}

pub async fn list_tools(State(state): State<Arc<AppState>>) -> Response {
    let tools = state.tools();
    let info: Vec<ToolInfoResponse> = tools
        .list()
        .into_iter()
        .map(|t| ToolInfoResponse {
            name: t.name,
            description: t.description,
            parameters: t.parameters,
        })
        .collect();
    Json(info).into_response()
}
