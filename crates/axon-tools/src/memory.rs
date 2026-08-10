use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use axon_core::Result;
use axon_store::Store;

use crate::registry::{ToolContext, ToolProvider, ToolResult};

pub struct MemoryTool {
    store: Arc<Store>,
    namespace: String,
}

impl MemoryTool {
    pub fn new(store: Arc<Store>, namespace: impl Into<String>) -> Self {
        MemoryTool {
            store,
            namespace: namespace.into(),
        }
    }
}

#[async_trait]
impl ToolProvider for MemoryTool {
    fn name(&self) -> &str {
        "memory"
    }

    fn description(&self) -> &str {
        "Read or write persistent memory (key-value store). Input: { \"action\": \"get\"|\"set\"|\"delete\"|\"list\", \"key\"?: string, \"value\"?: string }"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["get", "set", "delete", "list"], "description": "The memory operation" },
                "key": { "type": "string", "description": "The memory key (required for get/set/delete)" },
                "value": { "type": "string", "description": "The value to set (required for set)" }
            },
            "required": ["action"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult> {
        let action = input
            .get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| axon_core::AxonError::Tool("memory: missing 'action' field".into()))?;

        match action {
            "get" => {
                let key = input
                    .get("key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| axon_core::AxonError::Tool("memory: missing 'key'".into()))?;
                match self.store.get_memory(key, &self.namespace)? {
                    Some(value) => Ok(ToolResult::ok(value)),
                    None => Ok(ToolResult::ok(format!("(no value for key '{key}')"))),
                }
            }
            "set" => {
                let key = input
                    .get("key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| axon_core::AxonError::Tool("memory: missing 'key'".into()))?;
                let value = input
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| axon_core::AxonError::Tool("memory: missing 'value'".into()))?;
                self.store.set_memory(key, value, &self.namespace)?;
                Ok(ToolResult {
                    content: format!("OK: set '{key}'"),
                    metadata: json!({ "key": key, "bytes": value.len() }),
                    is_error: false,
                })
            }
            "delete" => {
                let key = input
                    .get("key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| axon_core::AxonError::Tool("memory: missing 'key'".into()))?;
                self.store.delete_memory(key, &self.namespace)?;
                Ok(ToolResult::ok(format!("OK: deleted '{key}'")))
            }
            "list" => {
                let entries = self.store.list_memory(&self.namespace)?;
                if entries.is_empty() {
                    return Ok(ToolResult::ok("(memory is empty)"));
                }
                let mut output = String::new();
                for e in &entries {
                    output.push_str(&format!("{}: {}\n", e.key, e.value));
                }
                Ok(ToolResult {
                    content: output,
                    metadata: json!({ "count": entries.len() }),
                    is_error: false,
                })
            }
            other => Ok(ToolResult::err(format!("unknown action: {other}"))),
        }
    }
}
