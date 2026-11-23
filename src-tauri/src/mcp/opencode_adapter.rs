//! OpenCode 的 `mcp.servers` 适配器。
//!
//! 格式：
//! ```json
//! { "mcp": { "servers": { "name": {
//!     "type": "local", "command": ["npx", "-y", "..."],
//!     "environment": { "KEY": "VALUE" }, "enabled": true
//! } } } }
//! ```

use crate::models::{McpServerDef, McpTransport};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn read_root(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }
    let content =
        fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {}", path.display(), e))?;
    if content.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    serde_json::from_str(&content).map_err(|e| format!("解析 {} 失败: {}", path.display(), e))
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

fn map_to_json(map: &BTreeMap<String, String>) -> Value {
    Value::Object(
        map.iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect(),
    )
}

pub fn read(path: &Path) -> Result<Vec<McpServerDef>, String> {
    let root = read_root(path)?;
    let Some(servers) = root
        .get("mcp")
        .and_then(|mcp| mcp.get("servers"))
        .and_then(Value::as_object)
    else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for (name, value) in servers {
        let Some(obj) = value.as_object() else {
            continue;
        };
        let kind = obj.get("type").and_then(Value::as_str).unwrap_or("local");
        let command_array = obj.get("command").and_then(Value::as_array);
        let command = command_array
            .and_then(|items| items.first())
            .and_then(Value::as_str)
            .map(str::to_string);
        let args = command_array
            .map(|items| {
                items
                    .iter()
                    .skip(1)
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let url = obj.get("url").and_then(Value::as_str).map(str::to_string);
        let transport = if kind == "remote" || url.is_some() {
            McpTransport::Http
        } else {
            McpTransport::Stdio
        };
        let enabled = obj.get("enabled").and_then(Value::as_bool).unwrap_or(true);

        let known = [
            "type",
            "command",
            "environment",
            "url",
            "headers",
            "enabled",
        ];
        let mut extra = Map::new();
        for (key, value) in obj {
            if !known.contains(&key.as_str()) {
                extra.insert(key.clone(), value.clone());
            }
        }

        out.push(McpServerDef {
            name: name.clone(),
            transport,
            command,
            args,
            env: string_map(obj.get("environment")),
            url,
            headers: string_map(obj.get("headers")),
            enabled,
            extra: Value::Object(extra),
        });
    }
    Ok(out)
}

fn servers_mut(root: &mut Value) -> Result<&mut Map<String, Value>, String> {
    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| "OpenCode 配置根节点不是对象".to_string())?;
    let mcp = root_obj
        .entry("mcp".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let mcp_obj = mcp
        .as_object_mut()
        .ok_or_else(|| "OpenCode 配置的 mcp 不是对象".to_string())?;
    let servers = mcp_obj
        .entry("servers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    servers
        .as_object_mut()
        .ok_or_else(|| "OpenCode 配置的 mcp.servers 不是对象".to_string())
}

fn apply_def(obj: &mut Map<String, Value>, def: &McpServerDef) {
    for key in ["type", "command", "environment", "url", "headers", "enabled"] {
        obj.remove(key);
    }

    match def.transport {
        McpTransport::Stdio => {
            obj.insert("type".to_string(), Value::String("local".to_string()));
            let mut command = Vec::new();
            if let Some(program) = &def.command {
                command.push(Value::String(program.clone()));
            }
            command.extend(def.args.iter().cloned().map(Value::String));
            obj.insert("command".to_string(), Value::Array(command));
            if !def.env.is_empty() {
                obj.insert("environment".to_string(), map_to_json(&def.env));
            }
        }
        McpTransport::Sse | McpTransport::Http => {
            obj.insert("type".to_string(), Value::String("remote".to_string()));
            if let Some(url) = &def.url {
                obj.insert("url".to_string(), Value::String(url.clone()));
            }
            if !def.headers.is_empty() {
                obj.insert("headers".to_string(), map_to_json(&def.headers));
            }
        }
    }

    obj.insert("enabled".to_string(), Value::Bool(def.enabled));

    if let Value::Object(extra) = &def.extra {
        for (key, value) in extra {
            obj.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
}

pub fn upsert(path: &Path, def: &McpServerDef) -> Result<(), String> {
    let mut root = read_root(path)?;
    let mut obj = match servers_mut(&mut root)?.get(&def.name) {
        Some(Value::Object(map)) => map.clone(),
        _ => Map::new(),
    };
    apply_def(&mut obj, def);
    servers_mut(&mut root)?.insert(def.name.clone(), Value::Object(obj));

    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 OpenCode 配置失败: {}", e))?;
    serde_json::from_str::<Value>(&content)
        .map_err(|e| format!("生成的 OpenCode 配置无效: {}", e))?;
    super::write_atomic(path, &content)
}

pub fn delete(path: &Path, name: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut root = read_root(path)?;
    let removed = servers_mut(&mut root)
        .ok()
        .and_then(|servers| servers.remove(name))
        .is_some();
    if !removed {
        return Ok(());
    }
    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 OpenCode 配置失败: {}", e))?;
    super::write_atomic(path, &content)
}

pub fn set_enabled(path: &Path, name: &str, enabled: bool) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut root = read_root(path)?;
    let Some(server) = servers_mut(&mut root)
        .ok()
        .and_then(|servers| servers.get_mut(name))
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };
    server.insert("enabled".to_string(), Value::Bool(enabled));
    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 OpenCode 配置失败: {}", e))?;
    super::write_atomic(path, &content)
}
