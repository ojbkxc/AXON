use serde::{Deserialize, Serialize};

use crate::models::{AgentDefinition, ModelDefinition, RouteDefinition, ToolDefinition};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxonConfig {
    pub server: ServerConfig,
    pub gateway: GatewayConfig,
    pub storage: StorageConfig,
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub models: Vec<ModelDefinition>,
    #[serde(default)]
    pub routes: Vec<RouteDefinition>,
    #[serde(default)]
    pub agents: Vec<AgentDefinition>,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
}

impl Default for AxonConfig {
    fn default() -> Self {
        AxonConfig {
            server: ServerConfig::default(),
            gateway: GatewayConfig::default(),
            storage: StorageConfig::default(),
            observability: ObservabilityConfig::default(),
            models: vec![],
            routes: vec![],
            agents: vec![],
            tools: vec![],
        }
    }
}

impl AxonConfig {
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("yaml");

        let config: AxonConfig = match ext {
            "json" => serde_json::from_str(&content)?,
            _ => serde_yaml::from_str(&content)?,
        };

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> crate::Result<()> {
        if self.server.addr.is_empty() {
            return Err(crate::AxonError::Validation("server.addr is empty".into()));
        }
        if self.storage.sqlite_path.is_empty() {
            return Err(crate::AxonError::Validation(
                "storage.sqlite_path is empty".into(),
            ));
        }
        for agent in &self.agents {
            if agent.id.is_empty() {
                return Err(crate::AxonError::Validation("agent.id is empty".into()));
            }
            if agent.max_iterations == 0 {
                return Err(crate::AxonError::Validation(format!(
                    "agent '{}' has max_iterations=0",
                    agent.id
                )));
            }
        }
        Ok(())
    }

    pub fn find_agent(&self, id: &str) -> Option<&AgentDefinition> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub fn find_model(&self, name: &str) -> Option<&ModelDefinition> {
        self.models.iter().find(|m| m.name == name)
    }

    pub fn find_route(&self, name: &str) -> Option<&RouteDefinition> {
        self.routes.iter().find(|r| r.name == name)
    }

    pub fn resolve_model(&self, reference: &str) -> Option<&ModelDefinition> {
        if let Some(model) = self.find_model(reference) {
            return Some(model);
        }
        if let Some(route) = self.find_route(reference) {
            if let Some(first) = route.targets.first() {
                return self.find_model(&first.model);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(default = "default_true")]
    pub web_ui_enabled: bool,
    #[serde(default = "default_max_body")]
    pub max_request_body_mb: usize,
    #[serde(default = "default_workers")]
    pub workers: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            addr: default_addr(),
            web_ui_enabled: true,
            max_request_body_mb: default_max_body(),
            workers: default_workers(),
        }
    }
}

fn default_addr() -> String {
    "0.0.0.0:8080".into()
}
fn default_true() -> bool {
    true
}
fn default_max_body() -> usize {
    10
}
fn default_workers() -> usize {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayConfig {
    #[serde(default = "default_upstream_timeout")]
    pub upstream_timeout_ms: u64,
    #[serde(default = "default_stream_timeout")]
    pub stream_timeout_ms: u64,
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,
    #[serde(default = "default_pool_idle")]
    pub pool_max_idle_per_host: usize,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        GatewayConfig {
            upstream_timeout_ms: default_upstream_timeout(),
            stream_timeout_ms: default_stream_timeout(),
            retry_count: default_retry_count(),
            retry_delay_ms: default_retry_delay(),
            pool_max_idle_per_host: default_pool_idle(),
        }
    }
}

fn default_upstream_timeout() -> u64 {
    120000
}
fn default_stream_timeout() -> u64 {
    300000
}
fn default_retry_count() -> u32 {
    2
}
fn default_retry_delay() -> u64 {
    1000
}
fn default_pool_idle() -> usize {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    #[serde(default = "default_sqlite_path")]
    pub sqlite_path: String,
    #[serde(default = "default_max_conn")]
    pub max_connections: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            sqlite_path: default_sqlite_path(),
            max_connections: default_max_conn(),
        }
    }
}

fn default_sqlite_path() -> String {
    "axon.db".into()
}
fn default_max_conn() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservabilityConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,
    #[serde(default = "default_true")]
    pub access_log_enabled: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        ObservabilityConfig {
            log_level: default_log_level(),
            metrics_enabled: true,
            access_log_enabled: true,
        }
    }
}

fn default_log_level() -> String {
    "info".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RouteTarget;

    fn sample_config() -> AxonConfig {
        AxonConfig {
            server: ServerConfig::default(),
            gateway: GatewayConfig::default(),
            storage: StorageConfig::default(),
            observability: ObservabilityConfig::default(),
            models: vec![ModelDefinition {
                name: "gpt-4".into(),
                provider: "openai".into(),
                model_name: "gpt-4o".into(),
                api_key: Some("sk-test".into()),
                api_key_env: None,
                api_base: None,
                max_concurrency: None,
                rate_limit: None,
            }],
            routes: vec![RouteDefinition {
                name: "default".into(),
                strategy: "failover".into(),
                targets: vec![RouteTarget {
                    model: "gpt-4".into(),
                    weight: 1,
                    tags: vec![],
                }],
            }],
            agents: vec![AgentDefinition {
                id: "translator".into(),
                name: "Translator".into(),
                description: "Translate text".into(),
                system_prompt: "You are a translator.".into(),
                model: "gpt-4".into(),
                tools: vec![],
                max_iterations: 5,
                temperature: None,
                max_tokens: None,
                metadata: serde_json::Value::Null,
            }],
            tools: vec![],
        }
    }

    #[test]
    fn test_default_config_validates() {
        let config = AxonConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_addr() {
        let mut config = AxonConfig::default();
        config.server.addr = "".into();
        assert!(matches!(
            config.validate(),
            Err(crate::AxonError::Validation(_))
        ));
    }

    #[test]
    fn test_validate_empty_sqlite_path() {
        let mut config = AxonConfig::default();
        config.storage.sqlite_path = "".into();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_agent_max_iterations_zero() {
        let mut config = sample_config();
        config.agents[0].max_iterations = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_find_agent() {
        let config = sample_config();
        assert!(config.find_agent("translator").is_some());
        assert!(config.find_agent("nonexistent").is_none());
    }

    #[test]
    fn test_find_model() {
        let config = sample_config();
        assert!(config.find_model("gpt-4").is_some());
        assert!(config.find_model("claude").is_none());
    }

    #[test]
    fn test_resolve_model_direct() {
        let config = sample_config();
        let resolved = config.resolve_model("gpt-4");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().model_name, "gpt-4o");
    }

    #[test]
    fn test_resolve_model_via_route() {
        let config = sample_config();
        let resolved = config.resolve_model("default");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "gpt-4");
    }

    #[test]
    fn test_resolve_model_not_found() {
        let config = sample_config();
        assert!(config.resolve_model("unknown").is_none());
    }

    #[test]
    fn test_from_file_yaml() {
        let yaml = r#"
server:
  addr: "127.0.0.1:9090"
gateway:
  upstream_timeout_ms: 60000
storage:
  sqlite_path: ":memory:"
observability:
  log_level: "debug"
"#;
        let path = "/tmp/axon_test_config.yaml";
        std::fs::write(path, yaml).unwrap();
        let config = AxonConfig::from_file(path).unwrap();
        assert_eq!(config.server.addr, "127.0.0.1:9090");
        assert_eq!(config.gateway.upstream_timeout_ms, 60000);
        std::fs::remove_file(path).ok();
    }
}
