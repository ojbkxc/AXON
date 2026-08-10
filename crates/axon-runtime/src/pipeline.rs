use std::sync::Arc;

use futures::stream::{BoxStream, StreamExt};
use futures::Stream;

use axon_core::{AgentDefinition, ChatMessage, ChatOptions, TokenUsage, ToolCall};
use axon_gateway::{EmbeddedGateway, StreamEvent};
use axon_tools::{ToolContext, ToolRegistry};

pub struct GenerationPipeline<'a> {
    gateway: &'a Arc<EmbeddedGateway>,
    tools: &'a Arc<ToolRegistry>,
    agent: &'a AgentDefinition,
    messages: Vec<ChatMessage>,
    conversation_id: String,
    working_dir: std::path::PathBuf,
    iterations: u32,
}

impl<'a> GenerationPipeline<'a> {
    pub fn new(
        gateway: &'a Arc<EmbeddedGateway>,
        tools: &'a Arc<ToolRegistry>,
        agent: &'a AgentDefinition,
        messages: Vec<ChatMessage>,
        conversation_id: &str,
        _parent_id: &str,
        working_dir: std::path::PathBuf,
    ) -> Self {
        GenerationPipeline {
            gateway,
            tools,
            agent,
            messages,
            conversation_id: conversation_id.to_string(),
            working_dir,
            iterations: 0,
        }
    }

    pub fn iterations(&self) -> u32 {
        self.iterations
    }

    pub async fn run(&mut self) -> BoxStream<'a, StreamEvent> {
        let gateway = self.gateway.clone();
        let tools = self.tools.clone();
        let agent = self.agent.clone();
        let mut messages = std::mem::take(&mut self.messages);
        let conversation_id = self.conversation_id.clone();
        let working_dir = self.working_dir.clone();

        let s = async_stream::stream! {
            let mut iterations = 0u32;
            loop {
                if iterations >= agent.max_iterations {
                    yield StreamEvent::Error {
                        message: format!("max iterations ({}) reached", agent.max_iterations),
                    };
                    yield StreamEvent::Done { finish_reason: Some("max_iterations".into()) };
                    return;
                }
                iterations += 1;

                let tool_schemas = tools.schemas_for(&agent.tools);
                let options = ChatOptions {
                    temperature: agent.temperature,
                    max_tokens: agent.max_tokens,
                    tools: tool_schemas,
                    stream: Some(true),
                };

                let stream = match gateway
                    .chat_stream(&agent.model, &messages, &options)
                    .await
                {
                    Ok(s) => s,
                    Err(e) => {
                        yield StreamEvent::Error { message: e.to_string() };
                        yield StreamEvent::Done { finish_reason: Some("error".into()) };
                        return;
                    }
                };

                let mut assistant_text = String::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();
                let mut usage = TokenUsage::default();
                let mut finish_reason: Option<String> = None;

                let mut pinned = stream;
                while let Some(event) = pinned.next().await {
                    match event {
                        StreamEvent::TextChunk { text } => {
                            assistant_text.push_str(&text);
                            yield StreamEvent::TextChunk { text };
                        }
                        StreamEvent::ToolCallRequest { tool_call } => {
                            tool_calls.push(tool_call);
                        }
                        StreamEvent::UsageUpdate { usage: u } => {
                            usage.prompt_tokens += u.prompt_tokens;
                            usage.completion_tokens += u.completion_tokens;
                            usage.total_tokens += u.total_tokens;
                            yield StreamEvent::UsageUpdate { usage: u };
                        }
                        StreamEvent::Done { finish_reason: fr } => {
                            finish_reason = fr;
                        }
                        StreamEvent::Error { message } => {
                            yield StreamEvent::Error { message };
                        }
                        _ => {}
                    }
                }

                if usage.prompt_tokens > 0 || usage.completion_tokens > 0 {
                    yield StreamEvent::UsageUpdate { usage: usage.clone() };
                }

                messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: assistant_text.clone(),
                    tool_calls: if tool_calls.is_empty() {
                        None
                    } else {
                        Some(tool_calls.clone())
                    },
                    tool_call_id: None,
                    name: None,
                });

                if tool_calls.is_empty() {
                    yield StreamEvent::Done { finish_reason };
                    return;
                }

                for call in &tool_calls {
                    yield StreamEvent::ToolCallRequest {
                        tool_call: call.clone(),
                    };

                    let input: serde_json::Value = if call.function.arguments.is_empty() {
                        serde_json::json!({})
                    } else {
                        serde_json::from_str(&call.function.arguments)
                            .unwrap_or(serde_json::json!({}))
                    };

                    let context = ToolContext {
                        agent_id: agent.id.clone(),
                        conversation_id: conversation_id.clone(),
                        working_dir: working_dir.clone(),
                    };

                    let result = tools.execute(&call.function.name, input, &context).await;

                    let (content, is_error) = match result {
                        Ok(r) => (r.content, r.is_error),
                        Err(e) => (e.to_string(), true),
                    };

                    yield StreamEvent::ToolCallResult {
                        tool_call_id: call.id.clone(),
                        result: content.clone(),
                        is_error,
                    };

                    messages.push(ChatMessage::tool(content, &call.id));
                }
            }
        };

        Box::pin(s)
    }
}
