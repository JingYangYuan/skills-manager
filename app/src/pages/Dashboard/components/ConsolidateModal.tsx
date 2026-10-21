import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AgentConfig } from '@/types';
import type { ConflictAction, ConsolidationPlan } from '@/api/tauri';
import { skillsApi } from '@/api/tauri';
import { Drawer } from '@/components/Drawer';
import { Icon } from '@/components/Icon';
import { useToast } from '@/components/Toast';

interface ConsolidateModalProps {
  isOpen: boolean;
  agent: AgentConfig | null;
  onClose: () => void;
  /** 归并成功后刷新技能与 agent 列表 */
  onDone: () => void | Promise<void>;
}

const ACTIONS: ConflictAction[] = ['skip', 'overwrite', 'rename'];

/**
 * Agent 一键归并：把该 Agent 目录下的原生技能搬进中央技能库，
 * 再以链接方式回填，DFS 只保留中央库一份真实副本。
 */
export function ConsolidateModal({ isOpen, agent, onClose, onDone }: ConsolidateModalProps) {
  const { t } = useTranslation();
  const { showToast } = useToast();

  const [plan, setPlan] = useState<ConsolidationPlan | null>(null);
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [applyErrors, setApplyErrors] = useState<string[]>([]);
  const [decisions, setDecisions] = useState<Record<string, ConflictAction>>({});

  const loadPlan = useCallback(async () => {
    if (!agent) return;
    setLoading(true);
    setError(null);
    setApplyErrors([]);
    try {
      const next = await skillsApi.planConsolidation(agent.name);
      setPlan(next);
      const defaults: Record<string, ConflictAction> = {};
      next.conflicts.forEach(c => {
        // 默认“覆盖”：归并的语义就是把 Agent 目录下的副本搬走，
        // 冲突项若默认跳过，用户点完归并会发现 Agent 目录里还留着一堆技能。
        defaults[c.id] = 'overwrite';
      });
      setDecisions(defaults);
    } catch (e) {
      setPlan(null);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [agent]);

  useEffect(() => {
    if (!isOpen || !agent) {
      setPlan(null);
      setError(null);
      return;
    }
    void loadPlan();
  }, [isOpen, agent, loadPlan]);

  const effectiveCount = useMemo(() => {
    if (!plan) return 0;
    const overwriteOrRename = plan.conflicts.filter(c => decisions[c.id] !== 'skip').length;
    return plan.movable.length + overwriteOrRename;
  }, [plan, decisions]);

  const handleApply = async () => {
    if (!agent || !plan) return;
    setApplying(true);
    try {
      const conflicts = plan.conflicts.map(c => ({
        id: c.id,
        action: decisions[c.id] ?? 'skip',
      }));
      const report = await skillsApi.applyConsolidation(agent.name, conflicts);
      if (report.errors.length > 0) {
        // 部分失败：留在抽屉里把原因逐条展示（之前只打 console.warn，用户看不到）
        setApplyErrors(report.errors);
        showToast(
          'error',
          t('dashboard.consolidate.resultFailed', {
            moved: report.moved.length,
            failed: report.errors.length,
          })
        );
        await onDone();
        return;
      }
      setApplyErrors([]);
      showToast(
        'success',
        t('dashboard.consolidate.result', {
          moved: report.moved.length,
          skipped: report.skipped.length + plan.skipped.length,
          failed: 0,
        })
      );
      await onDone();
      onClose();
    } catch (e) {
      showToast('error', String(e));
    } finally {
      setApplying(false);
    }
  };

  const agentLabel = agent?.display_name || agent?.name || '';

  return (
    <Drawer isOpen={isOpen} onClose={onClose} width="72vw">
      <div className="flex items-center justify-between border-b border-[var(--border-color)] p-4 dark:border-dark-border">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <h2 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">
              {t('dashboard.consolidate.title', { agent: agentLabel })}
            </h2>
          </div>
          <p className="mt-0.5 text-xs text-[var(--text-tertiary)] dark:text-gray-400">
            {t('dashboard.consolidate.hint')}
          </p>
        </div>
        <button
          onClick={onClose}
          className="rounded-lg p-1.5 transition-colors hover:bg-[var(--shell-surface-soft)] dark:hover:bg-dark-bg-tertiary"
        >
          <Icon name="close" className="text-[var(--text-secondary)] dark:text-gray-300" />
        </button>
      </div>

      <div className="flex-1 space-y-4 overflow-y-auto p-4">
        {loading && (
          <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)] dark:text-gray-300">
            <div className="h-4 w-4 animate-spin rounded-full border-2 border-slate-200 border-t-[var(--accent-primary)] dark:border-dark-border" />
            {t('dashboard.consolidate.planning')}
          </div>
        )}

        {applyErrors.length > 0 && (
          <section className="rounded-lg border border-red-300/70 bg-red-50 p-3 dark:border-red-900/40 dark:bg-red-950/30">
            <h3 className="mb-1 text-xs font-bold text-red-700 dark:text-red-300">
              {t('dashboard.consolidate.errorTitle', { count: applyErrors.length })}
            </h3>
            <ul className="space-y-1 font-mono text-[11px] leading-relaxed text-red-700/90 dark:text-red-300/90">
              {applyErrors.map((message, index) => (
                <li key={index} className="break-all">
                  {message}
                </li>
              ))}
            </ul>
          </section>
        )}

        {error && (
          <div className="rounded-lg border border-red-200 bg-red-50 p-3 text-xs text-red-700 dark:border-red-900/40 dark:bg-red-950/30 dark:text-red-300">
            {t('dashboard.consolidate.planFailed', { error })}
          </div>
        )}

        {!loading && plan && (
          <>
            <div className="flex flex-wrap items-center gap-2 text-[11px]">
              <span className="rounded-md bg-emerald-50 px-2 py-1 font-medium text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300">
                {t('dashboard.consolidate.summary.movable', { count: plan.movable.length })}
              </span>
              <span className="rounded-md bg-amber-50 px-2 py-1 font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">
                {t('dashboard.consolidate.summary.conflicts', { count: plan.conflicts.length })}
              </span>
              <span className="rounded-md bg-slate-100 px-2 py-1 font-medium text-slate-600 dark:bg-dark-bg-tertiary dark:text-gray-300">
                {t('dashboard.consolidate.summary.skipped', { count: plan.skipped.length })}
              </span>
              <span className="ml-auto font-mono text-[10px] text-[var(--text-tertiary)] dark:text-gray-500">
                {plan.agent_skills_dir}
              </span>
            </div>

            {plan.movable.length > 0 && (
              <section>
                <h3 className="mb-2 text-xs font-bold text-[var(--text-primary)] dark:text-gray-200">
                  {t('dashboard.consolidate.movableTitle')}
                </h3>
                <ul className="divide-y divide-[var(--border-color)] overflow-hidden rounded-lg border border-[var(--border-color)] dark:divide-dark-border dark:border-dark-border">
                  {plan.movable.map(item => (
                    <li key={item.id} className="flex items-center gap-2 px-3 py-2 text-xs">
                      <Icon name="arrow_forward" className="text-sm text-emerald-600 dark:text-emerald-400" />
                      <span className="font-medium text-[var(--text-primary)] dark:text-gray-200">{item.name}</span>
                      <span className="ml-auto truncate font-mono text-[10px] text-[var(--text-tertiary)] dark:text-gray-500">
                        {item.path}
                      </span>
                    </li>
                  ))}
                </ul>
              </section>
            )}

            {plan.conflicts.length > 0 && (
              <section>
                <h3 className="mb-2 text-xs font-bold text-[var(--text-primary)] dark:text-gray-200">
                  {t('dashboard.consolidate.conflictsTitle')}
                </h3>
                <ul className="space-y-2">
                  {plan.conflicts.map(conflict => (
                    <li
                      key={conflict.id}
                      className="rounded-lg border border-amber-300/60 bg-amber-50/60 p-3 dark:border-amber-900/40 dark:bg-amber-950/20"
                    >
                      <div className="mb-2 flex items-center gap-2">
                        <Icon name="warning" className="text-sm text-amber-600 dark:text-amber-400" />
                        <span className="text-xs font-bold text-[var(--text-primary)] dark:text-gray-200">
                          {conflict.name}
                        </span>
                        <span className="ml-auto font-mono text-[10px] text-[var(--text-tertiary)] dark:text-gray-500">
                          {conflict.central_path}
                        </span>
                      </div>
                      <div className="flex flex-wrap gap-1.5">
                        {ACTIONS.map(action => {
                          const active = (decisions[conflict.id] ?? 'skip') === action;
                          return (
                            <button
                              key={action}
                              type="button"
                              onClick={() =>
                                setDecisions(prev => ({ ...prev, [conflict.id]: action }))
                              }
                              className={`rounded-md px-2 py-1 text-[11px] font-medium transition-colors ${active
                                ? 'bg-[var(--accent-primary)] text-white'
                                : 'bg-white text-[var(--text-secondary)] hover:bg-[var(--hover-bg)] dark:bg-dark-bg-tertiary dark:text-gray-300 dark:hover:bg-dark-hover'
                              }`}
                            >
                              {t(`dashboard.consolidate.actions.${action}`)}
                            </button>
                          );
                        })}
                      </div>
                    </li>
                  ))}
                </ul>
              </section>
            )}

            {plan.skipped.length > 0 && (
              <section>
                <h3 className="mb-2 text-xs font-bold text-[var(--text-primary)] dark:text-gray-200">
                  {t('dashboard.consolidate.skippedTitle')}
                </h3>
                <ul className="space-y-1">
                  {plan.skipped.map(item => (
                    <li
                      key={item.id}
                      className="flex items-center gap-2 rounded-md bg-slate-50 px-2 py-1.5 text-[11px] dark:bg-dark-bg-tertiary"
                    >
                      <span className="font-medium text-[var(--text-secondary)] dark:text-gray-300">{item.id}</span>
                      <span className="ml-auto text-[var(--text-tertiary)] dark:text-gray-500">
                        {t(`dashboard.consolidate.skipReasons.${item.reason}`, {
                          defaultValue: item.reason,
                        })}
                      </span>
                    </li>
                  ))}
                </ul>
              </section>
            )}

            {plan.movable.length === 0 && plan.conflicts.length === 0 && (
              <p className="rounded-lg bg-slate-50 p-3 text-xs text-[var(--text-secondary)] dark:bg-dark-bg-tertiary dark:text-gray-300">
                {t('dashboard.consolidate.nothingToDo')}
              </p>
            )}
          </>
        )}
      </div>

      <div className="flex items-center justify-end gap-2 border-t border-[var(--border-color)] p-4 dark:border-dark-border">
        <button
          onClick={onClose}
          className="rounded-lg px-4 py-2 text-sm font-medium text-[var(--text-secondary)] transition-colors hover:bg-[var(--hover-bg)] dark:text-gray-300 dark:hover:bg-dark-hover"
        >
          {t('common.cancel')}
        </button>
        <button
          onClick={handleApply}
          disabled={applying || loading || !plan || effectiveCount === 0}
          className="rounded-lg bg-[var(--accent-primary)] px-4 py-2 text-sm font-bold text-white transition-colors hover:bg-[var(--accent-primary-strong)] disabled:cursor-not-allowed disabled:opacity-40"
        >
          {applying
            ? t('dashboard.consolidate.applying')
            : t('dashboard.consolidate.apply', { count: effectiveCount })}
        </button>
      </div>
    </Drawer>
  );
}
