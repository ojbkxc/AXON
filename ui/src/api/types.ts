export interface PromptTokensDetails {
  cached_tokens?: number;
}

export interface CompletionTokensDetails {
  reasoning_tokens?: number;
}

export interface TokenUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  prompt_tokens_details?: PromptTokensDetails;
  prompt_cache_hit_tokens?: number;
  prompt_cache_miss_tokens?: number;
  completion_tokens_details?: CompletionTokensDetails;
}

export interface ToolCallFunction {
  name: string;
  arguments: string;
}

export interface ToolCall {
  id: string;
  type: string;
  function: ToolCallFunction;
}

export interface ChatMessage {
  role: string;
  content: string;
  tool_calls?: ToolCall[];
  tool_call_id?: string;
  name?: string;
}

export interface ToolFunctionSchema {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}

export interface ToolSchema {
  type: string;
  function: ToolFunctionSchema;
}

export interface StreamOptions {
  include_usage: boolean;
}

export interface Reasoning {
  effort?: string;
  max_tokens?: number;
  enabled?: boolean;
}

export interface ChatCompletionRequest {
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
  tools?: ToolSchema[];
  stream_options?: StreamOptions;
  reasoning_effort?: string;
  reasoning?: Reasoning;
  service_tier?: string;
  top_p?: number;
  frequency_penalty?: number;
  presence_penalty?: number;
}

export interface ChatChoice {
  index: number;
  message: ChatMessage;
  finish_reason?: string;
}

export interface ChatCompletionResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: ChatChoice[];
  usage?: TokenUsage;
}

export interface ModelObject {
  id: string;
  object: string;
  created: number;
  owned_by: string;
}

export interface ModelsResponse {
  object: string;
  data: ModelObject[];
}

export interface AgentInfo {
  id: string;
  name: string;
  description: string;
  model: string;
  tools: string[];
  max_iterations: number;
}

export interface InvokeAgentRequest {
  input: string;
  conversation_id?: string;
  stream?: boolean;
}

export interface ToolCallRecord {
  id: string;
  name: string;
  arguments: string;
  result: string;
  tool_call_id: string;
}

export interface InvokeAgentResponse {
  output: string;
  conversation_id: string;
  usage: TokenUsage;
  iterations: number;
  tool_calls: ToolCallRecord[];
}

export interface CreateConversationRequest {
  agent_id: string;
  title?: string;
}

export interface ConversationResponse {
  id: string;
  agent_id: string;
  title?: string;
  created_at: number;
  updated_at: number;
}

export interface MessageResponse {
  id: string;
  role: string;
  content: string;
  tool_calls?: ToolCall[];
  tool_call_id?: string;
  model?: string;
  usage?: TokenUsage;
  created_at: number;
}

export interface HealthResponse {
  status: string;
  version: string;
}

export interface StatusResponse {
  status: string;
  version: string;
  uptime_secs: number;
  models: number;
  agents: number;
  routes: number;
}

export interface ModelUsageResponse {
  model: string;
  requests: number;
  tokens: number;
}

export interface UsageStatsResponse {
  total_requests: number;
  total_tokens: number;
  total_duration_ms: number;
  by_model: ModelUsageResponse[];
}

export interface ToolInfoResponse {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}

export interface ErrorDetail {
  message: string;
  code?: string;
}

export interface ErrorResponse {
  error: ErrorDetail;
}

export type StreamEvent =
  | { type: "text_chunk"; text: string }
  | { type: "thought_chunk"; text: string; title?: string; signature?: string }
  | { type: "tool_call_update"; stream_key: string; id?: string; name: string; arguments: string; signature?: string }
  | { type: "tool_call_request"; tool_call: ToolCall }
  | { type: "tool_call_result"; tool_call_id: string; result: string; is_error: boolean }
  | { type: "usage_update"; usage: TokenUsage }
  | { type: "error"; message: string }
  | { type: "retrying"; attempt: number; max_attempts: number }
  | { type: "done"; finish_reason: string };
