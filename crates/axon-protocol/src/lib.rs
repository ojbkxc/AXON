//! axon-protocol: HTTP API 协议定义 — OpenAI 兼容 + AXON 扩展

use serde::{Deserialize, Serialize};

use axon_core::{ChatMessage, TokenUsage};

pub mod openai;
pub mod axon;

pub use axon::*;
pub use openai::*;
