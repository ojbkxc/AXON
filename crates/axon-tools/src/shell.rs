use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tokio::process::Command;

use axon_core::Result;

use crate::registry::{ToolContext, ToolProvider, ToolResult};

pub struct ShellTool {
    timeout_ms: u64,
    allowed_commands: Vec<String>,
}

impl ShellTool {
    pub fn new(timeout_ms: u64, allowed_commands: Vec<String>) -> Self {
        ShellTool {
            timeout_ms,
            allowed_commands,
        }
    }

    pub fn unrestricted(timeout_ms: u64) -> Self {
        ShellTool {
            timeout_ms,
            allowed_commands: vec![],
        }
    }
}

#[async_trait]
impl ToolProvider for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return stdout/stderr. Input: { \"command\": string, \"args\"?: string[] }"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The command to execute" },
                "args": { "type": "array", "items": { "type": "string" }, "description": "Command arguments" }
            },
            "required": ["command"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        let command = input
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| axon_core::AxonError::Tool("shell: missing 'command' field".into()))?;

        if !self.allowed_commands.is_empty() && !self.allowed_commands.iter().any(|c| c == command) {
            return Ok(ToolResult::err(format!(
                "command '{command}' is not in the allowed list"
            )));
        }

        let args: Vec<String> = input
            .get("args")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut cmd = Command::new(command);
        cmd.args(&args);
        cmd.current_dir(&context.working_dir);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| axon_core::AxonError::Tool(format!("shell spawn: {e}")))?;

        let output = tokio::time::timeout(Duration::from_millis(self.timeout_ms), child.wait_with_output())
            .await
            .map_err(|_| {
                axon_core::AxonError::Tool(format!(
                    "shell command timed out after {}ms",
                    self.timeout_ms
                ))
            })?
            .map_err(|e| axon_core::AxonError::Tool(format!("shell wait: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        let content = if stderr.is_empty() {
            stdout
        } else {
            format!("{stdout}\n[stderr]\n{stderr}")
        };

        Ok(ToolResult {
            content,
            metadata: json!({ "exit_code": exit_code, "command": command }),
            is_error: !output.status.success(),
        })
    }
}
