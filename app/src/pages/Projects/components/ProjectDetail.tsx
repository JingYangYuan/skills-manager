import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import { useToast } from '@/components/Toast';
import { projectsApi } from '@/api/tauri';
import type { ProjectConfig, ProjectSkill } from '@/types';
import { useProjectSkills } from '../hooks/useProjectSkills';
import { ProjectSkillCard } from './ProjectSkillCard';
import { InstallToProjectModal } from './InstallToProjectModal';
import { ProjectMcpPanel } from './ProjectMcpPanel';

interface ProjectDetailProps {
  project: ProjectConfig;
  onRemove: (project: ProjectConfig) => void;
}

interface ConfirmState {
  title: string;
  description: string;
  confirmLabel: string;
  danger?: boolean;
  action: () => Promise<void>;
}

/** 右侧项目详情：头部信息 + skill 网格。 */
export function ProjectDetail({ project, onRemove }: ProjectDetailProps) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { skills, loading, error, refresh, installSkill, importSkill, deleteSkill } =
    useProjectSkills(project.id);
  const [activeTab, setActiveTab] = useState<'skills' | 'mcp'>('skills');
  const [installOpen, setInstallOpen] = useState(false);
  const [confirm, setConfirm] = useState<ConfirmState | null>(null);
  const [busy, setBusy] = useState(false);

  const openSkill = async (skill: ProjectSkill) => {
    try {
      await projectsApi.openPath(project.id, skill.rel_dir);
    } catch (e) {
      showToast('error', t('projects.actionFailed'), String(e));
    }
  };

  const handleImport = async (skill: ProjectSkill) => {
    try {
      const name = await importSkill(skill.path);
      showToast('success', t('projects.imported', { name }));
    } catch (e) {
      showToast('error', t('projects.actionFailed'), String(e));
    }
  };

  const requestDelete = (skill: ProjectSkill) => {
    if (skill.is_managed) {
      setConfirm({
        title: t('projects.deleteLink'),
        description: t('projects.deleteLinkConfirm', { name: skill.name }),
        confirmLabel: t('common.delete'),
        danger: true,
        action: () => deleteSkill(skill.path, 'link'),
      });
      return;
    }
    setConfirm({
      title: t('projects.moveToTrash'),
      description: t('projects.moveToTrashConfirm', { name: skill.name }),
      confirmLabel: t('projects.moveToTrash'),
      danger: true,
      action: () => deleteSkill(skill.path, 'trash'),
    });
  };

  const runConfirm = async () => {
    if (!confirm) return;
    setBusy(true);
    try {
      await confirm.action();
      showToast('success', t('projects.deleted'));
      setConfirm(null);
    } catch (e) {
      showToast('error', t('projects.actionFailed'), String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="h-full flex flex-col">
      <div className="px-6 pt-6 pb-4 border-b border-[var(--border-color)] dark:border-dark-border">
        <div className="flex items-start gap-4">
          <div className="w-12 h-12 rounded-2xl bg-[var(--accent-primary-soft)] flex items-center justify-center shrink-0">
            <Icon name="folder_shared" className="text-2xl text-[var(--accent-primary)]" />
          </div>
          <div className="min-w-0 flex-1">
            <h2 className="text-2xl font-black text-[var(--text-primary)] dark:text-white truncate">
              {project.name}
            </h2>
            <p className="text-xs font-mono text-[var(--text-tertiary)] truncate" title={project.path}>
              {project.path}
            </p>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <button
              type="button"
              onClick={() => projectsApi.openPath(project.id).catch((e) => showToast('error', t('projects.actionFailed'), String(e)))}
              className="flex items-center gap-1.5 px-3 py-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border text-sm font-semibold text-[var(--text-secondary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
            >
              <Icon name="folder_open" className="text-base" />
              {t('projects.openFolder')}
            </button>
            <button
              type="button"
              onClick={() => onRemove(project)}
              title={t('projects.remove')}
              className="p-2 rounded-xl border border-[var(--border-color)] dark:border-dark-border text-[var(--text-tertiary)] hover:text-red-500 hover:border-red-200 dark:hover:border-red-500/30 transition-colors"
            >
              <Icon name="delete" className="text-base" />
            </button>
          </div>
        </div>
      </div>

      <div className="px-6 pt-3 flex items-center gap-1 border-b border-[var(--border-color)] dark:border-dark-border">
        {([
          ['skills', t('projects.skillsTab'), 'extension'],
          ['mcp', t('projects.mcpTab'), 'hub'],
        ] as const).map(([id, label, icon]) => (
          <button
            key={id}
            type="button"
            onClick={() => setActiveTab(id)}
            className={`flex items-center gap-1.5 px-3 py-2 text-sm font-semibold border-b-2 -mb-px transition-colors ${
              activeTab === id
                ? 'border-[var(--accent-primary)] text-[var(--accent-primary)]'
                : 'border-transparent text-[var(--text-tertiary)] hover:text-[var(--text-primary)]'
            }`}
          >
            <Icon name={icon} className="text-base" />
            {label}
          </button>
        ))}
      </div>

      {activeTab === 'mcp' ? (
        <div className="flex-1 overflow-y-auto px-6 py-5">
          <ProjectMcpPanel projectId={project.id} />
        </div>
      ) : (
      <div className="flex-1 overflow-y-auto px-6 py-5">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
              {t('projects.skillsTab')}
            </h3>
            <span className="text-xs font-bold px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/10 text-[var(--text-secondary)]">
              {skills.length}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => void refresh()}
              className="p-2 rounded-lg text-[var(--text-tertiary)] hover:text-[var(--text-primary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
              title={t('common.refresh')}
            >
              <Icon name="refresh" className="text-base" />
            </button>
            <button
              type="button"
              onClick={() => setInstallOpen(true)}
              className="flex items-center gap-1.5 px-3 py-2 rounded-xl bg-[var(--accent-primary)] text-white text-sm font-semibold hover:opacity-90 active:scale-[0.99] transition-all"
            >
              <Icon name="add" className="text-base" />
              {t('projects.installSkill')}
            </button>
          </div>
        </div>

        {error && (
          <div className="mb-4 px-3 py-2 rounded-xl bg-red-50 dark:bg-red-500/10 text-sm text-red-600 dark:text-red-400">
            {error}
          </div>
        )}

        {loading ? (
          <p className="text-sm text-[var(--text-tertiary)]">{t('common.loading')}</p>
        ) : skills.length === 0 ? (
          <div className="flex flex-col items-center justify-center text-center py-16 rounded-3xl border border-dashed border-[var(--border-color)] dark:border-dark-border">
            <Icon name="inbox" className="text-4xl text-[var(--text-tertiary)] mb-3" />
            <h4 className="font-bold text-[var(--text-primary)] dark:text-white">
              {t('projects.noSkillsTitle')}
            </h4>
            <p className="mt-1 max-w-sm text-sm text-[var(--text-secondary)] dark:text-gray-400">
              {t('projects.noSkillsDesc')}
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-3">
            {skills.map((skill) => (
              <ProjectSkillCard
                key={`${skill.rel_dir}:${skill.id}`}
                skill={skill}
                onOpen={openSkill}
                onImport={handleImport}
                onDelete={requestDelete}
              />
            ))}
          </div>
        )}
      </div>
      )}

      <InstallToProjectModal
        open={installOpen}
        project={project}
        existingIds={skills.map((skill) => skill.id)}
        onClose={() => setInstallOpen(false)}
        onInstall={async (skillId, targetDir) => {
          const path = await installSkill(skillId, targetDir);
          showToast('success', t('projects.installed', { path }));
        }}
      />

      {confirm && (
        <div className="fixed inset-0 z-[9999] bg-black/30 backdrop-blur-sm flex items-center justify-center p-6" onClick={() => !busy && setConfirm(null)}>
          <div
            className="w-full max-w-md rounded-2xl bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-2xl p-5"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">{confirm.title}</h3>
            <p className="mt-2 text-sm text-[var(--text-secondary)] dark:text-gray-400">{confirm.description}</p>
            <div className="mt-5 flex justify-end gap-2">
              <button
                type="button"
                disabled={busy}
                onClick={() => setConfirm(null)}
                className="px-3 py-2 rounded-xl text-sm font-semibold text-[var(--text-secondary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => void runConfirm()}
                className={`px-3 py-2 rounded-xl text-sm font-semibold text-white transition-opacity ${
                  confirm.danger ? 'bg-red-500 hover:bg-red-600' : 'bg-[var(--accent-primary)] hover:opacity-90'
                } ${busy ? 'opacity-60 cursor-wait' : ''}`}
              >
                {confirm.confirmLabel}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
