use futures::Stream;
use serde::{Deserialize, Serialize};

use axon_core::{TokenUsage, ToolCall};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    TextChunk {
        text: String,
    },
    ThoughtChunk {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        signature: Option<String>,
    },
    ToolCallUpdate {
        stream_key: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        id: Option<String>,
        name: String,
        arguments: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        signature: Option<String>,
    },
    ToolCallRequest {
        tool_call: ToolCall,
    },
    ToolCallResult {
        tool_call_id: String,
        result: String,
        is_error: bool,
    },
    UsageUpdate {
        usage: TokenUsage,
    },
    Error {
        message: String,
    },
    Retrying {
        attempt: u32,
        max_attempts: u32,
    },
    Done {
        finish_reason: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub usage: TokenUsage,
    pub model: String,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatChunk {
    Text { text: String },
    ToolCall { tool_call: ToolCall },
    Usage { usage: TokenUsage },
    Done { finish_reason: Option<String> },
}

fn parse_usage(usage: &serde_json::Value) -> TokenUsage {
    let prompt_tokens_details = usage
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|t| t.as_u64())
        .map(|c| axon_core::PromptTokensDetails {
            cached_tokens: Some(c as u32),
        });
    let completion_tokens_details = usage
        .get("completion_tokens_details")
        .and_then(|d| d.get("reasoning_tokens"))
        .and_then(|t| t.as_u64())
        .map(|r| axon_core::CompletionTokensDetails {
            reasoning_tokens: Some(r as u32),
        });
    TokenUsage {
        prompt_tokens: usage
            .get("prompt_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32,
        completion_tokens: usage
            .get("completion_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32,
        total_tokens: usage
            .get("total_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32,
        prompt_tokens_details,
        prompt_cache_hit_tokens: usage
            .get("prompt_cache_hit_tokens")
            .and_then(|t| t.as_u64())
            .map(|v| v as u32),
        prompt_cache_miss_tokens: usage
            .get("prompt_cache_miss_tokens")
            .and_then(|t| t.as_u64())
            .map(|v| v as u32),
        completion_tokens_details,
    }
}

pub fn parse_openai_sse(resp: reqwest::Response) -> impl Stream<Item = StreamEvent> + Send {
    async_stream::stream! {
        let mut bytes = resp.bytes_stream();
        let mut buffer = String::new();
        let mut tool_call_args: std::collections::HashMap<u32, (String, String)> = std::collections::HashMap::new();
        let mut tool_call_names: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
        let mut tool_call_ids: std::collections::HashMap<u32, String> = std::collections::HashMap::new();

        use futures::StreamExt;
        while let Some(chunk_result) = bytes.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    yield StreamEvent::Error { message: format!("stream error: {e}") };
                    return;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        for (&idx, (name, args)) in &tool_call_args {
                            let id = tool_call_ids.get(&idx).cloned().unwrap_or_else(|| format!("call_{idx}"));
                            yield StreamEvent::ToolCallRequest {
                                tool_call: ToolCall {
                                    id,
                                    call_type: "function".into(),
                                    function: axon_core::ToolCallFunction {
                                        name: name.clone(),
                                        arguments: args.clone(),
                                    },
                                },
                            };
                        }
                        yield StreamEvent::Done { finish_reason: Some("stop".into()) };
                        return;
                    }

                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(outcome) = v.get("outcome").and_then(|o| o.as_str()) {
                            if !outcome.is_empty() && outcome != "success" {
                                yield StreamEvent::Error {
                                    message: format!("upstream outcome: {outcome}"),
                                };
                            }
                        }
                        if let Some(err) = v.get("error") {
                            if let Some(msg) = err.get("message").and_then(|m| m.as_str()) {
                                yield StreamEvent::Error { message: msg.into() };
                            }
                        }
                        if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
                            for choice in choices {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                        if !content.is_empty() {
                                            yield StreamEvent::TextChunk { text: content.into() };
                                        }
                                    }
                                    if let Some(rc) = delta.get("reasoning_content").and_then(|c| c.as_str()) {
                                        if !rc.is_empty() {
                                            yield StreamEvent::ThoughtChunk { text: rc.into(), title: None, signature: None };
                                        }
                                    }
                                    if let Some(r) = delta.get("reasoning").and_then(|c| c.as_str()) {
                                        if !r.is_empty() {
                                            yield StreamEvent::ThoughtChunk { text: r.into(), title: None, signature: None };
                                        }
                                    }
                                    if let Some(details) = delta.get("reasoning_details").and_then(|d| d.as_array()) {
                                        for detail in details {
                                            if let Some(text) = detail.get("text").and_then(|t| t.as_str()) {
                                                if !text.is_empty() {
                                                    yield StreamEvent::ThoughtChunk { text: text.into(), title: None, signature: None };
                                                }
                                            }
                                        }
                                    }
                                    if let Some(tool_calls) = delta.get("tool_calls").and_then(|c| c.as_array()) {
                                        for tc in tool_calls {
                                            let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
                                            if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                                tool_call_ids.insert(idx, id.into());
                                            }
                                            if let Some(func) = tc.get("function") {
                                                if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                                                    tool_call_names.insert(idx, name.into());
                                                }
                                                if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                                                    let entry = tool_call_args.entry(idx).or_insert_with(|| (String::new(), String::new()));
                                                    entry.0 = tool_call_names.get(&idx).cloned().unwrap_or_default();
                                                    entry.1.push_str(args);
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(finish) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                                    if !finish.is_empty() && finish != "null" {
                                        for (&idx, (name, args)) in &tool_call_args {
                                            let id = tool_call_ids.get(&idx).cloned().unwrap_or_else(|| format!("call_{idx}"));
                                            yield StreamEvent::ToolCallRequest {
                                                tool_call: ToolCall {
                                                    id,
                                                    call_type: "function".into(),
                                                    function: axon_core::ToolCallFunction {
                                                        name: name.clone(),
                                                        arguments: args.clone(),
                                                    },
                                                },
                                            };
                                        }
                                        tool_call_args.clear();
                                        yield StreamEvent::Done { finish_reason: Some(finish.into()) };
                                    }
                                }
                            }
                        }
                        if let Some(usage) = v.get("usage") {
                            yield StreamEvent::UsageUpdate {
                                usage: parse_usage(usage),
                            };
                        }
                    }
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<AnthropicDelta>,
    #[serde(default)]
    message: Option<AnthropicMessageStart>,
    #[serde(default)]
    content_block: Option<AnthropicContentBlock>,
    #[serde(default)]
    partial_json: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicDelta {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicMessageStart {
    #[serde(default)]
    usage: Option<AnthropicUsageStart>,
}

#[derive(Deserialize)]
struct AnthropicUsageStart {
    #[serde(default)]
    input_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

pub fn parse_anthropic_sse(resp: reqwest::Response) -> impl Stream<Item = StreamEvent> + Send {
    async_stream::stream! {
        let mut bytes = resp.bytes_stream();
        let mut buffer = String::new();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_args = String::new();

        use futures::StreamExt;
        while let Some(chunk_result) = bytes.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    yield StreamEvent::Error { message: format!("stream error: {e}") };
                    return;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                        match event.event_type.as_str() {
                            "message_start" => {
                                if let Some(msg) = event.message {
                                if let Some(usage) = msg.usage {
                                    yield StreamEvent::UsageUpdate {
                                        usage: TokenUsage {
                                            prompt_tokens: usage.input_tokens,
                                            completion_tokens: 0,
                                            total_tokens: usage.input_tokens,
                                            ..Default::default()
                                        },
                                    };
                                }
                                }
                            }
                            "content_block_start" => {
                                if let Some(block) = event.content_block {
                                    if block.block_type == "tool_use" {
                                        current_tool_id = block.id.unwrap_or_default();
                                        current_tool_name = block.name.unwrap_or_default();
                                        current_tool_args.clear();
                                    }
                                }
                            }
                            "content_block_delta" => {
                                if let Some(delta) = event.delta {
                                    if let Some(text) = delta.text {
                                        if !text.is_empty() {
                                            yield StreamEvent::TextChunk { text };
                                        }
                                    }
                                }
                                if let Some(pj) = event.partial_json {
                                    current_tool_args.push_str(&pj);
                                }
                            }
                            "content_block_stop" => {
                                if !current_tool_name.is_empty() {
                                    yield StreamEvent::ToolCallRequest {
                                        tool_call: ToolCall {
                                            id: current_tool_id.clone(),
                                            call_type: "function".into(),
                                            function: axon_core::ToolCallFunction {
                                                name: current_tool_name.clone(),
                                                arguments: current_tool_args.clone(),
                                            },
                                        },
                                    };
                                    current_tool_id.clear();
                                    current_tool_name.clear();
                                    current_tool_args.clear();
                                }
                            }
                            "message_delta" => {
                                if let Some(delta) = event.delta {
                                    if let Some(reason) = delta.stop_reason {
                                        yield StreamEvent::Done { finish_reason: Some(reason) };
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
