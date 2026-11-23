//! MCP 托管停用状态。
//!
//! Claude 的配置没有 `enabled` / `disabled` 字段，无法直接表达"停用但仍保留"。
//! 这类 agent 的开关记录在 `~/.skills-manager/mcp-state.json`，扫描时再应用。

use crate::settings::AppSettingsManager;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct McpState {
    /// state key（agent 或 `project:<id>:agent`） -> 被托管停用的 server 名
    #[serde(default)]
    pub disabled: BTreeMap<String, Vec<String>>,
}

fn state_path() -> std::path::PathBuf {
    AppSettingsManager::get_skills_manager_dir().join("mcp-state.json")
}

pub fn load() -> McpState {
    let Ok(content) = fs::read_to_string(state_path()) else {
        return McpState::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save(state: &McpState) -> Result<(), String> {
    let dir = AppSettingsManager::get_skills_manager_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {}", e))?;
    let content =
        serde_json::to_string_pretty(state).map_err(|e| format!("序列化 MCP 状态失败: {}", e))?;
    let path = state_path();
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content).map_err(|e| format!("写入 MCP 状态失败: {}", e))?;
    #[cfg(target_os = "windows")]
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("替换 MCP 状态失败: {}", e))?;
    }
    fs::rename(&tmp, &path).map_err(|e| format!("替换 MCP 状态失败: {}", e))
}

pub fn is_disabled(state_key: &str, name: &str) -> bool {
    load()
        .disabled
        .get(state_key)
        .map(|items| items.iter().any(|item| item == name))
        .unwrap_or(false)
}

pub fn set_disabled(state_key: &str, name: &str, disabled: bool) -> Result<(), String> {
    let mut state = load();
    let entry = state
        .disabled
        .entry(state_key.to_string())
        .or_default();
    entry.retain(|item| item != name);
    if disabled {
        entry.push(name.to_string());
    }
    if entry.is_empty() {
        state.disabled.remove(state_key);
    }
    save(&state)
}
