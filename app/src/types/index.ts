export type { Skill } from './skills';

// Skill Metadata (matches backend Schema v2)
//
// 后端按 id 合并物理副本：同一个 skill 只会返回一条记录。
// - `agent_enabled` 由 `sources`（原生自动开启）+ `open`（主动链接）派生
// - `sources` / `primary` / `open` / `source_paths` 描述多源布局
export interface SkillMetadata {
  id: string;
  name: string;
  description: string;
  category: string;
  enabled: boolean;
  agent_enabled: Record<string, boolean>;
  is_collected?: boolean;
  author?: string;
  version?: string;
  /** 技能目录体积（字节），后端可选返回 */
  size?: number;
  installed_at: string;
  last_updated: string;

  /** 物理副本所在位置（'global' 或 agent 名） */
  sources: string[];
  /** 链接到非原生 Agent 时默认用哪份副本为源 */
  primary: string;
  /** 用户主动启用的非原生 Agent */
  open: string[];
  /** 每个 source 对应的物理路径 */
  source_paths: Record<string, string>;
  /** 从 marketplace 下载时写入的来源仓库 URL */
  source_repository?: string;
}

/**
 * 多源删除场景用的行数据：一条 skill 在某个具体源下的视图。
 * DeleteConfirmModal 以此为单位展示/勾选/删除。
 */
export interface SkillDeletionRow {
  skill: SkillMetadata;
  /** 本行针对的具体 source（'global' 或 agent 名） */
  source: string;
  /** 本源对应的物理路径（从 `skill.source_paths` 取得，可能缺失） */
  path?: string;
}

// Skill file entry for directory tree
export interface SkillFileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number;
  children?: SkillFileEntry[];
}

// Agent Configuration (matches backend Phase 1)
export interface AgentConfig {
  name: string;                  // Unique identifier (e.g., "claude", "cursor")
  display_name: string;          // Human-readable name (e.g., "Claude", "Cursor")
  path: string;                  // Base path (e.g., "~/.claude")
  skills_path: string;           // Relative skills path (e.g., "skills" under agent home)
  enabled: boolean;              // Whether agent is active
  detected: boolean;             // Whether agent is installed on system
  extra_paths?: string[];        // Additional scan roots (relative to agent home or absolute)
}

// Linking strategy: how to "link" skills from Skills Manager root into agent directories.
// Backend uses PascalCase (matches Rust enum variants), keep that form for IPC compatibility.
export const LINK_TYPE = {
  Symlink: 'Symlink',
  Copy: 'Copy',
} as const;

export type LinkType = typeof LINK_TYPE[keyof typeof LINK_TYPE];

// Application Configuration (matches backend Phase 1)
export interface AppConfig {
  linking_strategy: LinkType;             // File linking strategy
  agents: AgentConfig[];                  // List of configured agents
  skill_hide_prefixes?: string[];         // Prefix rules used to hide skills in UI and tray
  pinned_skills?: string[];               // Skill ids pinned to the top of the dashboard
  projects?: ProjectConfig[];             // Registered projects (project-level skill management)
}

/** 已注册的项目（项目级 skill 管理） */
export interface ProjectConfig {
  /** 稳定 ID：slug(name) + '-' + path hash */
  id: string;
  /** 展示名，默认取目录名 */
  name: string;
  /** 项目根目录绝对路径 */
  path: string;
  /** 是否参与扫描 */
  enabled: boolean;
  /** 是否由自动发现加入 */
  auto_detected?: boolean;
}

/** 项目内扫描到的一个 skill */
export interface ProjectSkill {
  id: string;
  name: string;
  description: string;
  /** skill 目录绝对路径 */
  path: string;
  /** 相对项目根的目录，如 `.claude/skills/foo` */
  rel_dir: string;
  /** 所属约定目录对应的 agent 名 */
  agent: string;
  is_symlink: boolean;
  /** 是否是指向 Skills Manager 中央库的托管链接 */
  is_managed: boolean;
  last_updated: string;
}

/** 自动发现的项目候选 */
export interface ProjectCandidate {
  path: string;
  name: string;
  /** 该项目下检测到的约定 skill 目录（相对路径） */
  skill_dirs: string[];
  registered: boolean;
}

