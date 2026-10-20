import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Icon } from '@/components/Icon';
import { useToast } from '@/components/Toast';
import { trashApi, type TrashItem } from '@/api/tauri';
import { isTauri } from '@/lib/tauri-env';

/**
 * 回收站：删除技能时不再直接销毁，而是移入 ~/.skills-manager/trash/<id>/。
 * 这里提供恢复、彻底删除与清空。
 */
export function TrashSection() {
  const { t } = useTranslation();
  const { showToast } = useToast();

  const [items, setItems] = useState<TrashItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [workingId, setWorkingId] = useState<string | null>(null);
  const [confirmPurgeId, setConfirmPurgeId] = useState<string | null>(null);
  const [confirmEmpty, setConfirmEmpty] = useState(false);

  const load = useCallback(async () => {
    if (!isTauri()) return;
    setLoading(true);
    try {
      setItems(await trashApi.list());
    } catch (err) {
      console.error('list_trash failed:', err);
      showToast('error', t('settings.trash.loadFailed'), String(err));
    } finally {
      setLoading(false);
    }
  }, [showToast, t]);

  useEffect(() => {
    void load();
  }, [load]);

  const handleRestore = async (item: TrashItem) => {
    setWorkingId(item.trash_id);
    try {
      const report = await trashApi.restore([item.trash_id]);
      if (report.errors.length > 0) {
        showToast(
          'warning',
          t('settings.trash.restorePartial', {
            restored: report.restored.length,
            failed: report.errors.length,
          })
        );
      } else if (report.restored.length === 0) {
        showToast('warning', t('settings.trash.restoreSkipped'));
      } else {
        showToast('success', t('settings.trash.restoreDone', { name: item.name }));
      }
      await load();
    } catch (err) {
      console.error('restore_trash failed:', err);
      showToast('error', t('settings.trash.restoreFailed'), String(err));
    } finally {
      setWorkingId(null);
    }
  };

  const handlePurge = async (trashId: string) => {
    setWorkingId(trashId);
    try {
      await trashApi.purge(trashId);
      showToast('success', t('settings.trash.purgeDone'));
      setConfirmPurgeId(null);
      await load();
    } catch (err) {
      console.error('purge_trash_entry failed:', err);
      showToast('error', t('settings.trash.purgeFailed'), String(err));
    } finally {
      setWorkingId(null);
    }
  };

  const handleEmpty = async () => {
    setWorkingId('__all__');
    try {
      const count = await trashApi.empty();
      showToast('success', t('settings.trash.emptyDone', { count }));
      setConfirmEmpty(false);
      await load();
    } catch (err) {
      console.error('empty_trash failed:', err);
      showToast('error', t('settings.trash.emptyFailed'), String(err));
    } finally {
      setWorkingId(null);
    }
  };

  const formatTime = (value: string) => {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return date.toLocaleString();
  };

  return (
    <div className="bg-[var(--bg-card)] dark:bg-dark-bg-card rounded-2xl p-5 shadow-sm border border-[var(--border-color)] dark:border-dark-border">
      <div className="mb-4 flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
            <Icon name="inbox" className="text-2xl text-slate-600 dark:text-gray-300" />
            {t('settings.trash.title')}
            {items.length > 0 && (
              <span className="text-[10px] font-bold px-1.5 py-0.5 rounded bg-slate-200 dark:bg-dark-bg-tertiary text-slate-700 dark:text-gray-300">
                {items.length}
              </span>
            )}
          </h3>
          <p className="text-sm text-slate-500 dark:text-gray-400 mt-1">
            {t('settings.trash.description')}
          </p>
        </div>
        <div className="flex flex-shrink-0 items-center gap-1.5">
          <button
            onClick={() => void load()}
            disabled={loading}
            className="px-2.5 py-1.5 text-xs font-medium rounded-lg border border-[var(--border-color)] dark:border-dark-border text-slate-600 dark:text-gray-300 hover:bg-[var(--hover-bg)] dark:hover:bg-dark-hover disabled:opacity-50 transition-colors flex items-center gap-1"
          >
            <Icon name="refresh" className="text-sm" />
            {t('settings.trash.refresh')}
          </button>
          {items.length > 0 &&
            (confirmEmpty ? (
              <>
                <button
                  onClick={() => void handleEmpty()}
                  disabled={workingId === '__all__'}
                  className="px-2.5 py-1.5 text-xs font-medium rounded-lg bg-red-600 text-white hover:bg-red-700 disabled:opacity-50 transition-colors"
                >
                  {t('settings.trash.emptyConfirm')}
                </button>
                <button
                  onClick={() => setConfirmEmpty(false)}
                  className="px-2.5 py-1.5 text-xs font-medium rounded-lg text-slate-600 dark:text-gray-300 hover:bg-[var(--hover-bg)] dark:hover:bg-dark-hover transition-colors"
                >
                  {t('common.cancel')}
                </button>
              </>
            ) : (
              <button
                onClick={() => setConfirmEmpty(true)}
                className="px-2.5 py-1.5 text-xs font-medium rounded-lg text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/40 transition-colors flex items-center gap-1"
              >
                <Icon name="delete" className="text-sm" />
                {t('settings.trash.empty')}
              </button>
            ))}
        </div>
      </div>

      {items.length === 0 ? (
        <p className="rounded-xl border border-dashed border-[var(--border-color)] dark:border-dark-border p-4 text-xs text-slate-500 dark:text-gray-400">
          {loading ? t('common.loading') : t('settings.trash.emptyHint')}
        </p>
      ) : (
        <ul className="space-y-2">
          {items.map(item => {
            const busy = workingId === item.trash_id;
            const confirming = confirmPurgeId === item.trash_id;
            return (
              <li
                key={item.trash_id}
                className="rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-[var(--shell-surface-soft)] dark:bg-dark-bg-secondary p-3"
              >
                <div className="flex items-start gap-2">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-1.5">
                      <span className="text-sm font-bold text-slate-700 dark:text-gray-200 truncate">
                        {item.name}
                      </span>
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-slate-200 dark:bg-dark-bg-tertiary text-slate-600 dark:text-gray-400">
                        {t(`settings.trash.reason.${item.reason}`, { defaultValue: item.reason })}
                      </span>
                      {item.stale && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-100 dark:bg-amber-500/20 text-amber-700 dark:text-amber-300">
                          {t('settings.trash.stale')}
                        </span>
                      )}
                    </div>
                    <p className="mt-1 text-[11px] text-slate-500 dark:text-gray-400">
                      {formatTime(item.deleted_at)} · {t('settings.trash.entries', { count: item.entries.length })}
                    </p>
                    <p className="mt-0.5 text-[10px] font-mono text-slate-400 dark:text-gray-500 truncate">
                      {item.entries[0]?.original_path}
                    </p>
                  </div>
                  <div className="flex flex-shrink-0 items-center gap-1.5">
                    <button
                      onClick={() => void handleRestore(item)}
                      disabled={busy || item.stale}
                      title={item.stale ? t('settings.trash.staleHint') : undefined}
                      className="px-2.5 py-1 text-xs font-medium rounded-lg bg-[var(--accent-primary)] text-white hover:bg-[var(--accent-primary-strong)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors flex items-center gap-1"
                    >
                      <Icon name="history" className="text-sm" />
                      {t('settings.trash.restore')}
                    </button>
                    {confirming ? (
                      <>
                        <button
                          onClick={() => void handlePurge(item.trash_id)}
                          disabled={busy}
                          className="px-2.5 py-1 text-xs font-medium rounded-lg bg-red-600 text-white hover:bg-red-700 disabled:opacity-40 transition-colors"
                        >
                          {t('settings.trash.purgeConfirm')}
                        </button>
                        <button
                          onClick={() => setConfirmPurgeId(null)}
                          className="px-2 py-1 text-xs rounded-lg text-slate-500 dark:text-gray-400 hover:bg-[var(--hover-bg)] dark:hover:bg-dark-hover transition-colors"
                        >
                          {t('common.cancel')}
                        </button>
                      </>
                    ) : (
                      <button
                        onClick={() => setConfirmPurgeId(item.trash_id)}
                        disabled={busy}
                        className="px-2.5 py-1 text-xs font-medium rounded-lg text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/40 disabled:opacity-40 transition-colors flex items-center gap-1"
                      >
                        <Icon name="delete" className="text-sm" />
                        {t('settings.trash.purge')}
                      </button>
                    )}
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
