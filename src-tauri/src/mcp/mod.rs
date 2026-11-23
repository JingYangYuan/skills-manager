//! MCP 配置管理。
//!
//! 不同 agent 的 MCP 配置格式差异很大，这里把它们统一成 `McpServerDef`：
//! - Claude / Cursor / Pi / OMP / WorkBuddy / ZCode / Kiro：`mcpServers` JSON
//! - OpenClaw：`mcp.servers` JSON（`transport` 字段）
//! - OpenCode：`mcp.servers` JSON（local / remote）
//! - Kilo：顶层 `mcp.<name>`，形状与 OpenCode 相同
//! - Codex / Grok：`[mcp_servers.*]` TOML
//!
//! 写回时只改目标键，保留文件中的其它字段与注释（TOML 用 toml_edit）。

pub mod json_adapter;
pub mod opencode_adapter;
pub mod state;
pub mod toml_adapter;

use crate::models::{
    McpAgentInfo, McpBinding, McpConfigError, McpOverview, McpServerAggregate, McpServerDef,
};
use crate::projects::expand_path;
use json_adapter::{EnabledStyle, JsonLayout, JsonSpec};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpFormat {
    Json(JsonSpec),
    OpenCode,
    /// Kilo Code：顶层 `mcp.<name>`，形状与 OpenCode 相同。
    Kilo,
    Toml,
}

pub const STANDARD_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "url",
    type_key: "type",
    http_type: Some("http"),
    sse_type: Some("sse"),
    stdio_type: Some("stdio"),
    enabled: EnabledStyle::None,
};

/// Cursor 按 URL 推断传输方式，不写 `type`。
pub const CURSOR_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "url",
    type_key: "type",
    http_type: None,
    sse_type: None,
    stdio_type: None,
    enabled: EnabledStyle::Disabled,
};

/// Antigravity 用 `serverUrl` + `disabled`，不写 `type`。
pub const ANTIGRAVITY_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "serverUrl",
    type_key: "type",
    http_type: None,
    sse_type: None,
    stdio_type: None,
    enabled: EnabledStyle::Disabled,
};

/// WorkBuddy 的远程 type 为 `streamableHttp`。
pub const WORKBUDDY_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "url",
    type_key: "type",
    http_type: Some("streamableHttp"),
    sse_type: Some("sse"),
    stdio_type: Some("stdio"),
    enabled: EnabledStyle::Disabled,
};

/// ZCode 的 server 列表在 `mcp.servers` 下。
pub const ZCODE_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Nested,
    url_key: "url",
    type_key: "type",
    http_type: Some("http"),
    sse_type: Some("sse"),
    stdio_type: Some("stdio"),
    enabled: EnabledStyle::Enabled,
};

/// OpenClaw 的 server 列表在 `mcp.servers` 下，远程传输字段为 `transport`。
///
/// 配置文件是 JSON5；只有当文件本身是合法 JSON（无注释、无尾逗号）时才能读写，
/// 否则会在读取阶段报错，不会写入。
pub const OPENCLAW_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Nested,
    url_key: "url",
    type_key: "transport",
    http_type: Some("streamable-http"),
    sse_type: Some("sse"),
    stdio_type: None,
    enabled: EnabledStyle::Enabled,
};

pub const PI_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "url",
    type_key: "type",
    http_type: Some("http"),
    sse_type: None,
    stdio_type: Some("stdio"),
    enabled: EnabledStyle::Enabled,
};

pub const OMP_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "url",
    type_key: "type",
    http_type: Some("http"),
    sse_type: Some("sse"),
    stdio_type: Some("stdio"),
    enabled: EnabledStyle::None,
};

/// Kiro 的 stdio server 不写 `type`，使用 `disabled`。
pub const KIRO_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "url",
    type_key: "type",
    http_type: None,
    sse_type: None,
    stdio_type: None,
    enabled: EnabledStyle::Disabled,
};

/// Kimi Code 用 `transport` 表示传输方式（sse / http）。
pub const KIMI_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "url",
    type_key: "transport",
    http_type: None,
    sse_type: Some("sse"),
    stdio_type: None,
    enabled: EnabledStyle::Enabled,
};

/// Trae 不写 `type`，按 command / url 推断。
pub const TRAE_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "url",
    type_key: "type",
    http_type: None,
    sse_type: None,
    stdio_type: None,
    enabled: EnabledStyle::None,
};

