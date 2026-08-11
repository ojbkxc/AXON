# AXON 文档

AXON 是面向 Android 移动端的多智能体协作 + AI 网关轻量级平台,用 Rust 编写,产出 < 50MB 的 ARM64 单二进制,可在 Termux 或简易 APK 中后台运行。

## 文档索引

| 文档 | 内容 |
|---|---|
| [API.md](./API.md) | HTTP API 路由参考(14 条路由) |
| [CONFIG.md](./CONFIG.md) | 配置文件参考(config.yaml) |
| [DEPLOYMENT.md](./DEPLOYMENT.md) | 部署指南(Termux / Android APK) |

## 快速入门

```bash
# 1. 构建并运行
cargo run -p axon-server -- --config config.example.yaml

# 2. 打开 Web UI
open http://localhost:8080/ui/

# 3. 调用 OpenAI 兼容接口
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"hello"}],"stream":true}'
```

## 相关文档

- [AGENTS.md](../AGENTS.md) — 代理工作指引(单一事实源)
- [AXON_PROJECT_PLAN.md](../AXON_PROJECT_PLAN.md) — 完整计划书
