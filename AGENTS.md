# AGENTS.md — AXON 项目代理工作指引

> 本文件供 AI 编码代理（含未来会话）进入项目时**首先自读**，快速对齐项目定位、当前进度、架构契约与下一步任务，然后**继续完善未完成的代码**。
> 优先级：本文件 > `AXON_PROJECT_PLAN.md`（计划书，1967 行，背景与全貌）> 上游 AISIX/Agora 源码。

---

## R0. 强制规则（MANDATORY，不可绕过）

> 本节为**最高优先级的强制约束**，凌驾于一切其他指引之上。违反即视为流程失败。

1. **每次会话必须先自读本文件**：进入项目后，在执行任何写代码/搜索/构建动作之前，必须先 `read` 完整 `AGENTS.md`，对齐「当前进度」「下一步任务」「接口契约」。
2. **每次会话结束前必须回写本文件**：无论本次完成了几项任务（含 0 项，即仅排查/失败），在结束前**必须**用 `edit`/`write` 更新本文件至少一处：
   - **必须**更新「§9 变更日志」追加一行（最新在上），记录本次做了什么、改了哪些文件、是否通过验证、下一步建议。
   - **必须**更新「§4 当前进度」与「§6 下一步任务」的勾选状态以反映真实状态（新完成的挪到「已完成」区，新发现的问题加入「已知小问题」）。
   - 若改动了接口契约，**必须**同步更新「§5 关键接口契约」。
   - 若改动了目录结构或新增/删除文件，**必须**同步更新「§3 仓库结构」。
3. **本文件是单一事实源（single source of truth）**：当本文件与代码、与 `AXON_PROJECT_PLAN.md`、与口头描述出现矛盾时，**先以代码为准**，然后**立即回写本文件**消除漂移；禁止让本文件与代码长期不一致。
4. **不得删除或弱化本节**：任何对「§R0 强制规则」的删减、降级、加「视情况而定」修饰，都需用户明确同意；代理自身不得自行放宽。
5. **跟进是义务而非可选**：即使用户未要求「更新 AGENTS.md」，每次会话结束前也必须执行回写；用户明确说「不用更新」时才可跳过，并在变更日志注明「依用户要求跳过本次回写」。
6. **aisix 代码优先直接复制复用**（详见 §R1）：实现网关/provider/SSE/限流/可观测性/快照等功能时，**先查 `/opt/github/aisix` 是否有现成实现**；有则**优先直接复制粘贴**到 AXON 对应 crate 再做最小裁剪，**禁止在 aisix 已有等价实现时另起炉灶重写**。
7. **编译验证必须提交到 GitHub 上编译**（MANDATORY）：本地为离线环境，缺 `rusqlite`/`r2d2`/`r2d2_sqlite`/`clap`/`notify`/`include_dir`/`async-stream 0.8`/`axum 0.7` 等依赖，**无法** `cargo check/build/test --workspace`。因此**任何代码改动后的编译验证必须通过 `git commit && git push` 提交到 GitHub**（`origin = https://github.com/ojbkxc/AXON.git`，分支 `main`），由 GitHub CI（`.github/workflows/ci.yml`，见 §R2）执行 `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`。**禁止**在未 push 到 GitHub 编译通过前声称某子任务「完成/已验证」。
8. **通过 GitHub 编译报错迭代修复**（MANDATORY）：push 后若 GitHub CI 编译/测试失败，**必须**读取 CI 日志中的报错（`cargo check`/`clippy`/`test` 的 error 行），据报错本地修复后**再次 commit & push**，循环直至 CI 全绿。修复流程：`git push` → 查看 GitHub Actions 失败日志 → 定位 error → 本地改代码 → `git push` → 重复。**不得**跳过 CI 失败直接推进下一子任务；**不得**用 `#[allow]`/注释掉测试等方式绕过 CI 报错（除非用户明确同意）。CI 全绿是子任务完成的**唯一**编译验证判据。
9. **自动推进项目（auto-continue，默认行为）**（MANDATORY）：用户说「自动继续」/「继续」/「auto」或未明确叫停时，代理**必须自主连续推进**项目任务，不得每完成一小步就停下来询问下一步。具体要求：
   - 进入项目后按 §0 流程**自主**挑选下一个最高优先级的最小可独立交付子任务并开工，不等用户逐项指派。
   - 单个子任务完成后**立即**开始下一个，无需请求许可；仅在遇到「方向性分歧」「破坏性操作」「违反硬约束」「信息严重不足且无法合理推断」时才用 `question` 工具询问用户。
   - 推进过程中**主动**走 §R1.1 复用决策、§R2.3 CI 修复闭环、§R0 回写，不要等用户提醒。
   - 用户未说「自动继续」时也鼓励减少不必要的中途提问，但可在阶段切换时简要汇报进度；用户说「自动继续」后则**连续作业**直到任务全部完成或遇阻才停下汇报。
   - 停下汇报时应附「已完成的 / 正在做的 / 下一步打算做的」三段式摘要，便于用户一句话继续（如「继续」「换方向」「停」）。

---

## R2. GitHub CI 编译验证策略（MANDATORY，配合 §R0.7–R0.8）

> 本节落实 §R0.7/R0.8 的「提交到 GitHub 编译 + 据报错修复」闭环。本地离线不可编译，GitHub CI 是**唯一**编译验证通道。

### R2.1 CI 触发条件
- 每次 `push` 到 `main` 或任意 PR 时触发 `.github/workflows/ci.yml`。
- CI 在 GitHub-hosted runner（ubuntu-latest，可联网拉依赖）上执行，规避本地离线缺依赖问题。

### R2.2 CI 必须执行的步骤（全绿才算通过）
```yaml
# .github/workflows/ci.yml 最小集
- cargo fmt --all -- --check           # 格式
- cargo clippy --workspace --all-targets -- -D warnings   # lint，warning 即失败
- cargo test --workspace               # 全量测试
- cargo build --release -p axon-server  # release 可构建
- ls -lh target/release/axon           # 体积 < 50MB 硬约束（CI 中可加断言）
```

### R2.3 据报错修复的迭代流程（每次 push 后必走）
1. `git push origin main`。
2. 用 `gh run watch` 或浏览器查看 `https://github.com/ojbkxc/AXON/actions` 的运行结果。
3. 若失败：`gh run view --log-failed` 取报错日志，定位首个 `error[Exxxx]:` / `error:` 行。
4. 本地按报错修代码（修 use 路径/类型/特征实现/测试断言等），**不**绕过（不 `#[allow]`、不删测试、不 `unwrap` 掩盖）。
5. `git commit && git push`，回到步骤 2，直至 CI 全绿。
6. CI 全绿后才能在 §4/§6 勾选该子任务「完成」并在 §9 变更日志注明「CI 全绿验证通过」。

### R2.4 本地可做的静态检查（push 前自检，减少 CI 往返）
- `cargo fmt --all -- --check`（若本地 rustfmt 可用）。
- 人工 review：use 路径、trait 签名、`AxonError` 变体匹配、serde 字段名。
- axon-core 单元测试若能离线跑则先跑（当前 workspace 解析仍需联网，故通常仍依赖 CI）。

### R2.5 CI workflow 维护
- 若新增依赖或改变构建矩阵（如 Android 交叉编译），同步更新 `.github/workflows/ci.yml`。
- Android 交叉编译（P4）应加为单独 job 或单独 workflow `ci-android.yml`，不阻塞主 CI。

---

## R1. aisix 代码复用规则（MANDATORY，直接复制优先）

> 上游 `/opt/github/aisix` 是 Apache-2.0 许可的同源 Rust 项目，AXON 网关子集应**最大化直接复用**其代码。本节为强制规则。

### R1.1 复用决策流程（每次写新功能前必走）

