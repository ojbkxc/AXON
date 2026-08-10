use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::StreamExt;
use serde_json::json;

use axon_core::{ChatMessage, ChatOptions, TokenUsage};
use axon_gateway::StreamEvent;
use axon_protocol::{
    ChatChoice, ChatChunkChoice, ChatCompletionChunk, ChatCompletionRequest,
    ChatCompletionResponse, ChatDelta, ErrorDetail, ErrorResponse, ModelObject, ModelsResponse,
};

use crate::app::AppState;

pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatCompletionRequest>,
) -> Response {
    let gateway = state.gateway.clone();

    let options = ChatOptions {
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        tools: req.tools.clone(),
        stream: Some(req.stream),
    };

    if req.stream {
        let stream = match gateway
            .chat_stream(&req.model, &req.messages, &options)
            .await
        {
            Ok(s) => s,
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

        let model = req.model.clone();
        let sse_stream = async_stream::stream! {
            let mut pinned = stream;
            let chunk_id = format!("chatcmpl-{}", chrono::Utc::now().timestamp_millis());
            while let Some(event) = pinned.next().await {
                match event {
                    StreamEvent::TextChunk { text } => {
                        let chunk = ChatCompletionChunk {
                            id: chunk_id.clone(),
                            object: "chat.completion.chunk".into(),
                            created: chrono::Utc::now().timestamp(),
                            model: model.clone(),
                            choices: vec![ChatChunkChoice {
                                index: 0,
                                delta: ChatDelta {
                                    content: Some(text),
                                    role: None,
                                    tool_calls: None,
                                },
                                finish_reason: None,
                            }],
                        };
                        yield Ok::<_, std::convert::Infallible>(
                            axum::response::sse::Event::default()
                                .data(serde_json::to_string(&chunk).unwrap_or_default())
                        );
                    }
                    StreamEvent::ToolCallRequest { tool_call } => {
                        let chunk = ChatCompletionChunk {
                            id: chunk_id.clone(),
                            object: "chat.completion.chunk".into(),
                            created: chrono::Utc::now().timestamp(),
                            model: model.clone(),
                            choices: vec![ChatChunkChoice {
                                index: 0,
                                delta: ChatDelta {
                                    content: None,
                                    role: None,
                                    tool_calls: Some(vec![tool_call]),
                                },
                                finish_reason: None,
                            }],
                        };
                        yield Ok(
                            axum::response::sse::Event::default()
                                .data(serde_json::to_string(&chunk).unwrap_or_default())
                        );
                    }
                    StreamEvent::UsageUpdate { usage } => {
                        let chunk = ChatCompletionChunk {
                            id: chunk_id.clone(),
                            object: "chat.completion.chunk".into(),
                            created: chrono::Utc::now().timestamp(),
                            model: model.clone(),
                            choices: vec![ChatChunkChoice {
                                index: 0,
                                delta: ChatDelta::default(),
                                finish_reason: None,
                            }],
                        };
                        yield Ok(
                            axum::response::sse::Event::default()
                                .data(serde_json::to_string(&chunk).unwrap_or_default())
                        );
                        let _ = usage;
                    }
                    StreamEvent::Done { finish_reason } => {
                        let chunk = ChatCompletionChunk {
                            id: chunk_id.clone(),
                            object: "chat.completion.chunk".into(),
                            created: chrono::Utc::now().timestamp(),
                            model: model.clone(),
                            choices: vec![ChatChunkChoice {
                                index: 0,
                                delta: ChatDelta::default(),
                                finish_reason,
                            }],
                        };
                        yield Ok(
                            axum::response::sse::Event::default()
                                .data(serde_json::to_string(&chunk).unwrap_or_default())
                        );
                        yield Ok(axum::response::sse::Event::default().data("[DONE]"));
                    }
                    StreamEvent::Error { message } => {
                        let err = json!({ "error": { "message": message } });
                        yield Ok(
                            axum::response::sse::Event::default()
                                .data(serde_json::to_string(&err).unwrap_or_default())
                        );
                    }
                    _ => {}
                }
            }
        };

        return axum::response::sse::Sse::new(sse_stream).into_response();
    }

    match gateway.chat(&req.model, &req.messages, &options).await {
        Ok(resp) => {
            let response = ChatCompletionResponse {
                id: format!("chatcmpl-{}", chrono::Utc::now().timestamp_millis()),
                object: "chat.completion".into(),
                created: chrono::Utc::now().timestamp(),
                model: resp.model,
                choices: vec![ChatChoice {
                    index: 0,
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: resp.content,
                        tool_calls: if resp.tool_calls.is_empty() {
                            None
                        } else {
                            Some(resp.tool_calls)
                        },
                        tool_call_id: None,
                        name: None,
                    },
                    finish_reason: resp.finish_reason,
                }],
                usage: Some(resp.usage),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: ErrorDetail {
                    message: e.to_string(),
                    code: None,
                },
            }),
        )
            .into_response(),
    }
}

pub async fn list_models(State(state): State<Arc<AppState>>) -> Response {
    let models = state.gateway.list_models();
    let data: Vec<ModelObject> = models
        .into_iter()
        .map(|m| ModelObject {
            id: m.id,
            object: "model".into(),
            created: chrono::Utc::now().timestamp(),
            owned_by: m.provider,
        })
        .collect();
    let resp = ModelsResponse {
        object: "list".into(),
        data,
    };
    Json(resp).into_response()
}
