/**
 * 项目内约定的 skill 目录（与后端 `projects::PROJECT_SKILL_DIRS` 保持一致）。
 *
 * `agent` 用于图标 / 徽章映射，`dir` 是相对项目根的路径。
 */
export const PROJECT_SKILL_DIRS = [
  { dir: '.claude/skills', agent: 'claude' },
  { dir: '.agents/skills', agent: 'agents' },
  { dir: '.cursor/skills', agent: 'cursor' },
  { dir: '.codex/skills', agent: 'codex' },
  { dir: '.opencode/skills', agent: 'opencode' },
  { dir: '.gemini/skills', agent: 'gemini' },
  { dir: '.grok/skills', agent: 'grok' },
  { dir: '.kilocode/skills', agent: 'kilo' },
] as const;

export const DEFAULT_PROJECT_SKILL_DIR = PROJECT_SKILL_DIRS[0].dir;
