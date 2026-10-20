import { useTranslation } from 'react-i18next';
import type { SkillMetadata } from '@/types';
import { Drawer } from '@/components/Drawer';
import { Icon } from '@/components/Icon';

interface BatchDeleteConfirmProps {
  isOpen: boolean;
  skills: SkillMetadata[];
  busy: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/** 批量删除确认：明确告知会移入回收站，可恢复 */
export function BatchDeleteConfirm({ isOpen, skills, busy, onConfirm, onCancel }: BatchDeleteConfirmProps) {
  const { t } = useTranslation();

  return (
    <Drawer isOpen={isOpen} onClose={onCancel} width="46vw">
      <div className="flex items-center gap-2 border-b border-[var(--border-color)] p-4 dark:border-dark-border">
        <Icon name="delete" className="text-lg text-red-600 dark:text-red-400" />
        <h2 className="text-base font-bold text-[var(--text-primary)] dark:text-white">
          {t('dashboard.batch.deleteTitle', { count: skills.length })}
        </h2>
      </div>

      <div className="flex-1 space-y-3 overflow-y-auto p-4">
        <p className="rounded-lg bg-amber-50 p-3 text-xs text-amber-700 dark:bg-amber-950/30 dark:text-amber-300">
          {t('dashboard.batch.deleteWarning')}
        </p>
        <ul className="max-h-64 space-y-1 overflow-y-auto">
          {skills.map(skill => (
            <li
              key={skill.id}
              className="truncate rounded-md bg-slate-50 px-2 py-1.5 text-xs text-[var(--text-primary)] dark:bg-dark-bg-tertiary dark:text-gray-200"
            >
              {skill.name}
            </li>
          ))}
        </ul>
      </div>

      <div className="flex items-center justify-end gap-2 border-t border-[var(--border-color)] p-4 dark:border-dark-border">
        <button
          onClick={onCancel}
          disabled={busy}
          className="rounded-lg px-4 py-2 text-sm font-medium text-[var(--text-secondary)] transition-colors hover:bg-[var(--hover-bg)] disabled:opacity-40 dark:text-gray-300 dark:hover:bg-dark-hover"
        >
          {t('common.cancel')}
        </button>
        <button
          onClick={onConfirm}
          disabled={busy || skills.length === 0}
          className="rounded-lg bg-red-600 px-4 py-2 text-sm font-bold text-white transition-colors hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {busy ? t('common.processing') : t('dashboard.batch.deleteConfirm')}
        </button>
      </div>
    </Drawer>
  );
}