/// GitHub Copilot CLI 的 stdio type 为 `local`。
pub const COPILOT_JSON: JsonSpec = JsonSpec {
    layout: JsonLayout::Root,
    url_key: "url",
    type_key: "type",
    http_type: Some("http"),
    sse_type: Some("sse"),
    stdio_type: Some("local"),
    enabled: EnabledStyle::None,
};

#[derive(Debug, Clone, Copy)]
pub struct McpAgentSpec {
    pub agent: &'static str,
    pub label: &'static str,
    pub format: McpFormat,
    /// 相对路径；全局配置以 `~/` 开头，项目配置相对项目根。
    pub rel_path: &'static str,
}

pub const GLOBAL_AGENTS: &[McpAgentSpec] = &[
    McpAgentSpec {
        agent: "claude",
        label: "Claude Code",
        format: McpFormat::Json(STANDARD_JSON),
        rel_path: "~/.claude.json",
    },
    McpAgentSpec {
        agent: "cursor",
        label: "Cursor",
        format: McpFormat::Json(CURSOR_JSON),
        rel_path: "~/.cursor/mcp.json",
    },
    McpAgentSpec {
        agent: "antigravity",
        label: "Antigravity (GUI / CLI)",
        format: McpFormat::Json(ANTIGRAVITY_JSON),
        rel_path: "~/.gemini/config/mcp_config.json",
    },
    McpAgentSpec {
        agent: "opencode",
        label: "OpenCode",
        format: McpFormat::OpenCode,
        rel_path: "~/.config/opencode/opencode.json",
    },
    McpAgentSpec {
        agent: "openclaw",
        label: "OpenClaw",
        format: McpFormat::Json(OPENCLAW_JSON),
        rel_path: "~/.openclaw/openclaw.json",
    },
    McpAgentSpec {
        agent: "codex",
        label: "Codex",
        format: McpFormat::Toml,
        rel_path: "~/.codex/config.toml",
    },
    McpAgentSpec {
        agent: "grok",
        label: "Grok",
        format: McpFormat::Toml,
        rel_path: "~/.grok/config.toml",
    },
    McpAgentSpec {
        agent: "workbuddy",
        label: "WorkBuddy",
        format: McpFormat::Json(WORKBUDDY_JSON),
        rel_path: "~/.workbuddy/mcp.json",
    },
    McpAgentSpec {
        agent: "zcode",
        label: "ZCode",
        format: McpFormat::Json(ZCODE_JSON),
        rel_path: "~/.zcode/cli/config.json",
    },
    McpAgentSpec {
        agent: "pi",
        label: "Pi",
        format: McpFormat::Json(PI_JSON),
        rel_path: "~/.pi/agent/mcp.json",
    },
    McpAgentSpec {
        agent: "omp",
        label: "OMP",
        format: McpFormat::Json(OMP_JSON),
        rel_path: "~/.omp/agent/mcp.json",
    },
    McpAgentSpec {
        agent: "kiro",
        label: "Kiro",
        format: McpFormat::Json(KIRO_JSON),
        rel_path: "~/.kiro/settings/mcp.json",
    },
    McpAgentSpec {
        agent: "kilo",
        label: "Kilo Code",
        format: McpFormat::Kilo,
        rel_path: "~/.config/kilo/kilo.json",
    },
    McpAgentSpec {
        agent: "kimi",
        label: "Kimi Code",
        format: McpFormat::Json(KIMI_JSON),
        rel_path: "~/.kimi-code/mcp.json",
    },
    McpAgentSpec {
        agent: "copilot",
        label: "GitHub Copilot",
        format: McpFormat::Json(COPILOT_JSON),
        rel_path: "~/.copilot/mcp-config.json",
    },
    McpAgentSpec {
        agent: "trae",
        label: "Trae",
        format: McpFormat::Json(TRAE_JSON),
        rel_path: "~/.trae/mcp.json",
    },
    McpAgentSpec {
        agent: "qoder",
        label: "Qoder",
        format: McpFormat::Json(STANDARD_JSON),
        rel_path: "~/.qoder/settings.json",
    },
    McpAgentSpec {
        agent: "codebuddy",
        label: "CodeBuddy",
        format: McpFormat::Json(STANDARD_JSON),
        rel_path: "~/.codebuddy/mcp.json",
    },
    McpAgentSpec {
        agent: "marscode",
        label: "MarsCode",
        format: McpFormat::Json(STANDARD_JSON),
        rel_path: "~/.marscode/vscode.mcp.config.json",
    },
];

