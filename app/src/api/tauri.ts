import { invoke } from '@tauri-apps/api/core';
import type { SkillMetadata, GitHubConfig, AgentConfig, AppConfig, SkillFileEntry, LinkType } from '@/types';

/** 链接校准报告（对应后端 RepairReport） */
export interface RepairReport {
  relinked: number;
  removed: number;
  skipped: number;
  errors: string[];
}

/** 单条批量操作结果 */
export interface BatchDeleteReport {
  deleted: string[];
  failed: string[];
  errors: string[];
  trash_ids: string[];
}

/** 批量启用/停用汇总 */
export interface BatchToggleReport {
  succeeded: number;
  failed: number;
  errors: string[];
}

/** 回收站中的一条物理目录 */
export interface TrashEntryItem {
  source: string;
  original_path: string;
}

/** 回收站条目 */
export interface TrashItem {
  trash_id: string;
  skill_id: string;
  name: string;
  deleted_at: string;
  reason: string;
  entries: TrashEntryItem[];
  /** 原路径已被占用，只能彻底删除 */
  stale: boolean;
}

/** 回收站恢复结果 */
export interface TrashRestoreReport {
  restored: string[];
  skipped: string[];
  errors: string[];
}

/** 归并冲突处理方式 */
export type ConflictAction = 'skip' | 'overwrite' | 'rename';

export interface ConsolidationItem {
  id: string;
  name: string;
  path: string;
}

export interface ConsolidationConflict {
  id: string;
  name: string;
  agent_path: string;
  central_path: string;
}

/** 跳过原因：already_managed | missing_skill_md | plugin_dir | plugin_cache | extra_path | hidden | not_a_dir */
export interface ConsolidationSkip {
  id: string;
  reason: string;
}

export interface ConsolidationPlan {
  agent: string;
  agent_skills_dir: string;
  movable: ConsolidationItem[];
  conflicts: ConsolidationConflict[];
  skipped: ConsolidationSkip[];
}

export interface ConflictDecision {
  id: string;
  action: ConflictAction;
}

export interface MovedSkill {
  from_id: string;
  to_id: string;
}

export interface ConsolidationReport {
  moved: MovedSkill[];
  skipped: string[];
  errors: string[];
  trash_ids: string[];
}

export const skillsApi = {
  list: async (): Promise<SkillMetadata[]> => {
    return await invoke<SkillMetadata[]>('list_skills');
  },

  enable: async (skillId: string, agent: string): Promise<void> => {
    await invoke('enable_skill', { skillId, agent });
  },

  disable: async (skillId: string, agent: string): Promise<void> => {
    await invoke('disable_skill', { skillId, agent });
  },

  /** 按 skill_states[].open 校准磁盘链接，返回修复报告 */
  repairLinks: async (): Promise<RepairReport> => {
    return await invoke<RepairReport>('repair_skill_links');
  },

  setPrimary: async (skillId: string, newPrimary: string): Promise<void> => {
    await invoke('set_skill_primary', { skillId, newPrimary });
  },

  getContent: async (skillId: string): Promise<string> => {
    return await invoke('get_skill_content', { skillId });
  },

  getFiles: async (skillId: string, source?: string): Promise<SkillFileEntry[]> => {
    return await invoke('get_skill_files', { skillId, source });
  },

  readFile: async (skillId: string, filePath: string, source?: string): Promise<string> => {
    return await invoke('read_skill_file', { skillId, filePath, source });
  },

  rescan: async (): Promise<SkillMetadata[]> => {
    return await invoke<SkillMetadata[]>('rescan_skills');
  },

  delete: async (skillId: string, source?: string): Promise<void> => {
    await invoke('delete_skill', { skillId, source });
  },

  importFolder: async (folderPath: string): Promise<string> => {
    return await invoke<string>('import_skill_folder', { folderPath });
  },

  copyToAgent: async (skillId: string, sourceAgent: string, targetAgent: string, defaultEnabled: boolean): Promise<string> => {
    return await invoke<string>('copy_skill_to_agent', {
      skillId,
      sourceAgent,
      targetAgent,
      defaultEnabled,
    });
  },

  /** 批量删除（进回收站，可恢复） */
  batchDelete: async (skillIds: string[], source?: string): Promise<BatchDeleteReport> => {
    return await invoke<BatchDeleteReport>('batch_delete_skills', { skillIds, source });
  },

  /** 批量启用/停用同一 Agent */
  batchToggle: async (skillIds: string[], agent: string, enable: boolean): Promise<BatchToggleReport> => {
    return await invoke<BatchToggleReport>('batch_toggle_skills', { skillIds, agent, enable });
  },

  /** 归并预演（只读） */
  planConsolidation: async (agent: string): Promise<ConsolidationPlan> => {
    return await invoke<ConsolidationPlan>('plan_agent_consolidation', { agent });
  },

  /** 执行归并；冲突未给决策时默认跳过 */
  applyConsolidation: async (
    agent: string,
    conflicts?: ConflictDecision[],
  ): Promise<ConsolidationReport> => {
    return await invoke<ConsolidationReport>('apply_agent_consolidation', { agent, conflicts });
  },
};

