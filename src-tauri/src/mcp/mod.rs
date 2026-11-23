//! MCP 配置管理。
//!
//! 不同 agent 的 MCP 配置格式差异很大，这里把它们统一成 `McpServerDef`：
//! - Claude / Cursor / Antigravity：`mcpServers` JSON
//! - OpenCode：`mcp.servers` JSON（local / remote）
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
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpFormat {
    Json,
    OpenCode,
    Toml,
}

#[derive(Debug, Clone, Copy)]
pub struct McpAgentSpec {
    pub agent: &'static str,
    pub label: &'static str,
    pub format: McpFormat,
}

pub const GLOBAL_AGENTS: &[McpAgentSpec] = &[
    McpAgentSpec {
        agent: "claude",
        label: "Claude Code",
        format: McpFormat::Json,
    },
    McpAgentSpec {
        agent: "cursor",
        label: "Cursor",
        format: McpFormat::Json,
    },
    McpAgentSpec {
        agent: "antigravity",
        label: "Antigravity / Gemini",
        format: McpFormat::Json,
    },
    McpAgentSpec {
        agent: "opencode",
        label: "OpenCode",
        format: McpFormat::OpenCode,
    },
    McpAgentSpec {
        agent: "codex",
        label: "Codex",
        format: McpFormat::Toml,
    },
    McpAgentSpec {
        agent: "grok",
        label: "Grok",
        format: McpFormat::Toml,
    },
];

pub const PROJECT_AGENTS: &[McpAgentSpec] = &[
    McpAgentSpec {
        agent: "claude",
        label: "Claude Code (.mcp.json)",
        format: McpFormat::Json,
    },
    McpAgentSpec {
        agent: "cursor",
        label: "Cursor (.cursor/mcp.json)",
        format: McpFormat::Json,
    },
    McpAgentSpec {
        agent: "grok",
        label: "Grok (.grok/config.toml)",
        format: McpFormat::Toml,
    },
];

/// 一个可管理的配置文件：agent + 绝对路径。
#[derive(Debug, Clone)]
pub struct McpTarget {
    pub spec: McpAgentSpec,
    pub path: PathBuf,
}

pub fn global_targets() -> Vec<McpTarget> {
    let home = dirs::home_dir().unwrap_or_default();
    GLOBAL_AGENTS
        .iter()
        .map(|spec| {
            let path = match spec.agent {
                "claude" => home.join(".claude.json"),
                "cursor" => home.join(".cursor").join("mcp.json"),
                "antigravity" => home.join(".gemini").join("config").join("mcp_config.json"),
                "opencode" => home.join(".config").join("opencode").join("opencode.json"),
                "codex" => home.join(".codex").join("config.toml"),
                "grok" => home.join(".grok").join("config.toml"),
                _ => home.join(".skills-manager").join("unknown-mcp.json"),
            };
            McpTarget {
                spec: *spec,
                path,
            }
        })
        .collect()
}

pub fn project_targets(root: &Path) -> Vec<McpTarget> {
    PROJECT_AGENTS
        .iter()
        .map(|spec| {
            let path = match spec.agent {
                "claude" => root.join(".mcp.json"),
                "cursor" => root.join(".cursor").join("mcp.json"),
                "grok" => root.join(".grok").join("config.toml"),
                _ => root.join(".mcp.json"),
            };
            McpTarget {
                spec: *spec,
                path,
            }
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

/// 该 agent 的格式是否原生支持 enabled 字段。
pub fn supports_enabled_flag(spec: &McpAgentSpec) -> bool {
    match spec.format {
        McpFormat::Toml => true,
        McpFormat::OpenCode => true,
        McpFormat::Json => spec.agent == "cursor",
    }
}

pub fn read_servers(target: &McpTarget) -> Result<Vec<McpServerDef>, String> {
    match target.spec.format {
        McpFormat::Json => json_adapter::read(&target.path),
        McpFormat::OpenCode => opencode_adapter::read(&target.path),
        McpFormat::Toml => toml_adapter::read(&target.path),
    }
}

pub fn upsert_server(target: &McpTarget, def: &McpServerDef) -> Result<(), String> {
    match target.spec.format {
        McpFormat::Json => {
            let write_type = matches!(target.spec.agent, "claude" | "antigravity");
            json_adapter::upsert(
                &target.path,
                def,
                supports_enabled_flag(&target.spec),
                write_type,
            )
        }
        McpFormat::OpenCode => opencode_adapter::upsert(&target.path, def),
        McpFormat::Toml => toml_adapter::upsert(&target.path, def),
    }
}

pub fn delete_server(target: &McpTarget, name: &str) -> Result<(), String> {
    match target.spec.format {
        McpFormat::Json => json_adapter::delete(&target.path, name),
        McpFormat::OpenCode => opencode_adapter::delete(&target.path, name),
        McpFormat::Toml => toml_adapter::delete(&target.path, name),
    }
}

/// 只渲染某个配置中的 MCP 部分，用于详情面板预览。
///
/// 不返回整个文件：`~/.claude.json` 等文件还包含账号信息与项目历史。
pub fn raw_section(target: &McpTarget) -> Result<String, String> {
    let defs = read_servers(target)?;
    match target.spec.format {
        McpFormat::Json => {
            let mut map = serde_json::Map::new();
            for def in &defs {
                map.insert(def.name.clone(), json_adapter::def_to_value(def));
            }
            let root = serde_json::json!({ "mcpServers": map });
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
        McpFormat::Toml => toml_adapter::render(&defs),
    }
}

pub fn set_server_enabled(target: &McpTarget, name: &str, enabled: bool) -> Result<(), String> {
    match target.spec.format {
        McpFormat::Json => json_adapter::set_enabled(&target.path, name, enabled),
        McpFormat::OpenCode => opencode_adapter::set_enabled(&target.path, name, enabled),
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
