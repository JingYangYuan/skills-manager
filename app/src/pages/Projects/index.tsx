import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import PageHeader from '@/components/PageHeader';
import { Icon } from '@/components/Icon';
import { useToast } from '@/components/Toast';
import type { ProjectConfig } from '@/types';
import { useProjects } from './hooks/useProjects';
import { ProjectSidebar } from './components/ProjectSidebar';
import { ProjectDetail } from './components/ProjectDetail';
import { AddProjectModal } from './components/AddProjectModal';

/** 项目级 skill 管理页面。 */
export default function Projects() {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { projects, loading, error, refresh, addProject, removeProject } = useProjects();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [pendingRemove, setPendingRemove] = useState<ProjectConfig | null>(null);
  const [removing, setRemoving] = useState(false);

  // 首次加载 / 列表变化时，保证选中项有效。
  useEffect(() => {
    if (projects.length === 0) {
      if (selectedId !== null) setSelectedId(null);
      return;
    }
    if (!selectedId || !projects.some((project) => project.id === selectedId)) {
      setSelectedId(projects[0].id);
    }
  }, [projects, selectedId]);

  const selected = projects.find((project) => project.id === selectedId) ?? null;

  const confirmRemove = async () => {
    if (!pendingRemove) return;
    setRemoving(true);
    try {
      await removeProject(pendingRemove.id);
      showToast('success', t('projects.removed', { name: pendingRemove.name }));
      setPendingRemove(null);
    } catch (e) {
      showToast('error', t('projects.actionFailed'), String(e));
    } finally {
      setRemoving(false);
    }
  };

  return (
    <div className="h-full flex flex-col bg-[var(--bg-primary)] dark:bg-dark-bg-secondary">
      <PageHeader
        icon="folder_shared"
        title={t('projects.title')}
        actions={
          <button
            type="button"
            onClick={() => setAddOpen(true)}
            className="flex items-center gap-1.5 px-3 py-2 rounded-xl bg-[var(--accent-primary)] text-white text-sm font-semibold hover:opacity-90 active:scale-[0.99] transition-all"
          >
            <Icon name="add" className="text-base" />
            {t('projects.add')}
          </button>
        }
      />

      <div className="flex-1 flex overflow-hidden">
        <ProjectSidebar
          projects={projects}
          selectedId={selectedId}
          loading={loading}
          onSelect={setSelectedId}
          onAdd={() => setAddOpen(true)}
          onRefresh={() => void refresh()}
        />

        <main className="flex-1 min-w-0 overflow-hidden">
          {selected ? (
            <ProjectDetail project={selected} onRemove={setPendingRemove} />
          ) : (
            <div className="h-full flex flex-col items-center justify-center text-center px-8">
              <div className="w-16 h-16 rounded-3xl bg-[var(--accent-primary-soft)] flex items-center justify-center mb-4">
                <Icon name="folder_shared" className="text-3xl text-[var(--accent-primary)]" />
              </div>
              <h2 className="text-xl font-black text-[var(--text-primary)] dark:text-white">
                {t('projects.emptyTitle')}
              </h2>
              <p className="mt-2 max-w-md text-sm text-[var(--text-secondary)] dark:text-gray-400">
                {t('projects.emptyDesc')}
              </p>
              {error && (
                <p className="mt-3 text-xs text-red-500">{error}</p>
              )}
              <button
                type="button"
                onClick={() => setAddOpen(true)}
                className="mt-5 flex items-center gap-2 px-4 py-2 rounded-xl bg-[var(--accent-primary)] text-white font-semibold text-sm hover:opacity-90 transition-opacity"
              >
                <Icon name="add" className="text-base" />
                {t('projects.add')}
              </button>
            </div>
          )}
        </main>
      </div>

      <AddProjectModal
        open={addOpen}
        onClose={() => setAddOpen(false)}
        onAdd={addProject}
        onAdded={(project) => setSelectedId(project.id)}
      />

      {pendingRemove && (
        <div
          className="fixed inset-0 z-[9999] bg-black/30 backdrop-blur-sm flex items-center justify-center p-6"
          onClick={() => !removing && setPendingRemove(null)}
        >
          <div
            className="w-full max-w-md rounded-2xl bg-[var(--bg-card)] dark:bg-dark-bg-card shadow-2xl p-5"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-lg font-bold text-[var(--text-primary)] dark:text-white">
              {t('projects.remove')}
            </h3>
            <p className="mt-2 text-sm text-[var(--text-secondary)] dark:text-gray-400">
              {t('projects.removeConfirm', { name: pendingRemove.name })}
            </p>
            <div className="mt-5 flex justify-end gap-2">
              <button
                type="button"
                disabled={removing}
                onClick={() => setPendingRemove(null)}
                className="px-3 py-2 rounded-xl text-sm font-semibold text-[var(--text-secondary)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                disabled={removing}
                onClick={() => void confirmRemove()}
                className={`px-3 py-2 rounded-xl text-sm font-semibold text-white bg-red-500 hover:bg-red-600 transition-opacity ${
                  removing ? 'opacity-60 cursor-wait' : ''
                }`}
              >
                {t('projects.remove')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
