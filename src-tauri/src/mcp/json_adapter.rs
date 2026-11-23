//! `mcpServers` / `mcp.servers` 形式的 JSON 适配器。
//!
//! 不同 agent 的字段命名略有差异，用 [`JsonSpec`] 描述：
//! - Claude / Cursor / Pi / OMP：顶层 `mcpServers`
//! - ZCode：`mcp.servers`
//! - Antigravity：远程地址用 `serverUrl`
//! - WorkBuddy：远程 type 为 `streamableHttp`
//! - 启用状态：`enabled` / `disabled` / 无

use crate::models::{McpServerDef, McpTransport};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// server 列表在配置文件中的位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonLayout {
    /// 顶层 `mcpServers`
    Root,
    /// `mcp.servers`
    Nested,
}

/// 该格式如何表达启用状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnabledStyle {
    /// 没有原生字段，由 Skills Manager 托管停用状态
    None,
    /// `enabled: bool`
    Enabled,
    /// `disabled: bool`
    Disabled,
}

/// 一个 JSON MCP 配置的字段约定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonSpec {
    pub layout: JsonLayout,
    /// 远程地址字段名：`url` 或 `serverUrl`
    pub url_key: &'static str,
    /// 传输类型字段名：大部分是 `type`，Kimi 用 `transport`
    pub type_key: &'static str,
    /// 远程 server 写入的 `type` 值；`None` 表示不写（由字段推断）
    pub http_type: Option<&'static str>,
    /// SSE server 写入的 `type` 值；Kimi 只支持 `sse`，Pi 等不支持 SSE
    pub sse_type: Option<&'static str>,
    /// stdio server 写入的 `type` 值；`None` 表示不写
    pub stdio_type: Option<&'static str>,
    pub enabled: EnabledStyle,
}

const KNOWN_KEYS: &[&str] = &[
    "type",
    "transport",
    "command",
    "args",
    "env",
    "url",
    "serverUrl",
    "headers",
    "enabled",
    "disabled",
];

fn read_root(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }
    let content =
        fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {}", path.display(), e))?;
    if content.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    let root: Value = serde_json::from_str(&content)
        .map_err(|e| format!("解析 {} 失败: {}", path.display(), e))?;
    if !root.is_object() {
        return Err(format!("{} 的根节点不是对象", path.display()));
    }
    Ok(root)
}

fn string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| value.as_str().map(|s| (key.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn servers_ref<'a>(root: &'a Value, spec: JsonSpec) -> Option<&'a Map<String, Value>> {
    match spec.layout {
        JsonLayout::Root => root.get("mcpServers").and_then(Value::as_object),
        JsonLayout::Nested => root
            .get("mcp")
            .and_then(|mcp| mcp.get("servers"))
            .and_then(Value::as_object),
    }
}

pub fn read(path: &Path, spec: JsonSpec) -> Result<Vec<McpServerDef>, String> {
    let root = read_root(path)?;
    let Some(servers) = servers_ref(&root, spec) else {
        return Ok(Vec::new());
    };
    Ok(servers
        .iter()
        .filter_map(|(name, value)| parse_server(name, value, spec))
        .collect())
}

