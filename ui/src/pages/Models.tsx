import { api } from "../api/client";
import { useFetch } from "../hooks/useFetch";
import { Card, EmptyState, ErrorBanner, PageHeader, Spinner } from "../components/ui";

export default function Models() {
  const models = useFetch(() => api.listModels(), []);
  const usage = useFetch(() => api.usageStats().catch(() => null), []);

  return (
    <div>
      <PageHeader title="Models" subtitle="Available models & per-model usage (read-only)" />
      <div className="space-y-4 p-6">
        {models.error && <ErrorBanner message={models.error} />}

        <Card title="Configured models">
          {models.loading ? (
            <Spinner />
          ) : models.data && models.data.data.length > 0 ? (
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-left text-xs uppercase tracking-wide text-slate-500">
                  <th className="py-2 pr-4 font-medium">ID</th>
                  <th className="py-2 pr-4 font-medium">Owned by</th>
                  <th className="py-2 pr-4 font-medium">Created</th>
                  <th className="py-2 font-medium text-right">Requests</th>
                  <th className="py-2 font-medium text-right">Tokens</th>
                </tr>
              </thead>
              <tbody>
                {models.data.data.map((m) => {
                  const u = usage.data?.by_model.find((x) => x.model === m.id);
                  return (
                    <tr key={m.id} className="border-b border-border-soft hover:bg-bg-hover/40">
                      <td className="py-2 pr-4 font-mono text-slate-200">{m.id}</td>
                      <td className="py-2 pr-4 text-slate-400">{m.owned_by}</td>
                      <td className="py-2 pr-4 text-slate-400">
                        {new Date(m.created * 1000).toLocaleDateString()}
                      </td>
                      <td className="py-2 text-right text-slate-300">{u?.requests ?? 0}</td>
                      <td className="py-2 text-right text-slate-300">{u?.tokens ?? 0}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          ) : (
            <EmptyState message="No models configured" />
          )}
        </Card>

        <div className="rounded-md border border-border bg-bg-soft p-3 text-xs text-slate-500">
          Provider API keys are managed via <code className="font-mono">config.yaml</code>{" "}
          (<code className="font-mono">api_key</code> or <code className="font-mono">api_key_env</code>).
          A key-management endpoint is not yet exposed by the server.
        </div>
      </div>
    </div>
  );
}
