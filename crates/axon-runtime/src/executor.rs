use std::sync::Arc;

use chrono::Utc;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use axon_core::{AgentDefinition, AxonError, ChatMessage, Result, TokenUsage, ToolCall};
use axon_gateway::{EmbeddedGateway, StreamEvent};
use axon_store::{MessageRecord, Store};
use axon_tools::ToolRegistry;

pub struct AgentExecutor {
    gateway: Arc<EmbeddedGateway>,
    tools: Arc<ToolRegistry>,
    store: Arc<Store>,
    working_dir: std::path::PathBuf,
}

impl AgentExecutor {
    pub fn new(
        gateway: Arc<EmbeddedGateway>,
        tools: Arc<ToolRegistry>,
        store: Arc<Store>,
        working_dir: std::path::PathBuf,
    ) -> Self {
        AgentExecutor {
            gateway,
            tools,
            store,
            working_dir,
        }
    }

    pub async fn invoke(
        &self,
        agent: &AgentDefinition,
        input: &str,
        conversation_id: Option<&str>,
    ) -> Result<InvokeResult> {
        let conv_id = match conversation_id {
            Some(id) => id.to_string(),
            None => {
                self.store
                    .create_conversation(&agent.id, Some(&agent.name))?
                    .id
            }
        };

        let user_msg = MessageRecord::new(&conv_id, None, "user", input);
        let parent_id = user_msg.id.clone();
        self.store.add_message(&user_msg)?;

        let mut messages = vec![ChatMessage::system(&agent.system_prompt)];
        let history = self.store.get_messages(&conv_id)?;
        for m in &history {
            if m.role == "system" {
                continue;
            }
            messages.push(ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_calls: m.tool_calls.clone(),
                tool_call_id: m.tool_call_id.clone(),
                name: None,
            });
        }

        let mut pipeline = crate::pipeline::GenerationPipeline::new(
            &self.gateway,
            &self.tools,
            agent,
            messages,
            &conv_id,
            &parent_id,
            self.working_dir.clone(),
        );

        let mut output = String::new();
        let mut total_usage = TokenUsage::default();
        let mut iterations = 0u32;
        let mut tool_calls = Vec::new();
        let mut seen_tool_in_round = false;

        let mut stream = pipeline.run().await;
        while let Some(event) = stream.next().await {
            match event {
                StreamEvent::TextChunk { text } => output.push_str(&text),
                StreamEvent::UsageUpdate { usage } => {
                    total_usage.prompt_tokens += usage.prompt_tokens;
                    total_usage.completion_tokens += usage.completion_tokens;
                    total_usage.total_tokens += usage.total_tokens;
                }
                StreamEvent::ToolCallRequest { tool_call } => {
                    if !seen_tool_in_round {
                        iterations += 1;
                        seen_tool_in_round = true;
                    }
                    tool_calls.push(ToolCallRecord {
                        id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        arguments: tool_call.function.arguments.clone(),
                        result: String::new(),
                        tool_call_id: String::new(),
                    });
                }
                StreamEvent::ToolCallResult {
                    tool_call_id,
                    result,
                    ..
                } => {
                    if let Some(tc) = tool_calls.last_mut() {
                        tc.result = result;
                        tc.tool_call_id = tool_call_id;
                    }
                    seen_tool_in_round = false;
                }
                StreamEvent::Error { message } => {
                    return Err(AxonError::Internal(format!(
                        "agent execution error: {message}"
                    )));
                }
                StreamEvent::Done { .. } => {
                    if iterations == 0 {
                        iterations = 1;
                    }
                    break;
                }
                _ => {}
            }
        }

