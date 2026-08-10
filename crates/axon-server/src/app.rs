use std::sync::Arc;

use arc_swap::ArcSwap;

use axon_core::AxonConfig;
use axon_gateway::EmbeddedGateway;
use axon_runtime::AgentExecutor;
use axon_store::Store;
use axon_tools::ToolRegistry;

pub struct AppState {
    pub config: ArcSwap<AxonConfig>,
    pub gateway: Arc<EmbeddedGateway>,
    pub store: Arc<Store>,
    pub tools: ArcSwap<ToolRegistry>,
    pub executor: ArcSwap<AgentExecutor>,
    pub start_time: std::time::Instant,
    pub working_dir: std::path::PathBuf,
}

impl AppState {
    pub fn new(config: AxonConfig, working_dir: std::path::PathBuf) -> anyhow::Result<Arc<Self>> {
        let store = Store::open(&config.storage.sqlite_path, config.storage.max_connections)
            .map_err(|e| anyhow::anyhow!("store init: {e}"))?;

        let gateway = EmbeddedGateway::from_config(&config)
            .map_err(|e| anyhow::anyhow!("gateway init: {e}"))?;

        let tool_defs: Vec<axon_core::ToolDefinition> = config.tools.clone();
        let tools = axon_tools::build_registry(&tool_defs, store.clone());

        let tools_arc = Arc::new(tools);
        let executor = AgentExecutor::new(
            gateway.clone(),
            tools_arc.clone(),
            store.clone(),
            working_dir.clone(),
        );

        let state = Arc::new(AppState {
            config: ArcSwap::from_pointee(config),
            gateway,
            store,
            tools: ArcSwap::from_pointee(tools_arc),
            executor: ArcSwap::from_pointee(executor),
            start_time: std::time::Instant::now(),
            working_dir,
        });

        Ok(state)
    }

    pub fn reload_config(&self, config: AxonConfig) -> anyhow::Result<()> {
        self.gateway
            .reload(&config)
            .map_err(|e| anyhow::anyhow!("gateway reload: {e}"))?;

        let tool_defs: Vec<axon_core::ToolDefinition> = config.tools.clone();
        let new_tools = axon_tools::build_registry(&tool_defs, self.store.clone());
        let new_tools_arc = Arc::new(new_tools);

        let executor = AgentExecutor::new(
            self.gateway.clone(),
            new_tools_arc.clone(),
            self.store.clone(),
            self.working_dir.clone(),
        );

        self.tools.store(new_tools_arc);
        self.executor.store(Arc::new(executor));
        self.config.store(Arc::new(config));

        Ok(())
    }

    pub fn executor(&self) -> Arc<AgentExecutor> {
        self.executor.load_full()
    }

    pub fn tools(&self) -> Arc<ToolRegistry> {
        self.tools.load_full()
    }

    pub fn current_config(&self) -> Arc<AxonConfig> {
        self.config.load_full()
    }
}
