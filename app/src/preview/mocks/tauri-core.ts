/**
 * Mock for `@tauri-apps/api/core`.
 *
 * 把业务代码里所有 `invoke(cmd, args)` 调用分发到一个 in-memory store，
 * 让 `api/tauri.ts` 完全无感地运行在浏览器预览环境里。
 *
 * 设计原则：
 * - 所有写操作修改内存 + 同步到 sessionStorage，刷新页面后保留一会儿，关闭标签后丢弃
 * - 对浏览器无法实现的命令（打开系统文件夹、Tauri 窗口主题、系统托盘）返回成功但不执行
 * - 对可能抛错的命令（GitHub API）返回合理的空态
 */

import type { SkillMetadata, AppConfig, AgentConfig, GitHubConfig, SkillFileEntry, LinkType, ProjectConfig, ProjectSkill, ProjectCandidate, McpOverview, McpServerDef, McpInstallRecord } from '@/types';
import { PREVIEW_SKILLS, PREVIEW_SKILL_CONTENT } from '../data/skills';
import { PREVIEW_APP_CONFIG } from '../data/agents';
import { PREVIEW_PROJECTS, PREVIEW_PROJECT_SKILLS } from '../data/projects';
import { PREVIEW_MCP_OVERVIEW, PREVIEW_PROJECT_MCP } from '../data/mcp';
import { PREVIEW_MCP_MARKET_DETAILS } from '../data/mcpMarket';

const STORAGE_KEY = 'preview:skills-v2';
const CONFIG_KEY = 'preview:config-v2';

type State = {
  skills: SkillMetadata[];
  config: AppConfig;
  projects: ProjectConfig[];
  projectSkills: Record<string, ProjectSkill[]>;
  mcp: McpOverview;
  projectMcp: McpOverview;
  mcpInstalls: McpInstallRecord[];
};

function deepClone<T>(v: T): T {
  return typeof structuredClone === 'function'
    ? structuredClone(v)
    : JSON.parse(JSON.stringify(v));
}

function loadState(): State {
  try {
    const skillsRaw = sessionStorage.getItem(STORAGE_KEY);
    const configRaw = sessionStorage.getItem(CONFIG_KEY);
    const config: AppConfig = configRaw ? JSON.parse(configRaw) : deepClone(PREVIEW_APP_CONFIG);
    return {
      skills: skillsRaw ? JSON.parse(skillsRaw) : deepClone(PREVIEW_SKILLS),
      config,
      projects: config.projects?.length ? config.projects : deepClone(PREVIEW_PROJECTS),
      projectSkills: deepClone(PREVIEW_PROJECT_SKILLS),
      mcp: deepClone(PREVIEW_MCP_OVERVIEW),
      projectMcp: deepClone(PREVIEW_PROJECT_MCP),
      mcpInstalls: [],
    };
  } catch {
    return {
      skills: deepClone(PREVIEW_SKILLS),
      config: deepClone(PREVIEW_APP_CONFIG),
      projects: deepClone(PREVIEW_PROJECTS),
      projectSkills: deepClone(PREVIEW_PROJECT_SKILLS),
      mcp: deepClone(PREVIEW_MCP_OVERVIEW),
      projectMcp: deepClone(PREVIEW_PROJECT_MCP),
      mcpInstalls: [],
    };
  }
}

const state: State = loadState();

function persist() {
  try {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(state.skills));
    sessionStorage.setItem(CONFIG_KEY, JSON.stringify(state.config));
  } catch {
    /* quota exceeded or private mode — preview still works in memory */
  }
}

function findSkill(id: string): SkillMetadata | undefined {
  return state.skills.find((s) => s.id === id);
}

/** 派生 agent_enabled：native source 存在 ⇒ true；否则看 `open` 数组 */
function recomputeAgentEnabled(skill: SkillMetadata) {
  const map: Record<string, boolean> = {};
  for (const s of skill.sources) {
    if (s !== 'global') map[s] = true;
  }
  for (const o of skill.open) {
    map[o] = true;
  }
  skill.agent_enabled = map;
}

