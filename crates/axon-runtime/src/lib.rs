//! axon-runtime: 智能体编排引擎 — AgentExecutor + GenerationPipeline

pub mod executor;
pub mod pipeline;

pub use executor::{AgentExecutor, InvokeResult, ToolCallRecord};
pub use pipeline::GenerationPipeline;
