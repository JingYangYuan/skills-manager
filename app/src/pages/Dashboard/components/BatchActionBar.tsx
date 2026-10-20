import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AgentConfig } from '@/types';
import { Icon } from '@/components/Icon';

interface BatchActionBarProps {
  count: number;
  /** 全部 agent，内部只保留 detected && enabled 的可见项 */
  agents: AgentConfig[];
  /** 当前来源作用域：all / global / <agent name> */
  scope: string;
  busy: boolean;
  onClear: () => void;
  onCancel: () => void;
  onEnable: (agent: string) => void;
  onDisable: (agent: string) => void;
  onDelete: () => void;
}

/**
 * 批量操作栏：停靠在主区域底部（不覆盖技能卡片），
 * 左侧是选择概览与作用域，中间是目标 Agent + 启用/停用，右侧是删除与退出。
 */
export function BatchActionBar({
  count,
  agents,
  scope,
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
  const scopeLabel =
    scope === 'all'
      ? t('dashboard.batch.scopeAll')
      : scope === 'global'
        ? t('dashboard.batch.scopeGlobal')
        : t('dashboard.batch.scopeAgent', {
            agent: agents.find(a => a.name === scope)?.display_name || scope,
          });

  return (
    <div className="flex-shrink-0 px-5 pb-3 pt-1 dark:bg-dark-bg-secondary">
      <div className="mx-auto flex max-w-[1600px] items-center gap-3 rounded-xl border border-[var(--border-color)] bg-white px-3 py-2 shadow-[0_10px_30px_-16px_rgba(0,0,0,0.35)] dark:border-dark-border dark:bg-dark-bg-tertiary">
        {/* 选择概览 */}
        <div className="flex min-w-0 items-center gap-2">
          <span className="flex items-center gap-1.5 rounded-full bg-sky-500/10 px-2.5 py-1 text-xs font-semibold text-sky-600 dark:bg-sky-500/15 dark:text-sky-300">
            <Icon name="check_circle" className="text-sm" />
            {t('dashboard.batch.selected', { count })}
          </span>
          <span className="truncate text-[11px] text-[var(--text-tertiary)] dark:text-gray-400">
            {scopeLabel}
          </span>
          <button
            type="button"
            onClick={onClear}
            disabled={disabled}
            className="rounded-md px-1.5 py-1 text-[11px] text-[var(--text-secondary)] transition-colors hover:bg-[var(--hover-bg)] disabled:opacity-40 dark:text-gray-400 dark:hover:bg-dark-hover"
          >
            {t('dashboard.batch.clear')}
          </button>
        </div>

        <span className="h-6 w-px flex-shrink-0 bg-[var(--border-color)] dark:bg-dark-border" />

        {/* 目标 Agent 与开关 */}
        <div className="flex min-w-0 flex-1 items-center gap-2">
          {availableAgents.length > 0 ? (
            <>
              <select
                value={agent}
                onChange={e => setAgent(e.target.value)}
                disabled={busy}
                className="h-8 max-w-[11rem] rounded-lg border border-[var(--border-color)] bg-white px-2 text-xs text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)] disabled:opacity-50 dark:border-dark-border dark:bg-dark-bg-tertiary dark:text-slate-200"
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
                className="flex h-8 items-center gap-1 rounded-lg bg-[var(--accent-primary)] px-3 text-xs font-medium text-white transition-colors hover:bg-[var(--accent-primary-strong)] disabled:cursor-not-allowed disabled:opacity-40"
              >
                <Icon name="toggle_on" className="text-sm" />
                {t('dashboard.batch.enable')}
              </button>
              <button
                type="button"
                onClick={() => agent && onDisable(agent)}
                disabled={disabled || !agent}
                className="flex h-8 items-center gap-1 rounded-lg border border-[var(--border-color)] px-3 text-xs font-medium text-[var(--text-primary)] transition-colors hover:bg-[var(--hover-bg)] disabled:cursor-not-allowed disabled:opacity-40 dark:border-dark-border dark:text-slate-200 dark:hover:bg-dark-hover"
              >
                <Icon name="toggle_off" className="text-sm" />
                {t('dashboard.batch.disable')}
              </button>
            </>
          ) : (
            <span className="text-[11px] text-[var(--text-tertiary)] dark:text-gray-400">
              {t('dashboard.batch.noAgent')}
            </span>
          )}
        </div>

        {/* 危险操作与退出 */}
        <div className="flex flex-shrink-0 items-center gap-2">
          <button
            type="button"
            onClick={onDelete}
            disabled={disabled}
            className="flex h-8 items-center gap-1 rounded-lg border border-red-200 px-3 text-xs font-medium text-red-600 transition-colors hover:bg-red-50 disabled:cursor-not-allowed disabled:opacity-40 dark:border-red-900/60 dark:text-red-400 dark:hover:bg-red-950/40"
          >
            <Icon name="delete" className="text-sm" />
            {t('dashboard.batch.delete')}
          </button>
          <button
            type="button"
            onClick={onCancel}
            disabled={busy}
            className="flex h-8 items-center gap-1 rounded-lg px-2.5 text-xs text-[var(--text-secondary)] transition-colors hover:bg-[var(--hover-bg)] disabled:opacity-40 dark:text-gray-400 dark:hover:bg-dark-hover"
          >
            <Icon name="close" className="text-sm" />
            {t('dashboard.batch.exit')}
          </button>
        </div>
      </div>
    </div>
  );
}
