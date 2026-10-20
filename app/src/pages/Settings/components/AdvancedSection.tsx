import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Icon } from '@/components/Icon';
import { useToast } from '@/components/Toast';
import { skillsApi } from '@/api/tauri';
import { isTauri } from '@/lib/tauri-env';
interface AdvancedSectionProps {
  advancedMode: boolean;
  onToggle: (value: boolean) => void;
}

export const AdvancedSection: React.FC<AdvancedSectionProps> = ({
  advancedMode,
  onToggle,
}) => {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [repairing, setRepairing] = useState(false);

  return (
    <div className="bg-[var(--bg-card)] dark:bg-dark-bg-card rounded-2xl p-5 shadow-sm border border-[var(--border-color)] dark:border-dark-border">
      <div className="mb-4">
        <h3 className="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Icon name="build" className="text-2xl text-slate-600 dark:text-gray-300" />
          {t('settings.advanced.title')}
        </h3>
        <p className="text-sm text-slate-500 dark:text-gray-400 mt-1">
          {t('settings.advanced.description')}
        </p>
      </div>

      <div className="flex items-start justify-between gap-4 p-4 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-[var(--shell-surface-soft)] dark:bg-dark-bg-secondary">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-bold text-slate-700 dark:text-gray-200">
              {t('settings.advanced.modeLabel')}
            </span>
            {advancedMode && (
              <span className="text-[10px] font-bold px-1.5 py-0.5 rounded bg-amber-100 dark:bg-amber-500/20 text-amber-700 dark:text-amber-400">
                ON
              </span>
            )}
          </div>
          <p className="text-xs text-slate-500 dark:text-gray-400 mt-1 leading-relaxed">
            {advancedMode
              ? t('settings.advanced.enabledHint')
              : t('settings.advanced.disabledHint')}
          </p>
        </div>
        <button
          onClick={() => onToggle(!advancedMode)}
          className={`relative w-10 h-[22px] rounded-full transition-all flex-shrink-0 mt-0.5 ${
            advancedMode ? 'bg-amber-500' : 'bg-gray-300 dark:bg-gray-600'
          }`}
        >
          <span
            className={`absolute top-[2px] left-[2px] w-[18px] h-[18px] bg-white rounded-full shadow-sm transition-transform ${
              advancedMode ? 'translate-x-[18px]' : 'translate-x-0'
            }`}
          />
        </button>
      </div>

      {advancedMode && (
        <div className="mt-3 flex items-start gap-2 p-3 rounded-lg bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/20">
          <Icon name="warning" className="text-base text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
          <p className="text-xs text-amber-700 dark:text-amber-300 leading-relaxed">
            {t('settings.advanced.warning')}
          </p>
        </div>
      )}

      {/* 链接校准：按 skill_states 的 open 重建/清理磁盘链接 */}
      <div className="mt-4 flex items-start justify-between gap-4 p-4 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-[var(--shell-surface-soft)] dark:bg-dark-bg-secondary">
        <div className="flex-1 min-w-0">
          <span className="text-sm font-bold text-slate-700 dark:text-gray-200">
            {t('settings.advanced.repairLinks')}
          </span>
          <p className="text-xs text-slate-500 dark:text-gray-400 mt-1 leading-relaxed">
            {t('settings.advanced.repairLinksHint')}
          </p>
        </div>
        <button
          onClick={async () => {
            if (!isTauri() || repairing) return;
            setRepairing(true);
            try {
              const report = await skillsApi.repairLinks();
              if (report.errors.length > 0) {
                showToast(
                  'warning',
                  t('settings.advanced.repairLinksPartial', {
                    relinked: report.relinked,
                    removed: report.removed,
                    failed: report.errors.length,
                  }),
                );
              } else {
                showToast(
                  'success',
                  t('settings.advanced.repairLinksDone', {
                    relinked: report.relinked,
                    removed: report.removed,
                  }),
                );
              }
            } catch (err) {
              console.error('repair_skill_links failed:', err);
              showToast('error', t('settings.advanced.repairLinksFailed'), String(err));
            } finally {
              setRepairing(false);
            }
          }}
          disabled={repairing}
          className="flex-shrink-0 px-3 py-1.5 text-xs font-medium rounded-lg bg-slate-800 dark:bg-gray-200 text-white dark:text-slate-900 hover:opacity-90 disabled:opacity-50 transition-opacity"
        >
          {repairing ? t('settings.advanced.repairLinksRunning') : t('settings.advanced.repairLinksAction')}
        </button>
      </div>
    </div>
  );
};
