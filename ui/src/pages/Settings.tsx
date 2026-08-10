import { useState } from "react";
import { api } from "../api/client";
import { useFetch } from "../hooks/useFetch";
import { Card, EmptyState, ErrorBanner, PageHeader, Spinner } from "../components/ui";

export default function Settings() {
  const status = useFetch(() => api.status(), []);
  const tools = useFetch(() => api.listTools().catch(() => []), []);
  const [metrics, setMetrics] = useState<string | null>(null);
  const [metricsLoading, setMetricsLoading] = useState(false);

  async function loadMetrics() {
    setMetricsLoading(true);
    try {
      setMetrics(await api.metricsText());
    } catch (e) {
      setMetrics(`# error: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setMetricsLoading(false);
    }
  }

  return (
    <div>
      <PageHeader title="Settings" subtitle="System status, tools & metrics" />
      <div className="grid grid-cols-1 gap-4 p-6 lg:grid-cols-2">
        <Card title="System status">
          {status.loading ? (
            <Spinner />
          ) : status.error ? (
            <ErrorBanner message={status.error} />
          ) : status.data ? (
            <dl className="space-y-2 text-sm">
              <Row label="Status" value={status.data.status} />
              <Row label="Version" value={status.data.version} />
              <Row label="Uptime" value={`${status.data.uptime_secs}s`} />
              <Row label="Models" value={String(status.data.models)} />
              <Row label="Agents" value={String(status.data.agents)} />
              <Row label="Routes" value={String(status.data.routes)} />
            </dl>
          ) : null}
        </Card>

        <Card title="Tools">
          {tools.loading ? (
            <Spinner />
          ) : tools.data && tools.data.length > 0 ? (
            <ul className="space-y-2">
              {tools.data.map((t) => (
                <li key={t.name} className="rounded-md bg-bg-soft p-2">
                  <div className="font-mono text-sm text-slate-200">{t.name}</div>
                  <div className="mt-0.5 text-xs text-slate-500">{t.description}</div>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState message="No tools registered" />
          )}
        </Card>

        <Card title="Prometheus metrics" className="lg:col-span-2">
          <div className="mb-2 flex items-center gap-2">
            <button className="btn-ghost" onClick={loadMetrics} disabled={metricsLoading}>
              {metricsLoading ? "Loading…" : "Fetch /metrics"}
            </button>
          </div>
          {metrics ? (
            <pre className="max-h-80 overflow-auto rounded-md bg-bg-soft p-3 font-mono text-xs text-slate-300">
              {metrics}
            </pre>
          ) : (
            <EmptyState message="Click “Fetch /metrics” to load" />
          )}
        </Card>
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between">
      <dt className="text-xs uppercase tracking-wide text-slate-500">{label}</dt>
      <dd className="font-mono text-slate-200">{value}</dd>
    </div>
  );
}
