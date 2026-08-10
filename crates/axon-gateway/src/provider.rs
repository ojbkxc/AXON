use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::stream::{ChatResponse, StreamEvent};
use axon_core::{AxonError, ChatMessage, ChatOptions, Result, TokenUsage, ToolCall};

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;

    async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: &ChatOptions,
    ) -> Result<ChatResponse>;

    async fn chat_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: &ChatOptions,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = StreamEvent> + Send>>>;
}

pub struct OpenAiProvider {
    client: reqwest::Client,
    api_base: String,
    api_key: String,
}

impl OpenAiProvider {
    pub fn new(api_base: &str, api_key: &str) -> Self {
        OpenAiProvider {
            client: reqwest::Client::new(),
            api_base: api_base.trim_end_matches('/').into(),
            api_key: api_key.into(),
        }
    }
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<axon_core::ToolSchema>,
    stream: bool,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: OpenAiUsage,
    model: String,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Deserialize, Default)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: Option<OpenAiPromptTokensDetails>,
    #[serde(default)]
    prompt_cache_hit_tokens: Option<u32>,
    #[serde(default)]
    prompt_cache_miss_tokens: Option<u32>,
    #[serde(default)]
    completion_tokens_details: Option<OpenAiCompletionTokensDetails>,
}

#[derive(Deserialize, Default)]
struct OpenAiPromptTokensDetails {
    #[serde(default)]
    cached_tokens: Option<u32>,
}

#[derive(Deserialize, Default)]
struct OpenAiCompletionTokensDetails {
    #[serde(default)]
    reasoning_tokens: Option<u32>,
}

impl OpenAiUsage {
    fn to_token_usage(&self) -> TokenUsage {
        TokenUsage {
            prompt_tokens: self.prompt_tokens,
            completion_tokens: self.completion_tokens,
            total_tokens: self.total_tokens,
            prompt_tokens_details: self
                .prompt_tokens_details
                .as_ref()
                .map(|d| axon_core::PromptTokensDetails {
                    cached_tokens: d.cached_tokens,
                }),
            prompt_cache_hit_tokens: self.prompt_cache_hit_tokens,
            prompt_cache_miss_tokens: self.prompt_cache_miss_tokens,
            completion_tokens_details: self
                .completion_tokens_details
                .as_ref()
                .map(|d| axon_core::CompletionTokensDetails {
                    reasoning_tokens: d.reasoning_tokens,
                }),
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: &ChatOptions,
    ) -> Result<ChatResponse> {
        let req = OpenAiRequest {
            model: model.into(),
            messages: messages.to_vec(),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            tools: options.tools.clone(),
            stream: false,
        };

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| AxonError::Upstream(format!("openai request: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AxonError::Upstream(format!("openai {status}: {body}")));
        }

        let data: OpenAiResponse = resp
            .json()
            .await
            .map_err(|e| AxonError::Upstream(format!("openai parse: {e}")))?;

        let choice = data
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AxonError::Upstream("openai: no choices in response".into()))?;

        Ok(ChatResponse {
            content: choice.message.content.unwrap_or_default(),
            tool_calls: choice.message.tool_calls.unwrap_or_default(),
            usage: data.usage.to_token_usage(),
            model: data.model,
            finish_reason: choice.finish_reason,
        })
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: &ChatOptions,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = StreamEvent> + Send>>> {
        let req = OpenAiRequest {
            model: model.into(),
            messages: messages.to_vec(),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            tools: options.tools.clone(),
            stream: true,
        };

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| AxonError::Upstream(format!("openai stream request: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AxonError::Upstream(format!(
                "openai stream {status}: {body}"
            )));
        }

        let stream = crate::stream::parse_openai_sse(resp);
        Ok(Box::pin(stream))
    }
}

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_base: String,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(api_base: &str, api_key: &str) -> Self {
        AnthropicProvider {
            client: reqwest::Client::new(),
            api_base: api_base.trim_end_matches('/').into(),
            api_key: api_key.into(),
        }
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    #[serde(default)]
    usage: AnthropicUsage,
    model: String,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use", alias = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Deserialize, Default)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: &ChatOptions,
    ) -> Result<ChatResponse> {
        let (system, chat_msgs): (String, Vec<ChatMessage>) = {
            let mut sys = String::new();
            let mut msgs = Vec::new();
            for m in messages {
                if m.role == "system" {
                    if !sys.is_empty() {
                        sys.push('\n');
                    }
                    sys.push_str(&m.content);
                } else {
                    msgs.push(m.clone());
                }
            }
            (sys, msgs)
        };

        let req = AnthropicRequest {
            model: model.into(),
            max_tokens: options.max_tokens.unwrap_or(4096),
            system,
            messages: chat_msgs,
            temperature: options.temperature,
            stream: false,
        };

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.api_base))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&req)
            .send()
            .await
            .map_err(|e| AxonError::Upstream(format!("anthropic request: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AxonError::Upstream(format!("anthropic {status}: {body}")));
        }

        let data: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| AxonError::Upstream(format!("anthropic parse: {e}")))?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();
        for block in data.content {
            match block {
                AnthropicContent::Text { text } => content.push_str(&text),
                AnthropicContent::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id,
                        call_type: "function".into(),
                        function: axon_core::ToolCallFunction {
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_default(),
                        },
                    });
                }
            }
        }

        Ok(ChatResponse {
            content,
            tool_calls,
            usage: TokenUsage {
                prompt_tokens: data.usage.input_tokens,
                completion_tokens: data.usage.output_tokens,
                total_tokens: data.usage.input_tokens + data.usage.output_tokens,
                ..Default::default()
            },
            model: data.model,
            finish_reason: data.stop_reason,
        })
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: &ChatOptions,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = StreamEvent> + Send>>> {
        let (system, chat_msgs): (String, Vec<ChatMessage>) = {
            let mut sys = String::new();
            let mut msgs = Vec::new();
            for m in messages {
                if m.role == "system" {
                    if !sys.is_empty() {
                        sys.push('\n');
                    }
                    sys.push_str(&m.content);
                } else {
                    msgs.push(m.clone());
                }
            }
            (sys, msgs)
        };

        let req = AnthropicRequest {
            model: model.into(),
            max_tokens: options.max_tokens.unwrap_or(4096),
            system,
            messages: chat_msgs,
            temperature: options.temperature,
            stream: true,
        };

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.api_base))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&req)
            .send()
            .await
            .map_err(|e| AxonError::Upstream(format!("anthropic stream request: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AxonError::Upstream(format!(
                "anthropic stream {status}: {body}"
            )));
        }

        let stream = crate::stream::parse_anthropic_sse(resp);
        Ok(Box::pin(stream))
    }
}

pub fn create_provider(
    provider_type: &str,
    api_base: &str,
    api_key: &str,
) -> Result<Box<dyn Provider>> {
    match provider_type {
        "openai" => Ok(Box::new(OpenAiProvider::new(api_base, api_key))),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(api_base, api_key))),
        "vertex" => Ok(Box::new(OpenAiProvider::new(api_base, api_key))),
        other => Err(AxonError::Config(format!("unknown provider type: {other}"))),
    }
}
