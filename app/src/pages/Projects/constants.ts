/**
 * 项目内约定的 skill 目录（与后端 `projects::PROJECT_SKILL_DIRS` 保持一致）。
 *
 * `agent` 用于图标 / 徽章映射，`dir` 是相对项目根的路径。
 */
export const PROJECT_SKILL_DIRS = [
  { dir: '.claude/skills', agent: 'claude' },
  { dir: '.agents/skills', agent: 'antigravity' },
  { dir: '.cursor/skills', agent: 'cursor' },
  { dir: '.codex/skills', agent: 'codex' },
  { dir: '.opencode/skills', agent: 'opencode' },
  { dir: '.gemini/skills', agent: 'antigravity' },
  { dir: '.grok/skills', agent: 'grok' },
  { dir: '.kilocode/skills', agent: 'kilo' },
  { dir: '.github/skills', agent: 'copilot' },
  { dir: '.workbuddy/skills', agent: 'workbuddy' },
  { dir: '.codebuddy/skills', agent: 'codebuddy' },
  { dir: '.zcode/skills', agent: 'zcode' },
  { dir: '.openclaw/skills', agent: 'openclaw' },
  { dir: '.kimi-code/skills', agent: 'kimi' },
  { dir: '.marscode/skills', agent: 'marscode' },
  { dir: '.openclaude/skills', agent: 'openclaude' },
  { dir: '.hermes/skills', agent: 'hermes' },
  { dir: '.ima/skills', agent: 'ima' },
  { dir: '.trae/skills', agent: 'trae' },
  { dir: '.qoder/skills', agent: 'qoder' },
  { dir: '.kiro/skills', agent: 'kiro' },
  { dir: '.pi/skills', agent: 'pi' },
  { dir: '.omp/skills', agent: 'omp' },
] as const;

export const DEFAULT_PROJECT_SKILL_DIR = PROJECT_SKILL_DIRS[0].dir;
