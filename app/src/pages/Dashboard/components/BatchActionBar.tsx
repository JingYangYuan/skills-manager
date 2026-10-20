import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AgentConfig } from '@/types';
import { Icon } from '@/components/Icon';

interface BatchActionBarProps {
  count: number;
  /** 全部 agent，内部只保留 detected && enabled 的可见项 */
  agents: AgentConfig[];
  busy: boolean;
  onClear: () => void;
  onCancel: () => void;
  onEnable: (agent: string) => void;
  onDisable: (agent: string) => void;
  onDelete: () => void;
}

/**
 * 批量操作浮条：出现在 Dashboard 底部中央。
 * 启用/停用需要一个目标 Agent，因此这里内置一个下拉选择。
 */
export function BatchActionBar({
  count,
  agents,
  busy,
  onClear,
  onCancel,
  onEnable,
  onDisable,
  onDelete,
}: BatchActionBarProps) {
  const { t } = useTranslation();
  const availableAgents = useMemo(
    () => agents.filter(a => a.detected && a.enabled),
    [agents]
  );
  const [agent, setAgent] = useState<string>('');

  // agent 列表变化时保证选中项有效
  useEffect(() => {
    if (availableAgents.length === 0) {
      if (agent !== '') setAgent('');
      return;
    }
    if (!availableAgents.some(a => a.name === agent)) {
      setAgent(availableAgents[0].name);
    }
  }, [availableAgents, agent]);

  const disabled = busy || count === 0;

  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-5 z-40 flex justify-center px-6">
      <div className="pointer-events-auto flex max-w-full flex-wrap items-center gap-2 rounded-xl border border-[var(--border-color)] bg-white/95 px-3 py-2 shadow-[0_18px_40px_-24px_rgba(15,23,42,0.55)] backdrop-blur dark:border-dark-border dark:bg-dark-bg-card/95">
        <span className="flex items-center gap-1.5 pr-1 text-xs font-medium text-[var(--text-primary)] dark:text-slate-200">
          <Icon name="check_circle" className="text-sm text-[var(--accent-primary)]" />
          {t('dashboard.batch.selected', { count })}
        </span>

        <span className="h-5 w-px bg-[var(--border-color)] dark:bg-dark-border" />

        <button
          type="button"
          onClick={onClear}
          disabled={disabled}
          className="rounded-lg px-2 py-1 text-[11px] text-[var(--text-secondary)] transition-colors hover:bg-[var(--hover-bg)] disabled:opacity-40 dark:text-gray-400 dark:hover:bg-dark-hover"
        >
          {t('dashboard.batch.clear')}
        </button>

        {availableAgents.length > 0 && (
          <>
            <span className="h-5 w-px bg-[var(--border-color)] dark:bg-dark-border" />
            <select
              value={agent}
              onChange={e => setAgent(e.target.value)}
              disabled={busy}
              className="max-w-[10rem] rounded-lg border border-[var(--border-color)] bg-transparent px-2 py-1 text-[11px] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)] dark:border-dark-border dark:bg-dark-bg-tertiary dark:text-slate-200"
              aria-label={t('dashboard.batch.agentLabel')}
            >
              {availableAgents.map(a => (
                <option key={a.name} value={a.name}>
                  {a.display_name || a.name}
                </option>
              ))}
            </select>
            <button
              type="button"
              onClick={() => agent && onEnable(agent)}
              disabled={disabled || !agent}
              className="rounded-lg bg-[var(--accent-primary)] px-2.5 py-1 text-[11px] font-medium text-white transition-colors hover:bg-[var(--accent-primary-strong)] disabled:opacity-40"
            >
              {t('dashboard.batch.enable')}
            </button>
            <button
              type="button"
              onClick={() => agent && onDisable(agent)}
              disabled={disabled || !agent}
              className="rounded-lg border border-[var(--border-color)] px-2.5 py-1 text-[11px] font-medium text-[var(--text-primary)] transition-colors hover:bg-[var(--hover-bg)] disabled:opacity-40 dark:border-dark-border dark:text-slate-200 dark:hover:bg-dark-hover"
            >
              {t('dashboard.batch.disable')}
            </button>
          </>
        )}

        <span className="h-5 w-px bg-[var(--border-color)] dark:bg-dark-border" />

        <button
          type="button"
          onClick={onDelete}
          disabled={disabled}
          className="flex items-center gap-1 rounded-lg px-2.5 py-1 text-[11px] font-medium text-red-600 transition-colors hover:bg-red-50 disabled:opacity-40 dark:text-red-400 dark:hover:bg-red-950/40"
        >
          <Icon name="delete" className="text-sm" />
          {t('dashboard.batch.delete')}
        </button>

        <button
          type="button"
          onClick={onCancel}
          disabled={busy}
          className="flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] text-[var(--text-secondary)] transition-colors hover:bg-[var(--hover-bg)] disabled:opacity-40 dark:text-gray-400 dark:hover:bg-dark-hover"
        >
          <Icon name="close" className="text-sm" />
          {t('dashboard.batch.exit')}
        </button>
      </div>
    </div>
  );
}
