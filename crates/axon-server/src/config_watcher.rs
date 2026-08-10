use std::path::PathBuf;
use std::sync::Arc;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::app::AppState;

pub fn spawn_watcher(state: Arc<AppState>, config_path: PathBuf) -> anyhow::Result<()> {
    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
            Ok(event) => {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    tracing::info!("config file changed, reloading...");
                    match axon_core::AxonConfig::from_file(config_path.to_str().unwrap_or("")) {
                        Ok(new_config) => {
                            if let Err(e) = state.reload_config(new_config) {
                                tracing::error!("config reload failed: {e}");
                            } else {
                                tracing::info!("config reloaded successfully");
                            }
                        }
                        Err(e) => {
                            tracing::error!("config parse failed: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("watch error: {e}");
            }
        })?;

    let dir = config_path.parent().unwrap_or(std::path::Path::new("."));
    watcher.watch(dir, RecursiveMode::NonRecursive)?;

    std::mem::forget(watcher);
    Ok(())
}