        let assistant_msg = MessageRecord {
            id: Uuid::new_v4().to_string(),
            conversation_id: conv_id.clone(),
            parent_id: Some(parent_id),
            role: "assistant".into(),
            content: output.clone(),
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(
                    tool_calls
                        .iter()
                        .map(|tc| ToolCall {
                            id: tc.id.clone(),
                            call_type: "function".into(),
                            function: axon_core::ToolCallFunction {
                                name: tc.name.clone(),
                                arguments: tc.arguments.clone(),
                            },
                        })
                        .collect(),
                )
            },
            tool_call_id: None,
            model: Some(agent.model.clone()),
            usage: Some(total_usage.clone()),
            created_at: Utc::now().timestamp(),
        };
        self.store.add_message(&assistant_msg)?;

        self.store.record_usage(&axon_store::UsageRecord {
            conversation_id: Some(conv_id.clone()),
            agent_id: Some(agent.id.clone()),
            model: Some(agent.model.clone()),
            prompt_tokens: total_usage.prompt_tokens,
            completion_tokens: total_usage.completion_tokens,
            total_tokens: total_usage.total_tokens,
            duration_ms: 0,
            timestamp: Utc::now().timestamp(),
        })?;

        Ok(InvokeResult {
            output,
            conversation_id: conv_id,
            usage: total_usage,
            iterations,
            tool_calls,
        })
    }

    pub async fn invoke_stream(
        self: Arc<Self>,
        agent: Arc<AgentDefinition>,
        input: String,
        conversation_id: Option<String>,
    ) -> Result<impl futures::Stream<Item = StreamEvent>> {
        let conv_id = match conversation_id {
            Some(id) => id,
            None => {
                self.store
                    .create_conversation(&agent.id, Some(&agent.name))?
                    .id
            }
        };

        let user_msg = MessageRecord::new(&conv_id, None, "user", &input);
        let parent_id = user_msg.id.clone();
        self.store.add_message(&user_msg)?;

        let mut messages = vec![ChatMessage::system(&agent.system_prompt)];
        let history = self.store.get_messages(&conv_id)?;
        for m in &history {
            if m.role == "system" {
                continue;
            }
            messages.push(ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_calls: m.tool_calls.clone(),
                tool_call_id: m.tool_call_id.clone(),
                name: None,
            });
        }

        let gateway = self.gateway.clone();
        let tools = self.tools.clone();
        let store = self.store.clone();
        let working_dir = self.working_dir.clone();
        let agent_for_stream = agent.clone();
        let conv_id_stream = conv_id.clone();
        let parent_id_stream = parent_id.clone();

        Ok(async_stream::stream! {
            let mut pipeline = crate::pipeline::GenerationPipeline::new(
                &gateway,
                &tools,
                &agent_for_stream,
                messages,
                &conv_id_stream,
                &parent_id_stream,
                working_dir,
            );

            let mut output = String::new();
            let mut total_usage = TokenUsage::default();
            let mut collected_tool_calls: Vec<ToolCall> = Vec::new();

            let mut stream = pipeline.run().await;
            while let Some(event) = stream.next().await {
                match &event {
                    StreamEvent::TextChunk { text } => output.push_str(text),
                    StreamEvent::UsageUpdate { usage } => {
                        total_usage.prompt_tokens += usage.prompt_tokens;
                        total_usage.completion_tokens += usage.completion_tokens;
                        total_usage.total_tokens += usage.total_tokens;
                    }
                    StreamEvent::ToolCallRequest { tool_call } => {
                        collected_tool_calls.push(tool_call.clone());
                    }
                    _ => {}
                }
                yield event;
            }

            let assistant_msg = MessageRecord {
                id: Uuid::new_v4().to_string(),
                conversation_id: conv_id_stream.clone(),
                parent_id: Some(parent_id_stream),
                role: "assistant".into(),
                content: output,
                tool_calls: if collected_tool_calls.is_empty() {
                    None
                } else {
                    Some(collected_tool_calls)
                },
                tool_call_id: None,
                model: Some(agent.model.clone()),
                usage: Some(total_usage.clone()),
                created_at: Utc::now().timestamp(),
            };
            let _ = store.add_message(&assistant_msg);

            let _ = store.record_usage(&axon_store::UsageRecord {
                conversation_id: Some(conv_id_stream),
                agent_id: Some(agent.id.clone()),
                model: Some(agent.model.clone()),
                prompt_tokens: total_usage.prompt_tokens,
                completion_tokens: total_usage.completion_tokens,
                total_tokens: total_usage.total_tokens,
                duration_ms: 0,
                timestamp: Utc::now().timestamp(),
            });
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeResult {
    pub output: String,
    pub conversation_id: String,
    pub usage: TokenUsage,
    pub iterations: u32,
    pub tool_calls: Vec<ToolCallRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub id: String,
    pub name: String,
    pub arguments: String,
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub tool_call_id: String,
}
