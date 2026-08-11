# AXON 配置参考

配置文件为 YAML 或 JSON 格式,通过 `--config` 参数加载。

## 顶层结构

```yaml
server:         # 服务器配置
gateway:        # 网关配置
storage:        # 存储配置
observability:  # 可观测性配置
models:         # 模型定义列表
routes:         # 路由定义列表
agents:         # 智能体定义列表
tools:          # 工具定义列表
```

## server

| 字段 | 类型 | 默认 | 说明 |
|---|---|---|---|
| addr | string | `0.0.0.0:8080` | 监听地址 |
| web_ui_enabled | bool | `true` | 是否启用 Web UI |
| max_request_body_mb | u32 | `10` | 请求体最大 MB |
| workers | usize | `2` | tokio worker 线程数 |

## gateway

| 字段 | 类型 | 默认 | 说明 |
|---|---|---|---|
| upstream_timeout_ms | u64 | `120000` | 上游请求超时 |
| stream_timeout_ms | u64 | `300000` | 流式超时 |
| retry_count | u32 | `2` | 重试次数 |
| retry_delay_ms | u64 | `1000` | 重试间隔 |
| pool_max_idle_per_host | usize | `5` | 连接池每 host 最大空闲 |

## storage

| 字段 | 类型 | 默认 | 说明 |
|---|---|---|---|
| sqlite_path | string | `axon.db` | SQLite 路径(`:memory:` 可用) |
| max_connections | u32 | `5` | 连接池大小 |

## observability

| 字段 | 类型 | 默认 | 说明 |
|---|---|---|---|
| log_level | string | `info` | 日志级别(trace/debug/info/warn/error) |
| log_format | string | `plain` | 日志格式(`plain`/`json`) |
| metrics_enabled | bool | `true` | 启用 `/metrics` |
| access_log_enabled | bool | `true` | 启用访问日志 |

## models[]

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| name | string | 是 | 模型唯一名称(用于路由引用) |
| provider | string | 是 | `openai`/`anthropic`/`vertex` |
| model_name | string | 是 | 上游实际模型名 |
| api_key | string | 否 | API key(与 api_key_env 二选一) |
| api_key_env | string | 否 | API key 环境变量名 |
| api_base | string | 否 | 上游 base URL(按 provider 默认) |
| rate_limit | object | 否 | 限流配置 |

### rate_limit

| 字段 | 类型 | 说明 |
|---|---|---|
| rps | u64 | 每秒请求数 |
| rpm | u64 | 每分钟请求数 |
| rph | u64 | 每小时请求数 |
| rpd | u64 | 每天请求数 |
| tpm | u64 | 每分钟 token 数 |
| tpd | u64 | 每天 token 数 |
| concurrency | u32 | 并发数 |

所有字段可选,全 None 表示不限流。

## routes[]

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| name | string | 是 | 路由唯一名称 |
| strategy | string | 是 | `failover`/`round_robin`/`weighted` |
| targets[] | list | 是 | 目标模型列表 |

### targets[]

| 字段 | 类型 | 默认 | 说明 |
|---|---|---|---|
| model | string | — | 引用的 model name |
| weight | u32 | `1` | 权重(weighted 策略) |

## agents[]

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| id | string | 是 | 智能体唯一 ID |
| name | string | 是 | 显示名称 |
| description | string | 否 | 描述 |
| system_prompt | string | 是 | 系统提示词 |
| model | string | 是 | 引用的 model 或 route name |
| tools | string[] | 否 | 工具名称列表 |
| max_iterations | u32 | `1` | LLM↔Tool 最大循环次数 |
| temperature | float | 否 | 温度 |

## tools[]

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| name | string | 是 | 工具唯一名称 |
| kind | string | 是 | `web_search`/`shell`/`memory`/`http_fetch`/`code_exec` |
| config | object | 否 | 工具特定配置 |

### 内置工具 kind

| kind | 说明 | config |
|---|---|---|
| web_search | DuckDuckGo 搜索 | `max_results`(默认 5) |
| shell | Shell 命令执行 | `timeout_ms`/`whitelist` |
| memory | KV 记忆(走 store) | `namespace` |
| http_fetch | HTTP 抓取 | `timeout_ms`/`max_bytes` |
| code_exec | 代码执行 | `timeout_ms`/`allowed_languages`/`max_output_chars` |

## 完整示例

见 [config.example.yaml](../config.example.yaml)。

## 热重载

修改配置文件后,服务自动检测并重载(无需重启)。日志会输出 `config reloaded successfully`。重载会更新模型/路由/智能体/工具,但限流状态保留。
