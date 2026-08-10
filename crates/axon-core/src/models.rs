use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub system_prompt: String,
    pub model: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

fn default_max_iterations() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDefinition {
    pub name: String,
    pub provider: String,
    pub model_name: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub api_base: Option<String>,
    #[serde(default)]
    pub max_concurrency: Option<u32>,
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
}

impl ModelDefinition {
    pub fn resolve_api_key(&self) -> Option<String> {
        if let Some(key) = &self.api_key {
            return Some(key.clone());
        }
        if let Some(env_var) = &self.api_key_env {
            return std::env::var(env_var).ok();
        }
        None
    }

    pub fn resolve_api_base(&self) -> String {
        if let Some(base) = &self.api_base {
            return base.clone();
        }
        match self.provider.as_str() {
            "openai" => "https://api.openai.com/v1".into(),
            "anthropic" => "https://api.anthropic.com".into(),
            "vertex" => "https://generativelanguage.googleapis.com/v1beta".into(),
            _ => "https://api.openai.com/v1".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub rps: Option<u32>,
    #[serde(default)]
    pub rpm: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteDefinition {
    pub name: String,
    #[serde(default = "default_strategy")]
    pub strategy: String,
    pub targets: Vec<RouteTarget>,
}

fn default_strategy() -> String {
    "failover".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteTarget {
    pub model: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_weight() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDefinition {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        ChatMessage {
            role: "system".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        ChatMessage {
            role: "user".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        ChatMessage {
            role: "assistant".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        ChatMessage {
            role: "tool".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tools: Vec<ToolSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub function: ToolFunctionSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunctionSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("hello");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "hello");
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
    }

    #[test]
    fn test_chat_message_user() {
        let msg = ChatMessage::user("hi");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "hi");
    }

    #[test]
    fn test_chat_message_assistant() {
        let msg = ChatMessage::assistant("response");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "response");
    }

    #[test]
    fn test_chat_message_tool() {
        let msg = ChatMessage::tool("result", "call_123");
        assert_eq!(msg.role, "tool");
        assert_eq!(msg.content, "result");
        assert_eq!(msg.tool_call_id.as_deref(), Some("call_123"));
    }

    #[test]
    fn test_model_resolve_api_key_direct() {
        let model = ModelDefinition {
            name: "test".into(),
            provider: "openai".into(),
            model_name: "gpt-4".into(),
            api_key: Some("sk-direct".into()),
            api_key_env: None,
            api_base: None,
            max_concurrency: None,
            rate_limit: None,
        };
        assert_eq!(model.resolve_api_key().as_deref(), Some("sk-direct"));
    }

    #[test]
    fn test_model_resolve_api_key_env() {
        std::env::set_var("AXON_TEST_KEY", "sk-from-env");
        let model = ModelDefinition {
            name: "test".into(),
            provider: "openai".into(),
            model_name: "gpt-4".into(),
            api_key: None,
            api_key_env: Some("AXON_TEST_KEY".into()),
            api_base: None,
            max_concurrency: None,
            rate_limit: None,
        };
        assert_eq!(model.resolve_api_key().as_deref(), Some("sk-from-env"));
        std::env::remove_var("AXON_TEST_KEY");
    }

    #[test]
    fn test_model_resolve_api_key_none() {
        let model = ModelDefinition {
            name: "test".into(),
            provider: "openai".into(),
            model_name: "gpt-4".into(),
            api_key: None,
            api_key_env: Some("NONEXISTENT_VAR_XYZ".into()),
            api_base: None,
            max_concurrency: None,
            rate_limit: None,
        };
        assert!(model.resolve_api_key().is_none());
    }

    #[test]
    fn test_model_resolve_api_base_defaults() {
        let mk = |provider: &str| ModelDefinition {
            name: "test".into(),
            provider: provider.into(),
            model_name: "gpt-4".into(),
            api_key: None,
            api_key_env: None,
            api_base: None,
            max_concurrency: None,
            rate_limit: None,
        };
        assert_eq!(mk("openai").resolve_api_base(), "https://api.openai.com/v1");
        assert_eq!(
            mk("anthropic").resolve_api_base(),
            "https://api.anthropic.com"
        );
        assert!(mk("vertex").resolve_api_base().contains("googleapis.com"));
    }

    #[test]
    fn test_model_resolve_api_base_override() {
        let model = ModelDefinition {
            name: "test".into(),
            provider: "openai".into(),
            model_name: "gpt-4".into(),
            api_key: None,
            api_key_env: None,
            api_base: Some("https://custom.example.com/v1".into()),
            max_concurrency: None,
            rate_limit: None,
        };
        assert_eq!(model.resolve_api_base(), "https://custom.example.com/v1");
    }

    #[test]
    fn test_tool_call_serde() {
        let tc = ToolCall {
            id: "call_1".into(),
            call_type: "function".into(),
            function: ToolCallFunction {
                name: "web_search".into(),
                arguments: r#"{"query":"rust"}"#.into(),
            },
        };
        let json = serde_json::to_string(&tc).unwrap();
        assert!(json.contains(r#""type":"function""#));
        let de: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(de.id, "call_1");
        assert_eq!(de.function.name, "web_search");
    }

    #[test]
    fn test_token_usage_default() {
        let usage = TokenUsage::default();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_chat_options_default() {
        let opts = ChatOptions::default();
        assert!(opts.temperature.is_none());
        assert!(opts.max_tokens.is_none());
        assert!(opts.tools.is_empty());
        assert!(opts.stream.is_none());
    }
}
