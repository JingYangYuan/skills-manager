//! MCP 管理的 Tauri 命令（全局 + 项目级）。

use crate::mcp;
use crate::models::{McpOverview, McpServerDef, McpTransport, ProjectConfig};
use crate::projects;
use crate::state::AppState;
use log::info;
use std::path::PathBuf;
use tauri::State;

fn validate_server(server: &McpServerDef) -> Result<(), String> {
    match server.transport {
        McpTransport::Stdio => {
            if server.command.as_deref().unwrap_or("").trim().is_empty() {
                return Err("stdio 类型的 MCP server 需要 command".to_string());
            }
        }
        McpTransport::Sse | McpTransport::Http => {
            if server.url.as_deref().unwrap_or("").trim().is_empty() {
                return Err("远程 MCP server 需要 url".to_string());
            }
        }
    }
    Ok(())
}

fn project_root(state: &AppState, project_id: &str) -> Result<(ProjectConfig, PathBuf), String> {
    let settings = state
        .settings_manager
        .lock()
        .map_err(|e| format!("Failed to acquire lock: {}", e))?;
    let project = settings
        .get_config()
        .projects
        .iter()
        .find(|project| project.id == project_id)
        .cloned()
        .ok_or_else(|| format!("项目不存在: {}", project_id))?;
    let root = projects::normalize_project_path(&project.path);
    Ok((project, root))
}

// ========== 全局 ==========

#[tauri::command]
pub async fn list_mcp_servers() -> Result<McpOverview, String> {
    Ok(mcp::collect_overview(&mcp::global_targets(), ""))
}

#[tauri::command]
pub async fn save_mcp_server(server: McpServerDef, targets: Vec<String>) -> Result<(), String> {
    let name = server.name.trim().to_string();
    if name.is_empty() {
        return Err("MCP server 名称不能为空".to_string());
    }
    if targets.is_empty() {
        return Err("请至少选择一个目标 agent".to_string());
    }
    let mut server = server;
    server.name = name;
    validate_server(&server)?;

    for agent in &targets {
        let target = mcp::global_target(agent)
            .ok_or_else(|| format!("不支持的 MCP agent: {}", agent))?;
        mcp::upsert_server(&target, &server)?;
        let state_key = mcp::state_key("", agent);
        if mcp::supports_enabled_flag(&target.spec) {
            mcp::state::set_disabled(&state_key, &server.name, false)?;
        } else {
            mcp::state::set_disabled(&state_key, &server.name, !server.enabled)?;
        }
    }
    info!("MCP server '{}' saved to {:?}", server.name, targets);
    Ok(())
}

#[tauri::command]
pub async fn delete_mcp_server(name: String, targets: Vec<String>) -> Result<(), String> {
    if targets.is_empty() {
        return Err("请至少选择一个目标 agent".to_string());
    }
    for agent in &targets {
        let target = mcp::global_target(agent)
            .ok_or_else(|| format!("不支持的 MCP agent: {}", agent))?;
        mcp::delete_server(&target, &name)?;
        mcp::state::set_disabled(&mcp::state_key("", agent), &name, false)?;
    }
    info!("MCP server '{}' deleted from {:?}", name, targets);
    Ok(())
}

#[tauri::command]
pub async fn set_mcp_server_enabled(
    name: String,
    enabled: bool,
    targets: Vec<String>,
) -> Result<(), String> {
    if targets.is_empty() {
        return Err("请至少选择一个目标 agent".to_string());
    }
    for agent in &targets {
        let target = mcp::global_target(agent)
            .ok_or_else(|| format!("不支持的 MCP agent: {}", agent))?;
        let state_key = mcp::state_key("", agent);
        if mcp::supports_enabled_flag(&target.spec) {
            mcp::set_server_enabled(&target, &name, enabled)?;
            mcp::state::set_disabled(&state_key, &name, false)?;
        } else {
            mcp::state::set_disabled(&state_key, &name, !enabled)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn sync_mcp_server(
    name: String,
    from_agent: String,
    to_agents: Vec<String>,
) -> Result<(), String> {
    let source = mcp::global_target(&from_agent)
        .ok_or_else(|| format!("不支持的 MCP agent: {}", from_agent))?;
    let definition = mcp::read_servers(&source)?
        .into_iter()
        .find(|server| server.name == name)
        .ok_or_else(|| format!("在 {} 中找不到 MCP server '{}'", from_agent, name))?;

    for agent in &to_agents {
        if agent == &from_agent {
            continue;
        }
        let target = mcp::global_target(agent)
            .ok_or_else(|| format!("不支持的 MCP agent: {}", agent))?;
        mcp::upsert_server(&target, &definition)?;
        let state_key = mcp::state_key("", agent);
        if mcp::supports_enabled_flag(&target.spec) {
            mcp::state::set_disabled(&state_key, &name, false)?;
        } else {
            mcp::state::set_disabled(&state_key, &name, !definition.enabled)?;
        }
    }
    info!("MCP server '{}' synced from {} to {:?}", name, from_agent, to_agents);
    Ok(())
}

#[tauri::command]
pub async fn get_mcp_raw_config(agent: String) -> Result<String, String> {
    let target = mcp::global_target(&agent)
        .ok_or_else(|| format!("不支持的 MCP agent: {}", agent))?;
    mcp::raw_section(&target)
}

// ========== 项目级 ==========

#[tauri::command]
pub async fn list_project_mcp(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<McpOverview, String> {
    let (_project, root) = project_root(&state, &project_id)?;
    let prefix = format!("project:{}", project_id);
    Ok(mcp::collect_overview(&mcp::project_targets(&root), &prefix))
}

#[tauri::command]
pub async fn get_project_mcp_raw_config(
    project_id: String,
    target: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let (_project, root) = project_root(&state, &project_id)?;
    let mcp_target = mcp::project_target(&root, &target)
        .ok_or_else(|| format!("项目不支持该 MCP 目标: {}", target))?;
    mcp::raw_section(&mcp_target)
}

#[tauri::command]
pub async fn save_project_mcp(
    project_id: String,
    server: McpServerDef,
    target: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (_project, root) = project_root(&state, &project_id)?;
    let mcp_target = mcp::project_target(&root, &target)
        .ok_or_else(|| format!("项目不支持该 MCP 目标: {}", target))?;
    let mut server = server;
    server.name = server.name.trim().to_string();
    if server.name.is_empty() {
        return Err("MCP server 名称不能为空".to_string());
    }
    validate_server(&server)?;
    mcp::upsert_server(&mcp_target, &server)?;

    let state_key = mcp::state_key(&format!("project:{}", project_id), &target);
    if mcp::supports_enabled_flag(&mcp_target.spec) {
        mcp::state::set_disabled(&state_key, &server.name, false)?;
    } else {
        mcp::state::set_disabled(&state_key, &server.name, !server.enabled)?;
    }
    info!(
        "Project '{}' MCP server '{}' saved to {}",
        project_id, server.name, target
    );
    Ok(())
}

#[tauri::command]
pub async fn delete_project_mcp(
    project_id: String,
    name: String,
    target: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (_project, root) = project_root(&state, &project_id)?;
    let mcp_target = mcp::project_target(&root, &target)
        .ok_or_else(|| format!("项目不支持该 MCP 目标: {}", target))?;
    mcp::delete_server(&mcp_target, &name)?;
    mcp::state::set_disabled(
        &mcp::state_key(&format!("project:{}", project_id), &target),
        &name,
        false,
    )?;
    Ok(())
}
