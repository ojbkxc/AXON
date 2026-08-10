import type {
  AgentInfo,
  ChatCompletionRequest,
  ChatCompletionResponse,
  ConversationResponse,
  CreateConversationRequest,
  HealthResponse,
  InvokeAgentRequest,
  InvokeAgentResponse,
  MessageResponse,
  ModelsResponse,
  StatusResponse,
  ToolInfoResponse,
  UsageStatsResponse,
} from "./types";

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    const msg = body?.error?.message ?? res.statusText;
    throw new Error(`${res.status}: ${msg}`);
  }
  return res.json() as Promise<T>;
}

export const api = {
  async health(): Promise<HealthResponse> {
    return json(await fetch("/healthz"));
  },
  async ready(): Promise<HealthResponse> {
    return json(await fetch("/readyz"));
  },
  async status(): Promise<StatusResponse> {
    return json(await fetch("/status"));
  },
  async metricsText(): Promise<string> {
    const res = await fetch("/metrics");
    return res.text();
  },

  async listModels(): Promise<ModelsResponse> {
    return json(await fetch("/v1/models"));
  },
  async chatCompletions(req: ChatCompletionRequest): Promise<ChatCompletionResponse> {
    return json(
      await fetch("/v1/chat/completions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...req, stream: false }),
      })
    );
  },

  async listAgents(): Promise<AgentInfo[]> {
    return json(await fetch("/v1/agents"));
  },
  async getAgent(id: string): Promise<AgentInfo> {
    return json(await fetch(`/v1/agents/${encodeURIComponent(id)}`));
  },
  async invokeAgent(id: string, req: InvokeAgentRequest): Promise<InvokeAgentResponse> {
    return json(
      await fetch(`/v1/agents/${encodeURIComponent(id)}/invoke`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...req, stream: false }),
      })
    );
  },

  async createConversation(req: CreateConversationRequest): Promise<ConversationResponse> {
    return json(
      await fetch("/v1/conversations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(req),
      })
    );
  },
  async listConversations(): Promise<ConversationResponse[]> {
    return json(await fetch("/v1/conversations"));
  },
  async getConversation(id: string): Promise<ConversationResponse> {
    return json(await fetch(`/v1/conversations/${encodeURIComponent(id)}`));
  },
  async deleteConversation(id: string): Promise<void> {
    const res = await fetch(`/v1/conversations/${encodeURIComponent(id)}`, {
      method: "DELETE",
    });
    if (!res.ok) throw new Error(`${res.status}: ${res.statusText}`);
  },
  async getMessages(id: string): Promise<MessageResponse[]> {
    return json(await fetch(`/v1/conversations/${encodeURIComponent(id)}/messages`));
  },

  async listTools(): Promise<ToolInfoResponse[]> {
    return json(await fetch("/v1/tools"));
  },
  async usageStats(): Promise<UsageStatsResponse> {
    return json(await fetch("/v1/usage"));
  },
};
