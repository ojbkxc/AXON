//! axon-tools: 内置工具集 — web_search, shell, memory, http_fetch, code_exec

pub mod code_exec;
pub mod http_fetch;
pub mod memory;
pub mod registry;
pub mod shell;
pub mod web_search;

pub use code_exec::CodeExecTool;
pub use http_fetch::HttpFetchTool;
pub use memory::MemoryTool;
pub use registry::{ToolContext, ToolInfo, ToolProvider, ToolRegistry, ToolResult};
pub use shell::ShellTool;
pub use web_search::WebSearchTool;

use std::sync::Arc;

use axon_core::ToolDefinition;
use axon_store::Store;

pub fn build_registry(
    definitions: &[ToolDefinition],
    store: Arc<Store>,
) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    for def in definitions {
        let tool: Option<Arc<dyn ToolProvider>> = match def.kind.as_str() {
            "web_search" => {
                let max_results = def
                    .config
                    .get("max_results")
                    .and_then(|m| m.as_u64())
                    .map(|m| m as usize)
                    .unwrap_or(5);
                Some(Arc::new(WebSearchTool::new(max_results)))
            }
            "shell" => {
                let timeout_ms = def
                    .config
                    .get("timeout_ms")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(30_000);
                let allowed: Vec<String> = def
                    .config
                    .get("allowed_commands")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                Some(Arc::new(ShellTool::new(timeout_ms, allowed)))
            }
            "memory" => {
                let namespace = def
                    .config
                    .get("namespace")
                    .and_then(|n| n.as_str())
                    .unwrap_or("default")
                    .to_string();
                Some(Arc::new(MemoryTool::new(store.clone(), namespace)))
            }
            "http_fetch" => {
                let timeout_ms = def
                    .config
                    .get("timeout_ms")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(15_000);
                let max_chars = def
                    .config
                    .get("max_response_chars")
                    .and_then(|m| m.as_u64())
                    .map(|m| m as usize)
                    .unwrap_or(50_000);
                Some(Arc::new(HttpFetchTool::new(timeout_ms, max_chars)))
            }
            "code_exec" => {
                let timeout_ms = def
                    .config
                    .get("timeout_ms")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(10_000);
                let max_output = def
                    .config
                    .get("max_output_chars")
                    .and_then(|m| m.as_u64())
                    .map(|m| m as usize)
                    .unwrap_or(10_000);
                let allowed: Vec<String> = def
                    .config
                    .get("allowed_languages")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_else(|| vec!["python".into(), "javascript".into()]);
                Some(Arc::new(CodeExecTool::new(timeout_ms, max_output, allowed)))
            }
            _ => None,
        };

        if let Some(t) = tool {
            registry.register(t);
        }
    }

    registry
}
