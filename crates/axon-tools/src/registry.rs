use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use axon_core::{AxonError, Result, ToolSchema};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub is_error: bool,
}

impl ToolResult {
    pub fn ok(content: impl Into<String>) -> Self {
        ToolResult {
            content: content.into(),
            metadata: serde_json::Value::Null,
            is_error: false,
        }
    }

    pub fn err(content: impl Into<String>) -> Self {
        ToolResult {
            content: content.into(),
            metadata: serde_json::Value::Null,
            is_error: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub agent_id: String,
    pub conversation_id: String,
    pub working_dir: std::path::PathBuf,
}

#[async_trait]
pub trait ToolProvider: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;

    async fn execute(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ToolProvider>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn ToolProvider>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn ToolProvider>> {
        self.tools.get(name)
    }

    pub fn list(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|t| ToolInfo {
                name: t.name().into(),
                description: t.description().into(),
                parameters: t.parameters_schema(),
            })
            .collect()
    }

    pub fn schemas_for(&self, names: &[String]) -> Vec<ToolSchema> {
        let mut schemas = Vec::new();
        for name in names {
            if let Some(tool) = self.tools.get(name) {
                schemas.push(ToolSchema {
                    schema_type: "function".into(),
                    function: axon_core::ToolFunctionSchema {
                        name: tool.name().into(),
                        description: tool.description().into(),
                        parameters: tool.parameters_schema(),
                    },
                });
            }
        }
        schemas
    }

    pub async fn execute(
        &self,
        name: &str,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| AxonError::NotFound(format!("tool '{name}' not found")))?;
        tool.execute(input, context).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        ToolRegistry::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}
