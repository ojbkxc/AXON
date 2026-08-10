use serde::{Deserialize, Serialize};

use axon_core::TokenUsage;

#[derive(Debug, Clone, Deserialize)]
pub struct InvokeAgentRequest {
    pub input: String,
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvokeAgentResponse {
    pub output: String,
    pub conversation_id: String,
    pub usage: TokenUsage,
    pub iterations: u32,
    pub tool_calls: Vec<ToolCallRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallRecord {
    pub id: String,
    pub name: String,
    pub arguments: String,
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub tool_call_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub model: String,
    pub tools: Vec<String>,
    pub max_iterations: u32,
}

impl AgentInfo {
    pub fn from_definition(def: &axon_core::AgentDefinition) -> Self {
        AgentInfo {
            id: def.id.clone(),
            name: def.name.clone(),
            description: def.description.clone(),
            model: def.model.clone(),
            tools: def.tools.clone(),
            max_iterations: def.max_iterations,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateConversationRequest {
    pub agent_id: String,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversationResponse {
    pub id: String,
    pub agent_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ConversationResponse {
    pub fn from_conv(c: &axon_store::Conversation) -> Self {
        ConversationResponse {
            id: c.id.clone(),
            agent_id: c.agent_id.clone(),
            title: c.title.clone(),
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<axon_core::ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    pub created_at: i64,
}

impl MessageResponse {
    pub fn from_record(r: &axon_store::MessageRecord) -> Self {
        MessageResponse {
            id: r.id.clone(),
            role: r.role.clone(),
            content: r.content.clone(),
            tool_calls: r.tool_calls.clone(),
            tool_call_id: r.tool_call_id.clone(),
            model: r.model.clone(),
            usage: r.usage.clone(),
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub models: usize,
    pub agents: usize,
    pub routes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageStatsResponse {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_duration_ms: u64,
    pub by_model: Vec<ModelUsageResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelUsageResponse {
    pub model: String,
    pub requests: i64,
    pub tokens: i64,
}

impl UsageStatsResponse {
    pub fn from_stats(s: &axon_store::UsageStats) -> Self {
        UsageStatsResponse {
            total_requests: s.total_requests,
            total_tokens: s.total_tokens,
            total_duration_ms: s.total_duration_ms,
            by_model: s
                .by_model
                .iter()
                .map(|m| ModelUsageResponse {
                    model: m.model.clone(),
                    requests: m.requests,
                    tokens: m.tokens,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolInfoResponse {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorDetail {
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
}
