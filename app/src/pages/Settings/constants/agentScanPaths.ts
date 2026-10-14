import type { AgentConfig } from '@/types';

/**
 * 计算某个 agent 的技能扫描目录（用于 Settings 展示）。
 *
 * 与后端 `src-tauri/src/scanner.rs::collect_discoveries` 保持一致：
 * - 主路径与每个 extra_path 都会拼上 skills_path（`{root}/{skills_path}`）。
 * - cursor / claude 在 scanner.rs 中还有硬编码的插件目录额外扫描，这里同样列出。
 */
export const getAgentScanPaths = (agent: AgentConfig): string[] => {
  if (!agent) return [];

  const roots = [agent.path, ...(agent.extra_paths ?? [])].filter(Boolean);
  const paths = roots.map((root) => joinScanPath(root, agent.skills_path));

  if (agent.name === 'cursor') {
    paths.push('~/.cursor/skills-cursor', '~/.cursor/plugins/cache/cursor-public');
  } else if (agent.name === 'claude') {
    paths.push('~/.claude/plugins/cache');
  }

  return Array.from(new Set(paths));
};

const joinScanPath = (root: string, skillsPath: string): string => {
  const normalizedRoot = root.trim().replace(/\\/g, '/').replace(/\/+$/, '');
  const normalizedSkills = skillsPath.trim().replace(/\\/g, '/').replace(/^\/+|\/+$/g, '');
  return normalizedSkills ? `${normalizedRoot}/${normalizedSkills}` : normalizedRoot;
};
