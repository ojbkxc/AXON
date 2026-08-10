import { useState, type ReactNode } from "react";
import { api } from "../api/client";
import type { AgentInfo } from "../api/types";
import { useFetch } from "../hooks/useFetch";
import { Card, EmptyState, ErrorBanner, PageHeader, Spinner } from "../components/ui";

export default function Agents() {
  const agents = useFetch(() => api.listAgents(), []);
  const [selected, setSelected] = useState<AgentInfo | null>(null);

  return (
    <div>
      <PageHeader title="Agents" subtitle="Configured agents (read-only; edit via config.yaml)" />
      <div className="grid grid-cols-1 gap-4 p-6 lg:grid-cols-3">
        <div className="space-y-2 lg:col-span-1">
          {agents.loading ? (
            <Spinner />
          ) : agents.error ? (
            <ErrorBanner message={agents.error} />
          ) : agents.data && agents.data.length > 0 ? (
            agents.data.map((a) => (
              <button
                key={a.id}
                onClick={() => setSelected(a)}
                className={`card w-full p-3 text-left transition-colors ${
                  selected?.id === a.id ? "border-accent" : "hover:bg-bg-hover"
                }`}
              >
                <div className="font-medium text-slate-200">{a.name}</div>
                <div className="mt-0.5 truncate font-mono text-xs text-slate-500">{a.id}</div>
              </button>
            ))
          ) : (
            <EmptyState message="No agents configured" />
          )}
        </div>

        <div className="lg:col-span-2">
          {selected ? (
            <Card title={selected.name}>
              <dl className="space-y-3 text-sm">
                <Row label="ID" value={<code className="font-mono text-slate-300">{selected.id}</code>} />
                <Row label="Description" value={selected.description || "—"} />
                <Row label="Model" value={<code className="font-mono text-accent">{selected.model}</code>} />
                <Row label="Max iterations" value={String(selected.max_iterations)} />
                <Row
                  label="Tools"
                  value={
                    selected.tools.length > 0 ? (
                      <div className="flex flex-wrap gap-1">
                        {selected.tools.map((t) => (
                          <span
                            key={t}
                            className="rounded bg-bg-soft px-1.5 py-0.5 font-mono text-xs text-slate-300"
                          >
                            {t}
                          </span>
                        ))}
                      </div>
                    ) : (
                      "none"
                    )
                  }
                />
              </dl>
              <div className="mt-4 rounded-md border border-border bg-bg-soft p-2 text-xs text-slate-500">
                Editing agents requires modifying <code className="font-mono">config.yaml</code> and
                relying on hot-reload; no CRUD endpoint is exposed by the server yet.
              </div>
            </Card>
          ) : (
            <EmptyState message="Select an agent to view details" />
          )}
        </div>
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="flex gap-3">
      <dt className="w-32 shrink-0 text-xs uppercase tracking-wide text-slate-500">{label}</dt>
      <dd className="flex-1 text-slate-200">{value}</dd>
    </div>
  );
}
