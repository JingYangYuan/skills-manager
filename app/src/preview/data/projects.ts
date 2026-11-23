import type { ProjectConfig, ProjectSkill } from '@/types';

/** 预览环境的示例项目。 */
export const PREVIEW_PROJECTS: ProjectConfig[] = [
  {
    id: 'paper-workspace-3f7a1c9d2e40',
    name: 'Paper Workspace',
    path: '~/Documents/paper-workspace',
    enabled: true,
    auto_detected: false,
  },
  {
    id: 'lab-dashboard-8b2d4e6f0a13',
    name: 'Lab Dashboard',
    path: '~/Documents/lab-dashboard',
    enabled: true,
    auto_detected: true,
  },
];

/** 预览环境的示例项目 skill（按 project id 分组）。 */
export const PREVIEW_PROJECT_SKILLS: Record<string, ProjectSkill[]> = {
  'paper-workspace-3f7a1c9d2e40': [
    {
      id: 'paper-writing',
      name: 'paper-writing',
      description: '中文社科论文写作与润色：章节草稿、论证检查、语言风格校准。',
      path: '~/Documents/paper-workspace/.claude/skills/paper-writing',
      rel_dir: '.claude/skills/paper-writing',
      agent: 'claude',
      is_symlink: true,
      is_managed: true,
      last_updated: '1763800000',
    },
    {
      id: 'lit-review',
      name: 'lit-review',
      description: '项目内维护的文献综述流程，包含检索词、纳入标准与证据表模板。',
      path: '~/Documents/paper-workspace/.agents/skills/lit-review',
      rel_dir: '.agents/skills/lit-review',
      agent: 'agents',
      is_symlink: false,
      is_managed: false,
      last_updated: '1763700000',
    },
  ],
  'lab-dashboard-8b2d4e6f0a13': [
    {
      id: 'data-cleaning',
      name: 'data-cleaning',
      description: '调查数据清洗：缺失值、异常值、合并键与变量字典生成。',
      path: '~/Documents/lab-dashboard/.cursor/skills/data-cleaning',
      rel_dir: '.cursor/skills/data-cleaning',
      agent: 'cursor',
      is_symlink: true,
      is_managed: true,
      last_updated: '1763600000',
    },
  ],
};
