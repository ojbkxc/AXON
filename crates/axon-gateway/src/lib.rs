//! axon-gateway: 嵌入式 AI 网关 — 统一多 provider 的 chat 接口

pub mod gateway;
pub mod provider;
pub mod stream;

pub use gateway::{EmbeddedGateway, ModelInfo};
pub use provider::Provider;
pub use stream::{ChatChunk, ChatResponse, StreamEvent};
