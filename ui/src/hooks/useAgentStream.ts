import { useCallback, useRef, useState } from "react";
import type { StreamEvent, TokenUsage, ToolCall } from "../api/types";

export interface StreamAccumulator {
  text: string;
  thought: string;
  toolCalls: ToolCall[];
  usage: TokenUsage | null;
  finishReason: string | null;
  error: string | null;
}

export function emptyAccumulator(): StreamAccumulator {
  return {
    text: "",
    thought: "",
    toolCalls: [],
    usage: null,
    finishReason: null,
    error: null,
  };
}

function applyEvent(acc: StreamAccumulator, ev: StreamEvent): StreamAccumulator {
  switch (ev.type) {
    case "text_chunk":
      return { ...acc, text: acc.text + ev.text };
    case "thought_chunk":
      return { ...acc, thought: acc.thought + ev.text };
    case "tool_call_request":
      return { ...acc, toolCalls: [...acc.toolCalls, ev.tool_call] };
    case "tool_call_result":
      return acc;
    case "usage_update":
      return { ...acc, usage: ev.usage };
    case "error":
      return { ...acc, error: ev.message };
    case "done":
      return { ...acc, finishReason: ev.finish_reason };
    default:
      return acc;
  }
}

async function parseSSE(
  res: Response,
  onEvent: (ev: StreamEvent) => void
): Promise<void> {
  if (!res.body) throw new Error("no response body");
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let idx: number;
    while ((idx = buffer.indexOf("\n\n")) !== -1) {
      const raw = buffer.slice(0, idx);
      buffer = buffer.slice(idx + 2);
      const dataLine = raw
        .split("\n")
        .filter((l) => l.startsWith("data:"))
        .map((l) => l.slice(5).trimStart())
        .join("\n");
      if (!dataLine || dataLine === "[DONE]") continue;
      try {
        onEvent(JSON.parse(dataLine) as StreamEvent);
      } catch {
        // skip malformed chunk
      }
    }
  }
}

export function useAgentStream() {
  const [accumulator, setAccumulator] = useState<StreamAccumulator>(emptyAccumulator);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const start = useCallback(
    async (
      agentId: string,
      input: string,
      conversationId: string | null,
      onEvent?: (ev: StreamEvent) => void
    ): Promise<StreamAccumulator> => {
      const controller = new AbortController();
      abortRef.current = controller;
      setIsStreaming(true);
      setError(null);
      let acc = emptyAccumulator();
      setAccumulator(acc);
      try {
        const res = await fetch(
          `/v1/agents/${encodeURIComponent(agentId)}/invoke`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              input,
              conversation_id: conversationId ?? undefined,
              stream: true,
            }),
            signal: controller.signal,
          }
        );
        if (!res.ok) {
          const body = await res.json().catch(() => ({}));
          throw new Error(body?.error?.message ?? `${res.status}: ${res.statusText}`);
        }
        await parseSSE(res, (ev) => {
          acc = applyEvent(acc, ev);
          setAccumulator(acc);
          onEvent?.(ev);
        });
        if (acc.error) throw new Error(acc.error);
        return acc;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        if (msg === "The user aborted a request.") {
          return acc;
        }
        setError(msg);
        throw e;
      } finally {
        setIsStreaming(false);
        abortRef.current = null;
      }
    },
    []
  );

  const abort = useCallback(() => {
    abortRef.current?.abort();
  }, []);

  const reset = useCallback(() => {
    setAccumulator(emptyAccumulator());
    setError(null);
  }, []);

  return { accumulator, isStreaming, error, start, abort, reset };
}
