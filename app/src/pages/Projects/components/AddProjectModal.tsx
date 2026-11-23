import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import { projectsApi } from '@/api/tauri';
import type { ProjectCandidate, ProjectConfig } from '@/types';

interface AddProjectModalProps {
  open: boolean;
  onClose: () => void;
  onAdd: (path: string, name?: string) => Promise<ProjectConfig>;
  onAdded: (project: ProjectConfig) => void;
}

/** 添加项目：手动输入路径，或从自动发现列表一键注册。 */
export function AddProjectModal({ open, onClose, onAdd, onAdded }: AddProjectModalProps) {
  const { t } = useTranslation();
  const [path, setPath] = useState('');
  const [name, setName] = useState('');
  const [candidates, setCandidates] = useState<ProjectCandidate[]>([]);
  const [loadingCandidates, setLoadingCandidates] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setError(null);
    setLoadingCandidates(true);
    projectsApi
      .detectCandidates()
      .then((list) => setCandidates(list.filter((candidate) => !candidate.registered)))
      .catch(() => setCandidates([]))
      .finally(() => setLoadingCandidates(false));
  }, [open]);

  if (!open) return null;

  const register = async (targetPath: string, targetName?: string) => {
    setBusy(true);
    setError(null);
    try {
      const project = await onAdd(targetPath, targetName);
      onAdded(project);
      setPath('');
      setName('');
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-[9999] bg-black/30 backdrop-blur-sm flex items-center justify-center p-6"
      onClick={() => !busy && onClose()}
    >
      <div
        className="w-full max-w-xl max-h-[80vh] flex flex-col rounded-2xl bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-5 pt-5 pb-3 border-b border-[var(--border-color)] dark:border-dark-border">
          <h3 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">
            {t('projects.addTitle')}
          </h3>
          <p className="mt-1 text-sm text-[var(--text-secondary)] dark:text-gray-400">
            {t('projects.addDesc')}
          </p>
        </div>

        <div className="px-5 py-4 space-y-3">
          <label className="block">
            <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
              {t('projects.pathLabel')}
            </span>
            <input
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder={t('projects.pathPlaceholder')}
              className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm font-mono text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)]"
            />
          </label>
          <label className="block">
            <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
              {t('projects.nameLabel')}
            </span>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t('projects.namePlaceholder')}
              className="mt-1 w-full px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)]"
            />
          </label>
        </div>

        <div className="flex-1 overflow-y-auto px-5 pb-3">
          <div className="flex items-center gap-2 mb-2">
            <Icon name="search" className="text-base text-[var(--text-tertiary)]" />
            <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
              {t('projects.candidatesTitle')}
            </span>
          </div>
          {loadingCandidates && (
            <p className="text-sm text-[var(--text-tertiary)]">{t('projects.detecting')}</p>
          )}
          {!loadingCandidates && candidates.length === 0 && (
            <p className="text-sm text-[var(--text-tertiary)]">{t('projects.candidatesEmpty')}</p>
          )}
          <div className="space-y-1">
            {candidates.map((candidate) => (
              <div
                key={candidate.path}
                className="flex items-center gap-3 px-3 py-2 rounded-xl hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
              >
                <Icon name="folder_shared" className="text-lg text-[var(--text-tertiary)] shrink-0" />
                <div className="min-w-0 flex-1">
                  <p className="font-semibold text-sm text-[var(--text-primary)] dark:text-white truncate">
                    {candidate.name}
                  </p>
                  <p className="text-[11px] font-mono text-[var(--text-tertiary)] truncate" title={candidate.path}>
                    {candidate.path}
                  </p>
                </div>
                <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)] whitespace-nowrap">
                  {candidate.skill_dirs.length > 0
                    ? t('projects.candidateHasSkills', { count: candidate.skill_dirs.length })
                    : t('projects.candidateNoSkills')}
                </span>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void register(candidate.path)}
                  className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-[var(--accent-primary)] text-white hover:opacity-90 disabled:opacity-50 transition-opacity"
                >
                  {t('projects.register')}
                </button>
              </div>
            ))}
          </div>
        </div>

        {error && (
          <div className="mx-5 mb-3 px-3 py-2 rounded-xl bg-red-50 dark:bg-red-500/10 text-xs text-red-600 dark:text-red-400">
            {error}
          </div>
        )}

        <div className="px-5 py-4 border-t border-[var(--border-color)] dark:border-dark-border flex justify-end gap-2">
          <button
            type="button"
            disabled={busy}
            onClick={onClose}
            className="px-3 py-2 rounded-xl text-sm font-semibold text-[var(--text-secondary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
          >
            {t('common.cancel')}
          </button>
          <button
            type="button"
            disabled={busy || !path.trim()}
            onClick={() => void register(path.trim(), name.trim() || undefined)}
            className={`px-4 py-2 rounded-xl text-sm font-semibold text-white transition-opacity ${
              busy || !path.trim()
                ? 'bg-[var(--accent-primary)] opacity-50 cursor-not-allowed'
                : 'bg-[var(--accent-primary)] hover:opacity-90'
            }`}
          >
            {t('projects.addConfirm')}
          </button>
        </div>
      </div>
    </div>
  );
}
