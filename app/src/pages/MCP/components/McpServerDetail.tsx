import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Drawer } from '@/components/Drawer';
import { Icon } from '@/components/Icon';
import { getAgentDisplayName } from '@/constants';
import { mcpApi } from '@/api/tauri';
import type { McpAgentInfo, McpBinding, McpServerAggregate } from '@/types';

interface McpServerDetailProps {
  server: McpServerAggregate | null;
  agents: McpAgentInfo[];
  onClose: () => void;
  onEdit: (server: McpServerAggregate) => void;
  onDelete: (server: McpServerAggregate) => void;
  onToggle: (server: McpServerAggregate, enabled: boolean, target: string) => Promise<void>;
  onSync: (server: McpServerAggregate, fromAgent: string, toAgents: string[]) => Promise<void>;
  /** 项目级 MCP 读取原始配置的方式不同，默认走全局命令 */
  onRaw?: (agent: string) => Promise<string>;
}

export function McpServerDetail({
  server,
  agents,
  onClose,
  onEdit,
  onDelete,
  onToggle,
  onSync,
  onRaw,
}: McpServerDetailProps) {
  const { t } = useTranslation();
  const [expandedBinding, setExpandedBinding] = useState<string | null>(null);
  const [raw, setRaw] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!server) return null;

  const bindingAgents = server.bindings.map((binding) => binding.agent);
  // 只同步到本机已检测到的 agent，避免为未安装的工具创建无意义配置。
  const syncTargets = agents
    .filter((agent) => agent.exists)
    .map((agent) => agent.agent)
    .filter((agent) => !bindingAgents.includes(agent));

  const toggle = async (binding: McpBinding, next: boolean) => {
    setBusy(true);
    setError(null);
    try {
      await onToggle(server, next, binding.agent);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const sync = async (fromAgent: string) => {
    setBusy(true);
    setError(null);
    try {
      await onSync(server, fromAgent, syncTargets);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const loadRaw = async (agent: string) => {
    if (expandedBinding === agent) {
      setExpandedBinding(null);
      return;
    }
    setExpandedBinding(agent);
    if (raw[agent] !== undefined) return;
    try {
      const content = onRaw ? await onRaw(agent) : await mcpApi.rawConfig(agent);
      setRaw((current) => ({ ...current, [agent]: content }));
    } catch (e) {
      setRaw((current) => ({ ...current, [agent]: String(e) }));
    }
  };

  return (
    <Drawer isOpen onClose={onClose} title={server.name} width="480px">
      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)] uppercase">
            {server.definition.transport}
          </span>
          {server.conflict && (
            <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-amber-50 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400">
              {t('mcp.conflictHint')}
            </span>
          )}
        </div>

        {server.definition.transport === 'stdio' ? (
          <div className="rounded-xl bg-black/5 dark:bg-white/5 p-3 font-mono text-xs text-[var(--text-secondary)] break-all">
            {[server.definition.command, ...server.definition.args].filter(Boolean).join(' ') || '—'}
          </div>
        ) : (
          <div className="rounded-xl bg-black/5 dark:bg-white/5 p-3 font-mono text-xs text-[var(--text-secondary)] break-all">
            {server.definition.url || '—'}
          </div>
        )}

        {Object.keys(server.definition.env).length > 0 && (
          <div>
            <h4 className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)] mb-1">
              ENV
            </h4>
            <div className="space-y-1">
              {Object.keys(server.definition.env).map((key) => (
                <div key={key} className="flex items-center justify-between text-xs font-mono">
                  <span className="text-[var(--text-secondary)]">{key}</span>
                  <span className="text-[var(--text-tertiary)]">••••••</span>
                </div>
              ))}
            </div>
          </div>
        )}

        <div>
          <h4 className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)] mb-2">
            {t('mcp.bindings')}
          </h4>
          <div className="space-y-2">
            {server.bindings.map((binding) => (
              <div
                key={binding.agent}
                className="rounded-xl border border-[var(--border-color)] dark:border-dark-border p-3"
              >
                <div className="flex items-center gap-2">
                  <span className="font-semibold text-sm text-[var(--text-primary)] dark:text-white">
                    {getAgentDisplayName(binding.agent)}
                  </span>
                  {binding.managed_disabled && (
                    <span className="text-[10px] text-[var(--text-tertiary)]">
                      {t('mcp.managedDisabled')}
                    </span>
                  )}
                  <label className="ml-auto flex items-center gap-1.5 text-xs text-[var(--text-secondary)]">
                    <input
                      type="checkbox"
                      checked={binding.enabled}
                      disabled={busy}
                      onChange={(e) => void toggle(binding, e.target.checked)}
                      className="accent-[var(--accent-primary)]"
                    />
                    {t('common.enable')}
                  </label>
                </div>
                <p className="mt-1 text-[10px] font-mono text-[var(--text-tertiary)] break-all">
                  {binding.config_path}
                </p>
                <button
                  type="button"
                  onClick={() => void loadRaw(binding.agent)}
                  className="mt-1 text-[11px] font-semibold text-[var(--accent-primary)] hover:underline"
                >
                  {expandedBinding === binding.agent ? t('mcp.hideRaw') : t('mcp.showRaw')}
                </button>
                {expandedBinding === binding.agent && (
                  <pre className="mt-2 max-h-56 overflow-auto rounded-lg bg-black/5 dark:bg-white/5 p-2 text-[10px] text-[var(--text-secondary)] whitespace-pre-wrap break-all">
                    {raw[binding.agent] ?? t('common.loading')}
                  </pre>
                )}
              </div>
            ))}
          </div>
        </div>

        {syncTargets.length > 0 && (
          <div>
            <h4 className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)] mb-2">
              {t('mcp.syncTitle')}
            </h4>
            <p className="text-xs text-[var(--text-secondary)] dark:text-gray-400 mb-2">
              {t('mcp.syncDesc')}
            </p>
            <button
              type="button"
              disabled={busy}
              onClick={() => void sync(server.bindings[0].agent)}
              className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-[var(--accent-primary)] text-white hover:opacity-90 disabled:opacity-50"
            >
              {t('mcp.syncAction', { agents: syncTargets.map(getAgentDisplayName).join(', ') })}
            </button>
          </div>
        )}

        {error && (
          <div className="px-3 py-2 rounded-xl bg-red-50 dark:bg-red-500/10 text-xs text-red-600 dark:text-red-400">
            {error}
          </div>
        )}
      </div>

      <div className="px-5 py-4 border-t border-[var(--border-color)] dark:border-dark-border flex justify-end gap-2 flex-shrink-0">
        <button
          type="button"
          onClick={() => onDelete(server)}
          className="px-3 py-2 rounded-xl text-sm font-semibold text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 transition-colors"
        >
          {t('common.delete')}
        </button>
        <button
          type="button"
          onClick={() => onEdit(server)}
          className="flex items-center gap-1.5 px-4 py-2 rounded-xl text-sm font-semibold bg-[var(--accent-primary)] text-white hover:opacity-90 transition-opacity"
        >
          <Icon name="edit" className="text-base" />
          {t('common.edit')}
        </button>
      </div>
    </Drawer>
  );
}