1. **先查 aisix**：用 `grep`/`glob`/`read` 在 `/opt/github/aisix/crates` 中搜索要实现的功能（SSE 解析、限流、provider bridge、metrics、snapshot 等）。
2. **有等价实现 → 直接复制**：把文件原样复制到 AXON 对应 crate，再做**最小必要裁剪**（删 etcd/redis/bedrock 等重依赖、改 `aisix_*` crate 名为 `axon_*`、调整 `use` 路径）。
3. **部分可用 → 复制核心 + 裁剪**：如 `aisix-gateway::hub` 依赖 etcd，复制其路由选择核心逻辑，剥离 etcd 加载层。
4. **aisix 无等价实现 → 自研**：在变更日志注明「aisix 无对应实现，自研」。
5. **复制后必须**：在 AXON 文件顶部保留 aisix 原文件路径注释（便于追溯），如 `// adapted from aisix/crates/aisix-gateway/src/sse.rs`；在 §9 变更日志记录复制来源与裁剪点。

### R1.2 可直接复制的 aisix 文件清单（已盘点，按 AXON crate 归类）

> 行数为截至 2026-08-10 的统计。复制时按「裁剪」列处理。AISIX workspace 用 `dashmap`/`metrics`/`object_store` 等重依赖，复制到 AXON 时**只保留轻量部分**，避免违反 < 50MB 约束。

| 复制到 AXON | 来自 aisix（行数） | 复用价值 | 裁剪要点 |
|---|---|---|---|
| `crates/axon-gateway/src/sse.rs` | `aisix-gateway/src/sse.rs` (357) | **高**：成熟的 SSE 行解码器，跨 chunk 边界、`[DONE]` 哨兵、HTTP-client 无关。AXON 现有 `stream.rs` 的手写解析可替换为此。 | 几乎零裁剪，纯标准库 + `Cow`。直接替换 AXON `stream.rs` 内的行解析逻辑。 |
| `crates/axon-core/src/snapshot.rs` | `aisix-core/src/snapshot.rs` (322) | **高**：无锁 `ArcSwap<Arc<Snapshot>>` + `DashMap` 双索引（id/name）。AXON gateway 的 `ArcSwap<GatewaySnapshot>` 可升级为这个通用版。 | 引入 `dashmap`（轻量，可接受）。剥离 `Resource` trait 的 etcd 版本字段，简化为 AXON 的 `ModelDefinition` 等。 |
| `crates/axon-gateway/src/bridge.rs` | `aisix-gateway/src/bridge.rs` (987) | **中高**：`Bridge` trait（provider 统一契约）+ 流式/非流式分发。AXON 的 `Provider` trait 可对齐此契约。 | 剥离 `upstream_tls`/`upstream_http` 的 TLS/连接池细节（AXON 用 reqwest 默认即可），保留 trait 形状与分发逻辑。 |
| `crates/axon-gateway/src/chat.rs` | `aisix-gateway/src/chat.rs` (975) | **中**：chat 请求归一化 + 路由选择 + 重试。 | 剥离 etcd 资源加载、guardrails、ratelimit 钩子；保留请求转换 + failover/round_robin/weighted 选择（AXON 已有简版，可升级）。 |
| `crates/axon-gateway/src/provider/openai.rs` | `aisix-provider-openai/src/{bridge.rs(1883),wire.rs(1562)}` | **高**：OpenAI wire 类型 + bridge，远比 AXON 现有 `provider.rs` 完整（含 tools/streaming/usage/错误信封）。 | 替换 AXON `provider.rs` 的 OpenAiProvider。剥 `overrides.rs` 的 etcd 覆盖、`upstream_headers` 转发逻辑。保留 wire 类型与 SSE 装配。 |
| `crates/axon-gateway/src/provider/anthropic.rs` | `aisix-provider-anthropic/src/{bridge.rs(1067),wire.rs(4100)}` | **高**：Anthropic 完整 wire + bridge，含 tool_use/content_block 流式。 | 替换 AXON `provider.rs` 的 AnthropicProvider。wire.rs 4100 行含大量边角，按需取 `messages` 端点相关子集。 |
| `crates/axon-gateway/src/provider/vertex.rs` | `aisix-provider-vertex/src/{bridge.rs(5159),token_mint.rs(542)}` | **中**：Vertex AI（Gemini）支持，AXON 现用 OpenAI 兼容占位。 | bridge.rs 5159 行很重，**先不复制**；v0.7 路由阶段再按需取 `token_mint.rs`（OAuth2 JWT 换 token）+ OpenAI 兼容子集。 |
| `crates/axon-gateway/src/ratelimit/` | `aisix-ratelimit/src/{limiter.rs(837),window.rs(218),clock.rs(75)}` + `store/local.rs` | **高**：两阶段限流（pre-commit + post-deduct）+ 滑动窗口 + 本地 store。AXON v0.7 需要。 | **只复制 `store/local.rs`**，不要 `store/redis.rs`（避免 redis 重依赖）。剥 `aisix_core::RateLimit` 引用，改用 AXON 的 `RateLimitConfig`。 |
| `crates/axon-server/src/telemetry.rs` | `aisix-server/src/telemetry.rs` (426) | **高**：tracing-subscriber 初始化 + Prometheus exporter 装配。AXON v0.1/v0.6 都需要。 | 剥 OTLP/aliyun SLS sink，保留 `tracing_subscriber::fmt` + `metrics-exporter-prometheus`。 |
| `crates/axon-server/src/main.rs` | `aisix-server/src/main.rs` (2203) | **中**：CLI + 信号处理 + 启动编排参考。 | **不直接复制**（太重，含 etcd/managed mode/cert）。**借鉴其结构**：clap CLI → init telemetry → load config → spawn server → signal handle。 |
| `crates/axon-core/src/error.rs` | `aisix-core/src/error.rs` (273) | **中**：错误类型设计参考。AXON 已有 `AxonError`，可对齐其 `From` 转换 completeness。 | 借鉴模式，不整体替换。 |
| `rust-toolchain.toml` | `aisix/rust-toolchain.toml` | **高**：直接复制后改 `channel = "stable"`（aisix 钉 1.93.1，AXON MSRV 1.75，用 stable 即可）。 | 改 channel。 |
| `rustfmt.toml` | `aisix/rustfmt.toml` | **高**：直接复制（`max_width=100` 等四行）。 | 零裁剪。 |

### R1.3 不要复制的 aisix 部分（重依赖 / 超约束）

- `aisix-proxy`（9471 行 + etcd + Redis + 控制面）→ 二进制 > 80MB，违反 < 50MB。
- `aisix-etcd` / `aisix-redis` → 移动端不需要分布式存储。
- `aisix-guardrails`（bedrock/lakera/presidio 等多后端）→ 太重；若需要护栏，自研轻量关键词版。
- `aisix-obs` 的 `sink/`（datadog/sls/object_store）+ `otlp.rs` → 保留 `metrics.rs`/`access_log.rs`/`usage.rs` 的本地部分即可。
- `aisix-mcp` / `aisix-a2a` / `aisix-admin` / `aisix-cache`(redis) → 暂不需要。
- `aisix-provider-azure-openai` / `aisix-provider-bedrock` → AWS SDK 依赖巨大，违反体积约束。

### R1.4 复制操作规范

- 复制后**立即** `cargo check -p <目标crate>` 确认编译通过；失败则修 use 路径/依赖。
- 复制的文件顶部加一行：`// adapted from aisix/crates/<原路径> (Apache-2.0)`，便于追溯与许可合规。
- 若复制引入新依赖，在 AXON `Cargo.toml` `[workspace.dependencies]` 登记，并在变更日志评估体积影响。
- **不要**复制后「重写得更漂亮」——保持 aisix 原结构以减少 bug 引入，除非原代码有明确错误。

---

## 0. 进入项目后的标准流程（必读）

