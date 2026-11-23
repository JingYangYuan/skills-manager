import type { McpOverview } from '@/types';

const HOME = '~';

/** 预览环境的全局 MCP 总览。 */
export const PREVIEW_MCP_OVERVIEW: McpOverview = {
  agents: [
    { agent: 'claude', label: 'Claude Code', config_path: `${HOME}/.claude.json`, exists: true },
    { agent: 'cursor', label: 'Cursor', config_path: `${HOME}/.cursor/mcp.json`, exists: true },
    { agent: 'antigravity', label: 'Antigravity / Gemini', config_path: `${HOME}/.gemini/config/mcp_config.json`, exists: true },
    { agent: 'opencode', label: 'OpenCode', config_path: `${HOME}/.config/opencode/opencode.json`, exists: true },
    { agent: 'codex', label: 'Codex', config_path: `${HOME}/.codex/config.toml`, exists: true },
    { agent: 'grok', label: 'Grok', config_path: `${HOME}/.grok/config.toml`, exists: false },
  ],
  errors: [],
  servers: [
    {
      name: 'zotero-agent',
      conflict: false,
      managed_disabled: [],
      definition: {
        name: 'zotero-agent',
        transport: 'stdio',
        command: 'npx',
        args: ['-y', 'zotero-mcp'],
        env: { ZOTERO_API_KEY: 'preview' },
        headers: {},
        enabled: true,
      },
      bindings: [
        { agent: 'claude', config_path: `${HOME}/.claude.json`, editable: true, managed_disabled: false, enabled: true },
        { agent: 'codex', config_path: `${HOME}/.codex/config.toml`, editable: true, managed_disabled: false, enabled: true },
      ],
    },
    {
      name: 'notion',
      conflict: true,
      managed_disabled: [],
      definition: {
        name: 'notion',
        transport: 'http',
        args: [],
        env: {},
        url: 'https://mcp.notion.com/mcp',
        headers: { Authorization: 'Bearer preview' },
        enabled: true,
      },
      bindings: [
        { agent: 'cursor', config_path: `${HOME}/.cursor/mcp.json`, editable: true, managed_disabled: false, enabled: true },
        { agent: 'opencode', config_path: `${HOME}/.config/opencode/opencode.json`, editable: true, managed_disabled: false, enabled: true },
      ],
    },
    {
      name: 'stata-mcp',
      conflict: false,
      managed_disabled: ['claude'],
      definition: {
        name: 'stata-mcp',
        transport: 'stdio',
        command: 'stata-mcp',
        args: [],
        env: {},
        headers: {},
        enabled: false,
      },
      bindings: [
        { agent: 'claude', config_path: `${HOME}/.claude.json`, editable: true, managed_disabled: true, enabled: false },
      ],
    },
  ],
};

export const PREVIEW_PROJECT_MCP: McpOverview = {
  agents: [
    { agent: 'claude', label: 'Claude Code (.mcp.json)', config_path: '~/Documents/paper-workspace/.mcp.json', exists: true },
    { agent: 'cursor', label: 'Cursor (.cursor/mcp.json)', config_path: '~/Documents/paper-workspace/.cursor/mcp.json', exists: false },
    { agent: 'grok', label: 'Grok (.grok/config.toml)', config_path: '~/Documents/paper-workspace/.grok/config.toml', exists: false },
  ],
  errors: [],
  servers: [
    {
      name: 'project-zotero',
      conflict: false,
      managed_disabled: [],
      definition: {
        name: 'project-zotero',
        transport: 'stdio',
        command: 'npx',
        args: ['-y', 'zotero-mcp'],
        env: {},
        headers: {},
        enabled: true,
      },
      bindings: [
        { agent: 'claude', config_path: '~/Documents/paper-workspace/.mcp.json', editable: true, managed_disabled: false, enabled: true },
      ],
    },
  ],
};
