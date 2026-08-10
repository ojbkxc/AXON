use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tokio::process::Command;

use axon_core::Result;

use crate::registry::{ToolContext, ToolProvider, ToolResult};

pub struct CodeExecTool {
    timeout_ms: u64,
    max_output_chars: usize,
    allowed_languages: Vec<String>,
}

impl CodeExecTool {
    pub fn new(timeout_ms: u64, max_output_chars: usize, allowed_languages: Vec<String>) -> Self {
        CodeExecTool {
            timeout_ms,
            max_output_chars,
            allowed_languages,
        }
    }

    pub fn default_config() -> Self {
        CodeExecTool {
            timeout_ms: 10_000,
            max_output_chars: 10_000,
            allowed_languages: vec!["python".into(), "javascript".into()],
        }
    }
}

#[async_trait]
impl ToolProvider for CodeExecTool {
    fn name(&self) -> &str {
        "code_exec"
    }

    fn description(&self) -> &str {
        "Execute code in a supported language and return the output. Input: { \"language\": string, \"code\": string }"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "language": { "type": "string", "description": "Programming language (python, javascript)" },
                "code": { "type": "string", "description": "The code to execute" }
            },
            "required": ["language", "code"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult> {
        let language = input
            .get("language")
            .and_then(|l| l.as_str())
            .ok_or_else(|| axon_core::AxonError::Tool("code_exec: missing 'language'".into()))?;

        let code = input
            .get("code")
            .and_then(|c| c.as_str())
            .ok_or_else(|| axon_core::AxonError::Tool("code_exec: missing 'code'".into()))?;

        if !self.allowed_languages.is_empty()
            && !self.allowed_languages.iter().any(|l| l == language)
        {
            return Ok(ToolResult::err(format!(
                "language '{language}' is not allowed. Allowed: {:?}",
                self.allowed_languages
            )));
        }

        match language {
            "python" => self.exec_python(code).await,
            "javascript" | "js" => self.exec_javascript(code).await,
            other => Ok(ToolResult::err(format!(
                "unsupported language: {other}"
            ))),
        }
    }
}

impl CodeExecTool {
    async fn exec_python(&self, code: &str) -> Result<ToolResult> {
        let mut cmd = Command::new("python3");
        cmd.arg("-c").arg(code);
        self.run_command(&mut cmd, "python").await
    }

    async fn exec_javascript(&self, code: &str) -> Result<ToolResult> {
        let mut cmd = Command::new("node");
        cmd.arg("-e").arg(code);
        self.run_command(&mut cmd, "javascript").await
    }

    async fn run_command(
        &self,
        cmd: &mut Command,
        language: &str,
    ) -> Result<ToolResult> {
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| axon_core::AxonError::Tool(format!("code_exec spawn: {e}")))?;

        let output = tokio::time::timeout(Duration::from_millis(self.timeout_ms), child.wait_with_output())
            .await
            .map_err(|_| {
                axon_core::AxonError::Tool(format!(
                    "code_exec timed out after {}ms",
                    self.timeout_ms
                ))
            })?
            .map_err(|e| axon_core::AxonError::Tool(format!("code_exec wait: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        let mut content = if stderr.is_empty() {
            stdout
        } else {
            format!("{stdout}\n[stderr]\n{stderr}")
        };

        if content.len() > self.max_output_chars {
            content.truncate(self.max_output_chars);
            content.push_str("\n... [output truncated]");
        }

        Ok(ToolResult {
            content,
            metadata: json!({ "exit_code": exit_code, "language": language }),
            is_error: !output.status.success(),
        })
    }
}
