//! axon-core: 核心原语 — 配置、错误类型、资源模型

pub mod config;
pub mod error;
pub mod models;

pub use config::AxonConfig;
pub use error::{AxonError, Result};
pub use models::*;
