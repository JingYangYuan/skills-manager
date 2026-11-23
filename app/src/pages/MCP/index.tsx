import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import PageHeader from '@/components/PageHeader';
import { Icon } from '@/components/Icon';
import { useToast } from '@/components/Toast';
import type { McpServerAggregate } from '@/types';
import { useMcpServers } from './hooks/useMcpServers';
import { McpServerCard } from './components/McpServerCard';
import { McpServerDetail } from './components/McpServerDetail';
import { McpServerForm } from './components/McpServerForm';

/** 全局 MCP 管理页面。 */
export default function MCP() {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { overview, loading, error, refresh, save, remove, setEnabled, sync } = useMcpServers();
  const [query, setQuery] = useState('');
  const [conflictOnly, setConflictOnly] = useState(false);
  const [selectedName, setSelectedName] = useState<string | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<McpServerAggregate | null>(null);
  const [pendingDelete, setPendingDelete] = useState<McpServerAggregate | null>(null);
  const [deleting, setDeleting] = useState(false);

  const servers = overview?.servers ?? [];
  const agents = overview?.agents ?? [];

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return servers
      .filter((server) => (conflictOnly ? server.conflict : true))
      .filter(
        (server) =>
          !q ||
          server.name.toLowerCase().includes(q) ||
          (server.definition.command ?? '').toLowerCase().includes(q) ||
          (server.definition.url ?? '').toLowerCase().includes(q),
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [servers, query, conflictOnly]);

  const selected = servers.find((server) => server.name === selectedName) ?? null;

  const openAdd = () => {
    setEditing(null);
    setFormOpen(true);
  };

  const openEdit = (server: McpServerAggregate) => {
    setEditing(server);
    setFormOpen(true);
  };

  const confirmDelete = async () => {
    if (!pendingDelete) return;
    setDeleting(true);
    try {
      await remove(
        pendingDelete.name,
        pendingDelete.bindings.map((binding) => binding.agent),
      );
      showToast('success', t('mcp.deleted', { name: pendingDelete.name }));
      setPendingDelete(null);
      setSelectedName(null);
    } catch (e) {
      showToast('error', t('mcp.actionFailed'), String(e));
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div className="h-full flex flex-col bg-[var(--bg-primary)] dark:bg-dark-bg-secondary">
      <PageHeader
        icon="hub"
        title={t('mcp.title')}
        actions={
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => void refresh()}
              title={t('common.refresh')}
              className="p-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors"
            >
              <Icon name="refresh" className="text-base" />
            </button>
            <button
              type="button"
              onClick={openAdd}
              className="flex items-center gap-1.5 px-3 py-2 rounded-xl bg-[var(--accent-primary)] text-white text-sm font-semibold hover:opacity-90 active:scale-[0.99] transition-all"
            >
              <Icon name="add" className="text-base" />
              {t('mcp.add')}
            </button>
          </div>
        }
      />

      <div className="px-6 pt-5 pb-3 flex flex-wrap items-center gap-3">
        <div className="relative w-72">
          <Icon
            name="search"
            className="absolute left-3 top-1/2 -translate-y-1/2 text-base text-[var(--text-tertiary)]"
          />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t('mcp.search')}
            className="w-full pl-9 pr-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)]"
          />
        </div>
        <button
          type="button"
          onClick={() => setConflictOnly((value) => !value)}
          className={`flex items-center gap-1.5 px-3 py-2 rounded-xl text-xs font-bold transition-colors ${
            conflictOnly
              ? 'bg-amber-50 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400'
              : 'bg-black/5 dark:bg-white/5 text-[var(--text-secondary)]'
          }`}
        >
          <Icon name="warning" className="text-base" />
          {t('mcp.conflictOnly')}
        </button>
        <span className="text-xs text-[var(--text-tertiary)]">
          {t('mcp.count', { count: visible.length })}
        </span>
      </div>

      {(error || (overview?.errors.length ?? 0) > 0) && (
        <div className="mx-6 mb-2 space-y-1">
          {error && (
            <div className="px-3 py-2 rounded-xl bg-red-50 dark:bg-red-500/10 text-xs text-red-600 dark:text-red-400">
              {error}
            </div>
          )}
          {overview?.errors.map((item) => (
            <div
              key={item.agent}
              className="px-3 py-2 rounded-xl bg-amber-50 dark:bg-amber-500/10 text-xs text-amber-700 dark:text-amber-400"
            >
              {item.agent}: {item.error}
            </div>
          ))}
        </div>
      )}

      <div className="flex-1 overflow-y-auto px-6 pb-6">
        {loading && <p className="text-sm text-[var(--text-tertiary)]">{t('common.loading')}</p>}
        {!loading && visible.length === 0 && (
          <div className="flex flex-col items-center justify-center text-center py-20 rounded-3xl border border-dashed border-[var(--border-color)] dark:border-dark-border">
            <Icon name="hub" className="text-4xl text-[var(--text-tertiary)] mb-3" />
            <h4 className="font-bold text-[var(--text-primary)] dark:text-white">
              {t('mcp.emptyTitle')}
            </h4>
            <p className="mt-1 max-w-md text-sm text-[var(--text-secondary)] dark:text-gray-400">
              {t('mcp.emptyDesc')}
            </p>
          </div>
        )}
        {visible.length > 0 && (
          <div className="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-3">
            {visible.map((server) => (
              <McpServerCard
                key={server.name}
                server={server}
                onSelect={(item) => setSelectedName(item.name)}
              />
            ))}
          </div>
        )}
      </div>

      <McpServerDetail
        server={selected}
        agents={agents}
        onClose={() => setSelectedName(null)}
        onEdit={openEdit}
        onDelete={(server) => setPendingDelete(server)}
        onToggle={async (server, enabled, target) => {
          await setEnabled(server.name, enabled, [target]);
        }}
        onSync={async (server, fromAgent, toAgents) => {
          await sync(server.name, fromAgent, toAgents);
          showToast('success', t('mcp.synced', { name: server.name }));
        }}
      />

      <McpServerForm
        open={formOpen}
        initial={editing}
        agents={agents}
        onClose={() => setFormOpen(false)}
        onSubmit={async (server, targets) => {
          await save(server, targets);
          if (editing) {
            const removed = editing.bindings
              .map((binding) => binding.agent)
              .filter((agent) => !targets.includes(agent));
            if (removed.length > 0) {
              await remove(server.name, removed);
            }
          }
          setSelectedName(server.name);
          showToast('success', t('mcp.saved', { name: server.name }));
        }}
      />

      {pendingDelete && (
        <div
          className="fixed inset-0 z-[9999] bg-black/30 backdrop-blur-sm flex items-center justify-center p-6"
          onClick={() => !deleting && setPendingDelete(null)}
        >
          <div
            className="w-full max-w-md rounded-2xl bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-2xl p-5"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">
              {t('mcp.deleteTitle', { name: pendingDelete.name })}
            </h3>
            <p className="mt-2 text-sm text-[var(--text-secondary)] dark:text-gray-400">
              {t('mcp.deleteConfirm', {
                agents: pendingDelete.bindings
                  .map((binding) => binding.agent)
                  .join(', '),
              })}
            </p>
            <div className="mt-5 flex justify-end gap-2">
              <button
                type="button"
                disabled={deleting}
                onClick={() => setPendingDelete(null)}
                className="px-3 py-2 rounded-xl text-sm font-semibold text-[var(--text-secondary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                disabled={deleting}
                onClick={() => void confirmDelete()}
                className={`px-3 py-2 rounded-xl text-sm font-semibold text-white bg-red-500 hover:bg-red-600 transition-opacity ${
                  deleting ? 'opacity-60 cursor-wait' : ''
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