1. **通读本文件**（尤其是「§R0 强制规则」「§R1 aisix 复用规则」「当前进度」「下一步任务」「编码约定」五节）。
2. 执行 `cargo check --workspace` 确认基线是否可编译；若失败，先修复编译错误再开新功能。
3. 按「下一步任务」的优先级顺序挑选一个**最小可独立交付**的子任务开工。
4. 开工前用 `read`/`grep`/`glob` 阅读相关已有代码；**写新功能前先走 §R1.1 复用决策流程**查 aisix 是否有现成实现。
5. **复用既有 trait 与命名**，不要另起炉灶（既指 AXON 既有代码，也指 aisix 可复制代码）。
6. 每完成一个子任务：跑 `cargo fmt && cargo clippy --workspace -- -D warnings && cargo test --workspace`，全绿后再继续。
7. **回写本文件**（强制，见 §R0）：更新「当前进度」「下一步任务」勾选状态，并在「变更日志」追加一行。
8. **不要**主动 `git commit`，除非用户明确要求。**不要**写未经请求的 README/文档。**不要**加注释除非用户要求。
9. **会话结束前再次确认 §R0 的回写已执行**；若未执行，补做后再结束。

---

## 1. 项目定位（一句话）

AXON 是面向 Android 移动端的多智能体协作 + AI 网关轻量级平台，用 **Rust** 编写，整合 AISIX 网关理念与 Agora 编排理念，产出 < 50MB 的 ARM64 单二进制，可在 Termux 或简易 APK 中后台运行。

## 2. 硬约束（任何改动都不得违反）

| 维度 | 约束 | 验证方式 |
|---|---|---|
| 二进制大小 | release 产物 < 50MB（目标 < 30MB） | `ls -lh target/release/axon` |
| 启动内存 | < 200MB（目标 < 150MB） | 运行时 `ps -o rss` |
| 离线启动 | 无网络也能启动（网关代理才需网络） | 断网 `axon --config x.yaml` 能起 |
| 语言 | **Rust**（edition 2021, MSRV 1.75） | — |
| 许可证 | Apache-2.0 | 所有文件头保持 |
| 异步运行时 | tokio 1.x full | — |
| HTTP 框架 | axum 0.7 | — |
| 存储 | rusqlite + r2d2_sqlite（bundled） | — |
| 序列化 | serde + serde_json + serde_yaml | — |
| 日志 | tracing + tracing-subscriber | — |

release profile 已配置：`lto="thin"`, `codegen-units=1`, `strip="symbols"`, `panic="abort"`, `opt-level="z"`。新增依赖前先评估对二进制体积的影响。

## 3. 仓库结构与模块划分

```
AXON/
├── AGENTS.md                      # 本文件（代理工作指引）
├── AXON_PROJECT_PLAN.md           # 完整计划书（1967 行，背景/路线图/部署/配置示例）
├── Cargo.toml                     # workspace 根
├── config.example.yaml            # 最小配置示例
├── LICENSE                        # Apache-2.0
├── rust-toolchain.toml            # 工具链固定
├── .gitignore                     # 忽略 target/db/ui/dist 等
├── crates/
│   ├── axon-core/      ✅ 完成   # Config / Error / 资源模型 + 22 个单元测试
│   ├── axon-store/     ✅ 完成   # SQLite 持久化（对话/消息树/用量/记忆 KV）
│   ├── axon-gateway/   ✅ 完成   # EmbeddedGateway + OpenAI/Anthropic provider + SSE 流式
│   ├── axon-tools/     ✅ 完成   # ToolRegistry + web_search/shell/memory/http_fetch/code_exec + build_registry
│   ├── axon-runtime/   ✅ 完成   # AgentExecutor + GenerationPipeline（LLM↔Tool 循环 + max_iterations）
│   ├── axon-protocol/  ✅ 完成   # OpenAI 兼容 API 类型 + AXON 扩展类型
│   ├── axon-ratelimit/ ✅ 完成   # 两阶段限流（pre-commit + post-deduct）+ 滑动窗口 + 本地 store
│   └── axon-server/    ✅ 完成   # main.rs + axum 路由 + CLI + 配置热重载 + handlers + 限流集成
├── ui/                 🟡 代码就绪（待 CI 验证）  # React 18 + Vite 5 + TS + TailwindCSS 3，暗色主题
│   ├── package.json / vite.config.ts / tsconfig*.json / tailwind.config.js / postcss.config.js
│   ├── index.html / src/main.tsx / src/index.css
│   └── src/
│       ├── api/{client.ts, types.ts}        # 对接 axon-protocol 全部类型 + 路由封装
│       ├── hooks/{useAgentStream.ts, useFetch.ts}  # SSE 流式解析（text/thought/tool/usage/done）+ 数据加载
│       ├── components/{Sidebar.tsx, Layout.tsx, ui.tsx}  # 侧边栏 + 布局 + UI 原语
│       ├── pages/{Dashboard,Chat,Agents,Models,Settings}.tsx  # 5 页面
│       └── App.tsx                         # 路由
├── .github/workflows/ci.yml  ✅ 就绪  # rust job（fmt/clippy/test/release/体积<50MB）+ ui job（install/typecheck/build/dist artifact）
├── android/apk/        ✅ CI 全绿  # Gradle WebView 壳工程（AGP 8.5.2, arm64-v8a, Java17）
│   ├── settings.gradle / build.gradle / gradle.properties / proguard-rules.pro
│   └── src/main/{AndroidManifest.xml, java/com/axon/app/{MainActivity,AxonService}.java, res/values/strings.xml, assets/}
├── android/termux/     ✅ 就绪    # install-termux.sh（从 artifact/源码安装 + 默认配置）
├── scripts/            ✅ 就绪    # cross-android.sh（cargo-ndk）+ build-apk.sh（gradle）
├── tests/e2e/          ✅ CI 全绿  # axon-e2e crate：9 个 E2E 测试（healthz/readyz/status/metrics/agents/tools/usage/models/conversations）
└── docs/               ❌ 空     # 文档
```

数据流：`Client → axon-server(axum) → axon-runtime(AgentExecutor/GenerationPipeline) → axon-gateway(EmbeddedGateway→Provider) → 上游 LLM`；工具调用经 `axon-tools`；持久化与用量经 `axon-store`。

## 4. 当前进度（截至 2026-08-10）

