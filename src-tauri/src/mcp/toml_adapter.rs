//! Codex / Grok 的 `[mcp_servers.*]` TOML 适配器。
//!
//! 使用 `toml_edit` 就地修改，保留注释、顺序以及其它表。

use crate::models::{McpServerDef, McpTransport};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use toml_edit::{Array, Document, Item, Table, Value};

fn load_doc(path: &Path) -> Result<Document, String> {
    if !path.exists() {
        return Ok(Document::new());
    }
    let content =
        fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {}", path.display(), e))?;
    if content.trim().is_empty() {
        return Ok(Document::new());
    }
    content
        .parse::<Document>()
        .map_err(|e| format!("解析 {} 失败: {}", path.display(), e))
}

pub fn read(path: &Path) -> Result<Vec<McpServerDef>, String> {
    let doc = load_doc(path)?;
    let Some(servers) = doc
        .as_table()
        .get("mcp_servers")
        .and_then(Item::as_table)
    else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for (name, item) in servers.iter() {
        let Some(table) = item.as_table() else {
            continue;
        };
        let command = table
            .get("command")
            .and_then(Item::as_str)
            .map(str::to_string);
        let url = table.get("url").and_then(Item::as_str).map(str::to_string);
        let args = table
            .get("args")
            .and_then(Item::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let env: BTreeMap<String, String> = table
            .get("env")
            .and_then(Item::as_table)
            .map(|env| {
                env.iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|s| (key.to_string(), s.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let enabled = table
            .get("enabled")
            .and_then(Item::as_bool)
            .unwrap_or(true);
        let transport = if url.is_some() {
            McpTransport::Http
        } else {
            McpTransport::Stdio
        };
        out.push(McpServerDef {
            name: name.to_string(),
            transport,
            command,
            args,
            env,
            url,
            headers: BTreeMap::new(),
            enabled,
            extra: serde_json::Value::Null,
        });
    }
    Ok(out)
}

fn write_doc(path: &Path, doc: &Document) -> Result<(), String> {
    let content = doc.to_string();
    content
        .parse::<Document>()
        .map_err(|e| format!("生成的 TOML 无效: {}", e))?;
    super::write_atomic(path, &content)
}

pub fn upsert(path: &Path, def: &McpServerDef) -> Result<(), String> {
    let mut doc = load_doc(path)?;

    {
        let table = doc.as_table_mut();
        let servers_item = table
            .entry("mcp_servers")
            .or_insert(Item::Table(Table::new()));
        if !servers_item.is_table() {
            return Err("mcp_servers 不是 TOML 表".to_string());
        }
        let servers = servers_item
            .as_table_mut()
            .ok_or_else(|| "mcp_servers 不是 TOML 表".to_string())?;

        let server_item = servers
            .entry(def.name.as_str())
            .or_insert(Item::Table(Table::new()));
        if !server_item.is_table() {
            return Err(format!("mcp_servers.{} 不是 TOML 表", def.name));
        }
        let server = server_item
            .as_table_mut()
            .ok_or_else(|| format!("mcp_servers.{} 不是 TOML 表", def.name))?;

        for key in ["command", "args", "env", "url", "enabled"] {
            server.remove(key);
        }

        match def.transport {
            McpTransport::Stdio => {
                if let Some(command) = &def.command {
                    server.insert(
                        "command",
                        Item::Value(Value::from(command.as_str())),
                    );
                }
                if !def.args.is_empty() {
                    let mut array = Array::new();
                    for arg in &def.args {
                        array.push(arg.as_str());
                    }
                    server.insert("args", Item::Value(Value::Array(array)));
                }
                if !def.env.is_empty() {
                    let mut env = Table::new();
                    for (key, value) in &def.env {
                        env.insert(key.as_str(), Item::Value(Value::from(value.as_str())));
                    }
                    server.insert("env", Item::Table(env));
                }
            }
            McpTransport::Sse | McpTransport::Http => {
                if let Some(url) = &def.url {
                    server.insert("url", Item::Value(Value::from(url.as_str())));
                }
            }
        }

        server.insert("enabled", Item::Value(Value::from(def.enabled)));
    }

    write_doc(path, &doc)
}

/// 把统一模型渲染成只含 `[mcp_servers.*]` 的 TOML 文本（用于详情面板展示）。
pub fn render(defs: &[McpServerDef]) -> Result<String, String> {
    let mut doc = Document::new();
    for def in defs {
        let mut table = Table::new();
        match def.transport {
            McpTransport::Stdio => {
                if let Some(command) = &def.command {
                    table.insert("command", Item::Value(Value::from(command.as_str())));
                }
                if !def.args.is_empty() {
                    let mut array = Array::new();
                    for arg in &def.args {
                        array.push(arg.as_str());
                    }
                    table.insert("args", Item::Value(Value::Array(array)));
                }
                if !def.env.is_empty() {
                    let mut env = Table::new();
                    for (key, value) in &def.env {
                        env.insert(key.as_str(), Item::Value(Value::from(value.as_str())));
                    }
                    table.insert("env", Item::Table(env));
                }
            }
            McpTransport::Sse | McpTransport::Http => {
                if let Some(url) = &def.url {
                    table.insert("url", Item::Value(Value::from(url.as_str())));
                }
            }
        }
        table.insert("enabled", Item::Value(Value::from(def.enabled)));

        let servers = doc
            .as_table_mut()
            .entry("mcp_servers")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| "mcp_servers 不是 TOML 表".to_string())?;
        servers.insert(def.name.as_str(), Item::Table(table));
    }
    Ok(doc.to_string())
}

pub fn delete(path: &Path, name: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut doc = load_doc(path)?;
    let Some(servers) = doc
        .as_table_mut()
        .get_mut("mcp_servers")
        .and_then(Item::as_table_mut)
    else {
        return Ok(());
    };
    if servers.remove(name).is_none() {
        return Ok(());
    }
    write_doc(path, &doc)
}

pub fn set_enabled(path: &Path, name: &str, enabled: bool) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut doc = load_doc(path)?;
    let Some(server) = doc
        .as_table_mut()
        .get_mut("mcp_servers")
        .and_then(Item::as_table_mut)
        .and_then(|servers| servers.get_mut(name))
        .and_then(Item::as_table_mut)
    else {
        return Ok(());
    };
    server.insert("enabled", Item::Value(Value::from(enabled)));
    write_doc(path, &doc)
}