// ========== MCP ==========

export type McpTransport = 'stdio' | 'sse' | 'http';

/** 与具体 agent 配置格式无关的 MCP server 定义 */
export interface McpServerDef {
  name: string;
  transport: McpTransport;
  command?: string;
  args: string[];
  env: Record<string, string>;
  url?: string;
  headers: Record<string, string>;
  enabled: boolean;
  /** 原始配置里未建模的字段 */
  extra?: Record<string, unknown>;
}

export interface McpBinding {
  agent: string;
  config_path: string;
  editable: boolean;
  error?: string;
  /** 该格式没有原生 enabled 字段时，由 Skills Manager 托管停用 */
  managed_disabled: boolean;
  /** 该绑定当前是否启用（已应用托管状态） */
  enabled: boolean;
}

export interface McpServerAggregate {
  name: string;
  definition: McpServerDef;
  bindings: McpBinding[];
  /** 多个 agent 中同名 server 的定义不一致 */
  conflict: boolean;
  managed_disabled: string[];
}

export interface McpAgentInfo {
  agent: string;
  label: string;
  config_path: string;
  exists: boolean;
}

export interface McpConfigError {
  agent: string;
  config_path: string;
  error: string;
}

export interface McpOverview {
  servers: McpServerAggregate[];
  agents: McpAgentInfo[];
  errors: McpConfigError[];
}

// ========== MCP Marketplace ==========

export type McpInstallOptionKind = 'remote' | 'package';

export type McpInstallInputKind = 'env' | 'header' | 'argument' | 'variable';

export interface McpInstallInput {
  key: string;
  label: string;
  description?: string | null;
  required: boolean;
  secret: boolean;
  defaultValue?: string | null;
  placeholder?: string | null;
  choices: string[];
  kind: McpInstallInputKind;
}

export interface McpInstallOption {
  id: string;
  kind: McpInstallOptionKind;
  label: string;
  transport: string;
  url?: string | null;
  headers: Record<string, string>;
  command?: string | null;
  args: string[];
  env: Record<string, string>;
  inputs: McpInstallInput[];
  supported: boolean;
  unsupportedReason?: string | null;
  packageType?: string | null;
  warnings: string[];
}

export interface McpMarketplaceItem {
  registryName: string;
  title: string;
  description: string;
  version: string;
  iconUrl?: string | null;
  repositoryUrl?: string | null;
  websiteUrl?: string | null;
  categories: string[];
  installKinds: string[];
  installOptions: McpInstallOption[];
  source: 'featured' | 'registry' | string;
  updatedAt?: string | null;
}

export interface McpMarketplacePage {
  items: McpMarketplaceItem[];
  nextCursor?: string | null;
}

export interface McpMarketplaceDetail extends McpMarketplaceItem {
  publishedAt?: string | null;
  status?: string | null;
}

export interface McpProjectTarget {
  projectId: string;
  target: string;
}

export interface McpInstallRequest {
  registryName: string;
  version: string;
  optionId: string;
  localName: string;
  values: Record<string, string>;
  enabled: boolean;
  globalTargets: string[];
  projectTargets: McpProjectTarget[];
  overwrite: boolean;
}

export interface McpInstallResult {
  server: McpServerDef;
  installedGlobal: string[];
  installedProjects: McpProjectTarget[];
  version: string;
  warnings: string[];
}

/** 市场安装记录。只记录目标与版本，不包含 env / header 的值。 */
export interface McpInstallRecord {
  registryName: string;
  localName: string;
  title: string;
  description: string;
  version: string;
  optionId: string;
  installedAt: string;
  globalTargets: string[];
  projectTargets: McpProjectTarget[];
}

export interface GitHubRepoConfig {
  owner: string;
  repo: string;
  branch: string;
  token?: string;
  last_sync?: string;
}

export interface GitHubConfig {
  repositories: Record<string, GitHubRepoConfig>;
}

// 某个 source 及其物理路径；useMergedView 从 SkillMetadata.source_paths 派生。
export interface SourcePathInfo {
  source: string;
  path: string;
}
