import { useEffect, useRef, useState } from "react";
import { api } from "../api/client";
import type { ConversationResponse, MessageResponse } from "../api/types";
import { useFetch } from "../hooks/useFetch";
import { useAgentStream } from "../hooks/useAgentStream";
import { ErrorBanner, EmptyState, PageHeader } from "../components/ui";

interface Turn {
  role: "user" | "assistant";
  content: string;
  thought?: string;
  toolCalls?: number;
}

export default function Chat() {
  const agents = useFetch(() => api.listAgents(), []);
  const [agentId, setAgentId] = useState<string | null>(null);
  const [conversations, setConversations] = useState<ConversationResponse[]>([]);
  const [currentConv, setCurrentConv] = useState<ConversationResponse | null>(null);
  const [messages, setMessages] = useState<MessageResponse[]>([]);
  const [turns, setTurns] = useState<Turn[]>([]);
  const [input, setInput] = useState("");
  const stream = useAgentStream();
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!agentId && agents.data && agents.data.length > 0) {
      setAgentId(agents.data[0].id);
    }
  }, [agents.data, agentId]);

  useEffect(() => {
    api.listConversations().then(setConversations).catch(() => {});
  }, []);

  useEffect(() => {
    if (!currentConv) return;
    api.getMessages(currentConv.id).then(setMessages).catch(() => {});
  }, [currentConv]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [turns, stream.accumulator.text, stream.isStreaming]);

  async function send() {
    if (!agentId || !input.trim() || stream.isStreaming) return;
    const text = input.trim();
    setInput("");
    setTurns((t) => [...t, { role: "user", content: text }]);
    try {
      const acc = await stream.start(agentId, text, currentConv?.id ?? null, (ev) => {
        if (ev.type === "usage_update" && !currentConv) {
          // conversation id is created server-side; refresh list after done
        }
      });
      setTurns((t) => [
        ...t,
        {
          role: "assistant",
          content: acc.text || "(no output)",
          thought: acc.thought || undefined,
          toolCalls: acc.toolCalls.length || undefined,
        },
      ]);
      api.listConversations().then(setConversations).catch(() => {});
    } catch {
      // error surfaced via stream.error
    }
  }

  async function newConversation() {
    if (!agentId) return;
    try {
      const c = await api.createConversation({ agent_id: agentId });
      setCurrentConv(c);
      setConversations((cs) => [c, ...cs]);
      setTurns([]);
      setMessages([]);
    } catch {
      // ignore
    }
  }

  async function loadConversation(c: ConversationResponse) {
    setCurrentConv(c);
    setTurns([]);
  }

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Chat"
        subtitle="Stream a conversation with an agent"
        actions={
          <>
            <select
              className="input w-48"
              value={agentId ?? ""}
              onChange={(e) => setAgentId(e.target.value || null)}
            >
              <option value="">Select agent…</option>
              {agents.data?.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
            <button className="btn-ghost" onClick={newConversation}>
              New
            </button>
          </>
        }
      />
      <div className="flex flex-1 overflow-hidden">
        <div className="w-48 shrink-0 overflow-auto border-r border-border bg-bg-soft p-2">
          {conversations.length === 0 ? (
            <EmptyState message="No conversations" />
          ) : (
            <ul className="space-y-1">
              {conversations.map((c) => (
                <li key={c.id}>
                  <button
                    onClick={() => loadConversation(c)}
                    className={`w-full truncate rounded px-2 py-1.5 text-left text-sm ${
                      currentConv?.id === c.id
                        ? "bg-bg-hover text-white"
                        : "text-slate-400 hover:bg-bg-hover/60"
                    }`}
                    title={c.title ?? c.id}
                  >
                    {c.title ?? c.id.slice(0, 8)}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="flex flex-1 flex-col">
          <div ref={scrollRef} className="flex-1 space-y-4 overflow-auto p-6">
            {messages.length > 0 && (
              <div className="space-y-2">
                {messages.map((m) => (
                  <MessageBubble key={m.id} role={m.role} content={m.content} />
                ))}
                <div className="border-t border-border pt-2 text-xs text-slate-500">
                  — history loaded —
                </div>
              </div>
            )}
            {turns.map((t, i) => (
              <TurnBubble key={i} turn={t} />
            ))}
            {stream.isStreaming && (
              <TurnBubble
                turn={{
                  role: "assistant",
                  content: stream.accumulator.text,
                  thought: stream.accumulator.thought,
                  toolCalls: stream.accumulator.toolCalls.length,
                }}
                streaming
              />
            )}
            {turns.length === 0 && messages.length === 0 && !stream.isStreaming && (
              <EmptyState message="Send a message to start" />
            )}
          </div>

          <div className="border-t border-border p-3">
            {stream.error && <ErrorBanner message={stream.error} />}
            <div className="flex gap-2">
              <input
                className="input"
                placeholder={agentId ? "Message…" : "Select an agent first"}
                disabled={!agentId || stream.isStreaming}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    send();
                  }
                }}
              />
              {stream.isStreaming ? (
                <button className="btn-ghost" onClick={stream.abort}>
                  Stop
                </button>
              ) : (
                <button
                  className="btn-primary"
                  disabled={!agentId || !input.trim()}
                  onClick={send}
                >
                  Send
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function MessageBubble({ role, content }: { role: string; content: string }) {
  return (
    <div className={role === "user" ? "flex justify-end" : "flex justify-start"}>
      <div
        className={`max-w-[80%] rounded-lg px-3 py-2 text-sm ${
          role === "user"
            ? "bg-accent text-white"
            : "bg-bg-panel text-slate-200"
        }`}
      >
        {content}
      </div>
    </div>
  );
}

function TurnBubble({ turn, streaming }: { turn: Turn; streaming?: boolean }) {
  return (
    <div className={turn.role === "user" ? "flex justify-end" : "flex justify-start"}>
      <div className="max-w-[80%] space-y-1">
        {turn.thought && (
          <div className="rounded-md border border-border bg-bg-soft/50 px-2 py-1 text-xs italic text-slate-500">
            {turn.thought}
          </div>
        )}
        <div
          className={`rounded-lg px-3 py-2 text-sm ${
            turn.role === "user"
              ? "bg-accent text-white"
              : "bg-bg-panel text-slate-200"
          }`}
        >
          {turn.content || (streaming ? "…" : "")}
          {streaming && <span className="ml-1 inline-block animate-pulse">▍</span>}
        </div>
        {turn.toolCalls ? (
          <div className="text-xs text-slate-500">⚙ {turn.toolCalls} tool call(s)</div>
        ) : null}
      </div>
    </div>
  );
}