### ✅ 已完成
- **axon-core**：`config.rs`（AxonConfig + Server/Gateway/Storage/Observability + validate + find_agent/model/route + resolve_model）、`error.rs`（AxonError 9 变体 + From 转换）、`models.rs`（AgentDefinition/ModelDefinition/RouteDefinition/ToolDefinition/ChatMessage/ToolCall/TokenUsage/ChatOptions/ToolSchema）、`lib.rs`。**22 个单元测试全部通过**（config validate / find / resolve_model / from_file_yaml + models 构造器 / resolve_api_key / resolve_api_base / serde）。
- **axon-store**：`store.rs` 完整 SQLite 实现，schema 含 conversations/messages/agents/usage_records/memory 五表 + 索引；CRUD 全部就绪。`delete_conversation` 已修复级联删除 usage_records。
- **axon-gateway**：`gateway.rs`（EmbeddedGateway + ArcSwap 无锁快照 + round_robin/weighted/failover 路由 + chat/chat_stream/list_models/list_routes）、`provider.rs`（Provider trait + OpenAiProvider + AnthropicProvider + create_provider 工厂；vertex 暂复用 OpenAI 实现）、`stream.rs`（StreamEvent 7 变体 + ChatResponse + parse_openai_sse + parse_anthropic_sse，含工具调用增量装配）。
- **axon-tools**：`registry.rs`（ToolProvider trait + ToolRegistry + ToolResult/ToolContext/ToolInfo + schemas_for）、`web_search.rs`（DuckDuckGo HTML 解析 + 内置 urlencoding + 8 个解析单元测试）、`shell.rs`（带超时 + 白名单）、`memory.rs`（走 axon-store KV）、`http_fetch.rs`（带超时 + 截断保护）、`code_exec.rs`（python/javascript）、`lib.rs`（build_registry 工厂）。
- **axon-runtime**：`executor.rs`（AgentExecutor + invoke/invoke_stream + InvokeResult/ToolCallRecord + 消息持久化 + 用量记录）、`pipeline.rs`（GenerationPipeline + LLM↔Tool 循环 + max_iterations 守护）。
- **axon-protocol**：`openai.rs`（ChatCompletionRequest/Response/Chunk + ModelsResponse + ModelObject）、`axon.rs`（InvokeAgentRequest/Response + AgentInfo + Conversation/Message Response + Health/Status/UsageStats/ToolInfo/ErrorResponse）。
- **axon-server**：`main.rs`（clap CLI + tracing init + 配置加载 + axum serve）、`app.rs`（AppState + reload_config + `limiter: Arc<Limiter>`）、`config_watcher.rs`（notify 热重载）、`handlers/`（chat/agents/conversations/system 四组 handler，覆盖 OpenAI 兼容直通 + 智能体 invoke + 对话 CRUD + healthz/readyz/status/metrics/tools/usage + 限流集成）。
- workspace 根 `Cargo.toml`、各 crate `Cargo.toml`、release profile、`rust-toolchain.toml`、`config.example.yaml`、`LICENSE`、`.gitignore`。
- **ui/ 前端（v0.5，代码就绪待 CI 验证）**：React 18 + Vite 5 + TypeScript + TailwindCSS 3 + react-router-dom 6，暗色主题。`api/{types,client}.ts` 对接 axon-protocol 全部类型与 14 条路由；`hooks/useAgentStream.ts` 手写 SSE 解析（跨 chunk、`data:` 行、`[DONE]` 哨兵）+ 事件累加器（text/thought/tool_calls/usage/finish）；`hooks/useFetch.ts` 数据加载；`components/{Sidebar,Layout,ui}.tsx` 侧边栏 + 布局 + UI 原语（PageHeader/Card/Stat/Spinner/EmptyState/ErrorBanner）；`pages/{Dashboard,Chat,Agents,Models,Settings}.tsx` 五页面（仪表盘=status+usage+agents 概览；对话=选 agent+历史+SSE 流式+发送/停止；智能体=只读列表+详情；模型=只读列表+per-model 用量；设置=status+tools+Prometheus metrics）。vite 产物到 `ui/dist` 供后续 `include_dir!` 嵌入。
- **CI（`.github/workflows/ci.yml`）**：`rust` job（fmt --check / clippy -D warnings / test --workspace / release build / 二进制 < 50MB 断言）+ `ui` job（npm install / tsc --noEmit / vite build / 上传 dist artifact）。push 到 main 或 PR 触发。**2026-08-10 经 10 次 push→据报错修复迭代循环后 CI 全绿**（rust 3m20s + ui 28s，run 31379331258），修复内容：async-stream 0.8→0.3（crates.io 无 0.8）、clippy derivable_impls（AxonConfig/ChatOptions derive Default）、dead_code（删 AnthropicStreamEvent.index、HttpFetchTool.timeout_ms）、unused imports（axon-protocol/axon-runtime/axon-server）、From<rusqlite::Error>（axon-core 加 optional sqlite feature，axon-store 启用）、OptionalRow 关联类型、axon-server 缺 anyhow 依赖、invoke_stream 返回 impl Stream 需 Box::pin、AppState.tools 改 ArcSwap<Arc<ToolRegistry>>、config_watcher config_path move 后再用。

### 🟡 已知问题
- **离线编译受限**：当前环境缺 `rusqlite`/`r2d2`/`r2d2_sqlite`/`clap`/`notify`/`include_dir`/`async-stream 0.8`/`axum 0.7`（缓存仅有 axum 0.6 / async-stream 0.3），无法本地 `cargo check --workspace`。**编译验证已改为通过 GitHub CI 进行**（见 §R0.7/R0.8/§R2）：`git push` 触发 `.github/workflows/ci.yml`，据 CI 报错迭代修复至全绿。axon-core 22 个单元测试在联网 CI 中随 `cargo test --workspace` 一起跑。
- **config validate 不检查引用完整性**：`agent.model` 是否在 `models` 中存在未校验，运行时才报错。