function simulateLatency<T>(value: T, ms = 120): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}

// ---------------- 命令实现 ----------------

/** 预览环境的 MCP upsert：只维护内存中的绑定关系。 */
function upsertMockMcp(overview: McpOverview, def: McpServerDef, targets: string[]) {
  const existing = overview.servers.find((server) => server.name === def.name);
  const bindings = targets.map((agent) => {
    const info = overview.agents.find((item) => item.agent === agent);
    return {
      agent,
      config_path: info?.config_path ?? '',
      editable: true,
      managed_disabled: false,
      enabled: def.enabled,
    };
  });
  if (existing) {
    existing.definition = def;
    existing.bindings = bindings;
    existing.conflict = false;
    existing.managed_disabled = [];
  } else {
    overview.servers.push({
      name: def.name,
      definition: def,
      bindings,
      conflict: false,
      managed_disabled: [],
    });
  }
}


const PREVIEW_MCP_MARKET_ITEMS = Object.values(PREVIEW_MCP_MARKET_DETAILS);

const handlers: Record<string, (args: Record<string, unknown>) => unknown | Promise<unknown>> = {
  // ---- skills ----
  list_skills: () => simulateLatency(state.skills),
  rescan_skills: () => simulateLatency(state.skills),

  enable_skill: ({ skillId, agent }) => {
    const sk = findSkill(String(skillId));
    if (!sk) throw new Error(`Skill ${skillId} not found`);
    if (!agent) {
      // 与后端一致：agent 必填，不再支持一键广播到所有 agent
      throw new Error('agent 参数必填');
    }
    const a = String(agent);
    if (!sk.open.includes(a) && !sk.sources.includes(a)) sk.open.push(a);
    recomputeAgentEnabled(sk);
    persist();
    return simulateLatency(undefined);
  },

  disable_skill: ({ skillId, agent }) => {
    const sk = findSkill(String(skillId));
    if (!sk) throw new Error(`Skill ${skillId} not found`);
    if (!agent) {
      throw new Error('agent 参数必填');
    }
    const a = String(agent);
    sk.open = sk.open.filter((x) => x !== a);
    recomputeAgentEnabled(sk);
    persist();
    return simulateLatency(undefined);
  },

  repair_skill_links: () => simulateLatency({ relinked: 0, converted: 0, removed: 0, skipped: 0, errors: [] }),

  set_skill_primary: ({ skillId, newPrimary }) => {
    const sk = findSkill(String(skillId));
    if (!sk) throw new Error(`Skill ${skillId} not found`);
    sk.primary = String(newPrimary);
    persist();
    return simulateLatency(undefined);
  },

  get_skill_content: ({ skillId }) => {
    const sk = findSkill(String(skillId));
    if (!sk) throw new Error(`Skill ${skillId} not found`);
    return simulateLatency(PREVIEW_SKILL_CONTENT(sk));
  },

  get_skill_files: ({ skillId }): Promise<SkillFileEntry[]> => {
    const sk = findSkill(String(skillId));
    if (!sk) return simulateLatency([] as SkillFileEntry[]);
    const base = `~/.skills-manager/skills/${sk.id}`;
    const tree: SkillFileEntry[] = [
      {
        name: 'SKILL.md',
        path: `${base}/SKILL.md`,
        is_dir: false,
        size: 3200,
      },
      {
        name: 'references',
        path: `${base}/references`,
        is_dir: true,
        children: [
          { name: 'usage.md', path: `${base}/references/usage.md`, is_dir: false, size: 1800 },
          { name: 'examples.md', path: `${base}/references/examples.md`, is_dir: false, size: 2400 },
        ],
      },
    ];
    return simulateLatency(tree);
  },

  read_skill_file: ({ skillId, filePath }) => {
    const sk = findSkill(String(skillId));
    if (!sk) return simulateLatency('');
    if (String(filePath).endsWith('SKILL.md')) return simulateLatency(PREVIEW_SKILL_CONTENT(sk));
    return simulateLatency(
      `# ${String(filePath).split('/').pop()}\n\n> 预览环境示例内容。桌面版会展示 skill 真实文件内容。\n`,
    );
  },

  delete_skill: ({ skillId, source }) => {
    const sk = findSkill(String(skillId));
    if (!sk) return simulateLatency(undefined);
    if (!source) {
      // 全删
      state.skills = state.skills.filter((s) => s.id !== sk.id);
    } else {
      const src = String(source);
      sk.sources = sk.sources.filter((s) => s !== src);
      sk.open = sk.open.filter((s) => s !== src);
      if (sk.source_paths[src]) delete sk.source_paths[src];
      if (sk.primary === src) sk.primary = sk.sources[0] ?? 'global';
      if (sk.sources.length === 0) {
        state.skills = state.skills.filter((s) => s.id !== sk.id);
      } else {
        recomputeAgentEnabled(sk);
      }
    }
    persist();
    return simulateLatency(undefined);
  },

  import_skill_folder: () => {
    // 预览环境无文件系统，统一报可读错误，业务代码里已经处理了冲突提示
    return Promise.reject(
      '在线预览暂不支持拖拽导入（浏览器无法访问本地文件系统）。请下载桌面版体验完整导入流程。',
    );
  },

  // ---- projects ----
  list_projects: () => simulateLatency(state.projects),

  add_project: ({ path, name }) => {
    const projectPath = String(path ?? '').trim();
    if (!projectPath) throw new Error('项目路径不能为空');
    if (state.projects.some((p) => p.path === projectPath)) {
      throw new Error(`项目已注册: ${projectPath}`);
    }
    const folder = projectPath.split('/').filter(Boolean).pop() ?? 'project';
    const project: ProjectConfig = {
      id: `${folder.toLowerCase().replace(/[^a-z0-9]+/g, '-')}-${Date.now().toString(16).slice(-12)}`,
      name: String(name ?? '').trim() || folder,
      path: projectPath,
      enabled: true,
      auto_detected: false,
    };
    state.projects.push(project);
    state.projectSkills[project.id] = [];
    state.config.projects = state.projects;
    persist();
    return simulateLatency(project);
  },

  remove_project: ({ id }) => {
    const projectId = String(id);
    state.projects = state.projects.filter((p) => p.id !== projectId);
    delete state.projectSkills[projectId];
    state.config.projects = state.projects;
    persist();
    return simulateLatency(undefined);
  },

  detect_project_candidates: (): Promise<ProjectCandidate[]> =>
    simulateLatency([
      {
        path: '~/Documents/new-paper',
        name: 'new-paper',
        skill_dirs: ['.claude/skills'],
        registered: false,
      },
      {
        path: '~/Documents/teaching-slides',
        name: 'teaching-slides',
        skill_dirs: [],
        registered: false,
      },
    ]),

  list_project_skills: ({ projectId }) =>
    simulateLatency(state.projectSkills[String(projectId)] ?? []),

  install_skill_to_project: ({ projectId, skillId, targetDir }) => {
    const pid = String(projectId);
    const project = state.projects.find((p) => p.id === pid);
    if (!project) throw new Error(`项目不存在: ${pid}`);
    const skill = findSkill(String(skillId));
    if (!skill) throw new Error(`技能不存在: ${skillId}`);
    const dir = String(targetDir ?? '.claude/skills');
    const list = state.projectSkills[pid] ?? [];
    if (list.some((s) => s.id === skill.id)) {
      throw new Error(`项目目录中已存在技能 '${skill.name}'`);
    }
    const rel = `${dir}/${skill.id}`;
    list.push({
      id: skill.id,
      name: skill.name,
      description: skill.description,
      path: `${project.path}/${rel}`,
      rel_dir: rel,
      agent: dir.split('/')[0].replace('.', ''),
      is_symlink: true,
      is_managed: true,
      last_updated: '',
    });
    state.projectSkills[pid] = list;
    return simulateLatency(`${project.path}/${rel}`);
  },

  import_project_skill: ({ projectId, skillPath }) => {
    const list = state.projectSkills[String(projectId)] ?? [];
    const found = list.find((s) => s.path === String(skillPath));
    return simulateLatency(found?.name ?? 'project-skill');
  },

  delete_project_skill: ({ projectId, skillPath }) => {
    const pid = String(projectId);
    state.projectSkills[pid] = (state.projectSkills[pid] ?? []).filter(
      (s) => s.path !== String(skillPath),
    );
    return simulateLatency(undefined);
  },

  open_project_path: () => simulateLatency(undefined),


  // ---- MCP ----
  list_mcp_servers: () => simulateLatency(state.mcp),

  save_mcp_server: ({ server, targets }) => {
    upsertMockMcp(state.mcp, server as McpServerDef, (targets as string[]) ?? []);
    return simulateLatency(undefined);
  },

  delete_mcp_server: ({ name, targets }) => {
    const server = state.mcp.servers.find((item) => item.name === String(name));
    const list = (targets as string[]) ?? [];
    if (server) {
      server.bindings = server.bindings.filter((binding) => !list.includes(binding.agent));
      if (server.bindings.length === 0) {
        state.mcp.servers = state.mcp.servers.filter((item) => item.name !== server.name);
      }
    }
    return simulateLatency(undefined);
  },

  set_mcp_server_enabled: ({ name, enabled, targets }) => {
    const server = state.mcp.servers.find((item) => item.name === String(name));
    const list = (targets as string[]) ?? [];
    if (server) {
      for (const binding of server.bindings) {
        if (list.includes(binding.agent)) binding.enabled = Boolean(enabled);
      }
      server.definition.enabled = server.bindings.some((binding) => binding.enabled);
    }
    return simulateLatency(undefined);
  },

  sync_mcp_server: ({ name, toAgents }) => {
    const server = state.mcp.servers.find((item) => item.name === String(name));
    if (server) {
      for (const agent of (toAgents as string[]) ?? []) {
        if (server.bindings.some((binding) => binding.agent === agent)) continue;
        const info = state.mcp.agents.find((item) => item.agent === agent);
        server.bindings.push({
          agent,
          config_path: info?.config_path ?? '',
          editable: true,
          managed_disabled: false,
          enabled: server.definition.enabled,
        });
      }
    }
    return simulateLatency(undefined);
  },

  get_mcp_raw_config: ({ agent }) => {
    const servers = state.mcp.servers
      .filter((server) => server.bindings.some((binding) => binding.agent === String(agent)))
      .map((server) => server.definition);
    return simulateLatency(
      servers.length ? JSON.stringify({ mcpServers: Object.fromEntries(servers.map((def) => [def.name, def])) }, null, 2) : '',
    );
  },

  list_project_mcp: () => simulateLatency(state.projectMcp),

  save_project_mcp: ({ server, target }) => {
    upsertMockMcp(state.projectMcp, server as McpServerDef, [String(target ?? 'claude')]);
    return simulateLatency(undefined);
  },

  delete_project_mcp: ({ name, target }) => {
    const server = state.projectMcp.servers.find((item) => item.name === String(name));
    if (server) {
      server.bindings = server.bindings.filter((binding) => binding.agent !== String(target));
      if (server.bindings.length === 0) {
        state.projectMcp.servers = state.projectMcp.servers.filter((item) => item.name !== server.name);
      }
    }
    return simulateLatency(undefined);
  },

  get_project_mcp_raw_config: () => {
    const servers = state.projectMcp.servers.map((server) => server.definition);
    return simulateLatency(
      servers.length ? JSON.stringify({ mcpServers: Object.fromEntries(servers.map((def) => [def.name, def])) }, null, 2) : '',
    );
  },

  // ---- MCP marketplace ----
  fetch_mcp_marketplace_featured: () => simulateLatency(PREVIEW_MCP_MARKET_ITEMS),

  fetch_mcp_marketplace_servers: ({ query, cursor, limit }) => {
    const term = String(query ?? '').trim().toLowerCase();
    const matched = term
      ? PREVIEW_MCP_MARKET_ITEMS.filter((item) =>
          `${item.title} ${item.registryName} ${item.description}`.toLowerCase().includes(term),
        )
      : PREVIEW_MCP_MARKET_ITEMS;
    const start = cursor ? Number(cursor) || 0 : 0;
    const size = Number(limit) > 0 ? Number(limit) : 40;
    return simulateLatency({
      items: matched.slice(start, start + size),
      nextCursor: start + size < matched.length ? String(start + size) : null,
    });
  },

  fetch_mcp_marketplace_detail: ({ registryName }) => {
    const detail = PREVIEW_MCP_MARKET_DETAILS[String(registryName)];
    if (!detail) throw new Error(`预览环境未内置 MCP server: ${String(registryName)}`);
    return simulateLatency(detail);
  },

  list_mcp_marketplace_installs: () => simulateLatency(state.mcpInstalls),

  forget_mcp_marketplace_install: ({ registryName, localName }) => {
    state.mcpInstalls = state.mcpInstalls.filter(
      (record) =>
        record.registryName !== String(registryName) || record.localName !== String(localName),
    );
    return simulateLatency(undefined);
  },

  install_mcp_from_marketplace: ({ request }) => {
    const req = request as {
      registryName: string;
      optionId: string;
      localName: string;
      values?: Record<string, string>;
      enabled?: boolean;
      globalTargets?: string[];
      projectTargets?: Array<{ projectId: string; target: string }>;
    };
    const detail = PREVIEW_MCP_MARKET_DETAILS[req.registryName];
    if (!detail) throw new Error(`预览环境未内置 MCP server: ${req.registryName}`);
    const option =
      detail.installOptions.find((item) => item.id === req.optionId) ?? detail.installOptions[0];
    if (!option || !option.supported) throw new Error('该安装方式暂不支持');

    const values = req.values ?? {};
    const env: Record<string, string> = {};
    const headers: Record<string, string> = {};
    for (const input of option.inputs) {
      const value = (values[input.key] ?? input.defaultValue ?? '').trim();
      if (!value) continue;
      if (input.kind === 'header') headers[input.label] = value;
      else if (input.kind === 'env') env[input.label] = value;
    }

    const enabled = req.enabled ?? true;
    const server: McpServerDef =
      option.kind === 'remote'
        ? {
            name: req.localName,
            transport: option.transport as McpServerDef['transport'],
            args: [],
            env: {},
            url: option.url ?? '',
            headers,
            enabled,
          }
        : {
            name: req.localName,
            transport: 'stdio',
            command: option.command ?? 'npx',
            args: [...option.args],
            env,
            headers: {},
            enabled,
          };

    const globalTargets = req.globalTargets ?? [];
    const projectTargets = req.projectTargets ?? [];
    if (globalTargets.length > 0) upsertMockMcp(state.mcp, server, globalTargets);
    for (const entry of projectTargets) upsertMockMcp(state.projectMcp, server, [entry.target]);

    const record: McpInstallRecord = {
      registryName: detail.registryName,
      localName: server.name,
      title: detail.title,
      description: detail.description,
      version: detail.version,
      optionId: option.id,
      installedAt: new Date().toISOString(),
      globalTargets,
      projectTargets,
    };
    const existing = state.mcpInstalls.find(
      (item) =>
        item.registryName === record.registryName && item.localName === record.localName,
    );
    if (existing) {
      existing.version = record.version;
      existing.optionId = record.optionId;
      existing.installedAt = record.installedAt;
      for (const target of globalTargets) {
        if (!existing.globalTargets.includes(target)) existing.globalTargets.push(target);
      }
      for (const entry of projectTargets) {
        if (
          !existing.projectTargets.some(
            (item) => item.projectId === entry.projectId && item.target === entry.target,
          )
        ) {
          existing.projectTargets.push(entry);
        }
      }
    } else {
      state.mcpInstalls.push(record);
    }

    return simulateLatency({
      server,
      installedGlobal: globalTargets,
      installedProjects: projectTargets,
      version: detail.version,
      warnings: [],
    });
  },

  // ---- agents / config ----
  get_agents: () => simulateLatency(state.config.agents),
  detect_agents: () => {
    // 全部标记为 detected，视觉上更饱满
    const agents = state.config.agents.map((a: AgentConfig) => ({ ...a, detected: true }));
    return simulateLatency(agents);
  },
  add_agent: ({ agent }) => {
    const a = agent as AgentConfig;
    if (!state.config.agents.find((x) => x.name === a.name)) {
      state.config.agents.push(a);
      persist();
    }
    return simulateLatency(undefined);
  },
  remove_agent: ({ name }) => {
    state.config.agents = state.config.agents.filter((a) => a.name !== String(name));
    persist();
    return simulateLatency(undefined);
  },
  get_config: () => simulateLatency(state.config),
  set_linking_strategy: ({ strategy }) => {
    state.config.linking_strategy = strategy as LinkType;
    persist();
    return simulateLatency(undefined);
  },
  set_skill_hide_prefixes: ({ prefixes }) => {
    state.config.skill_hide_prefixes = (prefixes as string[]) ?? [];
    persist();
    return simulateLatency(undefined);
  },
  get_pinned_skills: () => simulateLatency(state.config.pinned_skills ?? []),
  set_skill_pinned: ({ skillId, pinned }) => {
    const id = String(skillId);
    const current = state.config.pinned_skills ?? [];
    if (pinned) {
      if (!current.includes(id)) {
        state.config.pinned_skills = [...current, id];
      }
    } else {
      state.config.pinned_skills = current.filter((x) => x !== id);
    }
    persist();
    return simulateLatency(undefined);
  },

  // ---- 打开系统文件夹：浏览器里做不到，静默 no-op ----
  open_skills_manager_folder: () => simulateLatency(undefined),
  open_folder: () => simulateLatency(undefined),

  // ---- GitHub：预览环境不走这些路由，给出安全默认值即可 ----
  get_github_config: (): Promise<GitHubConfig> => simulateLatency({ repositories: {} }),
  save_github_config: () => simulateLatency(undefined),
  test_github_connection: () => simulateLatency(undefined),
  sync_github_repo: () => simulateLatency(undefined),
  restore_from_github: () => simulateLatency(0),
  star_github_repo: () => simulateLatency(true),
  check_github_star: () => simulateLatency(false),

  // ---- Tauri 平台杂项：业务代码都有 .catch 兜底，这里 no-op ----
  update_tray_language: () => simulateLatency(undefined),
  set_window_theme: () => simulateLatency(undefined),
};

export async function invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const h = handlers[cmd];
  if (!h) {
    if (import.meta.env.DEV) {
      console.warn(`[preview/mock-invoke] unhandled command: ${cmd}`, args);
    }
    return undefined as T;
  }
  return (await h(args ?? {})) as T;
}

/**
 * 注入 `window.__TAURI__.core.invoke`，让 `lib/tauri-env.ts#isTauri()`
 * 返回 true，业务代码才会真正走 API 调用。否则像 `useSkillData` 这样显式
 * 校验 `isTauri()` 的地方会抛错，Dashboard 拿不到数据。
 */
if (typeof window !== 'undefined') {
  const w = window as unknown as {
    __TAURI__?: { core?: { invoke?: unknown } };
    __PREVIEW__?: boolean;
  };
  w.__TAURI__ = w.__TAURI__ ?? {};
  w.__TAURI__.core = w.__TAURI__.core ?? {};
  w.__TAURI__.core.invoke = invoke;
  // 标记"当前为预览环境"，供 `lib/preview-env.ts#isPreview()` 读取
  w.__PREVIEW__ = true;
}
