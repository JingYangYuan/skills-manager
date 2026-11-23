import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import type { ProjectConfig } from '@/types';

interface ProjectSidebarProps {
  projects: ProjectConfig[];
  selectedId: string | null;
  loading: boolean;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onRefresh: () => void;
}

/** 左侧项目列表。 */
export function ProjectSidebar({
  projects,
  selectedId,
  loading,
  onSelect,
  onAdd,
  onRefresh,
}: ProjectSidebarProps) {
  const { t } = useTranslation();

  return (
    <aside className="w-72 shrink-0 h-full flex flex-col border-r border-[var(--border-color)] dark:border-dark-border bg-[var(--bg-card)]/60 dark:bg-dark-bg-card/60">
      <div className="flex items-center justify-between px-4 pt-5 pb-3">
        <span className="text-xs font-bold uppercase tracking-widest text-[var(--text-tertiary)]">
          {t('projects.listTitle')}
        </span>
        <button
          type="button"
          onClick={onRefresh}
          title={t('common.refresh')}
          className="p-1.5 rounded-lg text-[var(--text-tertiary)] hover:text-[var(--text-primary)] hover:bg-black/5 dark:hover:bg-white/10 transition-colors"
        >
          <Icon name="refresh" className="text-base" />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-2 pb-3 space-y-1">
        {loading && (
          <p className="px-3 py-2 text-sm text-[var(--text-tertiary)]">{t('common.loading')}</p>
        )}
        {!loading && projects.length === 0 && (
          <p className="px-3 py-2 text-sm text-[var(--text-tertiary)]">{t('projects.emptyTitle')}</p>
        )}
        {projects.map((project) => {
          const active = project.id === selectedId;
          return (
            <button
              key={project.id}
              type="button"
              onClick={() => onSelect(project.id)}
              className={`w-full text-left px-3 py-2.5 rounded-xl transition-colors ${
                active
                  ? 'bg-[var(--accent-primary)] text-white shadow-[0_10px_24px_-18px_rgba(255,68,88,0.9)]'
                  : 'hover:bg-black/5 dark:hover:bg-white/5 text-[var(--text-primary)]'
              }`}
            >
              <div className="flex items-center gap-2">
                <Icon name="folder_shared" className="text-lg shrink-0" />
                <span className="font-semibold truncate">{project.name}</span>
                {!project.enabled && (
                  <span className="ml-auto text-[10px] uppercase opacity-70">
                    {t('projects.disabled')}
                  </span>
                )}
              </div>
              <p
                className={`mt-0.5 text-[11px] truncate ${
                  active ? 'text-white/75' : 'text-[var(--text-tertiary)]'
                }`}
                title={project.path}
              >
                {project.path}
              </p>
            </button>
          );
        })}
      </div>

      <div className="p-3 border-t border-[var(--border-color)] dark:border-dark-border">
        <button
          type="button"
          onClick={onAdd}
          className="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-xl bg-[var(--accent-primary)] text-white font-semibold text-sm hover:opacity-90 active:scale-[0.99] transition-all"
        >
          <Icon name="add" className="text-lg" />
          {t('projects.add')}
        </button>
      </div>
    </aside>
  );
}
