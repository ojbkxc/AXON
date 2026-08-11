# AXON HTTP API 参考

Base URL: `http://<addr>/` (默认 `0.0.0.0:8080`)

## OpenAI 兼容

### POST /v1/chat/completions

OpenAI 兼容的 chat 接口,支持流式与非流式。

**请求体**:
```json
{
  "model": "gpt-4o",
  "messages": [{"role": "user", "content": "hello"}],
  "stream": true,
  "temperature": 0.7,
  "max_tokens": 4096,
  "tools": [...],
  "reasoning_effort": "medium",
  "top_p": 1.0,
  "frequency_penalty": 0,
  "presence_penalty": 0
}
```

**响应**:
- 非流式:`ChatCompletionResponse`(OpenAI 格式)
- 流式:`text/event-stream`,每行 `data: {chunk}\n\n`,以 `data: [DONE]` 结尾

**限流**:若模型配置了 `rate_limit`,超出返回 `429 Too Many Requests`。

### GET /v1/models

列出所有已配置模型。

**响应**:
```json
{
  "object": "list",
  "data": [{"id": "gpt-4o", "object": "model", "owned_by": "axon"}]
}
```

## 智能体

### GET /v1/agents

列出所有已配置智能体。

### GET /v1/agents/:id

获取单个智能体详情。

### POST /v1/agents/:id/invoke

调用智能体,支持流式与非流式。

**请求体**:
```json
{
  "input": "Search for Rust async runtime best practices",
  "conversation_id": null,
  "stream": true
}
```

**响应**:
- 非流式:`InvokeAgentResponse`(`output`/`conversation_id`/`usage`/`iterations`/`tool_calls`)
- 流式:`text/event-stream`,`StreamEvent` 9 变体:`text_chunk`/`thought_chunk`/`tool_call_update`/`tool_call_request`/`tool_call_result`/`usage_update`/`error`/`retrying`/`done`

## 对话

### POST /v1/conversations

创建对话。**请求体**:`{"agent_id": "researcher", "title": "My chat"}`

### GET /v1/conversations

列出对话。**查询参数**:`?limit=50`

### GET /v1/conversations/:id

获取单个对话。

### DELETE /v1/conversations/:id

删除对话(级联删除消息和用量记录)。

### GET /v1/conversations/:id/messages

获取对话的消息历史。

## 运维

### GET /healthz

健康检查(始终返回 200)。

### GET /readyz

就绪检查(返回 200 表示服务就绪)。

### GET /status

服务状态:版本、运行时间、配置摘要、模型/路由/智能体数量。

### GET /metrics

Prometheus 格式指标:`axon_requests_total`/`axon_tokens_total`/`axon_duration_ms_total`(per-model)。

### GET /v1/tools

列出所有已注册工具。

### GET /v1/usage

用量统计:`total_requests`/`total_tokens`/`total_duration_ms`/`by_model`。

## Web UI

### GET /ui

Web UI 入口(SPA),静态资源嵌入二进制。

### GET /ui/*path

Web UI 静态资源(mime 推断 + SPA fallback)。