fn parse_server(name: &str, value: &Value, spec: JsonSpec) -> Option<McpServerDef> {
    let obj = value.as_object()?;
    let command = obj
        .get("command")
        .and_then(Value::as_str)
        .map(str::to_string);
    // 兼容其它 agent 常见的远程地址字段，便于从旧配置读取。
    let url = obj
        .get(spec.url_key)
        .or_else(|| obj.get("url"))
        .or_else(|| obj.get("serverUrl"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let transport = obj
        .get(spec.type_key)
        .and_then(Value::as_str)
        .map(|kind| match kind {
            "http" | "streamable-http" | "streamableHttp" => McpTransport::Http,
            "sse" => McpTransport::Sse,
            _ => McpTransport::Stdio,
        })
        .unwrap_or_else(|| {
            if url.is_some() {
                McpTransport::Http
            } else {
                McpTransport::Stdio
            }
        });
    let args = obj
        .get("args")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let enabled = match obj.get("enabled").and_then(Value::as_bool) {
        Some(value) => value,
        None => !obj
            .get("disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    let mut extra = Map::new();
    for (key, value) in obj {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            extra.insert(key.clone(), value.clone());
        }
    }
    Some(McpServerDef {
        name: name.to_string(),
        transport,
        command,
        args,
        env: string_map(obj.get("env")),
        url,
        headers: string_map(obj.get("headers")),
        enabled,
        extra: Value::Object(extra),
    })
}

/// 把统一模型渲染成便于查看的 JSON 对象（只含 MCP 字段）。
pub fn def_to_value(def: &McpServerDef, spec: JsonSpec) -> Value {
    let mut obj = Map::new();
    match def.transport {
        McpTransport::Stdio => {
            if let Some(stdio_type) = spec.stdio_type {
                obj.insert(spec.type_key.to_string(), Value::String(stdio_type.to_string()));
            }
            if let Some(command) = &def.command {
                obj.insert("command".to_string(), Value::String(command.clone()));
            }
            if !def.args.is_empty() {
                obj.insert(
                    "args".to_string(),
                    Value::Array(def.args.iter().cloned().map(Value::String).collect()),
                );
            }
            if !def.env.is_empty() {
                obj.insert("env".to_string(), env_to_json(&def.env));
            }
        }
        McpTransport::Sse | McpTransport::Http => {
            if let Some(url) = &def.url {
                obj.insert(spec.url_key.to_string(), Value::String(url.clone()));
            }
            let type_value = match def.transport {
                McpTransport::Sse => spec.sse_type,
                _ => spec.http_type,
            };
            if let Some(type_value) = type_value {
                obj.insert(
                    spec.type_key.to_string(),
                    Value::String(type_value.to_string()),
                );
            }
            if !def.headers.is_empty() {
                obj.insert("headers".to_string(), env_to_json(&def.headers));
            }
        }
    }
    match spec.enabled {
        EnabledStyle::None => {}
        EnabledStyle::Enabled => {
            obj.insert("enabled".to_string(), Value::Bool(def.enabled));
        }
        EnabledStyle::Disabled => {
            obj.insert("disabled".to_string(), Value::Bool(!def.enabled));
        }
    }
    Value::Object(obj)
}

fn env_to_json(env: &BTreeMap<String, String>) -> Value {
    Value::Object(
        env.iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect(),
    )
}

fn apply_def(obj: &mut Map<String, Value>, def: &McpServerDef, spec: JsonSpec) {
    let old_type = obj.get(spec.type_key).cloned();
    let had_enabled = obj.contains_key("enabled");
    let had_disabled = obj.contains_key("disabled");
    for key in KNOWN_KEYS {
        obj.remove(*key);
    }

    match def.transport {
        McpTransport::Stdio => {
            if let Some(stdio_type) = spec.stdio_type {
                obj.insert(spec.type_key.to_string(), Value::String(stdio_type.to_string()));
            }
            if let Some(command) = &def.command {
                obj.insert("command".to_string(), Value::String(command.clone()));
            }
            if !def.args.is_empty() {
                obj.insert(
                    "args".to_string(),
                    Value::Array(def.args.iter().cloned().map(Value::String).collect()),
                );
            }
            if !def.env.is_empty() {
                obj.insert("env".to_string(), env_to_json(&def.env));
            }
        }
        McpTransport::Sse | McpTransport::Http => {
            if let Some(url) = &def.url {
                obj.insert(spec.url_key.to_string(), Value::String(url.clone()));
            }
            let type_value = match def.transport {
                McpTransport::Sse => spec.sse_type,
                _ => spec.http_type,
            };
            if let Some(type_value) = type_value {
                obj.insert(
                    spec.type_key.to_string(),
                    Value::String(type_value.to_string()),
                );
            } else if spec.http_type.is_none()
                && spec.sse_type.is_none()
                && old_type.is_some()
            {
                // Cursor / Antigravity / Kiro 这类完全按字段推断传输方式的配置：
                // 保留原有 type，但不主动新增。
                obj.insert(spec.type_key.to_string(), old_type.unwrap());
            }
            if !def.headers.is_empty() {
                obj.insert("headers".to_string(), env_to_json(&def.headers));
            }
        }
    }

    match spec.enabled {
        EnabledStyle::None => {
            // 没有原生开关的格式（如 Claude / Trae）：只在文件原本就有开关字段时
            // 同步它的值，绝不主动新增，避免写入对方不认识的字段。
            if def.enabled {
                if had_enabled {
                    obj.insert("enabled".to_string(), Value::Bool(true));
                }
            } else if had_disabled {
                obj.insert("disabled".to_string(), Value::Bool(true));
            } else if had_enabled {
                obj.insert("enabled".to_string(), Value::Bool(false));
            }
        }
        EnabledStyle::Enabled => {
            obj.insert("enabled".to_string(), Value::Bool(def.enabled));
        }
        EnabledStyle::Disabled => {
            obj.insert("disabled".to_string(), Value::Bool(!def.enabled));
        }
    }

    if let Value::Object(extra) = &def.extra {
        for (key, value) in extra {
            obj.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
}

fn servers_mut<'a>(
    root: &'a mut Value,
    spec: JsonSpec,
    path: &Path,
) -> Result<&'a mut Map<String, Value>, String> {
    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| format!("{} 的根节点不是对象", path.display()))?;
    let servers_value = match spec.layout {
        JsonLayout::Root => root_obj
            .entry("mcpServers".to_string())
            .or_insert_with(|| Value::Object(Map::new())),
        JsonLayout::Nested => {
            let mcp = root_obj
                .entry("mcp".to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            let mcp_obj = mcp
                .as_object_mut()
                .ok_or_else(|| format!("{} 的 mcp 不是对象", path.display()))?;
            mcp_obj
                .entry("servers".to_string())
                .or_insert_with(|| Value::Object(Map::new()))
        }
    };
    servers_value
        .as_object_mut()
        .ok_or_else(|| format!("{} 的 MCP server 列表不是对象", path.display()))
}

pub fn upsert(path: &Path, def: &McpServerDef, spec: JsonSpec) -> Result<(), String> {
    let mut root = read_root(path)?;
    let existing = servers_mut(&mut root, spec, path)?.get(&def.name).cloned();
    let mut obj = match existing {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    apply_def(&mut obj, def, spec);
    servers_mut(&mut root, spec, path)?.insert(def.name.clone(), Value::Object(obj));

    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 MCP 配置失败: {}", e))?;
    // 写盘前自检，避免把坏 JSON 落盘。
    serde_json::from_str::<Value>(&content)
        .map_err(|e| format!("生成的 MCP 配置无效: {}", e))?;
    super::write_atomic(path, &content)
}

pub fn delete(path: &Path, name: &str, spec: JsonSpec) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut root = read_root(path)?;
    let removed = servers_mut(&mut root, spec, path)
        .ok()
        .and_then(|servers| servers.remove(name))
        .is_some();
    if !removed {
        return Ok(());
    }
    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 MCP 配置失败: {}", e))?;
    super::write_atomic(path, &content)
}

pub fn set_enabled(path: &Path, name: &str, enabled: bool, spec: JsonSpec) -> Result<(), String> {
    if !path.exists() || spec.enabled == EnabledStyle::None {
        return Ok(());
    }
    let mut root = read_root(path)?;
    let Some(server) = servers_mut(&mut root, spec, path)
        .ok()
        .and_then(|servers| servers.get_mut(name))
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };
    match spec.enabled {
        EnabledStyle::None => return Ok(()),
        EnabledStyle::Enabled => {
            server.remove("disabled");
            server.insert("enabled".to_string(), Value::Bool(enabled));
        }
        EnabledStyle::Disabled => {
            server.remove("enabled");
            server.insert("disabled".to_string(), Value::Bool(!enabled));
        }
    }
    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 MCP 配置失败: {}", e))?;
    super::write_atomic(path, &content)
}
