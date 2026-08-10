use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use futures::Stream;
use parking_lot::RwLock;

use axon_core::{
    AxonError, ChatMessage, ChatOptions, ModelDefinition, Result, RouteDefinition, TokenUsage,
};

use crate::provider::{create_provider, Provider};
use crate::stream::{ChatResponse, StreamEvent};

pub struct EmbeddedGateway {
    snapshot: ArcSwap<GatewaySnapshot>,
    round_robin: RwLock<HashMap<String, usize>>,
}

pub struct GatewaySnapshot {
    pub models: HashMap<String, ModelDefinition>,
    pub routes: HashMap<String, RouteDefinition>,
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl GatewaySnapshot {
    fn empty() -> Self {
        GatewaySnapshot {
            models: HashMap::new(),
            routes: HashMap::new(),
            providers: HashMap::new(),
        }
    }
}

impl EmbeddedGateway {
    pub fn new() -> Arc<Self> {
        Arc::new(EmbeddedGateway {
            snapshot: ArcSwap::from_pointee(GatewaySnapshot::empty()),
            round_robin: RwLock::new(HashMap::new()),
        })
    }

    pub fn from_config(config: &axon_core::AxonConfig) -> Result<Arc<Self>> {
        let mut models = HashMap::new();
        let mut providers = HashMap::new();

        for model in &config.models {
            let api_key = model.resolve_api_key().ok_or_else(|| {
                AxonError::Config(format!(
                    "model '{}' has no api_key (neither api_key nor api_key_env resolved)",
                    model.name
                ))
            })?;
            let api_base = model.resolve_api_base();

            let provider = create_provider(&model.provider, &api_base, &api_key)?;
            providers.insert(model.name.clone(), Arc::from(provider));
            models.insert(model.name.clone(), model.clone());
        }

        let mut routes = HashMap::new();
        for route in &config.routes {
            routes.insert(route.name.clone(), route.clone());
        }

        let snapshot = GatewaySnapshot {
            models,
            routes,
            providers,
        };

        Ok(Arc::new(EmbeddedGateway {
            snapshot: ArcSwap::from_pointee(snapshot),
            round_robin: RwLock::new(HashMap::new()),
        }))
    }

    pub fn reload(&self, config: &axon_core::AxonConfig) -> Result<()> {
        let mut models = HashMap::new();
        let mut providers = HashMap::new();

        for model in &config.models {
            let api_key = model.resolve_api_key().ok_or_else(|| {
                AxonError::Config(format!(
                    "model '{}' has no api_key (neither api_key nor api_key_env resolved)",
                    model.name
                ))
            })?;
            let api_base = model.resolve_api_base();

            let provider = create_provider(&model.provider, &api_base, &api_key)?;
            providers.insert(model.name.clone(), Arc::from(provider));
            models.insert(model.name.clone(), model.clone());
        }

        let mut routes = HashMap::new();
        for route in &config.routes {
            routes.insert(route.name.clone(), route.clone());
        }

        let snapshot = GatewaySnapshot {
            models,
            routes,
            providers,
        };
        self.snapshot.store(Arc::new(snapshot));
        Ok(())
    }

    fn resolve_model_name(&self, reference: &str) -> Option<String> {
        let snap = self.snapshot.load();
        if snap.models.contains_key(reference) {
            return Some(reference.to_string());
        }
        if let Some(route) = snap.routes.get(reference) {
            return self.select_from_route(reference, route);
        }
        None
    }

    fn select_from_route(
        &self,
        route_name: &str,
        route: &RouteDefinition,
    ) -> Option<String> {
        if route.targets.is_empty() {
            return None;
        }
        match route.strategy.as_str() {
            "round_robin" => {
                let mut rr = self.round_robin.write();
                let idx = rr.entry(route_name.to_string()).or_insert(0);
                let target = &route.targets[*idx % route.targets.len()];
                *idx = (*idx + 1) % route.targets.len();
                Some(target.model.clone())
            }
            "weighted" => {
                let total: u32 = route.targets.iter().map(|t| t.weight).sum();
                if total == 0 {
                    return Some(route.targets[0].model.clone());
                }
                let r = (chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u32) % total;
                let mut acc = 0;
                for target in &route.targets {
                    acc += target.weight;
                    if r < acc {
                        return Some(target.model.clone());
                    }
                }
                Some(route.targets[0].model.clone())
            }
            _ => {
                let snap = self.snapshot.load();
                for target in &route.targets {
                    if snap.providers.contains_key(&target.model) {
                        return Some(target.model.clone());
                    }
                }
                None
            }
        }
    }

    pub async fn chat(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: &ChatOptions,
    ) -> Result<ChatResponse> {
        let model_name = self.resolve_model_name(model).ok_or_else(|| {
            AxonError::NotFound(format!("model or route '{model}' not found"))
        })?;

        let snap = self.snapshot.load();
        let provider = snap.providers.get(&model_name).ok_or_else(|| {
            AxonError::NotFound(format!("provider for model '{model_name}' not found"))
        })?;

        let model_def = snap.models.get(&model_name).ok_or_else(|| {
            AxonError::NotFound(format!("model definition '{model_name}' not found"))
        })?;

        let result = provider
            .chat(&model_def.model_name, messages, options)
            .await;
        result
    }

    pub async fn chat_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        options: &ChatOptions,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = StreamEvent> + Send>>> {
        let model_name = self.resolve_model_name(model).ok_or_else(|| {
            AxonError::NotFound(format!("model or route '{model}' not found"))
        })?;

        let snap = self.snapshot.load();
        let provider = snap.providers.get(&model_name).ok_or_else(|| {
            AxonError::NotFound(format!("provider for model '{model_name}' not found"))
        })?;

        let model_def = snap.models.get(&model_name).ok_or_else(|| {
            AxonError::NotFound(format!("model definition '{model_name}' not found"))
        })?;

        provider
            .chat_stream(&model_def.model_name, messages, options)
            .await
    }

    pub fn list_models(&self) -> Vec<ModelInfo> {
        let snap = self.snapshot.load();
        snap.models
            .values()
            .map(|m| ModelInfo {
                id: m.name.clone(),
                provider: m.provider.clone(),
                model_name: m.model_name.clone(),
            })
            .collect()
    }

    pub fn list_routes(&self) -> Vec<RouteDefinition> {
        let snap = self.snapshot.load();
        snap.routes.values().cloned().collect()
    }
}

impl Default for EmbeddedGateway {
    fn default() -> Self {
        EmbeddedGateway {
            snapshot: ArcSwap::from_pointee(GatewaySnapshot::empty()),
            round_robin: RwLock::new(HashMap::new()),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub model_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct GatewayStats {
    pub total_requests: u64,
    pub total_tokens: TokenUsage,
}