pub const PROJECT_AGENTS: &[McpAgentSpec] = &[
    McpAgentSpec {
        agent: "claude",
        label: "Claude Code (.mcp.json)",
        format: McpFormat::Json(STANDARD_JSON),
        rel_path: ".mcp.json",
    },
    McpAgentSpec {
        agent: "cursor",
        label: "Cursor (.cursor/mcp.json)",
        format: McpFormat::Json(CURSOR_JSON),
        rel_path: ".cursor/mcp.json",
    },
    McpAgentSpec {
        agent: "antigravity",
        label: "Antigravity (.agents/mcp_config.json)",
        format: McpFormat::Json(ANTIGRAVITY_JSON),
        rel_path: ".agents/mcp_config.json",
    },
    McpAgentSpec {
        agent: "codex",
        label: "Codex (.codex/config.toml)",
        format: McpFormat::Toml,
        rel_path: ".codex/config.toml",
    },
    McpAgentSpec {
        agent: "grok",
        label: "Grok (.grok/config.toml)",
        format: McpFormat::Toml,
        rel_path: ".grok/config.toml",
    },
    McpAgentSpec {
        agent: "opencode",
        label: "OpenCode (opencode.json)",
        format: McpFormat::OpenCode,
        rel_path: "opencode.json",
    },
    McpAgentSpec {
        agent: "kiro",
        label: "Kiro (.kiro/settings/mcp.json)",
        format: McpFormat::Json(KIRO_JSON),
        rel_path: ".kiro/settings/mcp.json",
    },
    McpAgentSpec {
        agent: "kilo",
        label: "Kilo Code (.kilo/kilo.json)",
        format: McpFormat::Kilo,
        rel_path: ".kilo/kilo.json",
    },
    McpAgentSpec {
        agent: "zcode",
        label: "ZCode (.zcode/config.json)",
        format: McpFormat::Json(ZCODE_JSON),
        rel_path: ".zcode/config.json",
    },
    McpAgentSpec {
        agent: "copilot",
        label: "GitHub Copilot (.github/mcp.json)",
        format: McpFormat::Json(COPILOT_JSON),
        rel_path: ".github/mcp.json",
    },
    McpAgentSpec {
        agent: "trae",
        label: "Trae (.trae/mcp.json)",
        format: McpFormat::Json(TRAE_JSON),
        rel_path: ".trae/mcp.json",
    },
    McpAgentSpec {
        agent: "qoder",
        label: "Qoder (.qoder/settings.local.json)",
        format: McpFormat::Json(STANDARD_JSON),
        rel_path: ".qoder/settings.local.json",
    },
    McpAgentSpec {
        agent: "codebuddy",
        label: "CodeBuddy (.codebuddy/mcp.json)",
        format: McpFormat::Json(STANDARD_JSON),
        rel_path: ".codebuddy/mcp.json",
    },
    McpAgentSpec {
        agent: "kimi",
        label: "Kimi Code (.kimi-code/mcp.json)",
        format: McpFormat::Json(KIMI_JSON),
        rel_path: ".kimi-code/mcp.json",
    },
    McpAgentSpec {
        agent: "pi",
        label: "Pi (.pi/mcp.json)",
        format: McpFormat::Json(PI_JSON),
        rel_path: ".pi/mcp.json",
    },
    McpAgentSpec {
        agent: "omp",
        label: "OMP (.omp/mcp.json)",
        format: McpFormat::Json(OMP_JSON),
        rel_path: ".omp/mcp.json",
    },
];

/// 一个可管理的配置文件：agent + 绝对路径。
#[derive(Debug, Clone)]
pub struct McpTarget {
    pub spec: McpAgentSpec,
    pub path: PathBuf,
}

pub fn global_targets() -> Vec<McpTarget> {
    GLOBAL_AGENTS
        .iter()
        .map(|spec| McpTarget {
            spec: *spec,
            path: expand_path(spec.rel_path),
        })
        .collect()
}

