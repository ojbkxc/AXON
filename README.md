# AXON

> 面向 Android 移动端的多智能体协作 + AI 网关轻量级平台,用 Rust 编写,产出 < 50MB 的 ARM64 单二进制,可在 Termux 或简易 APK 中后台运行。

[![CI](https://github.com/ojbkxc/AXON/actions/workflows/ci.yml/badge.svg)](https://github.com/ojbkxc/AXON/actions/workflows/ci.yml)
[![APK](https://github.com/ojbkxc/AXON/actions/workflows/android-apk.yml/badge.svg)](https://github.com/ojbkxc/AXON/actions/workflows/android-apk.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

## 特性

- **AI 网关**:统一多 provider(OpenAI/Anthropic/Vertex)的 chat 接口,支持路由(failover/round_robin/weighted)、SSE 流式、配置热重载
- **智能体引擎**:AgentExecutor + GenerationPipeline,支持 LLM↔Tool 多轮循环、max_iterations 守护、消息持久化、用量记录
- **工具编排**:内置 5 个工具(web_search/shell/memory/http_fetch/code_exec)
- **限流**:两阶段限流(pre-commit + post-deduct),支持 RPS/RPM/RPH/RPD/TPM/TPD/concurrency 7 个维度
- **Web UI**:React 18 + Vite 5 + TS + TailwindCSS 3,暗色主题,5 个页面,嵌入二进制
- **移动端**:Android APK(WebView 壳工程 + 前台服务),Termux 一键安装
- **可观测性**:tracing + Prometheus /metrics + 结构化 JSON 日志

## 快速入门

```bash
# 1. 设置 API key
export OPENAI_API_KEY="sk-..."

# 2. 构建并运行
cargo run -p axon-server -- --config config.example.yaml

# 3. 打开 Web UI
open http://localhost:8080/ui/

# 4. 或用 OpenAI 兼容接口
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"hello"}]}'
```

## 架构

```
Client → axon-server(axum) → axon-runtime(AgentExecutor) → axon-gateway(Provider) → 上游 LLM
                                  ↕ axon-tools              ↕ axon-store
```

| Crate | 职责 |
|---|---|
| axon-core | Config / Error / 资源模型 |
| axon-store | SQLite 持久化(对话/消息/用量/记忆) |
| axon-gateway | 嵌入式 AI 网关 + OpenAI/Anthropic provider + SSE |
| axon-tools | 内置工具集(web_search/shell/memory/http_fetch/code_exec) |
| axon-runtime | 智能体编排引擎(AgentExecutor + GenerationPipeline) |
| axon-protocol | HTTP API 协议定义(OpenAI 兼容 + AXON 扩展) |
| axon-ratelimit | 两阶段限流(7 维度) |
| axon-server | HTTP 服务器 + CLI + 配置热重载 + UI 嵌入 |

## 文档

- [docs/API.md](docs/API.md) — HTTP API 路由参考
- [docs/CONFIG.md](docs/CONFIG.md) — 配置文件参考
- [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) — 部署指南
- [AGENTS.md](AGENTS.md) — 代理工作指引
- [AXON_PROJECT_PLAN.md](AXON_PROJECT_PLAN.md) — 完整计划书

## 开发

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p axon-server
ls -lh target/release/axon  # < 50MB
```

## 体积

| 产物 | 大小 |
|---|---|
| axon 二进制(arm64) | 5.1MB |
| APK | 5.0MB |

## 许可证

Apache-2.0
