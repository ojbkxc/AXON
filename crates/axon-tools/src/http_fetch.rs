use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use axon_core::Result;

use crate::registry::{ToolContext, ToolProvider, ToolResult};

pub struct HttpFetchTool {
    max_response_chars: usize,
    client: reqwest::Client,
}

impl HttpFetchTool {
    pub fn new(timeout_ms: u64, max_response_chars: usize) -> Self {
        HttpFetchTool {
            max_response_chars,
            client: reqwest::Client::builder()
                .timeout(Duration::from_millis(timeout_ms))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

#[async_trait]
impl ToolProvider for HttpFetchTool {
    fn name(&self) -> &str {
        "http_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from an HTTP/HTTPS URL. Input: { \"url\": string, \"method\"?: string, \"headers\"?: object, \"body\"?: string }"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "The URL to fetch" },
                "method": { "type": "string", "description": "HTTP method (default GET)" },
                "headers": { "type": "object", "description": "Request headers" },
                "body": { "type": "string", "description": "Request body" }
            },
            "required": ["url"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult> {
        let url = input
            .get("url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| axon_core::AxonError::Tool("http_fetch: missing 'url' field".into()))?;

        let method = input
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let mut req = match method.as_str() {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "DELETE" => self.client.delete(url),
            "PATCH" => self.client.patch(url),
            "HEAD" => self.client.head(url),
            other => {
                return Ok(ToolResult::err(format!("unsupported method: {other}")));
            }
        };

        if let Some(headers) = input.get("headers").and_then(|h| h.as_object()) {
            for (k, v) in headers {
                if let Some(vs) = v.as_str() {
                    req = req.header(k, vs);
                }
            }
        }

        if let Some(body) = input.get("body").and_then(|b| b.as_str()) {
            req = req.body(body.to_string());
        }

        let resp = req
            .send()
            .await
            .map_err(|e| axon_core::AxonError::Tool(format!("http_fetch: {e}")))?;

        let status = resp.status();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = resp
            .text()
            .await
            .map_err(|e| axon_core::AxonError::Tool(format!("http_fetch read: {e}")))?;

        let original_length = body.len();
        let truncated = if original_length > self.max_response_chars {
            let cut = body
                .char_indices()
                .take_while(|(i, _)| *i <= self.max_response_chars)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(self.max_response_chars);
            let mut s = body[..cut].to_string();
            s.push_str("... [truncated]");
            s
        } else {
            body
        };

        Ok(ToolResult {
            content: truncated,
            metadata: json!({
                "status": status.as_u16(),
                "content_type": content_type,
                "truncated": original_length > self.max_response_chars,
                "original_length": original_length,
            }),
            is_error: !status.is_success(),
        })
    }
}
