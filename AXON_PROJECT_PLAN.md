# AXON 项目完善方案

> 版本：v1.0 | 日期：2026-08-10 | 许可证：Apache-2.0

---

## 目录

1. [项目概述](#1-项目概述)
2. [技术选型与决策](#2-技术选型与决策)
3. [架构设计](#3-架构设计)
4. [开发路线图](#4-开发路线图)
5. [部署方案](#5-部署方案)
6. [配置示例](#6-配置示例)
7. [测试与调试](#7-测试与调试)
8. [文档结构](#8-文档结构)
9. [约束满足分析](#9-约束满足分析)

---

## 1. 项目概述

### 1.1 定位

AXON 是一个面向移动端（Android）的多智能体协作与 AI 网关轻量级平台。它整合了：

- **Agora** 的多智能体编排理念（树形消息、工具调用、生成管道）
- **AISIX** 的高性能 AI 网关能力（统一路由、故障转移、限流、护栏）

目标：为开发者提供一款可在手机上运行、随身携带的 AI 能力枢纽。

### 1.2 与上游项目的关系

| 上游项目 | 角色 | 复用方式 |
|---|---|---|
| [AISIX](https://github.com/api7/aisix) | AI 网关内核 | 直接复用 `aisix-core`、`aisix-gateway`、provider crate；裁剪 `aisix-proxy` 为嵌入式子集 |
| [Agora](https://github.com/ojbkxc/AXON) (参考) | 智能体编排设计 | 借鉴 `GenerationManager`、`ToolProvider`、消息树、`StreamEvent` 契约，用 Rust 重新实现轻量版 |

### 1.3 核心能力

```
┌─────────────────────────────────────────────────────┐
│                    AXON 二进制                        │
│                   (< 50MB, ARM64)                    │
├─────────────┬───────────────┬───────────────────────┤
│  智能体引擎  │   AI 网关     │    嵌入式 Web UI      │
│  (axon-     │  (复用 AISIX  │   (axum + 静态资源    │
│   runtime)  │   子集)       │    include_dir!)      │
├─────────────┴───────────────┴───────────────────────┤
│              SQLite (rusqlite) + 配置热重载           │
└─────────────────────────────────────────────────────┘
```

---

## 2. 技术选型与决策

### 2.1 核心语言：Rust

| 候选 | 优势 | 劣势 | 结论 |
|---|---|---|---|
| **Rust** | 直接复用 AISIX；交叉编译产出小二进制（< 15MB）；零成本抽象；内存可控 | 生态不如 Go 成熟 | **选定** |
| Go | 生态成熟；交叉编译简单 | 无法复用 AISIX Rust 代码；GC 增加内存不确定性；二进制偏大（> 20MB） | 否决 |

**关键依据**：AISIX 全部 18 个 crate 为 Rust，复用必须同语言；Rust `no_std` 友好、无运行时 GC，满足 < 200MB 启动内存约束。

### 2.2 技术栈总览

| 层 | 技术 | 说明 |
|---|---|---|
| 网关内核 | AISIX crate 子集 | `aisix-core` + `aisix-gateway` + `aisix-provider-openai` + `aisix-provider-anthropic` + `aisix-provider-vertex` |
| 智能体引擎 | 自研 `axon-runtime` | 借鉴 Agora `GenerationManager` / `ToolProvider` 契约 |
| HTTP 服务 | `axum` 0.7 | 与 AISIX 一致 |
| 存储 | `rusqlite` + `r2d2_sqlite` | SQLite 连接池，单文件数据库 |
| 配置 | `serde` + `config` crate + `notify` | YAML/JSON 配置 + 文件监听热重载 |
| Web UI | React + Vite + TailwindCSS | 构建产物通过 `include_dir!` 嵌入二进制 |
| 日志 | `tracing` + `tracing-subscriber` | 与 AISIX 一致 |
| 序列化 | `serde` + `serde_json` | 与 AISIX 一致 |
| 异步运行时 | `tokio` 1.x (full) | 与 AISIX 一致 |
| 交叉编译 | `cross` 工具链 / NDK | 目标 `aarch64-linux-android` |

### 2.3 不选型的方案

| 方案 | 否决理由 |
|---|---|
| 嵌入 AISIX 整体 | `aisix-proxy` 9471 行 + etcd + Redis 依赖，二进制 > 80MB，超出约束 |
| JNI 调 Agora Kotlin | 需要完整 Android Runtime/JVM，启动内存 > 300MB，违反约束 |
| 纯 Go 重写网关 | 丧失 AISIX 复用价值，开发周期翻倍 |

---

## 3. 架构设计

### 3.1 模块划分

AXON 采用 Cargo workspace 多 crate 结构，借鉴 AISIX 的 crate 切分哲学：

```
axon/
├── Cargo.toml                 # workspace 根
├── rust-toolchain.toml
├── config.example.yaml
├── crates/
│   ├── axon-core/             # 核心原语：Config、Error、资源模型
│   ├── axon-gateway/          # 网关子集（复用 aisix-gateway + provider crates）
│   ├── axon-runtime/          # 智能体编排引擎（核心自研）
│   ├── axon-tools/            # 内置工具集（web 搜索、shell、memory、code exec）
│   ├── axon-store/            # SQLite 持久化层
│   ├── axon-server/           # HTTP 服务 + Web UI + CLI 入口（二进制）
│   └── axon-protocol/         # 客户端 API 协议（OpenAI 兼容 + AXON 扩展）
├── ui/                        # React + Vite 前端源码
│   ├── package.json
│   ├── src/
│   └── vite.config.ts
├── android/                   # Android 封装
│   ├── termux/               # Termux 启动脚本
│   ├── apk/                  # 简易 APK 包装
│   └── NDK/                  # 交叉编译脚本
├── docs/                      # 文档
└── tests/                     # E2E 测试
    └── e2e/
```

#### 各 crate 职责

| Crate | 职责 | 依赖 |
|---|---|---|
| `axon-core` | `Config` 结构体（AXON 配置根）、`AxonError`、资源模型（Agent、Model、Route、Tool 定义） | `serde`, `serde_json`, `thiserror` |
| `axon-gateway` | 嵌入式 AI 网关：复用 `aisix-gateway` 的 `Hub`/`Bridge`/`ChatFormat`，裁剪掉 etcd/Redis/集群功能，仅保留单机文件配置 + memory cache + 本地限流 | `aisix-core`, `aisix-gateway`, `aisix-provider-*` |
| `axon-runtime` | 智能体编排引擎：`AgentExecutor`、`GenerationPipeline`、`MessageTree`、`StreamEvent` 归一化 | `axon-core`, `axon-gateway`, `axon-store`, `axon-tools` |
| `axon-tools` | 内置工具实现：`WebSearchTool`、`ShellTool`、`MemoryTool`、`CodeExecTool`、`HttpTool` | `axon-core`, `reqwest`, `tokio::process` |
| `axon-store` | SQLite 持久化：对话、消息树、智能体配置、用量统计、记忆向量 | `rusqlite`, `r2d2_sqlite`, `axon-core` |
| `axon-protocol` | HTTP API 协议定义：OpenAI 兼容端点 + AXON 扩展端点（agent 管理、任务编排） | `serde`, `axum` |
| `axon-server` | 二进制入口：CLI 解析、配置加载、服务启动、Web UI 静态资源服务 | 全部 crate |

### 3.2 数据流图

#### 3.2.1 请求处理主流程

```
用户请求 (HTTP/SSE)
    │
    ▼
┌──────────────────────────────────┐
│         axon-server              │
│    (axum router :8080)           │
└──────────┬───────────────────────┘
           │
           ├── /v1/chat/completions ──────► [网关直通模式]
           │                                    │
           │                                    ▼
           │                           ┌────────────────┐
           │                           │  axon-gateway  │
           │                           │  (AISIX Hub)   │
           │                           │  路由→Bridge   │
           │                           │  →上游 API     │
           │                           └────────────────┘
           │
           ├── /v1/agents/:id/invoke ─────► [智能体编排模式]
           │                                    │
           │                                    ▼
           │                           ┌────────────────────┐
           │                           │  axon-runtime       │
           │                           │  AgentExecutor      │
           │                           └────────┬───────────┘
           │                                    │
           │                           ┌────────▼───────────┐
           │                           │ GenerationPipeline │
           │                           │ (循环: LLM→Tool)   │
           │                           └────────┬───────────┘
           │                              │             │
           │                              ▼             ▼
           │                     ┌──────────┐  ┌──────────────┐
           │                     │gateway   │  │ axon-tools   │
           │                     │(LLM 调用)│  │ (工具执行)   │
           │                     └──────────┘  └──────────────┘
           │
           ├── /v1/agents ────────────────► [CRUD: 智能体管理]
           ├── /v1/conversations ─────────► [对话管理]
           ├── /metrics ──────────────────► [Prometheus 指标]
           └── /ui/* ─────────────────────► [Web UI 静态资源]
```

#### 3.2.2 智能体编排内部流程

```
AgentExecutor.invoke(input)
    │
    ▼
┌─────────────────────────────────────────┐
│ 1. 加载 Agent 定义 (system prompt,      │
│    tools, model, max_iterations)        │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│ 2. 构建 GenerationRequest               │
│    (messages + tools + model config)    │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│ 3. GenerationPipeline 循环              │
│    ┌───────────────────────────────┐    │
│    │ a. 调用 gateway.chat()        │    │
│    │    → StreamEvent 流           │    │
│    │    → 持久化到 MessageTree     │    │
│    ├───────────────────────────────┤    │
│    │ b. 检查是否有 ToolCall        │    │
│    │    → 无: 返回最终响应         │    │
│    │    → 有: 执行工具            │    │
│    ├───────────────────────────────┤    │
│    │ c. ToolProvider.execute()     │    │
│    │    → ToolExecutionEvent 流    │    │
│    │    → 持久化工具结果           │    │
│    ├───────────────────────────────┤    │
│    │ d. 将工具结果追加到 messages  │    │
│    │    → 回到步骤 a              │    │
│    └───────────────────────────────┘    │
│    (最多 max_iterations 轮)             │
└─────────────────────────────────────────┘
```

### 3.3 关键接口设计

#### 3.3.1 智能体定义（`axon-core`）

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub model: String,              // 引用 Model 别名
    pub tools: Vec<String>,         // 引用 Tool 名称
    pub max_iterations: u32,        // 最大工具调用轮数
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxonConfig {
    pub server: ServerConfig,
    pub gateway: GatewayConfig,
    pub agents: Vec<AgentDefinition>,
    pub models: Vec<ModelDefinition>,
    pub routes: Vec<RouteDefinition>,
    pub tools: Vec<ToolDefinition>,
    pub storage: StorageConfig,
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub addr: String,               // 默认 "0.0.0.0:8080"
    pub web_ui_enabled: bool,       // 默认 true
    pub max_request_body_mb: usize, // 默认 10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub upstream_timeout_ms: u64,   // 默认 120000
    pub stream_timeout_ms: u64,     // 默认 300000
    pub retry_count: u32,           // 默认 2
    pub retry_delay_ms: u64,        // 默认 1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub sqlite_path: String,        // 默认 "axon.db"
    pub max_connections: u32,       // 默认 5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub log_level: String,          // 默认 "info"
    pub metrics_enabled: bool,      // 默认 true
    pub access_log_enabled: bool,   // 默认 true
}
```

#### 3.3.2 网关接口（`axon-gateway`，复用 AISIX）

```rust
use aisix_gateway::{ChatFormat, ChatChunk, BridgeError};

/// 嵌入式网关：裁剪版 AISIX Hub
pub struct EmbeddedGateway {
    hub: aisix_gateway::Hub,
    snapshot: arc_swap::ArcSwap<GatewaySnapshot>,
}

/// 网关统一 chat 接口
impl EmbeddedGateway {
    /// 非流式 chat
    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: ChatOptions,
    ) -> Result<ChatResponse, BridgeError>;

    /// 流式 chat（返回 SSE 事件流）
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        options: ChatOptions,
    ) -> Result<impl Stream<Item = Result<ChatChunk, BridgeError>>, BridgeError>;

    /// 列出可用模型
    pub async fn list_models(&self) -> Vec<ModelInfo>;
}

/// 网关快照（无锁读路径，借鉴 AISIX ArcSwap 设计）
pub struct GatewaySnapshot {
    pub models: std::collections::HashMap<String, ModelDefinition>,
    pub routes: Vec<RouteDefinition>,
    pub provider_keys: std::collections::HashMap<String, ProviderKey>,
}
```

#### 3.3.3 智能体执行器（`axon-runtime`）

```rust
use async_trait::async_trait;
use futures::Stream;

/// 归一化流式事件（借鉴 Agora StreamEvent 契约）
#[derive(Debug, Clone)]
pub enum StreamEvent {
    TextChunk(String),
    ThoughtChunk(String),
    ToolCallRequest(ToolCall),
    ToolCallUpdate(String, serde_json::Value),
    ToolCallResult(String, ToolResult),
    UsageUpdate(TokenUsage),
    Error(String),
    Done,
}

/// 工具提供者 trait（借鉴 Agora ToolProvider）
#[async_trait]
pub trait ToolProvider: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;

    async fn execute(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError>;
}

/// 工具执行结果
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: String,
    pub metadata: serde_json::Value,
    pub is_error: bool,
}

/// 工具执行上下文
pub struct ToolContext {
    pub agent_id: String,
    pub conversation_id: String,
    pub store: std::sync::Arc<axon_store::Store>,
    pub working_dir: std::path::PathBuf,
}

/// 智能体执行器
pub struct AgentExecutor {
    gateway: std::sync::Arc<EmbeddedGateway>,
    tools: std::collections::HashMap<String, Box<dyn ToolProvider>>,
    store: std::sync::Arc<axon_store::Store>,
}

impl AgentExecutor {
    /// 执行智能体（流式）
    pub async fn invoke_stream(
        &self,
        agent_id: &str,
        input: &str,
        conversation_id: Option<&str>,
    ) -> Result<impl Stream<Item = StreamEvent>, AxonError>;

    /// 执行智能体（非流式，收集所有事件）
    pub async fn invoke(
        &self,
        agent_id: &str,
        input: &str,
        conversation_id: Option<&str>,
    ) -> Result<InvokeResult, AxonError>;
}

/// 调用结果
pub struct InvokeResult {
    pub output: String,
    pub conversation_id: String,
    pub usage: TokenUsage,
    pub iterations: u32,
    pub tool_calls: Vec<ToolCallRecord>,
}
```

#### 3.3.4 生成管道（`axon-runtime`）

```rust
/// 生成管道：编排 LLM 调用与工具执行的循环
pub struct GenerationPipeline<'a> {
    executor: &'a AgentExecutor,
    agent: &'a AgentDefinition,
    messages: Vec<ChatMessage>,
    iterations: u32,
}

impl<'a> GenerationPipeline<'a> {
    pub async fn run(&mut self) -> impl Stream<Item = StreamEvent> + 'a {
        async_stream::stream! {
            loop {
                if self.iterations >= self.agent.max_iterations {
                    yield StreamEvent::Error("Max iterations reached".into());
                    break;
                }
                self.iterations += 1;

                // 1. 调用 LLM
                let mut stream = self.executor.gateway
                    .chat_stream(&self.agent.model, self.messages.clone(), ChatOptions::default())
                    .await;

                let mut tool_calls = Vec::new();
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(ChatChunk::Text(text)) => yield StreamEvent::TextChunk(text),
                        Ok(ChatChunk::ToolCall(call)) => tool_calls.push(call),
                        Ok(ChatChunk::Usage(u)) => yield StreamEvent::UsageUpdate(u),
                        Err(e) => { yield StreamEvent::Error(e.to_string()); break; }
                    }
                }

                // 2. 无工具调用 → 完成
                if tool_calls.is_empty() {
                    yield StreamEvent::Done;
                    break;
                }

                // 3. 执行工具
                for call in tool_calls {
                    yield StreamEvent::ToolCallRequest(call.clone());
                    if let Some(tool) = self.executor.tools.get(&call.name) {
                        match tool.execute(call.arguments, &self.context()).await {
                            Ok(result) => {
                                yield StreamEvent::ToolCallResult(call.id.clone(), result.clone());
                                self.messages.push(ChatMessage::tool_result(call.id, result.content));
                            }
                            Err(e) => {
                                yield StreamEvent::Error(e.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
}
```

#### 3.3.5 持久化接口（`axon-store`）

```rust
use rusqlite::Connection;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub struct Store {
    pool: Pool<SqliteConnectionManager>,
}

impl Store {
    pub fn open(path: &str, max_connections: u32) -> Result<Self, AxonError>;

    // 对话管理
    pub async fn create_conversation(&self, agent_id: &str) -> Result<Conversation, AxonError>;
    pub async fn get_conversation(&self, id: &str) -> Result<Option<Conversation>, AxonError>;
    pub async fn list_conversations(&self, limit: u32) -> Result<Vec<Conversation>, AxonError>;
    pub async fn delete_conversation(&self, id: &str) -> Result<(), AxonError>;

    // 消息树（借鉴 Agora parentId 树形结构）
    pub async fn add_message(&self, msg: &Message) -> Result<(), AxonError>;
    pub async fn get_messages(&self, conversation_id: &str) -> Result<Vec<Message>, AxonError>;
    pub async fn get_message_tree(&self, conversation_id: &str) -> Result<MessageTree, AxonError>;

    // 用量统计
    pub async fn record_usage(&self, usage: &UsageRecord) -> Result<(), AxonError>;
    pub async fn get_usage_stats(&self, range: TimeRange) -> Result<UsageStats, AxonError>;

    // 记忆（简单 KV + 可选向量）
    pub async fn set_memory(&self, key: &str, value: &str) -> Result<(), AxonError>;
    pub async fn get_memory(&self, key: &str) -> Result<Option<String>, AxonError>;
}
```

#### 3.3.6 HTTP API 端点（`axon-server`）

```
# OpenAI 兼容（网关直通）
POST /v1/chat/completions
POST /v1/completions
POST /v1/embeddings
GET  /v1/models

# AXON 扩展（智能体编排）
POST /v1/agents                      # 创建智能体
GET  /v1/agents                      # 列出智能体
GET  /v1/agents/:id                  # 获取智能体
PUT  /v1/agents/:id                  # 更新智能体
DELETE /v1/agents/:id                # 删除智能体
POST /v1/agents/:id/invoke           # 调用智能体（支持 SSE 流式）
POST /v1/agents/:id/invoke/stream    # 显式流式端点

# 对话管理
POST /v1/conversations
GET  /v1/conversations
GET  /v1/conversations/:id
DELETE /v1/conversations/:id
GET  /v1/conversations/:id/messages

# 工具管理
GET  /v1/tools                       # 列出已注册工具

# 运维
GET  /healthz                        # 健康检查
GET  /readyz                         # 就绪检查
GET  /metrics                        # Prometheus 指标
GET  /status                         # 运行状态

# Web UI
GET  /ui/*                           # 嵌入式 Web 管理界面
```

### 3.4 SQLite 数据库 Schema

```sql
-- 对话表
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    title TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT DEFAULT '{}'
);

-- 消息表（树形结构，借鉴 Agora parentId 设计）
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    parent_id TEXT,                  -- 父消息 ID（树形分支）
    role TEXT NOT NULL,              -- user/assistant/tool/system
    content TEXT NOT NULL,
    tool_calls TEXT,                 -- JSON: 工具调用数组
    tool_call_id TEXT,               -- 工具结果关联 ID
    model TEXT,
    usage TEXT,                      -- JSON: token 用量
    created_at INTEGER NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id),
    FOREIGN KEY (parent_id) REFERENCES messages(id)
);

CREATE INDEX idx_messages_conversation ON messages(conversation_id);
CREATE INDEX idx_messages_parent ON messages(parent_id);

-- 智能体配置表（运行时可修改）
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    definition TEXT NOT NULL,       -- JSON: AgentDefinition
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 用量统计表
CREATE TABLE usage_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id TEXT,
    agent_id TEXT,
    model TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    duration_ms INTEGER,
    timestamp INTEGER NOT NULL
);

CREATE INDEX idx_usage_timestamp ON usage_records(timestamp);
CREATE INDEX idx_usage_model ON usage_records(model);

-- 记忆表（简单 KV 存储）
CREATE TABLE memory (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    namespace TEXT DEFAULT 'default',
    updated_at INTEGER NOT NULL
);

-- 配置版本表（支持配置回滚）
CREATE TABLE config_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    config TEXT NOT NULL,
    hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

---

## 4. 开发路线图

### 4.1 版本规划

| 版本 | 主题 | 核心交付 | 预估时间 |
|---|---|---|---|
| **v0.1** | 网关基座 | 嵌入式 AISIX 网关 + 基础 API + 配置 | 3 周 |
| **v0.2** | 智能体引擎 | Agent 定义 + 单轮调用 + 消息持久化 | 3 周 |
| **v0.3** | 工具编排 | 多轮工具调用 + 内置工具集 | 3 周 |
| **v0.4** | 移动端适配 | Android 交叉编译 + Termux 运行 | 2 周 |
| **v0.5** | Web UI | 嵌入式管理界面 + 实时对话 | 3 周 |
| **v0.6** | 可观测性 | 指标 + 日志 + 用量统计面板 | 2 周 |
| **v0.7** | 高级路由 | 多模型路由 + 故障转移 + 限流 | 2 周 |
| **v0.8** | APK 封装 | 简易 APK + 前台服务 | 2 周 |
| **v0.9** | 稳定化 | E2E 测试 + 性能优化 + 文档 | 3 周 |
| **v1.0** | 正式发布 | 全功能 + 文档 + 发布流程 | 2 周 |

### 4.2 详细功能清单

#### v0.1 - 网关基座（3 周）

- [ ] Cargo workspace 骨架搭建
- [ ] `axon-core`：Config 结构体 + 错误类型
- [ ] `axon-gateway`：嵌入 `aisix-core` + `aisix-gateway` + `aisix-provider-openai` + `aisix-provider-anthropic`
- [ ] `axon-server`：axum HTTP 服务 + CLI 入口（`--config` 参数）
- [ ] OpenAI 兼容端点：`POST /v1/chat/completions`（流式 + 非流式）
- [ ] `GET /v1/models` 端点
- [ ] YAML 配置文件加载 + `SIGHUP` 热重载
- [ ] `GET /healthz` + `GET /readyz`
- [ ] 基础日志（`tracing`）

**验收标准**：配置一个 OpenAI API Key 后，`curl` 调用 `POST /v1/chat/completions` 能正确代理并返回流式响应。

#### v0.2 - 智能体引擎（3 周）

- [ ] `axon-store`：SQLite 数据库初始化 + Schema 迁移
- [ ] 对话 CRUD API
- [ ] 消息持久化（树形结构）
- [ ] `axon-runtime`：`AgentDefinition` 加载 + `AgentExecutor`
- [ ] 单轮智能体调用（无工具）：`POST /v1/agents/:id/invoke`
- [ ] 流式响应（SSE）
- [ ] Token 用量记录

**验收标准**：定义一个 "翻译助手" 智能体，调用后能流式返回翻译结果并持久化对话历史。

#### v0.3 - 工具编排（3 周）

- [ ] `axon-tools`：`ToolProvider` trait + 工具注册机制
- [ ] `GenerationPipeline`：LLM ↔ Tool 循环
- [ ] 内置工具：
  - [ ] `web_search`：Web 搜索（DuckDuckGo 免费 API）
  - [ ] `shell`：本地命令执行（带超时 + 安全限制）
  - [ ] `memory`：KV 记忆读写
  - [ ] `http_fetch`：HTTP 请求
  - [ ] `code_exec`：代码执行（简易沙盒）
- [ ] 工具调用流式事件（`ToolCallRequest` / `ToolCallResult`）
- [ ] `max_iterations` 限制

**验收标准**：定义一个 "研究助手" 智能体，能自主搜索 Web 并总结结果。

#### v0.4 - 移动端适配（2 周）

- [ ] Android NDK 交叉编译脚本（`aarch64-linux-android`）
- [ ] 裁剪不必要的依赖（减小二进制体积）
- [ ] Termux 安装与运行脚本
- [ ] 路径适配（Termux 文件系统路径）
- [ ] 信号处理适配（Android 前台服务）
- [ ] 二进制体积优化（`strip` + LTO）
- [ ] 启动内存优化

**验收标准**：在 Termux 中一键安装并启动，二进制 < 30MB，启动内存 < 150MB。

#### v0.5 - Web UI（3 周）

- [ ] React + Vite + TailwindCSS 前端项目
- [ ] 页面：
  - [ ] 仪表盘（概览 + 用量统计）
  - [ ] 对话界面（实时 SSE 流式）
  - [ ] 智能体管理（CRUD + 配置编辑）
  - [ ] 模型配置（Provider Key 管理）
  - [ ] 设置页
- [ ] `include_dir!` 宏将构建产物嵌入二进制
- [ ] axum 静态文件服务路由
- [ ] 暗色主题

**验收标准**：浏览器打开 `http://localhost:8080/ui/` 能看到完整管理界面，可创建智能体并对话。

#### v0.6 - 可观测性（2 周）

- [ ] Prometheus 指标端点 `GET /metrics`
  - 请求数、延迟分布、错误率
  - Token 用量按模型统计
  - 工具调用次数
- [ ] 结构化访问日志（JSON 格式）
- [ ] 用量统计 API + UI 面板
- [ ] 请求追踪 ID（贯穿日志）

**验收标准**：Prometheus 可抓取指标，UI 仪表盘显示实时统计数据。

#### v0.7 - 高级路由（2 周）

- [ ] 多模型路由策略：
  - [ ] `round_robin`
  - [ ] `failover`
  - [ ] `weighted`
- [ ] 自动故障转移（健康检查 + 冷却）
- [ ] 本地限流（RPS + 并发）
- [ ] 请求重试配置
- [ ] 路由配置 UI

**验收标准**：配置两个模型（主 + 备），主模型故障时自动切换到备模型。

#### v0.8 - APK 封装（2 周）

- [ ] 简易 APK 工程（Gradle）
- [ ] 前台服务（Foreground Service）包装 AXON 二进制
- [ ] APK 打包脚本（将 ARM64 二进制嵌入 APK assets）
- [ ] 启动/停止/状态通知
- [ ] 权限声明（INTERNET、FOREGROUND_SERVICE）
- [ ] 可选：WebView 壳加载 Web UI

**验收标准**：安装 APK 后，一键启动 AXON 服务，通知栏显示运行状态。

#### v0.9 - 稳定化（3 周）

- [ ] E2E 测试框架（Vitest + 真实进程）
- [ ] 核心路径 E2E 用例（> 50 个）
- [ ] 错误处理完善（边界条件、超时、网络故障）
- [ ] 性能基准测试 + 优化
- [ ] 配置校验（JSON Schema）
- [ ] 安全审计（输入校验、路径遍历、注入防护）
- [ ] 用户文档编写

**验收标准**：E2E 测试全绿，核心 API P99 延迟 < 50ms（不含上游）。

#### v1.0 - 正式发布（2 周）

- [ ] 完整文档（README、快速入门、API 参考、部署指南）
- [ ] 发布流程（GitHub Release + 二进制产物 + 校验和）
- [ ] 示例配置库（常见场景模板）
- [ ] 迁移指南
- [ ] CHANGELOG
- [ ] LICENSE（Apache-2.0）

**验收标准**：新用户按文档可在 10 分钟内启动并使用。

---

## 5. 部署方案

### 5.1 方案一：Termux 运行（推荐，最轻量）

#### 5.1.1 一键安装脚本

```bash
#!/bin/bash
# scripts/install-termux.sh
# 在 Termux 中运行：bash install-termux.sh

set -euo pipefail

AXON_VERSION="1.0.0"
INSTALL_DIR="$HOME/.axon"
BIN_PATH="$INSTALL_DIR/axon"
CONFIG_PATH="$INSTALL_DIR/config.yaml"
ARCH=$(uname -m)

echo "=== AXON Installer for Termux ==="

# 1. 检查环境
if ! command -v termux-info &>/dev/null; then
    echo "Error: Not running in Termux"
    exit 1
fi

if [[ "$ARCH" != "aarch64" ]]; then
    echo "Error: Only aarch64 is supported, got $ARCH"
    exit 1
fi

# 2. 安装依赖
pkg update -y
pkg install -y proot wget

# 3. 创建目录
mkdir -p "$INSTALL_DIR"

# 4. 下载二进制
echo "Downloading AXON v${AXON_VERSION}..."
wget -qO "$BIN_PATH" \
    "https://github.com/ojbkxc/AXON/releases/download/v${AXON_VERSION}/axon-aarch64-linux-android"
chmod +x "$BIN_PATH"

# 5. 生成默认配置（如果不存在）
if [[ ! -f "$CONFIG_PATH" ]]; then
    cat > "$CONFIG_PATH" << 'YAML'
server:
  addr: "127.0.0.1:8080"
  web_ui_enabled: true

gateway:
  upstream_timeout_ms: 120000
  stream_timeout_ms: 300000
  retry_count: 2

storage:
  sqlite_path: "~/.axon/axon.db"
  max_connections: 3

observability:
  log_level: "info"
  metrics_enabled: true

agents: []
models: []
routes: []
tools: []
YAML
    echo "Default config created at $CONFIG_PATH"
fi

# 6. 创建启动脚本
cat > "$INSTALL_DIR/start.sh" << EOF
#!/bin/bash
export AXON_CONFIG="\$HOME/.axon/config.yaml"
exec "\$HOME/.axon/axon" --config "\$AXON_CONFIG"
EOF
chmod +x "$INSTALL_DIR/start.sh"

echo ""
echo "=== Installation Complete ==="
echo "Start AXON:  bash ~/.axon/start.sh"
echo "Web UI:      http://localhost:8080/ui/"
echo "Config:      $CONFIG_PATH"
```

#### 5.1.2 后台守护运行

```bash
#!/bin/bash
# scripts/termux-daemon.sh
# 使用 Termux:Boot 实现开机自启

INSTALL_DIR="$HOME/.axon"

# 创建 Termux:Boot 启动脚本
mkdir -p ~/.termux/boot
cat > ~/.termux/boot/start-axon.sh << 'EOF'
#!/bin/bash
termux-wake-lock
nohup bash ~/.axon/start.sh > ~/.axon/axon.log 2>&1 &
EOF
chmod +x ~/.termux/boot/start-axon.sh

echo "AXON will auto-start on boot."
echo "Install Termux:Boot from F-Droid to enable this feature."
```

#### 5.1.3 手动操作步骤

```bash
# 1. 安装 Termux（从 F-Droid，不要用 Play Store 版本）
# 2. 打开 Termux，执行安装脚本
bash <(curl -fsSL https://raw.githubusercontent.com/ojbkxc/AXON/main/scripts/install-termux.sh)

# 3. 编辑配置，添加 API Key
nano ~/.axon/config.yaml

# 4. 启动
bash ~/.axon/start.sh

# 5. 打开浏览器访问
# http://localhost:8080/ui/
```

### 5.2 方案二：简易 APK 封装

#### 5.2.1 APK 工程结构

```
android/apk/
├── build.gradle.kts
├── settings.gradle.kts
├── gradle/
├── app/
│   ├── build.gradle.kts
│   └── src/main/
│       ├── AndroidManifest.xml
│       ├── java/com/axon/mobile/
│       │   ├── MainActivity.kt          # WebView 壳
│       │   ├── AxonService.kt           # 前台服务
│       │   └── BinaryManager.kt         # 二进制提取与执行
│       ├── assets/
│       │   └── axon-aarch64             # 编译好的二进制
│       └── res/
│           ├── layout/
│           ├── values/
│           └── drawable/
└── scripts/
    └── build-apk.sh                     # 打包脚本
```

#### 5.2.2 前台服务（Kotlin）

```kotlin
// app/src/main/java/com/axon/mobile/AxonService.kt
package com.axon.mobile

import android.app.*
import android.content.Intent
import android.os.*
import androidx.core.app.NotificationCompat
import java.io.File

class AxonService : Service() {

    companion object {
        private const val CHANNEL_ID = "axon_service"
        private const val NOTIFICATION_ID = 1
        private const val AXON_PORT = 8080
    }

    private var axonProcess: Process? = null

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        startForeground(NOTIFICATION_ID, buildNotification("AXON starting..."))

        Thread {
            try {
                val binary = BinaryManager.extractBinary(this)
                val config = BinaryManager.ensureConfig(this)

                axonProcess = ProcessBuilder(binary.absolutePath, "--config", config.absolutePath)
                    .redirectErrorStream(true)
                    .start()

                updateNotification("AXON running on port $AXON_PORT")
            } catch (e: Exception) {
                updateNotification("AXON error: ${e.message}")
            }
        }.start()

        return START_STICKY
    }

    override fun onDestroy() {
        axonProcess?.destroy()
        super.onDestroy()
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID, "AXON Service", NotificationManager.IMPORTANCE_LOW
        )
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    private fun buildNotification(text: String): Notification =
        NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("AXON")
            .setContentText(text)
            .setSmallIcon(R.drawable.ic_notification)
            .setOngoing(true)
            .build()

    private fun updateNotification(text: String) {
        getSystemService(NotificationManager::class.java)
            .notify(NOTIFICATION_ID, buildNotification(text))
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
```

#### 5.2.3 WebView 壳（Kotlin）

```kotlin
// app/src/main/java/com/axon/mobile/MainActivity.kt
package com.axon.mobile

import android.content.Intent
import android.os.Bundle
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // 启动前台服务
        startForegroundService(Intent(this, AxonService::class.java))

        // WebView 加载 Web UI
        val webView = WebView(this).apply {
            settings.javaScriptEnabled = true
            settings.domStorageEnabled = true
            webViewClient = WebViewClient()
            loadUrl("http://127.0.0.1:8080/ui/")
        }
        setContentView(webView)
    }
}
```

#### 5.2.4 打包脚本

```bash
#!/bin/bash
# android/apk/scripts/build-apk.sh
# 需要环境：Android SDK + NDK + Rust (aarch64-linux-android target)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APK_DIR="$SCRIPT_DIR/.."
ROOT_DIR="$SCRIPT_DIR/../.."

echo "=== Building AXON APK ==="

# 1. 交叉编译 Rust 二进制
echo "[1/4] Cross-compiling AXON for aarch64-linux-android..."
cargo build --release --target aarch64-linux-android --bin axon
BINARY="$ROOT_DIR/target/aarch64-linux-android/release/axon"

# 2. strip 二进制
echo "[2/4] Stripping binary..."
${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/*/bin/llvm-strip "$BINARY"

# 3. 复制到 APK assets
echo "[3/4] Copying binary to APK assets..."
cp "$BINARY" "$APK_DIR/app/src/main/assets/axon-aarch64"

# 4. 构建 APK
echo "[4/4] Building APK..."
cd "$APK_DIR"
./gradlew assembleRelease

APK_PATH="$APK_DIR/app/build/outputs/apk/release/app-release.apk"
echo ""
echo "=== APK Built ==="
echo "Path: $APK_PATH"
echo "Size: $(du -h "$APK_PATH" | cut -f1)"
```

### 5.3 方案三：Docker 运行（开发/桌面端）

```dockerfile
# Dockerfile
FROM rust:1.93-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY ui/ ./ui/

# 构建 Web UI
RUN apt-get update && apt-get install -y nodejs npm
RUN cd ui && npm install && npm run build

# 构建 Rust 二进制
RUN cargo build --release --bin axon

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/axon /usr/local/bin/axon
COPY config.example.yaml /etc/axon/config.yaml

EXPOSE 8080
ENTRYPOINT ["axon", "--config", "/etc/axon/config.yaml"]
```

```bash
# 构建并运行
docker build -t axon .
docker run -p 8080:8080 -v ./axon-data:/data axon
```

### 5.4 交叉编译环境搭建

```bash
#!/bin/bash
# scripts/setup-cross-compile.sh
# 在 Linux/macOS 上执行，搭建 Android 交叉编译环境

set -euo pipefail

echo "=== Setting up Android cross-compile environment ==="

# 1. 安装 Android NDK
NDK_VERSION="28.2.13676358"
NDK_DIR="$HOME/Android/Sdk/ndk/$NDK_VERSION"

if [[ ! -d "$NDK_DIR" ]]; then
    echo "Installing NDK $NDK_VERSION..."
    mkdir -p "$HOME/Android/Sdk/ndk"
    wget -qO /tmp/ndk.zip \
        "https://dl.google.com/android/repository/android-ndk-r${NDK_VERSION}-linux.zip"
    unzip -q /tmp/ndk.zip -d "$HOME/Android/Sdk/ndk/"
    mv "$HOME/Android/Sdk/ndk/android-ndk-r${NDK_VERSION}" "$NDK_DIR"
fi

export ANDROID_NDK_HOME="$NDK_DIR"
export ANDROID_NDK_ROOT="$NDK_DIR"

# 2. 添加 Rust target
echo "Adding Rust target aarch64-linux-android..."
rustup target add aarch64-linux-android

# 3. 配置 cargo linker
mkdir -p ~/.cargo
cat >> ~/.cargo/config.toml << 'TOML'

[target.aarch64-linux-android]
linker = "ANDROID_NDK_HOME_PLACEHOLDER/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"
ar = "ANDROID_NDK_HOME_PLACEHOLDER/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
TOML

sed -i "s|ANDROID_NDK_HOME_PLACEHOLDER|$NDK_DIR|g" ~/.cargo/config.toml

echo ""
echo "=== Setup Complete ==="
echo "Build with: cargo build --release --target aarch64-linux-android --bin axon"
```

---

## 6. 配置示例

### 6.1 最小可用配置

```yaml
# config.example.yaml
# AXON 最小配置示例

server:
  addr: "0.0.0.0:8080"
  web_ui_enabled: true
  max_request_body_mb: 10

gateway:
  upstream_timeout_ms: 120000
  stream_timeout_ms: 300000
  retry_count: 2
  retry_delay_ms: 1000

storage:
  sqlite_path: "axon.db"
  max_connections: 5

observability:
  log_level: "info"
  metrics_enabled: true
  access_log_enabled: true

# 模型定义：配置上游 API
models:
  - name: "gpt-4o"
    provider: "openai"
    model_name: "gpt-4o"
    api_key_env: "OPENAI_API_KEY"      # 从环境变量读取
    api_base: "https://api.openai.com/v1"

  - name: "claude-sonnet"
    provider: "anthropic"
    model_name: "claude-sonnet-4-20250514"
    api_key_env: "ANTHROPIC_API_KEY"
    api_base: "https://api.anthropic.com"

  - name: "gemini-pro"
    provider: "vertex"
    model_name: "gemini-2.0-flash"
    api_key_env: "GOOGLE_API_KEY"
    api_base: "https://generativelanguage.googleapis.com/v1beta"

  - name: "deepseek-chat"
    provider: "openai"
    model_name: "deepseek-chat"
    api_key_env: "DEEPSEEK_API_KEY"
    api_base: "https://api.deepseek.com/v1"

# 路由定义
routes:
  - name: "default"
    strategy: "failover"               # 主模型故障自动切换
    targets:
      - model: "gpt-4o"
      - model: "claude-sonnet"         # 备用
      - model: "deepseek-chat"         # 再备用

# 智能体定义
agents:
  - id: "researcher"
    name: "研究助手"
    description: "能搜索网络并总结信息的研究助手"
    system_prompt: |
      你是一个研究助手。当用户提问时，如果需要最新信息，
      请使用 web_search 工具搜索相关内容，然后基于搜索结果
      给出全面、准确的回答。引用信息时请注明来源。
    model: "default"                   # 引用路由
    tools:
      - "web_search"
      - "memory"
    max_iterations: 5
    temperature: 0.7

  - id: "translator"
    name: "翻译助手"
    description: "多语言翻译助手"
    system_prompt: |
      你是一个专业翻译助手。请将用户输入翻译为指定语言。
      如果用户未指定目标语言，默认翻译为英文。
      保持原文的语气和格式。
    model: "gpt-4o"
    tools: []
    max_iterations: 1
    temperature: 0.3

  - id: "coder"
    name: "编程助手"
    description: "能执行代码的编程助手"
    system_prompt: |
      你是一个编程助手。你可以使用 code_exec 工具执行代码
      来验证你的回答。请先给出代码解释，然后执行验证。
    model: "claude-sonnet"
    tools:
      - "code_exec"
      - "memory"
    max_iterations: 5
    temperature: 0.5

# 工具配置
tools:
  - name: "web_search"
    kind: "web_search"
    config:
      engine: "duckduckgo"             # 免费无需 API Key
      max_results: 5

  - name: "memory"
    kind: "memory"
    config:
      namespace: "default"

  - name: "code_exec"
    kind: "code_exec"
    config:
      timeout_ms: 10000
      allowed_languages: ["python", "javascript"]
      max_output_chars: 10000
```

### 6.2 高级配置（多模型 + 限流）

```yaml
# config.advanced.yaml
server:
  addr: "0.0.0.0:8080"
  web_ui_enabled: true

gateway:
  upstream_timeout_ms: 120000
  stream_timeout_ms: 300000
  retry_count: 3
  retry_delay_ms: 1000

storage:
  sqlite_path: "axon.db"
  max_connections: 10

observability:
  log_level: "debug"
  metrics_enabled: true
  access_log_enabled: true

models:
  - name: "gpt-4o"
    provider: "openai"
    model_name: "gpt-4o"
    api_key_env: "OPENAI_API_KEY"
    api_base: "https://api.openai.com/v1"
    max_concurrency: 5
    rate_limit:
      rps: 10
      rpm: 100

  - name: "gpt-4o-mini"
    provider: "openai"
    model_name: "gpt-4o-mini"
    api_key_env: "OPENAI_API_KEY"
    api_base: "https://api.openai.com/v1"

  - name: "claude-sonnet"
    provider: "anthropic"
    model_name: "claude-sonnet-4-20250514"
    api_key_env: "ANTHROPIC_API_KEY"
    api_base: "https://api.anthropic.com"
    max_concurrency: 3

  - name: "claude-haiku"
    provider: "anthropic"
    model_name: "claude-haiku-4-20250514"
    api_key_env: "ANTHROPIC_API_KEY"
    api_base: "https://api.anthropic.com"

  - name: "local-llama"
    provider: "openai"
    model_name: "llama3.2"
    api_base: "http://127.0.0.1:11434/v1"  # Ollama 本地
    api_key: "ollama"                        # Ollama 不需要真实 Key

routes:
  - name: "premium"
    strategy: "failover"
    targets:
      - model: "gpt-4o"
      - model: "claude-sonnet"

  - name: "fast"
    strategy: "round_robin"
    targets:
      - model: "gpt-4o-mini"
      - model: "claude-haiku"

  - name: "local"
    strategy: "round_robin"
    targets:
      - model: "local-llama"

  - name: "cost-optimized"
    strategy: "weighted"
    targets:
      - model: "gpt-4o-mini"
        weight: 70
      - model: "claude-haiku"
        weight: 30

agents:
  - id: "researcher"
    name: "研究助手"
    description: "高级研究助手，使用优质模型"
    system_prompt: |
      你是一个研究助手。使用 web_search 工具搜索信息，
      使用 memory 工具记住重要事实。
      给出详尽、有引用的回答。
    model: "premium"
    tools: ["web_search", "memory", "http_fetch"]
    max_iterations: 10
    temperature: 0.7
    max_tokens: 4096

  - id: "quick-chat"
    name: "快速对话"
    description: "日常对话，使用快速模型"
    system_prompt: "你是一个友好的对话助手。"
    model: "fast"
    tools: []
    max_iterations: 1
    temperature: 0.8

  - id: "offline-assistant"
    name: "离线助手"
    description: "使用本地模型，无需网络"
    system_prompt: "你是一个离线运行的助手。"
    model: "local"
    tools: ["memory"]
    max_iterations: 3

tools:
  - name: "web_search"
    kind: "web_search"
    config:
      engine: "duckduckgo"
      max_results: 5

  - name: "memory"
    kind: "memory"
    config:
      namespace: "default"

  - name: "http_fetch"
    kind: "http_fetch"
    config:
      timeout_ms: 15000
      max_response_chars: 50000
```

### 6.3 环境变量配置

```bash
# .env 或 shell 导出
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="AI..."
export DEEPSEEK_API_KEY="sk-..."

# AXON 配置路径
export AXON_CONFIG="/path/to/config.yaml"

# 日志级别覆盖
export AXON_OBSERVABILITY__LOG_LEVEL="debug"
```

---

## 7. 测试与调试

### 7.1 测试策略

```
测试金字塔
    ┌───────────┐
    │   E2E     │  ← Vitest + 真实进程（> 50 用例）
    │  (少量)   │     验证完整请求→响应链路
    ├───────────┤
    │ 集成测试   │  ← Rust #[tokio::test] + SQLite
    │ (中等量)  │     验证模块间交互
    ├───────────┤
    │ 单元测试   │  ← Rust #[test]
    │ (大量)    │     验证单个函数/结构体
    └───────────┘
```

### 7.2 单元测试

```rust
// crates/axon-core/src/config.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_deserialization() {
        let yaml = r#"
server:
  addr: "0.0.0.0:8080"
  web_ui_enabled: true
gateway:
  upstream_timeout_ms: 120000
  stream_timeout_ms: 300000
  retry_count: 2
  retry_delay_ms: 1000
storage:
  sqlite_path: "axon.db"
  max_connections: 5
observability:
  log_level: "info"
  metrics_enabled: true
  access_log_enabled: true
agents: []
models: []
routes: []
tools: []
"#;
        let config: AxonConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.server.addr, "0.0.0.0:8080");
        assert!(config.server.web_ui_enabled);
    }

    #[test]
    fn test_agent_definition_validation() {
        let agent = AgentDefinition {
            id: "test".into(),
            name: "Test".into(),
            description: "Test agent".into(),
            system_prompt: "You are a test agent.".into(),
            model: "gpt-4o".into(),
            tools: vec![],
            max_iterations: 5,
            temperature: Some(0.7),
            max_tokens: None,
            metadata: serde_json::Value::Null,
        };
        assert!(agent.max_iterations > 0);
    }
}
```

### 7.3 集成测试

```rust
// crates/axon-runtime/tests/integration.rs
use axon_runtime::{AgentExecutor, StreamEvent};
use axon_store::Store;

#[tokio::test]
async fn test_agent_invoke_with_mock_gateway() {
    let store = Store::open(":memory:", 5).unwrap();
    let gateway = MockGateway::new();
    let executor = AgentExecutor::new(gateway, store);

    let result = executor.invoke("test-agent", "Hello", None).await.unwrap();

    assert!(!result.output.is_empty());
    assert!(result.iterations >= 1);
}

#[tokio::test]
async fn test_message_tree_persistence() {
    let store = Store::open(":memory:", 5).unwrap();
    let conv = store.create_conversation("agent-1").await.unwrap();

    let msg1 = Message::user(&conv.id, "Hello");
    store.add_message(&msg1).await.unwrap();

    let msg2 = Message::assistant(&conv.id, Some(&msg1.id), "Hi there!");
    store.add_message(&msg2).await.unwrap();

    let tree = store.get_message_tree(&conv.id).await.unwrap();
    assert_eq!(tree.root().content, "Hello");
    assert_eq!(tree.root().children.len(), 1);
}
```

### 7.4 E2E 测试（TypeScript + Vitest，借鉴 AISIX）

```typescript
// tests/e2e/src/cases/agent-invoke.test.ts
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { AxonApp } from '../harness/app';

describe('Agent Invocation', () => {
  let app: AxonApp;

  beforeAll(async () => {
    app = new AxonApp();
    await app.start();
    await app.seedConfig('configs/minimal.yaml');
  });

  afterAll(async () => app.stop());

  it('should invoke a simple agent and get response', async () => {
    const res = await app.post('/v1/agents/translator/invoke', {
      input: '你好，世界',
    });
    expect(res.status).toBe(200);
    expect(res.data.output).toBeTruthy();
    expect(res.data.conversation_id).toBeTruthy();
  });

  it('should stream agent response via SSE', async () => {
    const stream = await app.postStream('/v1/agents/translator/invoke', {
      input: 'Hello, world',
    });

    const events = await stream.collect();
    expect(events.some(e => e.type === 'text_chunk')).toBe(true);
    expect(events.some(e => e.type === 'done')).toBe(true);
  });

  it('should execute tools in multi-turn', async () => {
    const res = await app.post('/v1/agents/researcher/invoke', {
      input: 'What is the latest version of Rust?',
    });
    expect(res.status).toBe(200);
    expect(res.data.tool_calls.length).toBeGreaterThan(0);
    expect(res.data.output).toContain('Rust');
  });
});
```

### 7.5 本地调试方法

#### 7.5.1 开发模式运行

```bash
# 1. 构建 Web UI（开发模式，热重载）
cd ui && npm run dev

# 2. 运行 AXON（Rust 热重载需 cargo-watch）
cargo watch -x 'run --bin axon -- --config config.example.yaml'

# 3. 访问 API
curl http://localhost:8080/healthz

# 4. 测试智能体调用
curl -X POST http://localhost:8080/v1/agents/translator/invoke \
  -H "Content-Type: application/json" \
  -d '{"input": "你好，世界"}'
```

#### 7.5.2 日志调试

```bash
# 启用 debug 日志
AXON_OBSERVABILITY__LOG_LEVEL=debug axon --config config.yaml

# 查看结构化日志
tail -f axon.log | jq .

# 查看指标
curl http://localhost:8080/metrics
```

#### 7.5.3 SQLite 调试

```bash
# 使用 sqlite3 CLI 查看数据
sqlite3 axon.db

# 常用查询
SELECT * FROM conversations ORDER BY created_at DESC LIMIT 10;
SELECT * FROM messages WHERE conversation_id = 'xxx' ORDER BY created_at;
SELECT model, SUM(total_tokens) as tokens FROM usage_records GROUP BY model;
```

#### 7.5.4 性能分析

```bash
# 构建带 debug 信息的二进制
cargo build --release --features profiling

# 使用 perf 分析
perf record -g ./axon --config config.yaml
perf report

# 内存分析（valgrind，仅 Linux）
valgrind --tool=massif ./axon --config config.yaml
ms_print massif.out.*
```

---

## 8. 文档结构

### 8.1 文档目录

```
docs/
├── README.md                    # 项目介绍（根目录也有）
├── getting-started.md           # 快速入门（5 分钟）
├── installation/
│   ├── termux.md               # Termux 安装指南
│   ├── apk.md                  # APK 安装指南
│   └── docker.md               # Docker 运行指南
├── configuration/
│   ├── reference.md            # 配置项完整参考
│   ├── models.md               # 模型配置指南
│   ├── agents.md               # 智能体配置指南
│   ├── routes.md               # 路由配置指南
│   └── tools.md                # 工具配置指南
├── api/
│   ├── overview.md             # API 概览
│   ├── openapi.json            # OpenAPI 3.1 规范（自动生成）
│   ├── chat-completions.md     # OpenAI 兼容端点
│   ├── agents.md               # 智能体 API
│   └── conversations.md        # 对话 API
├── development/
│   ├── architecture.md         # 架构设计文档
│   ├── contributing.md         # 贡献指南
│   ├── building.md             # 构建指南
│   └── cross-compile.md        # 交叉编译指南
├── tools/
│   ├── web-search.md           # Web 搜索工具
│   ├── code-exec.md            # 代码执行工具
│   ├── memory.md               # 记忆工具
│   └── custom.md               # 自定义工具开发
└── examples/
    ├── minimal.yaml            # 最小配置
    ├── multi-model.yaml        # 多模型配置
    └── research-agent.yaml     # 研究助手示例
```

### 8.2 README.md 提纲

```markdown
# AXON

> 移动端多智能体协作与 AI 网关轻量级平台

## 简介
- 一句话定位
- 核心特性列表
- 截图/GIF

## 快速入门
- 安装（3 种方式）
- 最小配置
- 首次调用

## 核心功能
- 智能体编排
- 统一模型网关
- 工具调用
- Web UI

## 配置
- 配置文件说明
- 智能体定义
- 模型路由

## 部署
- Termux
- APK
- Docker

## API
- OpenAI 兼容
- AXON 扩展

## 开发
- 构建
- 测试
- 贡献

## 致谢
- AISIX (Apache APISIX 团队)
- Agora (设计灵感)

## 许可证
Apache-2.0
```

### 8.3 快速入门提纲

```markdown
# 快速入门（5 分钟）

## 1. 安装
## 2. 配置 API Key
## 3. 启动 AXON
## 4. 创建第一个智能体
## 5. 调用智能体
## 6. 查看 Web UI
```

### 8.4 API 文档提纲

```markdown
# API 参考

## 认证
## OpenAI 兼容端点
  ### POST /v1/chat/completions
  ### POST /v1/completions
  ### POST /v1/embeddings
  ### GET /v1/models
## 智能体端点
  ### POST /v1/agents
  ### GET /v1/agents
  ### POST /v1/agents/:id/invoke
## 对话端点
  ### POST /v1/conversations
  ### GET /v1/conversations/:id/messages
## 运维端点
  ### GET /healthz
  ### GET /metrics
## 错误码参考
```

---

## 9. 约束满足分析

### 9.1 二进制大小 < 50MB

| 组成 | 预估大小 | 优化措施 |
|---|---|---|
| AISIX 网关子集（core + gateway + 3 provider） | ~8 MB | 裁剪 etcd/Redis/Bedrock/Vertex/Azure 未用 crate |
| axon-runtime + axon-tools | ~3 MB | 精简工具实现 |
| axon-store（rusqlite + SQLite 静态链接） | ~2 MB | SQLite 静态编译 |
| axum + tokio + hyper | ~4 MB | release LTO + strip |
| serde + 其他工具库 | ~2 MB | |
| Web UI 静态资源（gzip 后嵌入） | ~1 MB | Vite 构建 + gzip + include_dir! |
| **总计** | **~20 MB** | 远低于 50MB 约束 |

**优化命令**：
```bash
# Cargo.toml 中配置
[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"        # 比 "debuginfo" 更激进
panic = "abort"          # 减少 unwinding 代码

# 构建后进一步压缩
upx --best --lzma target/aarch64-linux-android/release/axon
# UPX 可将 20MB 压缩到 ~7MB
```

### 9.2 启动内存 < 200MB

| 组成 | 预估内存 | 说明 |
|---|---|---|
| Tokio 运行时（2 worker 线程） | ~5 MB | 移动端限制 worker 数 |
| SQLite 连接池（3 连接） | ~15 MB | 每连接 ~5MB |
| 配置快照（ArcSwap） | ~2 MB | 配置数据 |
| HTTP 连接池（reqwest） | ~10 MB | 空闲连接池 |
| Web UI 静态资源（内存映射） | ~1 MB | include_dir! 按需读取 |
| **总计** | **~33 MB** | 远低于 200MB 约束 |

**优化配置**：
```yaml
# 限制资源使用
server:
  workers: 2              # 限制 worker 线程数
storage:
  max_connections: 3      # 限制 SQLite 连接数
gateway:
  pool_max_idle_per_host: 2  # 限制 HTTP 连接池
```

### 9.3 支持离线配置

- 配置文件为本地 YAML，无需网络即可加载
- 智能体定义、模型路由、工具配置全部在本地
- 支持 Ollama 等本地模型提供商（`api_base: "http://127.0.0.1:11434/v1"`）
- 网关代理功能需网络（调用云端 API），但本地模型不需要
- SQLite 数据库本地存储，离线可读写

### 9.4 开源协议

- **Apache-2.0**：与 AISIX 一致，允许商业使用
- 所有依赖均为兼容许可证（MIT/Apache-2.0/BSD）

---

## 附录 A：项目初始化脚手架

```bash
#!/bin/bash
# scripts/init-project.sh
# 初始化 AXON 项目结构

set -euo pipefail

mkdir -p axon && cd axon

# Cargo workspace
cat > Cargo.toml << 'EOF'
[workspace]
resolver = "2"
members = [
    "crates/axon-core",
    "crates/axon-gateway",
    "crates/axon-runtime",
    "crates/axon-tools",
    "crates/axon-store",
    "crates/axon-protocol",
    "crates/axon-server",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
axum = "0.7"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"

[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"
panic = "abort"
EOF

# 创建各 crate 骨架
for crate in axon-core axon-gateway axon-runtime axon-tools axon-store axon-protocol axon-server; do
    mkdir -p "crates/$crate/src"
    cat > "crates/$crate/Cargo.toml" << EOF
[package]
name = "$crate"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
EOF
    echo "//! $crate" > "crates/$crate/src/lib.rs"
done

# axon-server 是二进制
cat >> crates/axon-server/Cargo.toml << 'EOF'

[[bin]]
name = "axon"
path = "src/main.rs"
EOF

cat > crates/axon-server/src/main.rs << 'EOF'
fn main() {
    println!("AXON v0.1.0");
}
EOF

echo "Project scaffold created."
```

## 附录 B：Cargo.toml 完整示例

```toml
# crates/axon-server/Cargo.toml
[package]
name = "axon-server"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "axon"
path = "src/main.rs"

[dependencies]
axon-core = { path = "../axon-core" }
axon-gateway = { path = "../axon-gateway" }
axon-runtime = { path = "../axon-runtime" }
axon-store = { path = "../axon-store" }
axon-protocol = { path = "../axon-protocol" }

tokio = { workspace = true }
axum = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
clap = { version = "4", features = ["derive"] }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
notify = "6"              # 文件监听（热重载）
include_dir = "0.7"       # 嵌入 Web UI 静态资源
tower-http = { version = "0.6", features = ["fs", "cors", "trace"] }

[features]
default = ["web-ui"]
web-ui = []
profiling = []
```

---

*本方案基于对 [AISIX](https://github.com/api7/aisix)（v0.3.0，18 crate Rust workspace）和 [Agora](https://github.com/ojbkxc/AXON)（v1.0.1，Android Kotlin LLM 客户端）的深入分析编制。*
