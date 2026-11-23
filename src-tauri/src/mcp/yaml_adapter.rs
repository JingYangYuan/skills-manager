//! Hermes Agent 的 `~/.hermes/config.yaml` 适配器。
//!
//! Hermes 的 MCP server 位于顶层 `mcp_servers` 映射下，字段与其它 Agent 接近：
//! `command` / `args` / `env` / `url` / `headers` / `transport` / `enabled`。
//!
//! 写回时只替换 `mcp_servers` 这一段，其它配置段与注释保持不变。

use crate::models::{McpServerDef, McpTransport};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const KEY: &str = "mcp_servers";

const KNOWN_KEYS: &[&str] = &[
    "command",
    "args",
    "env",
    "url",
    "headers",
    "transport",
    "enabled",
    "disabled",
];

fn read_text(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {}", path.display(), e))
}

fn parse_root(path: &Path, text: &str) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Ok(Value::Mapping(Mapping::new()));
    }
    let root: Value = serde_yaml::from_str(text)
        .map_err(|e| format!("解析 {} 失败: {}", path.display(), e))?;
    if !root.is_mapping() {
        return Err(format!("{} 的根节点不是映射", path.display()));
    }
    Ok(root)
}

fn get<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    map.get(&Value::String(key.to_string()))
}

fn string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_mapping)
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| {
                    Some((key.as_str()?.to_string(), value.as_str()?.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn map_to_yaml(map: &BTreeMap<String, String>) -> Value {
    let mut out = Mapping::new();
    for (key, value) in map {
        out.insert(
            Value::String(key.clone()),
            Value::String(value.clone()),
        );
    }
    Value::Mapping(out)
}

pub fn read(path: &Path) -> Result<Vec<McpServerDef>, String> {
    let text = read_text(path)?;
    let root = parse_root(path, &text)?;
    let Some(servers) = root
        .as_mapping()
        .and_then(|map| get(map, KEY))
        .and_then(Value::as_mapping)
    else {
        return Ok(Vec::new());
    };
    Ok(servers
        .iter()
        .filter_map(|(name, value)| parse_server(name.as_str()?, value))
        .collect())
}

fn parse_server(name: &str, value: &Value) -> Option<McpServerDef> {
    let map = value.as_mapping()?;
    let command = get(map, "command")
        .and_then(Value::as_str)
        .map(str::to_string);
    let url = get(map, "url")
        .and_then(Value::as_str)
        .map(str::to_string);
    let transport = match get(map, "transport").and_then(Value::as_str) {
        Some("sse") => McpTransport::Sse,
        Some("http") | Some("streamable-http") | Some("streamable_http") => McpTransport::Http,
        Some(_) => McpTransport::Stdio,
        None if url.is_some() => McpTransport::Http,
        None => McpTransport::Stdio,
    };
    let args = get(map, "args")
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let enabled = match get(map, "enabled").and_then(Value::as_bool) {
        Some(value) => value,
        None => !get(map, "disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    let mut extra = Mapping::new();
    for (key, value) in map {
        if let Some(key) = key.as_str() {
            if !KNOWN_KEYS.contains(&key) {
                extra.insert(Value::String(key.to_string()), value.clone());
            }
        }
    }
    Some(McpServerDef {
        name: name.to_string(),
        transport,
        command,
        args,
        env: string_map(get(map, "env")),
        url,
        headers: string_map(get(map, "headers")),
        enabled,
        extra: serde_json::to_value(Value::Mapping(extra)).unwrap_or(serde_json::Value::Null),
    })
}

/// 把统一模型渲染成 Hermes 的 server 映射（只含 MCP 字段）。
pub fn def_to_value(def: &McpServerDef) -> Value {
    let mut map = Mapping::new();
    match def.transport {
        McpTransport::Stdio => {
            if let Some(command) = &def.command {
                map.insert(
                    Value::String("command".to_string()),
                    Value::String(command.clone()),
                );
            }
            if !def.args.is_empty() {
                map.insert(
                    Value::String("args".to_string()),
                    Value::Sequence(
                        def.args
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
            if !def.env.is_empty() {
                map.insert(Value::String("env".to_string()), map_to_yaml(&def.env));
            }
        }
        McpTransport::Sse | McpTransport::Http => {
            if let Some(url) = &def.url {
                map.insert(
                    Value::String("url".to_string()),
                    Value::String(url.clone()),
                );
            }
            if def.transport == McpTransport::Sse {
                map.insert(
                    Value::String("transport".to_string()),
                    Value::String("sse".to_string()),
                );
            }
            if !def.headers.is_empty() {
                map.insert(
                    Value::String("headers".to_string()),
                    map_to_yaml(&def.headers),
                );
            }
        }
    }
    map.insert(
        Value::String("enabled".to_string()),
        Value::Bool(def.enabled),
    );
    if let serde_json::Value::Object(extra) = &def.extra {
        for (key, value) in extra {
            if value.is_null() {
                continue;
            }
            let key = Value::String(key.clone());
            if !map.contains_key(&key) {
                if let Ok(value) = serde_yaml::to_value(value) {
                    map.insert(key, value);
                }
            }
        }
    }
    Value::Mapping(map)
}

fn apply_def(map: &mut Mapping, def: &McpServerDef) {
    for key in KNOWN_KEYS {
        map.remove(&Value::String((*key).to_string()));
    }

    match def.transport {
        McpTransport::Stdio => {
            if let Some(command) = &def.command {
                map.insert(
                    Value::String("command".to_string()),
                    Value::String(command.clone()),
                );
            }
            if !def.args.is_empty() {
                map.insert(
                    Value::String("args".to_string()),
                    Value::Sequence(
                        def.args
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
            if !def.env.is_empty() {
                map.insert(Value::String("env".to_string()), map_to_yaml(&def.env));
            }
        }
        McpTransport::Sse | McpTransport::Http => {
            if let Some(url) = &def.url {
                map.insert(
                    Value::String("url".to_string()),
                    Value::String(url.clone()),
                );
            }
            // Hermes 默认使用 Streamable HTTP，只有 SSE 需要显式 transport。
            if def.transport == McpTransport::Sse {
                map.insert(
                    Value::String("transport".to_string()),
                    Value::String("sse".to_string()),
                );
            }
            if !def.headers.is_empty() {
                map.insert(
                    Value::String("headers".to_string()),
                    map_to_yaml(&def.headers),
                );
            }
        }
    }

    map.insert(
        Value::String("enabled".to_string()),
        Value::Bool(def.enabled),
    );

    if let serde_json::Value::Object(extra) = &def.extra {
        for (key, value) in extra {
            if value.is_null() {
                continue;
            }
            let key = Value::String(key.clone());
            if !map.contains_key(&key) {
                if let Ok(value) = serde_yaml::to_value(value) {
                    map.insert(key, value);
                }
            }
        }
    }
}

/// 把 `mcp_servers` 映射渲染成带缩进的 YAML 段。
fn render_block(servers: &Mapping) -> Result<String, String> {
    if servers.is_empty() {
        return Ok("mcp_servers: {}\n".to_string());
    }
    let yaml = serde_yaml::to_string(&Value::Mapping(servers.clone()))
        .map_err(|e| format!("序列化 Hermes MCP 配置失败: {}", e))?;
    let yaml = yaml.strip_prefix("---\n").unwrap_or(&yaml);
    let mut out = String::from("mcp_servers:\n");
    for line in yaml.lines() {
        if line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str("  ");
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(out)
}

/// 替换文本中的顶层 `mcp_servers` 段；找不到时追加到末尾。
fn replace_block(text: &str, block: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines
        .iter()
        .position(|line| line.starts_with("mcp_servers:"))
    else {
        let mut out = text.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(block);
        return out;
    };

    let mut end = start + 1;
    while end < lines.len() {
        let line = lines[end];
        // 下一个顶层键（或顶层注释）结束当前段；空行与缩进行属于该段。
        if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        end += 1;
    }

    let mut out = String::new();
    for line in &lines[..start] {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(block);
    for line in &lines[end..] {
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn upsert(path: &Path, def: &McpServerDef) -> Result<(), String> {
    let text = read_text(path)?;
    let mut root = parse_root(path, &text)?;
    let root_map = root
        .as_mapping_mut()
        .ok_or_else(|| format!("{} 的根节点不是映射", path.display()))?;
    if !root_map.contains_key(&Value::String(KEY.to_string())) {
        root_map.insert(
            Value::String(KEY.to_string()),
            Value::Mapping(Mapping::new()),
        );
    }
    let servers = root_map
        .get_mut(&Value::String(KEY.to_string()))
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| format!("{} 的 mcp_servers 不是映射", path.display()))?;
    let mut entry = match servers.get(&Value::String(def.name.clone())) {
        Some(Value::Mapping(map)) => map.clone(),
        _ => Mapping::new(),
    };
    apply_def(&mut entry, def);
    servers.insert(Value::String(def.name.clone()), Value::Mapping(entry));

    let new_text = replace_block(&text, &render_block(servers)?);
    serde_yaml::from_str::<Value>(&new_text)
        .map_err(|e| format!("生成的 Hermes 配置无效: {}", e))?;
    super::write_atomic(path, &new_text)
}

pub fn delete(path: &Path, name: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let text = read_text(path)?;
    let mut root = parse_root(path, &text)?;
    let Some(root_map) = root.as_mapping_mut() else {
        return Ok(());
    };
    let Some(servers) = root_map
        .get_mut(&Value::String(KEY.to_string()))
        .and_then(Value::as_mapping_mut)
    else {
        return Ok(());
    };
    if servers
        .remove(&Value::String(name.to_string()))
        .is_none()
    {
        return Ok(());
    }

    let new_text = replace_block(&text, &render_block(servers)?);
    serde_yaml::from_str::<Value>(&new_text)
        .map_err(|e| format!("生成的 Hermes 配置无效: {}", e))?;
    super::write_atomic(path, &new_text)
}

pub fn set_enabled(path: &Path, name: &str, enabled: bool) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let text = read_text(path)?;
    let mut root = parse_root(path, &text)?;
    let Some(root_map) = root.as_mapping_mut() else {
        return Ok(());
    };
    let Some(servers) = root_map
        .get_mut(&Value::String(KEY.to_string()))
        .and_then(Value::as_mapping_mut)
    else {
        return Ok(());
    };
    let Some(server) = servers
        .get_mut(&Value::String(name.to_string()))
        .and_then(Value::as_mapping_mut)
    else {
        return Ok(());
    };
    server.insert(
        Value::String("enabled".to_string()),
        Value::Bool(enabled),
    );
    server.remove(&Value::String("disabled".to_string()));

    let new_text = replace_block(&text, &render_block(servers)?);
    serde_yaml::from_str::<Value>(&new_text)
        .map_err(|e| format!("生成的 Hermes 配置无效: {}", e))?;
    super::write_atomic(path, &new_text)
}

/// 渲染原始配置预览（只含 `mcp_servers` 段）。
pub fn render(defs: &[McpServerDef]) -> Result<String, String> {
    let mut servers = Mapping::new();
    for def in defs {
        servers.insert(
            Value::String(def.name.clone()),
            def_to_value(def),
        );
    }
    render_block(&servers)
}
