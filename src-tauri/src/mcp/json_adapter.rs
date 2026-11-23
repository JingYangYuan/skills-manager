//! `mcpServers` 形式的 JSON 适配器（Claude / Cursor / Antigravity）。

use crate::models::{McpServerDef, McpTransport};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const KNOWN_KEYS: &[&str] = &[
    "type", "command", "args", "env", "url", "headers", "enabled", "disabled",
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

pub fn read(path: &Path) -> Result<Vec<McpServerDef>, String> {
    let root = read_root(path)?;
    let Some(servers) = root.get("mcpServers").and_then(Value::as_object) else {
        return Ok(Vec::new());
    };
    Ok(servers
        .iter()
        .filter_map(|(name, value)| parse_server(name, value))
        .collect())
}

fn parse_server(name: &str, value: &Value) -> Option<McpServerDef> {
    let obj = value.as_object()?;
    let command = obj
        .get("command")
        .and_then(Value::as_str)
        .map(str::to_string);
    let url = obj.get("url").and_then(Value::as_str).map(str::to_string);
    let transport = obj
        .get("type")
        .and_then(Value::as_str)
        .map(|kind| match kind {
            "http" | "streamable-http" => McpTransport::Http,
            "sse" => McpTransport::Sse,
            _ => McpTransport::Stdio,
        })
        .unwrap_or_else(|| {
            if url.is_some() {
                McpTransport::Sse
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
pub fn def_to_value(def: &McpServerDef) -> Value {
    let mut obj = Map::new();
    match def.transport {
        McpTransport::Stdio => {
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
                obj.insert("url".to_string(), Value::String(url.clone()));
            }
            if write_type {
                obj.insert(
                    "type".to_string(),
                    Value::String(
                        match def.transport {
                            McpTransport::Sse => "sse",
                            _ => "http",
                        }
                        .to_string(),
                    ),
                );
            } else if let Some(old_type) = old_type.clone() {
                // Cursor 这类按 URL 推断传输方式的配置：保留原有 type，不主动新增。
                obj.insert("type".to_string(), old_type);
            }
            if !def.headers.is_empty() {
                obj.insert("headers".to_string(), env_to_json(&def.headers));
            }
        }
    }
    obj.insert("enabled".to_string(), Value::Bool(def.enabled));
    Value::Object(obj)
}

fn env_to_json(env: &BTreeMap<String, String>) -> Value {
    Value::Object(
        env.iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect(),
    )
}

fn apply_def(
    obj: &mut Map<String, Value>,
    def: &McpServerDef,
    supports_enabled: bool,
    write_type: bool,
) {
    let had_enabled = obj.contains_key("enabled");
    let had_disabled = obj.contains_key("disabled");
    let old_type = obj.get("type").cloned();
    for key in KNOWN_KEYS {
        obj.remove(*key);
    }

    match def.transport {
        McpTransport::Stdio => {
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
                obj.insert("url".to_string(), Value::String(url.clone()));
            }
            obj.insert(
                "type".to_string(),
                Value::String(
                    match def.transport {
                        McpTransport::Sse => "sse",
                        _ => "http",
                    }
                    .to_string(),
                ),
            );
            if !def.headers.is_empty() {
                obj.insert("headers".to_string(), env_to_json(&def.headers));
            }
        }
    }

    if supports_enabled {
        obj.insert("disabled".to_string(), Value::Bool(!def.enabled));
    } else if def.enabled {
        if had_enabled {
            obj.insert("enabled".to_string(), Value::Bool(true));
        }
    } else if had_disabled {
        obj.insert("disabled".to_string(), Value::Bool(true));
    } else if had_enabled {
        obj.insert("enabled".to_string(), Value::Bool(false));
    }

    if let Value::Object(extra) = &def.extra {
        for (key, value) in extra {
            obj.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
}

pub fn upsert(
    path: &Path,
    def: &McpServerDef,
    supports_enabled: bool,
    write_type: bool,
) -> Result<(), String> {
    let mut root = read_root(path)?;
    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| format!("{} 的根节点不是对象", path.display()))?;
    let servers_value = root_obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let servers = servers_value
        .as_object_mut()
        .ok_or_else(|| format!("{} 的 mcpServers 不是对象", path.display()))?;

    let mut obj = match servers.get(&def.name) {
        Some(Value::Object(map)) => map.clone(),
        _ => Map::new(),
    };
    apply_def(&mut obj, def, supports_enabled, write_type);
    servers.insert(def.name.clone(), Value::Object(obj));

    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 MCP 配置失败: {}", e))?;
    // 写盘前自检，避免把坏 JSON 落盘。
    serde_json::from_str::<Value>(&content)
        .map_err(|e| format!("生成的 MCP 配置无效: {}", e))?;
    super::write_atomic(path, &content)
}

pub fn delete(path: &Path, name: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut root = read_root(path)?;
    let Some(servers) = root
        .as_object_mut()
        .and_then(|obj| obj.get_mut("mcpServers"))
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };
    if servers.remove(name).is_none() {
        return Ok(());
    }
    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 MCP 配置失败: {}", e))?;
    super::write_atomic(path, &content)
}

/// 仅用于原生支持 `disabled` 字段的 JSON agent（Cursor）。
pub fn set_enabled(path: &Path, name: &str, enabled: bool) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut root = read_root(path)?;
    let Some(server) = root
        .as_object_mut()
        .and_then(|obj| obj.get_mut("mcpServers"))
        .and_then(Value::as_object_mut)
        .and_then(|servers| servers.get_mut(name))
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };
    server.remove("enabled");
    server.insert("disabled".to_string(), Value::Bool(!enabled));
    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 MCP 配置失败: {}", e))?;
    super::write_atomic(path, &content)
}
