import { getAgentShortName, isRegisteredAgentName } from '@/constants';

/**
 * 技能来源枚举：
 * - All: 所有来源
 * - Global: Skills Manager 根目录（`~/.skills-manager/skills`）
 * - Cursor: Cursor 原生目录
 * - Claude: Claude Code 原生目录
 * - OpenClaw: OpenClaw 原生目录
 * - Codex: Codex 原生目录
 *
 * 使用 `as const` 对象 + 派生类型，保持运行时零开销。
 */
export const SOURCE = {
  All: 'all',
  Global: 'global',
  Claude: 'claude',
  Cursor: 'cursor',
  Antigravity: 'antigravity',
  Codex: 'codex',
  Agents: 'agents',
  Copilot: 'copilot',
  WorkBuddy: 'workbuddy',
  ZCode: 'zcode',
  OpenClaw: 'openclaw',
  OpenCode: 'opencode',
  Kimi: 'kimi',
  MarsCode: 'marscode',
  OpenClaude: 'openclaude',
  Hermes: 'hermes',
  Grok: 'grok',
  Trae: 'trae',
  Qoder: 'qoder',
  Kiro: 'kiro',
  Pi: 'pi',
  Omp: 'omp',
} as const;

export type Source = typeof SOURCE[keyof typeof SOURCE];

const SOURCE_BADGE_STYLES: Record<string, string> = {
  [SOURCE.All]: 'bg-gray-50 dark:bg-gray-500/10 text-gray-600 dark:text-gray-400',
  [SOURCE.Global]: 'bg-red-50 dark:bg-red-500/10 text-red-600 dark:text-red-400',
  [SOURCE.Claude]: 'bg-orange-50 dark:bg-orange-500/10 text-orange-600 dark:text-orange-400',
  [SOURCE.Cursor]: 'bg-purple-50 dark:bg-purple-500/10 text-purple-600 dark:text-purple-400',
  [SOURCE.Antigravity]: 'bg-blue-50 dark:bg-blue-500/10 text-blue-600 dark:text-blue-400',
  [SOURCE.Codex]: 'bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
  [SOURCE.Agents]: 'bg-indigo-50 dark:bg-indigo-500/10 text-indigo-600 dark:text-indigo-400',
  [SOURCE.Copilot]: 'bg-slate-100 dark:bg-slate-500/10 text-slate-700 dark:text-slate-300',
  [SOURCE.WorkBuddy]: 'bg-sky-50 dark:bg-sky-500/10 text-sky-600 dark:text-sky-400',
  [SOURCE.ZCode]: 'bg-violet-50 dark:bg-violet-500/10 text-violet-600 dark:text-violet-400',
  [SOURCE.OpenClaw]: 'bg-[var(--accent-primary-soft)] text-[var(--accent-primary)]',
  [SOURCE.OpenCode]: 'bg-teal-50 dark:bg-teal-500/10 text-teal-600 dark:text-teal-400',
  [SOURCE.Kimi]: 'bg-cyan-50 dark:bg-cyan-500/10 text-cyan-600 dark:text-cyan-400',
  [SOURCE.MarsCode]: 'bg-amber-50 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400',
  [SOURCE.OpenClaude]: 'bg-yellow-50 dark:bg-yellow-500/10 text-yellow-600 dark:text-yellow-400',
  [SOURCE.Hermes]: 'bg-emerald-50 dark:bg-emerald-500/10 text-emerald-700 dark:text-emerald-300',
  [SOURCE.Grok]: 'bg-neutral-100 dark:bg-neutral-500/10 text-neutral-800 dark:text-neutral-200',
  [SOURCE.Trae]: 'bg-rose-50 dark:bg-rose-500/10 text-rose-600 dark:text-rose-400',
  [SOURCE.Qoder]: 'bg-fuchsia-50 dark:bg-fuchsia-500/10 text-fuchsia-600 dark:text-fuchsia-400',
  [SOURCE.Kiro]: 'bg-pink-50 dark:bg-pink-500/10 text-pink-600 dark:text-pink-400',
  [SOURCE.Pi]: 'bg-indigo-50 dark:bg-indigo-500/10 text-indigo-600 dark:text-indigo-400',
  [SOURCE.Omp]: 'bg-teal-50 dark:bg-teal-500/10 text-teal-600 dark:text-teal-400',
};

/** 自定义 agent 徽章兑底色（按 name 哈希取一个，同一 agent 颜色稳定） */
const CUSTOM_SOURCE_BADGE_STYLES = [
  'bg-indigo-50 dark:bg-indigo-500/10 text-indigo-600 dark:text-indigo-400',
  'bg-sky-50 dark:bg-sky-500/10 text-sky-600 dark:text-sky-400',
  'bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
  'bg-amber-50 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400',
  'bg-violet-50 dark:bg-violet-500/10 text-violet-600 dark:text-violet-400',
  'bg-teal-50 dark:bg-teal-500/10 text-teal-600 dark:text-teal-400',
];

const hashBadgeIndex = (src?: string): number => {
  if (!src) return 0;
  let hash = 0;
  for (let i = 0; i < src.length; i += 1) {
    hash = (hash * 31 + src.charCodeAt(i)) >>> 0;
  }
  return hash % CUSTOM_SOURCE_BADGE_STYLES.length;
};

export const sourceLabel = (s: string) => (s === SOURCE.Global ? 'Root' : s);

export const badgeClass = (src?: string) =>
  SOURCE_BADGE_STYLES[src ?? SOURCE.Global] ||
  // 未知（自定义）agent：哈希调色板兑底，避免全部落到 Global 的红色样式
  CUSTOM_SOURCE_BADGE_STYLES[hashBadgeIndex(src)] ||
  SOURCE_BADGE_STYLES[SOURCE.Global];

/**
 * Toast / 描述性文案里使用的简短展示名。
 * 非 Global 的展示名直接从 AGENT_META 派生。
 */
export const sourceDisplayName = (s?: string) => {
  if (!s || s === SOURCE.Global) return 'Root';
  if (s === SOURCE.All) return 'All';
  return getAgentShortName(s);
};

export const isSource = (v: unknown): v is string =>
  typeof v === 'string' &&
  (v === SOURCE.All || v === SOURCE.Global || isRegisteredAgentName(v));
