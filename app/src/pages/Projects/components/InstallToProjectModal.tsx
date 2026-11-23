import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import { skillsApi } from '@/api/tauri';
import type { ProjectConfig, SkillMetadata } from '@/types';
import { DEFAULT_PROJECT_SKILL_DIR, PROJECT_SKILL_DIRS } from '../constants';

interface InstallToProjectModalProps {
  open: boolean;
  project: ProjectConfig;
  existingIds: string[];
  onClose: () => void;
  onInstall: (skillId: string, targetDir: string) => Promise<void>;
}

/** 从全局 skill 列表选择并安装到项目目录。 */
export function InstallToProjectModal({
  open,
  project,
  existingIds,
  onClose,
  onInstall,
}: InstallToProjectModalProps) {
  const { t } = useTranslation();
  const [skills, setSkills] = useState<SkillMetadata[]>([]);
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const [targetDir, setTargetDir] = useState<string>(DEFAULT_PROJECT_SKILL_DIR);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    setError(null);
    setSelected(null);
    skillsApi
      .list()
      .then(setSkills)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [open]);

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return skills
      .filter((skill) => !existingIds.includes(skill.id))
      .filter(
        (skill) =>
          !q ||
          skill.name.toLowerCase().includes(q) ||
          skill.description.toLowerCase().includes(q),
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [skills, existingIds, query]);

  if (!open) return null;

  const handleInstall = async () => {
    if (!selected) return;
    setBusy(true);
    setError(null);
    try {
      await onInstall(selected, targetDir);
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
        className="w-full max-w-2xl max-h-[80vh] flex flex-col rounded-2xl bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-5 pt-5 pb-3 border-b border-[var(--border-color)] dark:border-dark-border">
          <h3 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">
            {t('projects.installTitle')}
          </h3>
          <p className="mt-1 text-sm text-[var(--text-secondary)] dark:text-gray-400">
            {t('projects.installDesc', { project: project.name })}
          </p>
        </div>

        <div className="px-5 py-3 flex flex-col gap-3">
          <div className="relative">
            <Icon
              name="search"
              className="absolute left-3 top-1/2 -translate-y-1/2 text-base text-[var(--text-tertiary)]"
            />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t('projects.searchSkill')}
              className="w-full pl-9 pr-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border bg-transparent text-sm text-[var(--text-primary)] dark:text-white outline-none focus:border-[var(--accent-primary)]"
            />
          </div>
          <label className="flex items-center gap-3 text-sm">
            <span className="font-semibold text-[var(--text-secondary)]">{t('projects.targetDir')}</span>
            <select
              value={targetDir}
              onChange={(e) => setTargetDir(e.target.value)}
              className="px-3 py-1.5 rounded-lg border border-[var(--border-color)] dark:border-dark-border bg-[var(--bg-card)] dark:bg-dark-bg-card text-[var(--text-primary)] dark:text-white text-sm outline-none"
            >
              {PROJECT_SKILL_DIRS.map(({ dir, agent }) => (
                <option key={dir} value={dir}>
                  {dir} ({agent})
                </option>
              ))}
            </select>
          </label>
        </div>

        <div className="flex-1 overflow-y-auto px-5 pb-3 space-y-1">
          {loading && <p className="text-sm text-[var(--text-tertiary)]">{t('common.loading')}</p>}
          {!loading && visible.length === 0 && (
            <p className="text-sm text-[var(--text-tertiary)]">{t('projects.noAvailableSkills')}</p>
          )}
          {visible.map((skill) => {
            const active = selected === skill.id;
            return (
              <button
                key={skill.id}
                type="button"
                onClick={() => setSelected(skill.id)}
                className={`w-full text-left px-3 py-2 rounded-xl border transition-colors ${
                  active
                    ? 'border-[var(--accent-primary)] bg-[var(--accent-primary-soft)]'
                    : 'border-transparent hover:bg-black/5 dark:hover:bg-white/5'
                }`}
              >
                <div className="flex items-center gap-2">
                  <Icon
                    name={active ? 'check_circle' : 'radio_button_unchecked'}
                    className={`text-base ${active ? 'text-[var(--accent-primary)]' : 'text-[var(--text-tertiary)]'}`}
                  />
                  <span className="font-semibold text-sm text-[var(--text-primary)] dark:text-white">
                    {skill.name}
                  </span>
                  <span className="ml-auto text-[10px] font-mono text-[var(--text-tertiary)]">
                    {skill.primary}
                  </span>
                </div>
                {skill.description && (
                  <p className="mt-0.5 pl-6 text-xs text-[var(--text-secondary)] dark:text-gray-400 line-clamp-2">
                    {skill.description}
                  </p>
                )}
              </button>
            );
          })}
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
            disabled={!selected || busy}
            onClick={() => void handleInstall()}
            className={`px-4 py-2 rounded-xl text-sm font-semibold text-white transition-opacity ${
              !selected || busy
                ? 'bg-[var(--accent-primary)] opacity-50 cursor-not-allowed'
                : 'bg-[var(--accent-primary)] hover:opacity-90'
            }`}
          >
            {t('projects.installConfirm')}
          </button>
        </div>
      </div>
    </div>
  );
}
