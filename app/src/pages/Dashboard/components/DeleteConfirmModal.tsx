import { useEffect } from 'react';
import { useSet } from 'ahooks';
import { useTranslation } from 'react-i18next';
import type { SkillMetadata, SkillDeletionRow } from '@/types';
import { badgeClass, sourceLabel, SOURCE } from '@/pages/Dashboard/utils/source';

import { Icon } from '@/components/Icon';
type DeletePurpose = 'root-only' | 'multi-source';

interface DeleteConfirmModalProps {
  target: SkillMetadata | null;
  /** 多源删除：每行一个 source；root-only 场景传空或单元素。 */
  rows?: SkillDeletionRow[];
  advancedMode?: boolean;
  /** 点击「开启高阶模式」时调用：立即写入并广播开关，无需跳转设置页 */
  onEnableAdvancedMode?: () => void;
  purpose?: DeletePurpose;
  onConfirm: (selected: SkillDeletionRow[]) => void;
  onCancel: () => void;
}

export const DeleteConfirmModal: React.FC<DeleteConfirmModalProps> = ({
  target,
  rows,
  advancedMode = false,
  onEnableAdvancedMode,
  purpose = 'multi-source',
  onConfirm,
  onCancel,
}) => {
  const { t } = useTranslation();
  const isWindows = typeof navigator !== 'undefined' && navigator.platform.toLowerCase().includes('win');
  const norm = (p: string) => (isWindows ? p.replace(/\//g, '\\') : p);

  // root-only 场景下 rows 为空 → 用 target 兜底成单条 global 行
  const items: SkillDeletionRow[] = rows && rows.length > 0
    ? rows
    : target
      ? [{ skill: target, source: SOURCE.Global, path: target.source_paths?.[SOURCE.Global] }]
      : [];
  const isMulti = items.length > 1;
  const isRootOnly = purpose === 'root-only';
  // 是否存在被高阶模式锁住的非根目录来源
  const hasLockedRows = items.some(r => r.source !== SOURCE.Global) && !advancedMode;

  const [checked, { add: checkItem, remove: uncheckItem, reset: resetChecked }] = useSet<string>();
  const isChecked = (key: string) => checked.has(key);

  useEffect(() => {
    resetChecked();
    items
      .filter(r => r.source === SOURCE.Global || advancedMode)
      .forEach(r => checkItem(`${r.source}:${r.skill.id}`));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target, rows, advancedMode]);

  // ESC 键关闭删除确认弹窗
  useEffect(() => {
    if (!target) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        onCancel();
      }
    };
    window.addEventListener('keydown', onKeyDown, true); // 使用捕获阶段优先处理
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, [target, onCancel]);

  if (!target) return null;

  const toggleCheck = (key: string) => {
    if (isChecked(key)) uncheckItem(key);
    else checkItem(key);
  };

  const handleConfirm = () => {
    const selected = items.filter(r => isChecked(`${r.source}:${r.skill.id}`));
    if (selected.length > 0) onConfirm(selected);
  };

  const hasSelection = isRootOnly ? true : checked.size > 0;

  return (
    <div className="fixed inset-0 z-[9999] flex items-center justify-center p-4 bg-black/10 backdrop-blur-[2px]">
      <div className="w-full max-w-md bg-white/95 dark:bg-dark-bg-card backdrop-blur-xl rounded-3xl shadow-[0_30px_60px_-12px_rgba(0,0,0,0.25),0_18px_36px_-18px_rgba(0,0,0,0.3)] border border-white/50 dark:border-dark-border overflow-hidden flex flex-col items-center text-center p-8">
        <div className="w-16 h-16 rounded-2xl bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-6">
          <Icon name="error" className="text-red-500 text-4xl" />
        </div>
        <h3 className="font-bold text-2xl text-slate-900 dark:text-white mb-2">
          {isRootOnly ? t('dashboard.delete.fromRootTitle') : t('dashboard.delete.title')}
        </h3>
        <p className="text-sm text-slate-500 dark:text-gray-400 leading-relaxed mb-3 px-4">
          {isRootOnly
            ? t('dashboard.delete.fromRootMessage', { name: target.name })
            : t('dashboard.delete.message', { name: target.name })
          }
        </p>

        <div className="w-full text-left mb-3 space-y-2">
          {isMulti && !isRootOnly && (
            <p className="text-xs text-slate-500 dark:text-gray-400 mb-1 px-1">{t('dashboard.delete.selectSources')}</p>
          )}
          {items.map(row => {
            const key = `${row.source}:${row.skill.id}`;
            const rowChecked = isChecked(key);
            const isLocked = row.source !== SOURCE.Global && !advancedMode;
            const isClickDisabled = isLocked || isRootOnly;
            return (
              <div key={key}>
                <label
                  className={`flex items-center gap-2.5 p-2.5 rounded-xl border transition-all ${
                    isClickDisabled
                      ? 'cursor-default border-gray-200 dark:border-dark-border bg-gray-50 dark:bg-dark-bg-secondary'
                      : rowChecked
                        ? 'cursor-pointer border-red-300 dark:border-red-500/40 bg-red-50/60 dark:bg-red-900/15'
                        : 'cursor-pointer border-gray-200 dark:border-dark-border bg-white dark:bg-dark-bg-secondary hover:border-gray-300 dark:hover:border-gray-600'
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={rowChecked}
                    disabled={isLocked}
                    onChange={() => !isClickDisabled && toggleCheck(key)}
                    className={`w-4 h-4 rounded border-red-300 text-red-600 focus:ring-red-500 focus:ring-red-500/30 focus:ring-offset-0 accent-red-600 flex-shrink-0 disabled:opacity-40 ${isRootOnly ? 'hidden' : ''}`}
                  />
                  {isMulti && !isRootOnly && (
                    <span className={`text-[10px] font-bold py-0.5 px-1.5 rounded flex-shrink-0 ${badgeClass(row.source)}`}>
                      {sourceLabel(row.source)}
                    </span>
                  )}
                  <span className={`text-xs font-mono truncate min-w-0 flex-1 ${isLocked ? 'text-slate-400 dark:text-gray-500' : 'text-red-500 dark:text-red-400'}`}>
                    {row.path ? norm(row.path) : row.skill.id}
                  </span>
                </label>
              </div>
            );
          })}
        </div>

        {hasLockedRows && (
          <div className="mb-3 w-full flex items-start gap-2 rounded-xl border border-amber-200 bg-amber-50 p-3 text-left dark:border-amber-500/30 dark:bg-amber-950/30">
            <Icon name="warning" className="mt-0.5 flex-shrink-0 text-sm text-amber-600 dark:text-amber-400" />
            <div className="min-w-0 flex-1">
              <p className="text-[11px] leading-relaxed text-amber-700 dark:text-amber-300">
                {t('dashboard.delete.advancedRequired')}
              </p>
              {onEnableAdvancedMode && (
                <button
                  type="button"
                  onClick={onEnableAdvancedMode}
                  className="mt-1.5 text-[11px] font-bold text-amber-700 underline underline-offset-2 transition-colors hover:text-amber-900 dark:text-amber-300 dark:hover:text-amber-200"
                >
                  {t('dashboard.delete.enableAdvanced')}
                </button>
              )}
            </div>
          </div>
        )}

        <p className="text-xs text-slate-400 dark:text-gray-500 leading-relaxed mb-6 px-4">
          {t('dashboard.delete.description')}
        </p>
        <div className="w-full flex gap-3">
          <button
            onClick={handleConfirm}
            disabled={!hasSelection}
            className={`w-full py-3.5 font-bold rounded-2xl shadow-lg transition-all active:scale-[0.98] ${
              hasSelection
                ? 'bg-red-600 hover:bg-red-700 text-white shadow-red-600/20 hover:brightness-110'
                : 'bg-red-300 dark:bg-red-800/50 text-white/70 cursor-not-allowed shadow-none'
            }`}
          >
            {t('dashboard.delete.confirm')}{isMulti && checked.size > 0 ? ` (${checked.size})` : ''}
          </button>
          <button
            onClick={onCancel}
            className="w-full py-3.5 bg-slate-100 dark:bg-dark-bg-tertiary text-slate-700 dark:text-gray-300 font-semibold rounded-2xl hover:bg-slate-200 dark:hover:bg-dark-bg-secondary transition-colors"
          >
            {t('common.cancel')}
          </button>
        </div>
      </div>
    </div>
  );
};