pub fn project_targets(root: &Path) -> Vec<McpTarget> {
    PROJECT_AGENTS
        .iter()
        .map(|spec| McpTarget {
            spec: *spec,
            path: root.join(spec.rel_path),
        })
        .collect()
}

pub fn global_target(agent: &str) -> Option<McpTarget> {
    global_targets()
        .into_iter()
        .find(|target| target.spec.agent == agent)
}

pub fn project_target(root: &Path, agent: &str) -> Option<McpTarget> {
    project_targets(root)
        .into_iter()
        .find(|target| target.spec.agent == agent)
}

/// 该 agent 的格式是否原生支持 enabled / disabled 字段。
pub fn supports_enabled_flag(spec: &McpAgentSpec) -> bool {
    match spec.format {
        McpFormat::Toml | McpFormat::OpenCode | McpFormat::Kilo => true,
        McpFormat::Json(json) => json.enabled != EnabledStyle::None,
    }
}

pub fn read_servers(target: &McpTarget) -> Result<Vec<McpServerDef>, String> {
    match target.spec.format {
        McpFormat::Json(json) => json_adapter::read(&target.path, json),
        McpFormat::OpenCode => opencode_adapter::read(&target.path),
        McpFormat::Kilo => opencode_adapter::read_flat(&target.path),
        McpFormat::Toml => toml_adapter::read(&target.path),
    }
}

pub fn upsert_server(target: &McpTarget, def: &McpServerDef) -> Result<(), String> {
    match target.spec.format {
        McpFormat::Json(json) => json_adapter::upsert(&target.path, def, json),
        McpFormat::OpenCode => opencode_adapter::upsert(&target.path, def),
        McpFormat::Kilo => opencode_adapter::upsert_flat(&target.path, def),
        McpFormat::Toml => toml_adapter::upsert(&target.path, def),
    }
}

pub fn delete_server(target: &McpTarget, name: &str) -> Result<(), String> {
    match target.spec.format {
        McpFormat::Json(json) => json_adapter::delete(&target.path, name, json),
        McpFormat::OpenCode => opencode_adapter::delete(&target.path, name),
        McpFormat::Kilo => opencode_adapter::delete_flat(&target.path, name),
        McpFormat::Toml => toml_adapter::delete(&target.path, name),
    }
}

/// 只渲染某个配置中的 MCP 部分，用于详情面板预览。
///
/// 不返回整个文件：`~/.claude.json` 等文件还包含账号信息与项目历史。
pub fn raw_section(target: &McpTarget) -> Result<String, String> {
    let defs: Vec<McpServerDef> = read_servers(target)?.iter().map(redact).collect();
    match target.spec.format {
        McpFormat::Json(json) => {
            let mut map = serde_json::Map::new();
            for def in &defs {
                map.insert(def.name.clone(), json_adapter::def_to_value(def, json));
            }
            let root = match json.layout {
                JsonLayout::Root => serde_json::json!({ "mcpServers": map }),
                JsonLayout::Nested => serde_json::json!({ "mcp": { "servers": map } }),
            };
            serde_json::to_string_pretty(&root)
                .map_err(|e| format!("序列化 MCP 配置失败: {}", e))
        }
        McpFormat::OpenCode => {
            let mut map = serde_json::Map::new();
            for def in &defs {
                map.insert(def.name.clone(), opencode_adapter::def_to_value(def));
            }
            let root = serde_json::json!({ "mcp": { "servers": map } });
            serde_json::to_string_pretty(&root)
                .map_err(|e| format!("序列化 MCP 配置失败: {}", e))
        }
        McpFormat::Kilo => {
            let mut map = serde_json::Map::new();
            for def in &defs {
                map.insert(def.name.clone(), opencode_adapter::def_to_value(def));
            }
            let root = serde_json::json!({ "mcp": map });
            serde_json::to_string_pretty(&root)
                .map_err(|e| format!("序列化 MCP 配置失败: {}", e))
        }
        McpFormat::Toml => toml_adapter::render(&defs),
    }
}

pub fn set_server_enabled(target: &McpTarget, name: &str, enabled: bool) -> Result<(), String> {
    match target.spec.format {
        McpFormat::Json(json) => json_adapter::set_enabled(&target.path, name, enabled, json),
        McpFormat::OpenCode => opencode_adapter::set_enabled(&target.path, name, enabled),
        McpFormat::Kilo => opencode_adapter::set_enabled_flat(&target.path, name, enabled),
        McpFormat::Toml => toml_adapter::set_enabled(&target.path, name, enabled),
    }
}

