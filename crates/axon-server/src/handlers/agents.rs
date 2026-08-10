use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::StreamExt;
use serde_json::json;

use axon_gateway::StreamEvent;
use axon_protocol::{
    AgentInfo, ErrorDetail, ErrorResponse, InvokeAgentRequest, InvokeAgentResponse, ToolCallRecord,
};

use crate::app::AppState;

pub async fn list_agents(State(state): State<Arc<AppState>>) -> Response {
    let config = state.current_config();
    let agents: Vec<AgentInfo> = config
        .agents
        .iter()
        .map(AgentInfo::from_definition)
        .collect();
    Json(agents).into_response()
}

pub async fn get_agent(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let config = state.current_config();
    match config.find_agent(&id) {
        Some(agent) => Json(AgentInfo::from_definition(agent)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorDetail {
                    message: format!("agent '{id}' not found"),
                    code: Some("not_found".into()),
                },
            }),
        )
            .into_response(),
    }
}

pub async fn invoke_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<InvokeAgentRequest>,
) -> Response {
    let config = state.current_config();
    let agent = match config.find_agent(&id) {
        Some(a) => Arc::new(a.clone()),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: ErrorDetail {
                        message: format!("agent '{id}' not found"),
                        code: Some("not_found".into()),
                    },
                }),
            )
                .into_response();
        }
    };

    let executor = state.executor();

    if req.stream {
        let agent_stream = agent.clone();
        let executor_stream = executor.clone();
        let input = req.input.clone();
        let conv_id = req.conversation_id.clone();

        let sse_stream = async_stream::stream! {
            let exec = executor_stream;
            let result = match exec.invoke_stream(agent_stream, input, conv_id).await {
                Ok(s) => s,
                Err(e) => {
                    let err = json!({ "error": { "message": e.to_string() } });
                    yield Ok::<_, std::convert::Infallible>(
                        axum::response::sse::Event::default()
                            .data(serde_json::to_string(&err).unwrap_or_default())
                    );
                    return;
                }
            };

            let mut pinned = Box::pin(result);
            while let Some(event) = pinned.as_mut().next().await {
                let payload = match &event {
                    StreamEvent::TextChunk { text } => json!({ "type": "text_chunk", "text": text }),
                    StreamEvent::ThoughtChunk { text } => json!({ "type": "thought_chunk", "text": text }),
                    StreamEvent::ToolCallRequest { tool_call } => json!({ "type": "tool_call_request", "tool_call": tool_call }),
                    StreamEvent::ToolCallResult { tool_call_id, result, is_error } => json!({ "type": "tool_call_result", "tool_call_id": tool_call_id, "result": result, "is_error": is_error }),
                    StreamEvent::UsageUpdate { usage } => json!({ "type": "usage_update", "usage": usage }),
                    StreamEvent::Error { message } => json!({ "type": "error", "message": message }),
                    StreamEvent::Done { finish_reason } => json!({ "type": "done", "finish_reason": finish_reason }),
                };
                yield Ok(
                    axum::response::sse::Event::default()
                        .data(serde_json::to_string(&payload).unwrap_or_default())
                );
            }
        };

        return axum::response::sse::Sse::new(sse_stream).into_response();
    }

    let result = match executor
        .invoke(&agent, &req.input, req.conversation_id.as_deref())
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: ErrorDetail {
                        message: e.to_string(),
                        code: None,
                    },
                }),
            )
                .into_response();
        }
    };

    let response = InvokeAgentResponse {
        output: result.output,
        conversation_id: result.conversation_id,
        usage: result.usage,
        iterations: result.iterations,
        tool_calls: result
            .tool_calls
            .into_iter()
            .map(|tc| ToolCallRecord {
                id: tc.id,
                name: tc.name,
                arguments: tc.arguments,
                result: tc.result,
                tool_call_id: tc.tool_call_id,
            })
            .collect(),
    };

    Json(response).into_response()
}
