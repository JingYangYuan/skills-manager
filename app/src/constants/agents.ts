import { OCTOPUS_LOGO_URL } from '@/lib/assets';
import claudeIcon from '@/assets/agents/claude.svg';
import cursorIcon from '@/assets/agents/cursor.svg';
import gptIcon from '@/assets/agents/GPT.svg';
import openclawIcon from '@/assets/agents/openclaw.svg';
import opencodeIcon from '@/assets/agents/opencode.svg';
import traeIcon from '@/assets/agents/trae.svg';
import qoderIcon from '@/assets/agents/qoder.svg';
import antigravityIcon from '@/assets/agents/antigravity.svg';
import kiroIcon from '@/assets/agents/kiro.svg';
import copilotIcon from '@/assets/agents/copilot.svg';
import workbuddyIcon from '@/assets/agents/workbuddy.svg';
import zcodeIcon from '@/assets/agents/zcode.svg';
import kimiIcon from '@/assets/agents/kimi.svg';
import marscodeIcon from '@/assets/agents/marscode.svg';
import openclaudeIcon from '@/assets/agents/openclaude.svg';
import hermesIcon from '@/assets/agents/hermes.svg';
import grokIcon from '@/assets/agents/grok.svg';
import piIcon from '@/assets/agents/pi.svg';
import ompIcon from '@/assets/agents/omp.svg';

/**
 * 前端维护的 agent 元信息 —— 单一数据源。
 *
 * 新增/调整 agent 时只改动这里，所有依赖（Settings 列表、Dashboard 路径、
 * Detail Modal 全名、Source 短名、图标）都会自动同步。
 *
 * 字段说明：
 * - `name`         : agent 的机读名，与后端 SkillSource / agent key 一致。
 * - `shortName`    : 用于 badge / tab 等狭窄位置的简短名称。
 * - `displayName`  : 用于卡片/详情的完整名称，可能包含品牌后缀（如 "Claude Code"）。
 * - `rootPath`     : 用户主目录下的根目录，打开文件夹时使用。
 * - `icon`         : agent 图标资源（svg import 后的 URL）。
 * - `invertInDark` : 深色模式下是否需要反色（白图反成黑图）。
 */
interface AgentMeta {
  name: string;
  shortName: string;
  displayName: string;
  rootPath: string;
  icon: string;
  invertInDark?: boolean;
}

const AGENT_META = {
  claude: {
    name: 'claude',
    shortName: 'Claude',
    displayName: 'Claude Code',
    rootPath: '~/.claude',
    icon: claudeIcon,
  },
  cursor: {
    name: 'cursor',
    shortName: 'Cursor',
    displayName: 'Cursor',
    rootPath: '~/.cursor',
    icon: cursorIcon,
  },
  antigravity: {
    name: 'antigravity',
    shortName: 'Antigravity',
    displayName: 'Google Antigravity',
    rootPath: '~/.gemini',
    icon: antigravityIcon,
  },
  codex: {
    name: 'codex',
    shortName: 'Codex',
    displayName: 'Codex',
    rootPath: '~/.codex',
    icon: gptIcon,
    invertInDark: true,
  },
  copilot: {
    name: 'copilot',
    shortName: 'Copilot',
    displayName: 'GitHub Copilot',
    rootPath: '~/.copilot',
    icon: copilotIcon,
    invertInDark: true,
  },
  workbuddy: {
    name: 'workbuddy',
    shortName: 'WorkBuddy',
    displayName: 'WorkBuddy',
    rootPath: '~/.workbuddy',
    icon: workbuddyIcon,
  },
  zcode: {
    name: 'zcode',
    shortName: 'ZCode',
    displayName: 'ZCode',
    rootPath: '~/.zcode',
    icon: zcodeIcon,
  },
  openclaw: {
    name: 'openclaw',
    shortName: 'OpenClaw',
    displayName: 'OpenClaw',
    rootPath: '~/.openclaw',
    icon: openclawIcon,
  },
  opencode: {
    name: 'opencode',
    shortName: 'OpenCode',
    displayName: 'OpenCode',
    rootPath: '~/.config/opencode',
    icon: opencodeIcon,
    invertInDark: true,
  },
  kimi: {
    name: 'kimi',
    shortName: 'Kimi',
    displayName: 'Kimi Code',
    rootPath: '~/.kimi-code',
    icon: kimiIcon,
  },
  marscode: {
    name: 'marscode',
    shortName: 'MarsCode',
    displayName: 'MarsCode',
    rootPath: '~/.marscode',
    icon: marscodeIcon,
  },
  openclaude: {
    name: 'openclaude',
    shortName: 'OpenClaude',
    displayName: 'OpenClaude',
    rootPath: '~/.openclaude',
    icon: openclaudeIcon,
  },
  hermes: {
    name: 'hermes',
    shortName: 'Hermes',
    displayName: 'Hermes Agent',
    rootPath: '~/.hermes',
    icon: hermesIcon,
  },
  grok: {
    name: 'grok',
    shortName: 'Grok',
    displayName: 'Grok',
    rootPath: '~/.grok',
    icon: grokIcon,
    invertInDark: true,
  },
  trae: {
    name: 'trae',
    shortName: 'Trae',
    displayName: 'Trae',
    rootPath: '~/.trae',
    icon: traeIcon,
  },
  qoder: {
    name: 'qoder',
    shortName: 'Qoder',
    displayName: 'Qoder',
    rootPath: '~/.qoder',
    icon: qoderIcon,
  },
  kiro: {
    name: 'kiro',
    shortName: 'Kiro',
    displayName: 'Kiro',
    rootPath: '~/.kiro',
    icon: kiroIcon,
  },
  pi: {
    name: 'pi',
    shortName: 'Pi',
    displayName: 'Pi',
    rootPath: '~/.pi',
    icon: piIcon,
  },
  omp: {
    name: 'omp',
    shortName: 'OMP',
    displayName: 'OMP',
    rootPath: '~/.omp',
    icon: ompIcon,
  },
} as const satisfies Record<string, AgentMeta>;

