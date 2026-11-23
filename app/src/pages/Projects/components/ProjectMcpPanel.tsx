import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import { useToast } from '@/components/Toast';
import { mcpApi } from '@/api/tauri';
import type { McpOverview, McpServerAggregate } from '@/types';
import { McpServerCard } from '@/pages/MCP/components/McpServerCard';
import { McpServerDetail } from '@/pages/MCP/components/McpServerDetail';
import { McpServerForm } from '@/pages/MCP/components/McpServerForm';

interface ProjectMcpPanelProps {
  projectId: string;
}

/** 项目页的 MCP tab：读写项目内的 .mcp.json / .cursor/mcp.json / .grok/config.toml。 */
export function ProjectMcpPanel({ projectId }: ProjectMcpPanelProps) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [overview, setOverview] = useState<McpOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<McpServerAggregate | null>(null);
  const [selectedName, setSelectedName] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<McpServerAggregate | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setOverview(await mcpApi.listProject(projectId));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const servers = overview?.servers ?? [];
  const agents = overview?.agents ?? [];
  const selected = servers.find((server) => server.name === selectedName) ?? null;

  const openEdit = (server: McpServerAggregate) => {
    setEditing(server);
    setFormOpen(true);
  };

  const confirmDelete = async () => {
    if (!pendingDelete) return;
    setBusy(true);
    try {
      for (const binding of pendingDelete.bindings) {
        await mcpApi.removeProject(projectId, pendingDelete.name, binding.agent);
      }
      showToast('success', t('projects.mcpDeleted'));
      setPendingDelete(null);
      setSelectedName(null);
      await refresh();
    } catch (e) {
      showToast('error', t('mcp.actionFailed'), String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
            {t('projects.mcpTab')}
          </h3>
          <span className="text-xs font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]">
            {servers.length}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => void refresh()}
            title={t('common.refresh')}
            className="p-2 rounded-lg text-[var(--text-tertiary)] hover:text-[var(--text-primary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
          >
            <Icon name="refresh" className="text-base" />
          </button>
          <button
            type="button"
            onClick={() => {
              setEditing(null);
              setFormOpen(true);
            }}
            className="flex items-center gap-1.5 px-3 py-2 rounded-xl bg-[var(--accent-primary)] text-white text-sm font-semibold hover:opacity-90 active:scale-[0.99] transition-all"
          >
            <Icon name="add" className="text-base" />
            {t('projects.addMcp')}
          </button>
        </div>
      </div>

      {error && (
        <div className="mb-4 px-3 py-2 rounded-xl bg-red-50 dark:bg-red-500/10 text-sm text-red-600 dark:text-red-400">
          {error}
        </div>
      )}
      {overview?.errors.map((item) => (
        <div
          key={item.agent}
          className="mb-2 px-3 py-2 rounded-xl bg-amber-50 dark:bg-amber-500/10 text-xs text-amber-700 dark:text-amber-400"
        >
          {item.agent}: {item.error}
        </div>
      ))}

      {loading ? (
        <p className="text-sm text-[var(--text-tertiary)]">{t('common.loading')}</p>
      ) : servers.length === 0 ? (
        <div className="flex flex-col items-center justify-center text-center py-16 rounded-3xl border border-dashed border-[var(--border-color)] dark:border-dark-border">
          <Icon name="hub" className="text-4xl text-[var(--text-tertiary)] mb-3" />
          <h4 className="font-bold text-[var(--text-primary)] dark:text-white">
            {t('projects.mcpEmptyTitle')}
          </h4>
          <p className="mt-1 max-w-sm text-sm text-[var(--text-secondary)] dark:text-gray-400">
            {t('projects.mcpEmptyDesc')}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-3">
          {servers.map((server) => (
            <McpServerCard
              key={server.name}
              server={server}
              onSelect={(item) => setSelectedName(item.name)}
            />
          ))}
        </div>
      )}

      <McpServerDetail
        server={selected}
        agents={agents}
        onClose={() => setSelectedName(null)}
        onEdit={openEdit}
        onDelete={(server) => setPendingDelete(server)}
        onToggle={async (server, enabled, target) => {
          await mcpApi.saveProject(
            projectId,
            { ...server.definition, enabled },
            target,
          );
          await refresh();
        }}
        onSync={async (server, _fromAgent, toAgents) => {
          for (const target of toAgents) {
            await mcpApi.saveProject(projectId, server.definition, target);
          }
          await refresh();
          showToast('success', t('mcp.synced', { name: server.name }));
        }}
        onRaw={(agent) => mcpApi.rawProjectConfig(projectId, agent)}
      />

      <McpServerForm
        open={formOpen}
        initial={editing}
        agents={agents}
        onClose={() => setFormOpen(false)}
        onSubmit={async (server, targets) => {
          for (const target of targets) {
            await mcpApi.saveProject(projectId, server, target);
          }
          await refresh();
          setSelectedName(server.name);
          showToast('success', t('projects.mcpSaved'));
        }}
      />

      {pendingDelete && (
        <div
          className="fixed inset-0 z-[9999] bg-black/30 backdrop-blur-sm flex items-center justify-center p-6"
          onClick={() => !busy && setPendingDelete(null)}
        >
          <div
            className="w-full max-w-md rounded-2xl bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-2xl p-5"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">
              {t('mcp.deleteTitle', { name: pendingDelete.name })}
            </h3>
            <p className="mt-2 text-sm text-[var(--text-secondary)] dark:text-gray-400">
              {t('projects.deleteMcpConfirm', { name: pendingDelete.name })}
            </p>
            <div className="mt-5 flex justify-end gap-2">
              <button
                type="button"
                disabled={busy}
                onClick={() => setPendingDelete(null)}
                className="px-3 py-2 rounded-xl text-sm font-semibold text-[var(--text-secondary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => void confirmDelete()}
                className={`px-3 py-2 rounded-xl text-sm font-semibold text-white bg-red-500 hover:bg-red-600 transition-opacity ${
                  busy ? 'opacity-60 cursor-wait' : ''
                }`}
              >
                {t('common.delete')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