/// 聚合多个配置文件中的同名 server。
///
/// `state_prefix` 为空时用 agent 名作为托管状态 key；项目级用 `project:<id>`
/// 前缀，避免与全局状态互相影响。
pub fn collect_overview(targets: &[McpTarget], state_prefix: &str) -> McpOverview {
    let mut agents = Vec::new();
    let mut errors = Vec::new();
    let mut grouped: BTreeMap<String, Vec<(McpBinding, McpServerDef)>> = BTreeMap::new();

    for target in targets {
        let state_key = state_key(state_prefix, target.spec.agent);
        agents.push(McpAgentInfo {
            agent: target.spec.agent.to_string(),
            label: target.spec.label.to_string(),
            config_path: target.path.to_string_lossy().to_string(),
            exists: target.path.exists(),
        });

        match read_servers(target) {
            Ok(defs) => {
                for mut def in defs {
                    let managed_disabled = state::is_disabled(&state_key, &def.name);
                    if managed_disabled {
                        def.enabled = false;
                    }
                    let enabled = def.enabled;
                    grouped
                        .entry(def.name.clone())
                        .or_default()
                        .push((
                            McpBinding {
                                agent: target.spec.agent.to_string(),
                                config_path: target.path.to_string_lossy().to_string(),
                                editable: true,
                                error: None,
                                managed_disabled,
                                enabled,
                            },
                            def,
                        ));
                }
            }
            Err(error) => errors.push(McpConfigError {
                agent: target.spec.agent.to_string(),
                config_path: target.path.to_string_lossy().to_string(),
                error,
            }),
        }
    }

    let servers = grouped
        .into_iter()
        .map(|(name, entries)| {
            let conflict = entries
                .windows(2)
                .any(|pair| !same_definition(&pair[0].1, &pair[1].1));
            let mut definition = entries[0].1.clone();
            definition.name = name.clone();
            let managed_disabled: Vec<String> = entries
                .iter()
                .filter(|(binding, _)| binding.managed_disabled)
                .map(|(binding, _)| binding.agent.clone())
                .collect();
            let bindings = entries.into_iter().map(|(binding, _)| binding).collect();
            McpServerAggregate {
                name,
                definition,
                bindings,
                conflict,
                managed_disabled,
            }
        })
        .collect();

    McpOverview {
        servers,
        agents,
        errors,
    }
}

pub fn state_key(prefix: &str, agent: &str) -> String {
    if prefix.is_empty() {
        agent.to_string()
    } else {
        format!("{}:{}", prefix, agent)
    }
}

/// 预览原始配置时隐藏 env / headers 的值，避免密钥出现在界面上。
fn redact(def: &McpServerDef) -> McpServerDef {
    let mut clone = def.clone();
    for value in clone.env.values_mut() {
        *value = "••••••".to_string();
    }
    for value in clone.headers.values_mut() {
        *value = "••••••".to_string();
    }
    clone
}

fn same_definition(a: &McpServerDef, b: &McpServerDef) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}

/// 原子写入：内容先落到临时文件，再替换目标；写前保留 `.skills-manager.bak`。
///
/// 调用方必须保证 `content` 已经能被对应格式解析（各 adapter 在写入前会自检）。
pub fn write_atomic(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败 {}: {}", parent.display(), e))?;
    }

    if path.exists() {
        let backup = backup_path(path);
        fs::copy(path, &backup)
            .map_err(|e| format!("备份配置失败 {}: {}", path.display(), e))?;
    }

    let tmp = PathBuf::from(format!("{}.skills-manager.tmp", path.display()));
    fs::write(&tmp, content).map_err(|e| format!("写入临时文件失败: {}", e))?;

    #[cfg(target_os = "windows")]
    if path.exists() {
        // Windows 的 rename 不能覆盖已存在的目标。
        fs::remove_file(path).map_err(|e| format!("替换配置失败: {}", e))?;
    }

    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("替换配置失败 {}: {}", path.display(), e)
    })?;
    Ok(())
}

fn backup_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.skills-manager.bak", path.display()))
}
