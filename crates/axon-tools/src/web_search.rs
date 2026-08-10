use async_trait::async_trait;
use serde_json::json;

use axon_core::Result;

use crate::registry::{ToolContext, ToolProvider, ToolResult};

pub struct WebSearchTool {
    max_results: usize,
    client: reqwest::Client,
}

impl WebSearchTool {
    pub fn new(max_results: usize) -> Self {
        WebSearchTool {
            max_results,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

#[async_trait]
impl ToolProvider for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web using DuckDuckGo and return relevant results. Input: { \"query\": string, \"max_results\"?: number }"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "max_results": {
                    "type": "number",
                    "description": "Maximum number of results to return"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult> {
        let query = input.get("query").and_then(|q| q.as_str()).ok_or_else(|| {
            axon_core::AxonError::Tool("web_search: missing 'query' field".into())
        })?;

        let max = input
            .get("max_results")
            .and_then(|m| m.as_u64())
            .map(|m| m as usize)
            .unwrap_or(self.max_results);

        let encoded_query: String = query
            .chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                ' ' => "+".to_string(),
                _ => format!("%{:02X}", c as u32),
            })
            .collect();

        let url = format!("https://html.duckduckgo.com/html/?q={encoded_query}");

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (compatible; AXON/1.0)")
            .send()
            .await
            .map_err(|e| axon_core::AxonError::Tool(format!("web_search request: {e}")))?;

        let html = resp
            .text()
            .await
            .map_err(|e| axon_core::AxonError::Tool(format!("web_search read: {e}")))?;

        let results = parse_ddg_html(&html, max);

        if results.is_empty() {
            return Ok(ToolResult::ok(format!("No results found for: {query}")));
        }

        let mut output = String::new();
        for (i, r) in results.iter().enumerate() {
            output.push_str(&format!(
                "{}. {}\n   {}\n   {}\n\n",
                i + 1,
                r.title,
                r.body,
                r.href
            ));
        }

        Ok(ToolResult {
            content: output,
            metadata: json!({ "result_count": results.len(), "query": query }),
            is_error: false,
        })
    }
}

struct ParsedResult {
    title: String,
    href: String,
    body: String,
}

fn parse_ddg_html(html: &str, max: usize) -> Vec<ParsedResult> {
    let mut results = Vec::new();
    let mut iter = html.split("class=\"result__a\"");
    iter.next();

    for _ in 0..max {
        let rest = match iter.next() {
            Some(r) => r,
            None => break,
        };

        let href = extract_between(rest, "href=\"", "\"")
            .unwrap_or_default()
            .replace("https://duckduckgo.com/l/?uddg=", "");
        let title = extract_between(rest, ">", "</a>")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let body = extract_after(rest, "class=\"result__snippet\">")
            .and_then(|s| extract_between(s, "", "<"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        if !title.is_empty() {
            results.push(ParsedResult { title, href, body });
        }
    }
    results
}

fn extract_between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_idx = if start.is_empty() {
        0
    } else {
        s.find(start)? + start.len()
    };
    let rest = &s[start_idx..];
    let end_idx = if end.is_empty() {
        rest.len()
    } else {
        rest.find(end)?
    };
    Some(&rest[..end_idx])
}

fn extract_after<'a>(s: &'a str, marker: &str) -> Option<&'a str> {
    let idx = s.find(marker)?;
    Some(&s[idx + marker.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_between_normal() {
        let s = r#"<a href="https://example.com">Example</a>"#;
        let result = extract_between(s, "href=\"", "\"");
        assert_eq!(result, Some("https://example.com"));
    }

    #[test]
    fn test_extract_between_empty_start() {
        let s = "hello world";
        let result = extract_between(s, "", " ");
        assert_eq!(result, Some("hello"));
    }

    #[test]
    fn test_extract_between_empty_end() {
        let s = "hello world";
        let result = extract_between(s, "hello ", "");
        assert_eq!(result, Some("world"));
    }

    #[test]
    fn test_extract_between_not_found() {
        let s = "hello";
        assert!(extract_between(s, "xyz", "abc").is_none());
    }

    #[test]
    fn test_extract_after() {
        let s = "prefix::suffix";
        let result = extract_after(s, "::");
        assert_eq!(result, Some("suffix"));
    }

    #[test]
    fn test_extract_after_not_found() {
        let s = "hello";
        assert!(extract_after(s, "xyz").is_none());
    }

    #[test]
    fn test_parse_ddg_html_empty() {
        let results = parse_ddg_html("", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_ddg_html_basic() {
        let html = r#"
        <a class="result__a" href="https://duckduckgo.com/l/?uddg=https%3A%2F%2Frust-lang.org">Rust Programming</a>
        <a class="result__a" href="https://duckduckgo.com/l/?uddg=https%3A%2F%2Fdoc.rust-lang.org">Rust Docs</a>
        "#;
        let results = parse_ddg_html(html, 5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Rust Programming");
    }

    #[test]
    fn test_parse_ddg_html_max_results() {
        let html = r#"
        <a class="result__a" href="https://a.com">A</a>
        <a class="result__a" href="https://b.com">B</a>
        <a class="result__a" href="https://c.com">C</a>
        "#;
        let results = parse_ddg_html(html, 2);
        assert_eq!(results.len(), 2);
    }
}
