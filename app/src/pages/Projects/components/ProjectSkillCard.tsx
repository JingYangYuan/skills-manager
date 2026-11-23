import { useTranslation } from 'react-i18next';
import { Icon } from '@/components/Icon';
import { getAgentDisplayName, getAgentIconUrl } from '@/constants';
import { badgeClass, sourceDisplayName } from '@/pages/Dashboard/utils/source';
import type { ProjectSkill } from '@/types';

interface ProjectSkillCardProps {
  skill: ProjectSkill;
  onOpen: (skill: ProjectSkill) => void;
  onImport: (skill: ProjectSkill) => void;
  onDelete: (skill: ProjectSkill) => void;
}

/** 项目内的单个 skill 卡片（轻量，不复用 Dashboard 的 SkillCard）。 */
export function ProjectSkillCard({ skill, onOpen, onImport, onDelete }: ProjectSkillCardProps) {
  const { t } = useTranslation();
  const agentLabel = getAgentDisplayName(skill.agent);
  const managed = skill.is_managed || skill.is_symlink;

  return (
    <div className="group flex flex-col gap-3 p-4 rounded-2xl border border-[var(--border-color)] dark:border-dark-border bg-[var(--bg-card)] dark:bg-dark-bg-card hover:shadow-[0_18px_40px_-30px_rgba(39,50,72,0.7)] transition-shadow">
      <div className="flex items-start gap-3">
        <img
          src={getAgentIconUrl(skill.agent)}
          alt={agentLabel}
          className="w-9 h-9 rounded-lg object-contain bg-white/70 dark:bg-white/5 p-1 shrink-0"
        />
        <div className="min-w-0 flex-1">
          <button
            type="button"
            onClick={() => onOpen(skill)}
            className="text-left font-bold text-[var(--text-primary)] dark:text-white truncate block max-w-full hover:underline"
            title={skill.name}
          >
            {skill.name}
          </button>
          <p className="text-[11px] text-[var(--text-tertiary)] font-mono truncate" title={skill.rel_dir}>
            {skill.rel_dir}
          </p>
        </div>
      </div>

      {skill.description && (
        <p className="text-xs leading-relaxed text-[var(--text-secondary)] dark:text-gray-400 line-clamp-3">
          {skill.description}
        </p>
      )}

      <div className="flex flex-wrap items-center gap-1.5">
        <span className={`text-[10px] font-bold px-2 py-0.5 rounded-full ${badgeClass(skill.agent)}`}>
          {sourceDisplayName(skill.agent)}
        </span>
        <span
          className={`text-[10px] font-bold px-2 py-0.5 rounded-full ${
            managed
              ? 'bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
              : 'bg-gray-100 dark:bg-gray-500/10 text-gray-600 dark:text-gray-400'
          }`}
        >
          {managed ? t('projects.managed') : t('projects.native')}
        </span>
      </div>

      <div className="mt-auto flex items-center gap-2 pt-1">
        <button
          type="button"
          onClick={() => onOpen(skill)}
          className="flex items-center gap-1 text-xs font-semibold text-[var(--text-secondary)] hover:text-[var(--text-primary)] px-2 py-1 rounded-lg hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
        >
          <Icon name="folder_open" className="text-sm" />
          {t('projects.reveal')}
        </button>
        {!skill.is_managed && (
          <button
            type="button"
            onClick={() => onImport(skill)}
            className="flex items-center gap-1 text-xs font-semibold text-[var(--text-secondary)] hover:text-[var(--text-primary)] px-2 py-1 rounded-lg hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
          >
            <Icon name="drive_folder_upload" className="text-sm" />
            {t('projects.importToRoot')}
          </button>
        )}
        <button
          type="button"
          onClick={() => onDelete(skill)}
          title={managed ? t('projects.deleteLink') : t('projects.moveToTrash')}
          className="ml-auto p-1.5 rounded-lg text-[var(--text-tertiary)] hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 transition-colors"
        >
          <Icon name="delete" className="text-base" />
        </button>
      </div>
    </div>
  );
}