/** 按声明顺序列出的 agent 列表，可用于渲染 Settings/Dashboard 列表 */
export const KNOWN_AGENTS: readonly AgentMeta[] = Object.values(AGENT_META);

/** 首字母头像的调色板（与内置 agent 图标风格一致的饱和色） */
const AVATAR_PALETTE = [
  '#6366F1',
  '#0EA5E9',
  '#10B981',
  '#F59E0B',
  '#EF4444',
  '#8B5CF6',
  '#EC4899',
  '#14B8A6',
  '#F97316',
  '#0D9488',
] as const;

/**
 * 为未知 agent 生成首字母头像（内联 SVG data URI）。
 * 颜色由 name 哈希决定，同一 agent 每次渲染颜色一致。
 */
export function letterAvatarDataUri(name: string): string {
  const key = name.trim() || '?';
  let hash = 0;
  for (let i = 0; i < key.length; i += 1) {
    hash = (hash * 31 + key.charCodeAt(i)) >>> 0;
  }
  const bg = AVATAR_PALETTE[hash % AVATAR_PALETTE.length];
  const letter = Array.from(key)[0]?.toUpperCase() ?? '?';
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="112" height="112">` +
    `<rect x="2" y="2" width="20" height="20" rx="6" fill="${bg}"/>` +
    `<text x="12" y="16.4" font-family="-apple-system,'Segoe UI','Microsoft YaHei',sans-serif" ` +
    `font-size="11.5" font-weight="600" fill="#FFFFFF" text-anchor="middle">${letter}</text>` +
    `</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

/** 注册动态 agent 所需的最小字段（与后端 AgentConfig 结构兼容） */
export interface DynamicAgentLike {
  name: string;
  display_name?: string;
  path?: string;
}

/**
 * 运行时注册的自定义 agent 元信息（name -> meta）。
 * 由后端 detect_agents / get_agents 的返回值同步，构建于 AGENT_META 之后的查找层。
 */
const DYNAMIC_AGENT_META = new Map<string, AgentMeta>();

/**
 * 用后端返回的 agent 列表刷新动态注册表。
 * 内置 agent（已在 AGENT_META 中）不注册；不在列表里的动态条目会被清掉，
 * 因此删除自定义 agent 后注册表自动收敛。
 */
export function registerDynamicAgents(agents: readonly DynamicAgentLike[]): void {
  const next = new Map<string, AgentMeta>();
  for (const agent of agents) {
    if (!agent?.name) continue;
    if (getBuiltinMeta(agent.name)) continue;
    const displayName = agent.display_name?.trim() || agent.name;
    next.set(agent.name, {
      name: agent.name,
      shortName: displayName,
      displayName,
      rootPath: agent.path?.trim() || '',
      icon: letterAvatarDataUri(agent.name),
    });
  }
  DYNAMIC_AGENT_META.clear();
  for (const [name, meta] of next) {
    DYNAMIC_AGENT_META.set(name, meta);
  }
}

const getBuiltinMeta = (name: string): AgentMeta | undefined =>
  (AGENT_META as Record<string, AgentMeta>)[name];

/** 内置 meta 优先，其次动态注册的自定义 agent */
const resolveMeta = (name: string): AgentMeta | undefined =>
  getBuiltinMeta(name) ?? DYNAMIC_AGENT_META.get(name);

/** 是否为用户手动添加的自定义 agent（非内置 preset） */
export const isCustomAgent = (name: string): boolean => !getBuiltinMeta(name);

/** name 是否已被识别（内置或动态注册）—— 用于 source 合法性校验 */
export const isRegisteredAgentName = (name: string): boolean =>
  name === 'global' || resolveMeta(name) !== undefined;

export const getAgentRootPath = (name: string): string =>
  resolveMeta(name)?.rootPath ?? '';

export const getAgentDisplayName = (name: string): string =>
  resolveMeta(name)?.displayName ?? name;

export const getAgentShortName = (name: string): string =>
  resolveMeta(name)?.shortName ?? name;

export const getAgentIconUrl = (name: string): string => {
  if (name === 'global') return OCTOPUS_LOGO_URL;
  return resolveMeta(name)?.icon ?? letterAvatarDataUri(name);
};

export const agentNeedsInvertInDark = (name: string): boolean =>
  resolveMeta(name)?.invertInDark === true;
