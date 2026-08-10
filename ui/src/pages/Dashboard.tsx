import { api } from "../api/client";
import { useFetch } from "../hooks/useFetch";
import { Card, EmptyState, ErrorBanner, PageHeader, Spinner, Stat } from "../components/ui";

export default function Dashboard() {
  const status = useFetch(() => api.status(), []);
  const usage = useFetch(() => api.usageStats().catch(() => null), []);
  const agents = useFetch(() => api.listAgents().catch(() => []), []);

  return (
    <div>
      <PageHeader title="Dashboard" subtitle="Overview & usage statistics" />
      <div className="space-y-4 p-6">
        {status.error && <ErrorBanner message={status.error} />}

        <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
          <Stat
            label="Status"
            value={status.loading ? <Spinner /> : status.data?.status ?? "—"}
            hint={status.data ? `v${status.data.version}` : undefined}
          />
          <Stat
            label="Uptime"
            value={status.data ? formatUptime(status.data.uptime_secs) : "—"}
            hint="since start"
          />
          <Stat
            label="Models"
            value={status.data?.models ?? "—"}
            hint="configured"
          />
          <Stat
            label="Agents"
            value={status.data?.agents ?? "—"}
            hint="configured"
          />
        </div>

        <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <Card title="Token usage">
            {usage.loading ? (
              <Spinner />
            ) : usage.data ? (
              <div className="space-y-3">
                <div className="grid grid-cols-3 gap-2 text-center">
                  <div>
                    <div className="label">Requests</div>
                    <div className="text-lg font-semibold text-white">
                      {usage.data.total_requests}
                    </div>
                  </div>
                  <div>
                    <div className="label">Tokens</div>
                    <div className="text-lg font-semibold text-white">
                      {usage.data.total_tokens}
                    </div>
                  </div>
                  <div>
                    <div className="label">Duration</div>
                    <div className="text-lg font-semibold text-white">
                      {(usage.data.total_duration_ms / 1000).toFixed(1)}s
                    </div>
                  </div>
                </div>
                {usage.data.by_model.length > 0 && (
                  <div className="mt-2 space-y-1">
                    {usage.data.by_model.map((m) => (
                      <div
                        key={m.model}
                        className="flex items-center justify-between rounded bg-bg-soft px-2 py-1 text-xs"
                      >
                        <span className="font-mono text-slate-300">{m.model}</span>
                        <span className="text-slate-400">
                          {m.requests} req · {m.tokens} tok
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <EmptyState message="No usage data yet" />
            )}
          </Card>

          <Card title="Agents">
            {agents.loading ? (
              <Spinner />
            ) : agents.data && agents.data.length > 0 ? (
              <ul className="space-y-1">
                {agents.data.map((a) => (
                  <li
                    key={a.id}
                    className="flex items-center justify-between rounded bg-bg-soft px-2 py-1.5 text-sm"
                  >
                    <span className="font-medium text-slate-200">{a.name}</span>
                    <span className="font-mono text-xs text-slate-500">{a.model}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <EmptyState message="No agents configured" />
            )}
          </Card>
        </div>
      </div>
    </div>
  );
}

function formatUptime(secs: number): string {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}