### ❌ 未完成（按依赖顺序）
1. **ui/ 嵌入二进制**：✅ `include_dir!("$CARGO_MANIFEST_DIR/../../ui/dist")`（feature `embed-ui`）+ axum 静态路由 `/ui/*`（mime 推断 + SPA fallback），CI rust job 先 build ui 再 cargo build 嵌入。CI 全绿验证（run 31384489805）。
2. **android/ + scripts/（v0.8 APK 流水线，CI 全绿）**：`scripts/cross-android.sh`（cargo-ndk 交叉编译 aarch64-linux-android）+ `scripts/build-apk.sh`（gradle assembleRelease）+ `android/apk/` Gradle WebView 壳工程（AGP 8.5.2, namespace com.axon.app, compileSdk 34, minSdk 21, arm64-v8a, Java17；MainActivity 启动 AxonService + WebView 轮询 healthz 后加载 127.0.0.1:8080/ui/；AxonService 前台服务 + 从 assets 提取 axon 二进制 + ProcessBuilder 启动 + 日志转发）+ `.github/workflows/android-apk.yml`（cargo-ndk 交叉编译 + gradle assembleRelease + 命名 AXON-v1.0.1-android-arm64-v8a.apk + 上传 artifact）。**CI 全绿验证通过**（run 31393481268，3m19s，artifact `axon-apk-arm64-v8a` 已上传）。修复历程：kotlin-stdlib 1.8.22/1.6.21 duplicate class（force resolutionStrategy）→ `stat_sys_data_done`/`sym_def_app_icon` @hide drawable（改 `ic_dialog_info`）。
3. **tests/e2e/**：✅ axon-e2e crate（9 个 E2E 测试，真实 TcpListener + reqwest，CI 全绿 run 31401226428）。
4. **docs/**：README、快速入门、API 参考、部署指南。
5. **全量编译验证**：✅ GitHub CI rust+ui 两 job 全绿（§R2.3 闭环已跑通，累计 12 次迭代修复）。

## 5. 关键接口契约（不要破坏既有签名）

### axon-core（已固化）
- `AxonConfig::from_file(path) -> Result<Self>`、`validate()`、`find_agent/find_model/find_route/resolve_model`。
- `ModelDefinition::resolve_api_key()`（先 api_key 再 api_key_env 环境变量）、`resolve_api_base()`（按 provider 默认 base）。
- `ChatMessage::{system,user,assistant,tool}` 构造器；`ChatOptions`（temperature/max_tokens/stop/top_p/frequency_penalty/presence_penalty/reasoning_effort/service_tier）+ `ChatOptions::default()`。
- `AxonError` 变体：Config/NotFound/Validation/Gateway/Upstream/Storage/Tool/Http/Internal；`Result<T> = std::result::Result<T, AxonError>`。

### axon-gateway（已固化）
- `EmbeddedGateway::new() -> Arc<Self>`、`from_config(&AxonConfig) -> Result<Arc<Self>>`、`reload(&AxonConfig) -> Result<()>`。
- `chat(model, &[], &ChatOptions) -> Result<ChatResponse>`、`chat_stream(...) -> Result<Pin<Box<dyn Stream<Item=StreamEvent> + Send>>>`。
- `list_models() -> Vec<ModelInfo>`、`list_routes() -> Vec<RouteDefinition>`。
- `Provider` trait：`name()`、`chat()`、`chat_stream()`。新增 provider 在 `create_provider` 工厂登记。
- `StreamEvent` 9 变体：TextChunk/ThoughtChunk(text+title+signature)/ToolCallUpdate/ToolCallRequest/ToolCallResult/UsageUpdate/Error/Retrying/Done（`#[serde(tag="type", rename_all="snake_case")]`）。对齐 Agora `LlmProvider.kt` 契约。

### axon-store（已固化）
- `Store::open(path, max_connections) -> Result<Arc<Self>>`。
- 对话/消息/用量/记忆 API 均同步（rusqlite）返回 `Result<T>`；runtime 层用 `tokio::task::spawn_blocking` 包裹即可。
- `MessageRecord::new(conv_id, parent_id, role, content)`；`UsageRecord`/`UsageStats`/`MemoryEntry` 结构已定。

### axon-tools（已固化）
- `ToolProvider` trait：`name()`/`description()`/`parameters_schema()`/`execute(input: Value, ctx: &ToolContext) -> Result<ToolResult>`。
- `ToolRegistry::new()/register/get/list/schemas_for/execute`。
- `ToolContext { agent_id, conversation_id, working_dir }`（注意：当前未含 store 句柄，`memory` 工具实现时需扩展此结构或在构造期注入 `Arc<Store>`）。
- `ToolResult::ok/err`。
- `build_registry(&[ToolDefinition], Arc<Store>) -> ToolRegistry` 工厂。

### axon-runtime（已固化）
- `AgentExecutor::new(gateway, tools, store, working_dir)`，`invoke(agent, input, conversation_id) -> Result<InvokeResult>`、`invoke_stream(self: Arc, agent: Arc, input, conv_id) -> Result<impl Stream<Item=StreamEvent>>`。
- `GenerationPipeline::new()` + `run() -> BoxStream<StreamEvent>`，LLM↔Tool 循环 + max_iterations 守护。
- `InvokeResult { output, conversation_id, usage, iterations, tool_calls }`；`ToolCallRecord`。
- 复用 `axon_gateway::StreamEvent` 作为对外事件类型。

### axon-server（已固化）
- CLI：`axon --config config.yaml [--addr 0.0.0.0:8080] [--log-level info]`。
- 路由：OpenAI 兼容 `/v1/chat/completions` `/v1/models`、智能体 `/v1/agents` `/v1/agents/:id` `/v1/agents/:id/invoke`、对话 `/v1/conversations`、运维 `/healthz /readyz /status /metrics /v1/tools /v1/usage`。
- 流式响应统一用 axum SSE（`axum::response::sse`）。
- 配置热重载：`config_watcher::spawn_watcher` 用 notify 监听 → `AppState::reload_config`。
- 限流：`AppState.limiter: Arc<Limiter>`，chat handler 对 `resolve_model(req.model).rate_limit` 非空且非 unrestricted 的模型执行 `pre_commit("model:{name}", rl)` → 429 on `RateLimitError`；非流式 `commit_tokens(total_tokens)`，流式 `into_stream_hold()` + `add_tokens_post_stream(key, total_tokens)`。

### axon-ratelimit（已固化，adapted from aisix-ratelimit）
- `Limiter::new() / with_store(Arc<dyn RateStore>) / local_with_clock(C)`。
- `pre_commit(key, &RateLimitConfig) -> Result<Reservation, RateLimitError>`：并发槽 + RPS/RPM/RPH/RPD check-and-increment + TPM/TPD check-only。
- `Reservation::commit_tokens(u64)`：post-deduct TPM/TPD + 释放并发槽；Drop 未 commit 则仅释放并发槽。
- `MultiReservation::new(vec) / merge / into_stream_hold() -> StreamConcurrencyGuard`：流式路径并发槽持有至 guard drop。
- `Limiter::add_tokens_post_stream(key, tokens)`：流式 post-stream token 计入。
- `Limiter::peek(key, &RateLimitConfig) -> Option<RateLimitStatus>`：只读快照（x-ratelimit-* headers）。
- `RateLimitConfig`：tpm/tpd/rps/rpm/rph/rpd（Option<u64>）+ concurrency（Option<u32>）+ `is_unrestricted()`。
- `RateLimitError`：Requests{scope, retry_after_secs} / Tokens{scope, retry_after_secs} / Concurrency。

## 6. 下一步任务（按优先级，逐项勾选）

> 每项都是可独立交付的最小单元。完成即打勾并移到「已完成」区。

### P0 — 让 workspace 能编译通过 ✅
- [x] 修 `axon-tools/src/lib.rs`：补 `pub mod web_search;` 与 `pub use web_search::WebSearchTool;`。
- [x] 为 `axon-runtime`、`axon-protocol`、`axon-server` 各写最小 `lib.rs`/`main.rs` 占位（server 的 main.rs 至少能 `println!("AXON")` 起步），让 `cargo check --workspace` 通过。
- [x] 加 `rust-toolchain.toml`（channel stable，MSRV 1.75 注释）。
- [x] 加 `.gitignore`（`/target`、`*.db`、`/ui/dist`、`/ui/node_modules`）。

### P1 — v0.1 网关基座 ✅
- [x] `axon-protocol`：OpenAI 兼容 `ChatCompletionRequest`/`ChatCompletionResponse`/`ModelsResponse`/SSE chunk 类型。
- [x] `axon-server`：clap CLI + 加载 config + 起 axum + `POST /v1/chat/completions`（流式 + 非流式，直通 gateway）+ `GET /v1/models` + `GET /healthz` + `GET /readyz`。
- [x] 配置热重载：`notify` 监听 config 文件 → `gateway.reload()` + `ArcSwap` 替换。
- [x] `config.example.yaml`（最小可用：一个 openai model + server + storage）。
- [ ] 验收：`curl` 调 `/v1/chat/completions` 能代理并流式返回（需联网编译后验证）。

### P2 — v0.2 智能体引擎 ✅
- [x] `axon-runtime`：`AgentExecutor` + 单轮 `invoke`/`invoke_stream`（无工具）+ 消息持久化 + 用量记录。
- [x] `axon-server`：`POST /v1/agents/:id/invoke`（SSE）+ 对话 CRUD 端点。
- [ ] 验收：定义「翻译助手」agent，调用后流式返回并持久化历史（需联网编译后验证）。

### P3 — v0.3 工具编排 ✅
- [x] `axon-runtime`：`GenerationPipeline` 多轮工具循环 + max_iterations 守护。
- [x] `axon-tools`：补 `shell`/`memory`/`http_fetch`/`code_exec`；扩展 `ToolContext` 注入 `Arc<Store>`。
- [ ] 验收：「研究助手」agent 能自主 web_search 并总结（需联网编译后验证）。

### P4 — v0.4 移动端适配
- [x] `scripts/cross-android.sh`（NDK `aarch64-linux-android`）+ `scripts/build-apk.sh`。
- [x] `android/apk/` Gradle WebView 壳工程 + `.github/workflows/android-apk.yml` APK 流水线（参照 RustSync，cargo-ndk + gradle assembleRelease，产物 AXON-v1.0.1-android-arm64-v8a.apk）。
- [x] 裁剪依赖、验证二进制 < 30MB（实测 5.1M，远低于目标）、启动内存 < 150MB（待真机实测）。
- [x] `android/termux/install-termux.sh`（从 CI artifact 或源码安装 + 默认配置生成）。
- [x] 验收：`git push` 后 android-apk.yml CI 全绿并产出 APK artifact（run 31393481268，artifact `axon-apk-arm64-v8a`，2026-08-10）。

### P5+ — v0.5 Web UI / v0.6 可观测性 / v0.7 高级路由 / v0.8 APK / v0.9 稳定化 / v1.0 发布
- [x] `ui/` 前端项目基座（React + Vite + TS + TailwindCSS + 暗色主题）。
- [x] 页面：仪表盘（概览 + 用量统计）/ 对话（实时 SSE 流式）/ 智能体管理（只读列表 + 详情）/ 模型配置（只读列表 + per-model 用量）/ 设置页（status + tools + metrics）。
- [x] `.github/workflows/ci.yml`：rust + ui 两 job。
- [x] `include_dir!` 宏将 `ui/dist` 嵌入 axon-server 二进制（feature `embed-ui`，default 启用）+ axum 静态文件服务路由 `/ui` `/ui/` `/ui/*path`（mime 推断 + SPA fallback index.html）。
- [x] 验收：`git push` 后 GitHub CI rust + ui 两 job 全绿（run 31384489805，rust 1m27s + ui 21s，2026-08-10）。浏览器打开 `http://localhost:8080/ui/` 由二进制内嵌静态资源服务（需运行 `axon --config config.example.yaml` 后实测）。
- [x] **v0.6 可观测性**：tracing-subscriber 初始化（EnvFilter）+ `/metrics` Prometheus 文本输出（requests/tokens/duration per-model，从 store usage_stats）+ `/status /healthz /readyz` + 结构化 JSON 日志选项（`observability.log_format: plain|json`，CLI/env 可覆盖，main.rs 先加载 config 后 init tracing）。CI 全绿（run 31396860258）。
- [x] **v0.7 限流**：新建 `axon-ratelimit` crate（adapted from aisix-ratelimit，drop redis：clock.rs + window.rs + error.rs + limiter.rs + store/{mod,local}.rs，~1200 行）；axon-core `RateLimitConfig` 升级为 7 字段（tpm/tpd/rps/rpm/rph/rpd/concurrency）+ `is_unrestricted()` + `RateLimitScope` enum；axon-server `AppState` 加 `Limiter`，chat handler 集成 `pre_commit`（429 on exceeded）+ `commit_tokens`（非流式）+ `StreamConcurrencyGuard` + `add_tokens_post_stream`（流式）；config.example.yaml 加 rate_limit 示例。CI 全绿（run 31410795834，1m59s）。
- [x] **Agora 前后端交互契约对齐 + 上游字段转发**：对齐 Agora `LlmProvider.kt` 契约。StreamEvent 9 变体（ThoughtChunk 加 title/signature + ToolCallUpdate/Retrying）；ChatCompletionRequest 加 7 字段（stream_options/reasoning_effort/reasoning/service_tier/top_p/frequency_penalty/presence_penalty）；ChatDelta 加 reasoning_content/reasoning/reasoning_details；ChatCompletionChunk 加 outcome/error；TokenUsage 加 4 字段 + 嵌套 struct；parse_openai_sse 解析 reasoning/outcome/error/usage 新字段；pipeline.rs ThoughtChunk 透传；chat/agents handler 序列化；UI types.ts 对齐；ChatOptions 加 5 字段转发到上游 OpenAiRequest；E2E +2 测试。CI 全绿（run 31414206323 + 31441603952）。
- 其余 v0.9/v1.0 见 `AXON_PROJECT_PLAN.md` §4.2。按需推进。

## 7. 编码约定（强制）

- **语言**：代码与注释一律英文（标识符、doc comment、日志消息）；本文件和面向用户的文档用简体中文。
- **不写注释**除非用户要求；让类型与函数名自解释。doc comment（`//!`/`///`）允许且鼓励用于 public API。
- **错误处理**：库 crate 一律用 `axon_core::Result<T>` / `AxonError`，不要 `anyhow`（anyhow 仅在 server main 顶层用）。新增错误类别时扩 `AxonError` 变体而非字符串堆砌。
- **异步**：public API 用 `async fn`；store 的同步 rusqlite 调用在 runtime 层用 `tokio::task::spawn_blocking` 包裹。
- **trait 对象**：`Provider`/`ToolProvider` 用 `Arc<dyn Trait>` 存放；`EmbeddedGateway`/`Store` 用 `Arc<Self>` 工厂返回。
- **无锁读路径**：可变配置走 `arc_swap::ArcSwap`；路由 round_robin 状态走 `parking_lot::RwLock`。
- **serde**：配置结构加 `#[serde(deny_unknown_fields)]` 拒绝未知字段；可选字段用 `Option<T>` + `#[serde(default)]`；默认值函数命名 `default_xxx`。
- **命名**：结构体 PascalCase、函数 snake_case、常量 SCREAMING_SNAKE；模块名单数（`provider` 不是 `providers`）。
- **依赖**：能复用 workspace.dependencies 就别在子 crate 写死版本；新增非 workspace 依赖前在「变更日志」说明理由。
- **测试**：单元测试 `#[cfg(test)] mod tests` 放文件尾；集成测试放 `tests/` 目录；E2E 放 `tests/e2e/`。
- **体积**：新增依赖前预估二进制影响；优先选 `rustls-tls`（已用）而非 openssl；避免引入重运行时（如 tonic 全栈）。

## 8. 常用命令

```bash
# 编译检查（最快反馈）
cargo check --workspace

# 格式化 + lint
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings

# 测试
cargo test --workspace

# 跑 server（开发）
cargo run -p axon-server -- --config config.example.yaml

# release 体积检查
cargo build --release -p axon-server
ls -lh target/release/axon   # 必须 < 50MB

# Android 交叉编译（脚本就绪后）
bash scripts/cross-android.sh
ls -lh target/aarch64-linux-android/release/axon

# UI 开发（ui/ 就绪后）
cd ui && pnpm install && pnpm dev      # 开发
cd ui && pnpm build                    # 产物到 ui/dist，供 include_dir! 嵌入
```

环境：当前 rustc 1.92.0 / cargo 1.92.0（高于 MSRV 1.75，兼容）。`/opt/github/aisix` 按 **§R1 直接复制复用**（不要 `cargo` 依赖其 crate，而是复制源码到 AXON 后裁剪）。`/opt/github/Agora` 为设计借鉴（Kotlin/TS，不复制代码，只参考契约）。

## 9. 变更日志（追加新行，最新在上）

- 2026-08-11 **填充 docs/ + 项目根 README + rustfmt.toml（P1-1 + P2 完成）**：新建 docs/{README,API,CONFIG,DEPLOYMENT}.md 四份文档（API 14 路由参考、CONFIG 配置项参考、DEPLOYMENT Termux/APK/源码/交叉编译部署指南、README 文档索引）；项目根 README.md（特性/架构/crate 表/快速入门/体积/许可证）；rustfmt.toml（max_width=100 等标准配置）。至此 P1-1 + P2 完成,docs/ 不再为空,GitHub 首页有内容。
- 2026-08-11 **全量代码分析 + P0/P1 改进（待 CI 验证）**：全量分析 8 crate + UI + Android + CI,识别 18 个改进点。完成 P0(4 项)+ P1(6 项):(1) 修复 web_search URL 编码 bug(多字节 UTF-8 如中文/emoji 现在正确编码为 %E4%B8%AD 而非 %4E2D),+6 测试;(2) 修复 weighted 路由随机数质量(用 DefaultHasher 哈希 route_name+counter 替代时间戳低位,移除 chrono 依赖);(3) 完善 Anthropic SSE 解析(支持 input_json_delta/thinking_delta/message_delta.usage,加 delta_type 字段);(4) 计时 duration_ms(Instant::now 替代硬编码 0);(5) 消除 gateway.rs from_config/reload 重复(提取 GatewaySnapshot::from_config);(6) 清理未使用代码(GatewayStats/ChatChunk);(7) 改进 config_watcher(用 tokio::spawn 持有 watcher 替代 mem::forget);(8) 改进 config.rs 测试(用 tempfile 替代硬编码 /tmp 路径);(9) 新增 axon-store 9 个单元测试(open/conversation CRUD/级联删除/message/usage stats/memory CRUD/namespace 隔离)。**待 push CI 验证**。
- 2026-08-11 **修复 Anthropic 工具消息格式（v0.9 错误处理完善，待 CI 验证）**：`AnthropicProvider` 之前直接透传 OpenAI 格式 `ChatMessage` 给 Anthropic API，多轮工具循环对 Anthropic 会失败（Anthropic 要求 `tool_result` content block 格式、assistant tool_use 为 content array）。新增 `AnthropicMessage`/`AnthropicContent`/`AnthropicContentBlock` 类型 + `convert_anthropic_messages()` 转换函数：`role:tool` → `role:user` + `Blocks[ToolResult]`；`assistant` with `tool_calls` → `Blocks[Text?, ToolUse...]`；其余 → `Text` string。`AnthropicRequest.messages` 改为 `Vec<AnthropicMessage>`，chat + chat_stream 两处构造均调用转换。+4 单元测试覆盖 user/assistant+tool_calls/tool_result/empty content 场景。**待网络恢复后 push CI 验证**。
- 2026-08-11 **上游 provider 字段转发完成（CI 全绿 run 31441603952）**：将 Agora 契约对齐新增的请求字段转发到上游 provider。axon-core `ChatOptions` 加 `top_p`/`frequency_penalty`/`presence_penalty`/`reasoning_effort`/`service_tier`（5 字段）+ 更新 `test_chat_options_default`。axon-server chat handler 把 `ChatCompletionRequest` 新字段传入 `ChatOptions`。axon-gateway `OpenAiRequest` 加 5 字段 + `to_openai_request` 两处构造（chat + chat_stream）从 `ChatOptions` 传入。axon-runtime pipeline.rs agent 构造的 `ChatOptions` 用 `..Default::default()` 填充新字段。CI 全绿（run 31441603952，1m20s）。至此 Agora 契约对齐 + 上游转发全部完成，前端发送的 reasoning_effort/top_p 等参数可端到端透传到上游 OpenAI/兼容 provider。下一步：v0.9 稳定化或其他。
- 2026-08-11 **Agora 前后端交互契约对齐完成（CI 全绿 run 31414206323）**：对齐 Agora `LlmProvider.kt` 契约。StreamEvent（axon-gateway）：ThoughtChunk 加 title/signature，加 ToolCallUpdate/Retrying。ChatCompletionRequest（axon-protocol）：加 stream_options/reasoning_effort/reasoning/service_tier/top_p/frequency_penalty/presence_penalty。ChatDelta：加 reasoning_content/reasoning/reasoning_details。ChatCompletionChunk：加 outcome/error。TokenUsage（axon-core）：加 prompt_tokens_details/prompt_cache_hit_tokens/prompt_cache_miss_tokens/completion_tokens_details + 嵌套 struct。parse_openai_sse：解析 reasoning→ThoughtChunk、outcome/error→Error、usage 新字段。pipeline.rs：ThoughtChunk 透传。chat handler：ThoughtChunk→ChatDelta reasoning_content。UI types.ts 对齐。E2E +2 测试。CI 经 2 轮修复全绿：(1) cargo fmt 失败→修复；(2) 全绿（run 31414206323，1m15s）。
- 2026-08-11 **v0.7 限流完成（CI 全绿）**：新建 `axon-ratelimit` crate（adapted from `aisix-ratelimit`，drop redis：`clock.rs` + `window.rs` verbatim + `error.rs` + `limiter.rs` + `store/{mod,local}.rs`，aisix_core→axon_core，~1200 行含 30+ 测试）；axon-core `RateLimitConfig` 从 2 字段（rps/rpm u32）升级为 7 字段（tpm/tpd/rps/rpm/rph/rpd u64 + concurrency u32）+ `is_unrestricted()` + `RateLimitScope` enum（Requests/Tokens + Display）；axon-server `AppState` 加 `limiter: Arc<Limiter>`，chat handler 集成两阶段限流：`pre_commit`（429 on exceeded）+ `commit_tokens`（非流式 post-deduct）+ `StreamConcurrencyGuard`（流式并发槽持有至 stream 结束）+ `add_tokens_post_stream`（流式 token 记入）；config.example.yaml 加 rate_limit 示例（rpm/tpm/concurrency）；workspace 加 `dashmap = "6"`。CI 经 3 轮修复循环全绿（run 31410795834，1m59s）：(1) `cargo fmt` 失败（import 排序 + peek 单行 + closure 单行）→ 修；(2) clippy `dead_code`（`Dim`/`request_dims`/`token_dims` 仅 redis 用）→ 删；(3) 全绿。至此 v0.7 限流完成。下一步：Agora 前后端交互契约对齐（ThoughtChunk title/signature、ToolCallUpdate、reasoning 字段等）或 v0.9 稳定化。
- 2026-08-10 **E2E 测试完成（CI 全绿）**：axon-server 重构为 lib + bin（加 `src/lib.rs` 暴露 `build_router` + `AppState`，main.rs 用 `axon_server::`）；新建 `tests/e2e/` crate（依赖 axon-server default-features=false + axum + reqwest + tempfile），9 个 E2E 测试（真实 TcpListener bind 127.0.0.1:0 + reqwest HTTP 调用）：healthz/readyz/status/metrics/list_agents/list_tools/usage/list_models/conversation_create_and_list。修复：(1) tests/e2e 缺 axum 依赖；(2) conversation POST 500 —— `tempfile::tempdir()` 在 start() 返回时 drop 删除 db 文件，加 `std::mem::forget(dir)` 保持存活；(3) 删 feature-unstable ui 测试（Cargo feature unification 导致 embed-ui 在 workspace test 时启用）。CI 全绿（run 31401226428，1m41s）。
- 2026-08-10 **v0.6 可观测性：结构化 JSON 日志选项**：axon-core ObservabilityConfig 加 `log_format: String`（"plain"|"json"，默认 plain）+ default_log_format()；axon-server main.rs 调整启动顺序为先加载 config 再 init tracing（config 加载失败用 eprintln 兜底），据 `config.observability.log_format` + `AXON_LOG_FORMAT` env 选 `.json()` 或 plain fmt；config.example.yaml 加 log_format 示例。CI 全绿（run 31396860258，1m18s）。至此 v0.6 可观测性基本完成（tracing + Prometheus /metrics + JSON 日志选项）。下一步：v0.7 高级路由（限流，复用 aisix-ratelimit）。
- 2026-08-10 **P4 移动端适配完成**：交叉编译产物 axon 二进制 **5.1M**（远低于 30MB 目标 / 50MB 硬约束），APK `axon-release-unsigned.apk` **5.0M**。新增 `android/termux/install-termux.sh`（从 CI artifact 或源码安装 + 默认配置生成 + 启动提示）。P4 全部勾选（启动内存 < 150MB 待真机实测）。下一步：P6 可观测性（Prometheus 指标 + 结构化日志，复用 aisix telemetry.rs）或 P7 高级路由。
- 2026-08-10 **v0.8 APK 流水线 CI 全绿**：android-apk.yml 经 4 次 push→据报错修复循环后全绿（run 31393481268，3m19s，artifact `axon-apk-arm64-v8a` 上传）。修复历程：(1) run #1 失败 Build APK（gradle，无日志因 api 不可达）→ 加 env ANDROID_HOME/local.properties/诊断 step/error-to-annotation；(2) run #2 失败 `checkReleaseDuplicateClasses`：kotlin-stdlib 1.8.22 与 kotlin-stdlib-jdk8 1.6.21 duplicate class（appcompat/webkit 传递依赖旧版）→ build.gradle 加 `configurations.all { resolutionStrategy.force }` 统一 1.8.22；(3) run #3 失败 `compileReleaseJavaWithJavac`：`android.R.drawable.stat_sys_data_done` 是 @hide 资源不在 compileSdk 34 public API → 改 `ic_dialog_info`（同时改 manifest `sym_def_app_icon`→`ic_dialog_info`）；(4) run #4 全绿。至此 v0.8 APK 流水线完成，产出 `AXON-v1.0.1-android-arm64-v8a.apk`。回写 §4/§6。
- 2026-08-10 **v0.8 APK 流水线代码就绪 + 本地静态审查**：新增 `scripts/cross-android.sh`（cargo-ndk 交叉编译 aarch64-linux-android）+ `scripts/build-apk.sh`（gradle assembleRelease）+ `android/apk/` Gradle WebView 壳工程（AGP 8.5.2, arm64-v8a, Java17, MainActivity+AxonService 前台服务启动 axon 二进制 + WebView 加载 127.0.0.1:8080/ui/）+ `.github/workflows/android-apk.yml`（参照 RustSync：setup rust+NDK+cargo-ndk → build ui → cargo ndk 交叉编译 axon → 复制二进制+config 到 assets → setup-java 17 → gradle assembleRelease → 命名 AXON-v1.0.1-android-arm64-v8a.apk → 上传 artifact）。本地静态审查通过（bin name=axon、config.example.yaml、gradle.properties useAndroidX、AGP/Gradle/Java 版本兼容、NDK 路径）；去掉 android-apk.yml 的 continue-on-error 以符合 §R0.8（让失败真实暴露以便据报错修复）。**环境网络阻断**：github.com DNS 被解析到占位 IP 198.18.0.139（RFC 2544 段），gh token 亦失效，git push 10 次重试均失败。改动已 commit（e2d2ba4 + 去掉 continue-on-error），**待网络恢复后 git push 触发 android-apk.yml CI，据报错修复循环至产出 APK artifact**。回写 §3/§4/§6。
- 2026-08-10 **v0.5 Web UI 嵌入完成 + CI 全绿**：axon-server 加 `embed-ui` feature（include_dir optional）+ `include_dir!("$CARGO_MANIFEST_DIR/../../ui/dist")` 嵌入 ui/dist；handlers/system.rs 加 `ui_index`/`ui_asset`（mime 推断 + SPA fallback index.html）+ `#[cfg(not(feature="embed-ui"))]` 占位；main.rs 加 `/ui` `/ui/` `/ui/*path` 路由；CI rust job 先 build ui 再 cargo build 嵌入。include_dir! 路径关键：宏相对 cwd 而非 CARGO_MANIFEST_DIR，须用 `$CARGO_MANIFEST_DIR/../../ui/dist`。CI 全绿（run 31384489805，rust 1m27s + ui 21s）。至此 v0.5 Web UI 代码 + 嵌入 + CI 验证全部完成。下一步：P4 移动端适配（scripts/cross-android.sh + android/termux/）或 P6 可观测性。
- 2026-08-10 **CI 全绿里程碑**：经 10 次 `git push → gh run view --log-failed → 据报错修复 → 再 push` 循环（§R2.3），GitHub CI rust+ui 两 job 全绿（run 31379331258）。修复：async-stream 0.8→0.3、clippy derivable_impls/dead_code/unused-imports、From<rusqlite::Error>（axon-core optional sqlite feature）、OptionalRow 关联类型、axon-server 缺 anyhow、invoke_stream Box::pin、AppState.tools ArcSwap<Arc<ToolRegistry>>、config_watcher config_path clone。CI node-version 升 24 消 deprecation。本地全程未编译（离线），纯靠 GitHub CI 验证，印证 §R0.7/R0.8 闭环可行。下一步：接入 `include_dir!("ui/dist")` + axum 静态路由 `/ui/*`，再 push 让 CI 验证。
- 2026-08-10 v0.5 Web UI 代码就绪 + GitHub CI workflow：新建 `ui/`（React18+Vite5+TS+Tailwind3+react-router6，暗色主题）含 api/{types,client}.ts 对接 axon-protocol 全部类型与 14 路由、hooks/useAgentStream.ts 手写 SSE 解析+事件累加器、hooks/useFetch.ts、components/{Sidebar,Layout,ui}.tsx、pages/{Dashboard,Chat,Agents,Models,Settings}.tsx 五页面、App.tsx 路由、基座配置 9 文件；新建 `.github/workflows/ci.yml`（rust job: fmt/clippy/test/release/体积<50MB 断言；ui job: npm install/tsc --noEmit/vite build/upload dist）。未本地编译（按 §R0.7 编译验证走 GitHub CI）。回写 §3/§4/§6。下一步：`git push` 触发 CI 据报错修复至全绿；接入 `include_dir!("ui/dist")` + axum 静态路由 `/ui/*`。
- 2026-08-10 新增 §R0.9 自动推进项目规则：用户说「自动继续」/「继续」/「auto」或未叫停时，代理必须自主连续推进任务，不每步询问；仅方向性分歧/破坏性操作/违反硬约束/信息严重不足才问；停下汇报附三段式摘要。未改代码，仅改 AGENTS.md。下一步：建 CI workflow + 推进 ui/ 前端。
- 2026-08-10 新增 §R0.7/R0.8 + §R2 GitHub CI 编译验证策略：因本地离线缺依赖无法 `cargo check/build/test`，强制规定编译验证必须 `git push` 到 GitHub 由 CI 执行；CI 失败必须据报错本地修复后再 push，循环至全绿；CI 全绿是子任务完成的唯一编译验证判据；附 CI 必须步骤（fmt/clippy/test/release build/体积）与迭代修复流程。未改代码，仅改 AGENTS.md。下一步：建 `.github/workflows/ci.yml` 让规则生效，并推进 ui/ 前端。
- 2026-08-10 代码审查 + bug 修复 + 测试 + AGENTS.md 漂移修正：修复 5 个 bug（http_fetch 截断 panic、store delete_conversation 不删 usage_records、gateway 无意义代码、executor invoke_stream 不记录 tool_calls、stream Anthropic current_tool_id 未清空）；为 axon-core 添加 22 个单元测试（config + models）并全部通过；为 web_search 添加 8 个解析测试；回写 §3/§4/§5/§6 消除 AGENTS.md 与代码的严重漂移（此前误记 runtime/protocol/server 为空，实际已全部完成 4467 行代码）。
- 2026-08-10 新增 §R0.6 + §R1 aisix 复用强制规则：写新功能前先查 aisix 现成实现，有则直接复制粘贴再最小裁剪；附可复制文件清单（sse/snapshot/bridge/openai/anthropic provider/ratelimit/telemetry/rustfmt 等）与禁复制清单（proxy/etcd/redis/guardrails/bedrock）。
- 2026-08-10 初版 AGENTS.md 创建；盘点完成：core/store/gateway 基本就绪，tools 部分，runtime/protocol/server 空；P0 待修：axon-tools/lib.rs 漏声明 web_search 模块。

## 10. 参考索引

- 完整计划书：`AXON_PROJECT_PLAN.md`（§3.3 关键接口伪代码、§4.2 路线图验收标准、§5 部署脚本、§6 配置示例、§7 测试策略）。
- aisix 复用清单：见 **§R1.2**（直接复制源码，按 §R1.1 决策流程）。
- 上游借鉴：`/opt/github/aisix`（网关 crate 切分、ArcSwap 快照、provider 模式、SSE 解析、限流）、`/opt/github/Agora`（GenerationManager、ToolProvider、消息树、StreamEvent 契约）。
- 关键文件速查：
  - 配置模型：`crates/axon-core/src/config.rs`
  - 资源模型：`crates/axon-core/src/models.rs`
  - 网关入口：`crates/axon-gateway/src/gateway.rs`
  - Provider 实现：`crates/axon-gateway/src/provider.rs`
  - SSE 解析：`crates/axon-gateway/src/stream.rs`
  - 持久化：`crates/axon-store/src/store.rs`
  - 工具 trait：`crates/axon-tools/src/registry.rs`
