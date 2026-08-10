use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use axon_protocol::{
    ConversationResponse, CreateConversationRequest, ErrorResponse, ErrorDetail,
    MessageResponse,
};

use crate::app::AppState;

pub async fn create_conversation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateConversationRequest>,
) -> Response {
    match state
        .store
        .create_conversation(&req.agent_id, req.title.as_deref())
    {
        Ok(conv) => Json(ConversationResponse::from_conv(&conv)).into_response(),
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

pub async fn list_conversations(State(state): State<Arc<AppState>>) -> Response {
    match state.store.list_conversations(100) {
        Ok(convs) => {
            let resp: Vec<ConversationResponse> =
                convs.iter().map(ConversationResponse::from_conv).collect();
            Json(resp).into_response()
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

pub async fn get_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    match state.store.get_conversation(&id) {
        Ok(Some(conv)) => Json(ConversationResponse::from_conv(&conv)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorDetail {
                    message: format!("conversation '{id}' not found"),
                    code: Some("not_found".into()),
                },
            }),
        )
            .into_response(),
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

pub async fn delete_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    match state.store.delete_conversation(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
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

pub async fn get_messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    match state.store.get_messages(&id) {
        Ok(msgs) => {
            let resp: Vec<MessageResponse> =
                msgs.iter().map(MessageResponse::from_record).collect();
            Json(resp).into_response()
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