export const trashApi = {
  list: async (): Promise<TrashItem[]> => {
    return await invoke<TrashItem[]>('list_trash');
  },

  restore: async (trashIds: string[]): Promise<TrashRestoreReport> => {
    return await invoke<TrashRestoreReport>('restore_trash', { trashIds });
  },

  purge: async (trashId: string): Promise<void> => {
    await invoke('purge_trash_entry', { trashId });
  },

  empty: async (): Promise<number> => {
    return await invoke<number>('empty_trash');
  },
};

export const agentsApi = {
  /**
   * Get list of all configured agents
   */
  list: async (): Promise<AgentConfig[]> => {
    return await invoke<AgentConfig[]>('get_agents');
  },

  /**
   * Add a new agent configuration
   */
  add: async (agent: AgentConfig): Promise<void> => {
    await invoke('add_agent', { agent });
  },

  /**
   * Remove an agent by name
   */
  remove: async (name: string): Promise<void> => {
    await invoke('remove_agent', { name });
  },

  /**
   * Get full application configuration
   */
  getConfig: async (): Promise<AppConfig> => {
    return await invoke<AppConfig>('get_config');
  },

  /**
   * Set the file linking strategy
   */
  setLinkingStrategy: async (strategy: LinkType): Promise<void> => {
    await invoke('set_linking_strategy', { strategy });
  },

  /**
   * Open the skills manager folder in system file manager
   */
  openFolder: async (): Promise<void> => {
    await invoke('open_skills_manager_folder');
  },

  /**
   * Open a specific folder path in system file manager
   */
  openFolderPath: async (path: string): Promise<void> => {
    await invoke('open_folder', { path });
  },

  /**
   * Detect which agents are installed on the system
   * Returns updated agent list with detection status
   */
  detect: async (): Promise<AgentConfig[]> => {
    return await invoke<AgentConfig[]>('detect_agents');
  },

  /**
   * Persist skill hide prefixes to backend config and refresh the tray menu.
   * Frontend still reads localStorage as the source of truth for UI; this only
   * keeps the native tray in sync.
   */
  setSkillHidePrefixes: async (prefixes: string[]): Promise<void> => {
    await invoke('set_skill_hide_prefixes', { prefixes });
  },
};

/** Skill 置顶（pin）相关命令 */
export const pinApi = {
  /** 设置某个 skill 是否置顶；持久化到 AppConfig.pinned_skills */
  setPinned: async (skillId: string, pinned: boolean): Promise<void> => {
    await invoke('set_skill_pinned', { skillId, pinned });
  },
  /** 读取持久化的置顶 skill_id 列表（按用户置顶顺序返回） */
  list: async (): Promise<string[]> => {
    return await invoke<string[]>('get_pinned_skills');
  },
};

export const githubApi = {
  syncRepo: async (
    name: string,
    _branch: string,
    options?: { overwriteRemote?: boolean }
  ): Promise<void> => {
    await invoke('sync_github_repo', {
      request: {
        name,
        overwriteRemote: options?.overwriteRemote ?? false,
      },
    });
  },

  restoreFromGithub: async (name: string, overwriteLocal = false): Promise<number> => {
    return await invoke<number>('restore_from_github', { request: { name, overwriteLocal } });
  },

  getConfig: async (): Promise<GitHubConfig> => {
    return await invoke('get_github_config');
  },

  testConnection: async (config: {
    owner: string;
    repo: string;
    branch: string;
    token: string;
  }): Promise<void> => {
    await invoke('test_github_connection', { owner: config.owner, repo: config.repo, branch: config.branch, token: config.token });
  },

  saveConfig: async (config: {
    owner: string;
    repo: string;
    branch: string;
    token?: string;
  }): Promise<void> => {
    await invoke('save_github_config', { owner: config.owner, repo: config.repo, branch: config.branch, token: config.token });
  },

  openSkillsManagerFolder: async (): Promise<void> => {
    await invoke('open_skills_manager_folder');
  },

};
