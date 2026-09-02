/**
 * 各 agent 的技能扫描目录映射。
 *
 * 与 `src-tauri/src/scanner.rs::scan_all_skill_sources` 中的硬编码保持一致：
 * - 若该文件的扫描路径调整，请同步更新此处常量。
 * - 未列出的 agent（opencode）当前无专门的扫描路径。
 */
const AGENT_SCAN_PATHS: Record<string, string[]> = {
  claude: [
    '~/.claude/skills',
    '~/.claude/plugins/cache',
  ],
  cursor: [
    '~/.cursor/skills',
    '~/.cursor/skills-cursor',
    '~/.cursor/plugins/cache/cursor-public',
  ],
  antigravity: [
    '~/.gemini/skills',
    '~/.gemini/config/skills',
    '~/.antigravity/skills',
  ],
  codex: [
    '~/.codex/skills',
  ],
  agents: [
    '~/.agents/skills',
    '~/.config/agents/skills',
  ],
  copilot: [
    '~/.copilot/skills',
  ],
  workbuddy: [
    '~/.workbuddy/skills',
    '~/WorkBuddy/skills',
  ],
  zcode: [
    '~/.zcode/skills',
    '~/ZCodeProject/skills',
  ],
  openclaw: [
    '~/.openclaw/skills',
  ],
  opencode: [
    '~/.config/opencode/skills',
    '~/.opencode/skills',
  ],
  kimi: [
    '~/.kimi-code/skills',
  ],
  marscode: [
    '~/.marscode/skills',
    '~/.marscode/builtin_skills',
  ],
  openclaude: [
    '~/.openclaude/skills',
  ],
  hermes: [
    '~/.hermes/skills',
  ],
  grok: [
    '~/.grok/skills',
  ],
  trae: [
    '~/.trae/skills',
    '~/.trae-cn/skills',
  ],
  qoder: [
    '~/.qoder/skills',
  ],
  kiro: [
    '~/.kiro/skills',
  ],
};

export const getAgentScanPaths = (name: string): string[] =>
  AGENT_SCAN_PATHS[name] ?? [];
