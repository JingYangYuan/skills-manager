//! MCP 市场的 Tauri 命令。
//!
//! 数据来自官方 MCP Registry（实时搜索/详情）+ `featured.json`（内置精选）。
//! 安装流程：拉取最新详情 → 重建安装方式 → 校验必填输入 → 预检同名冲突 →
//! 写入全局 Agent / 项目配置 → 记录安装（只记录目标与版本，不记录密钥值）。

use crate::commands::mcp::{
    apply_server_to_global_target, apply_server_to_project_target, project_root, validate_server,
};
use crate::mcp;
use crate::mcp::registry::{
    self, McpInstallRecord, McpInstallRequest, McpInstallResult, McpMarketplaceDetail,
    McpMarketplaceItem, McpMarketplacePage, McpProjectTarget,
};
use crate::state::AppState;
use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn fetch_mcp_marketplace_featured() -> Result<Vec<McpMarketplaceItem>, String> {
    registry::fetch_featured()
}

#[tauri::command]
pub async fn fetch_mcp_marketplace_servers(
    query: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<McpMarketplacePage, String> {
    tokio::task::spawn_blocking(move || registry::fetch_servers(query, cursor, limit))
        .await
        .map_err(|error| format!("MCP 市场请求被中断: {}", error))?
}

#[tauri::command]
pub async fn fetch_mcp_marketplace_detail(
    registry_name: String,
) -> Result<McpMarketplaceDetail, String> {
    tokio::task::spawn_blocking(move || registry::fetch_detail(&registry_name))
        .await
        .map_err(|error| format!("MCP 市场请求被中断: {}", error))?
}

#[tauri::command]
pub async fn list_mcp_marketplace_installs() -> Result<Vec<McpInstallRecord>, String> {
    Ok(registry::list_installs())
}

#[tauri::command]
pub async fn forget_mcp_marketplace_install(
    registry_name: String,
    local_name: String,
) -> Result<(), String> {
    registry::remove_install(registry_name.trim(), local_name.trim())
}

#[tauri::command]
pub async fn install_mcp_from_marketplace(
    request: McpInstallRequest,
    state: State<'_, AppState>,
) -> Result<McpInstallResult, String> {
    let registry_name = request.registry_name.trim().to_string();
    if registry_name.is_empty() {
        return Err("MCP Registry 名称不能为空".to_string());
    }
    if request.option_id.trim().is_empty() {
        return Err("请选择安装方式".to_string());
    }

    let mut global_targets: Vec<String> = Vec::new();
    for agent in &request.global_targets {
        let agent = agent.trim().to_string();
        if agent.is_empty() {
            continue;
        }
        if !global_targets.contains(&agent) {
            global_targets.push(agent);
        }
    }

    let mut project_targets: Vec<McpProjectTarget> = Vec::new();
    for entry in &request.project_targets {
        let project_id = entry.project_id.trim().to_string();
        let target = entry.target.trim().to_string();
        if project_id.is_empty() || target.is_empty() {
            continue;
        }
        let item = McpProjectTarget { project_id, target };
        if !project_targets.contains(&item) {
            project_targets.push(item);
        }
    }

    if global_targets.is_empty() && project_targets.is_empty() {
        return Err("请至少选择一个安装目标".to_string());
    }

    // 先把项目路径解析出来，避免安装到一半才发现项目不存在。
    let mut roots: HashMap<String, PathBuf> = HashMap::new();
    for entry in &project_targets {
        if !roots.contains_key(&entry.project_id) {
            let (_project, root) = project_root(&state, &entry.project_id)?;
            roots.insert(entry.project_id.clone(), root);
        }
        let root = roots
            .get(&entry.project_id)
            .ok_or_else(|| format!("项目不存在: {}", entry.project_id))?;
        if !root.is_dir() {
            return Err(format!(
                "项目目录不存在: {}（请先在项目页重新注册或修复路径）",
                root.display()
            ));
        }
        if mcp::project_target(root, &entry.target).is_none() {
            return Err(format!("项目不支持该 MCP 目标: {}", entry.target));
        }
    }

    let local_name = request.local_name.clone();
    let option_id = request.option_id.clone();
    let values = request.values.clone();
    let enabled = request.enabled;
    let registry_name_for_build = registry_name.clone();
    let built = tokio::task::spawn_blocking(move || {
        registry::build_server_for_install(
            &registry_name_for_build,
            &option_id,
            &local_name,
            &values,
            enabled,
        )
    })
    .await
    .map_err(|error| format!("MCP 安装准备被中断: {}", error))??;

    validate_server(&built.server)?;

    let mut warnings = built.warnings.clone();
    if !request.version.trim().is_empty() && request.version.trim() != built.version {
        warnings.push(format!(
            "Registry 当前最新版本为 {}，已按最新版安装",
            built.version
        ));
    }

    // 预检同名冲突。配置损坏的 target 直接报错，避免“部分安装”。
    let mut conflicts = Vec::new();
    for agent in &global_targets {
        let target = mcp::global_target(agent)
            .ok_or_else(|| format!("不支持的 MCP agent: {}", agent))?;
        if !built.server.headers.is_empty() && !mcp::supports_remote_headers(&target.spec) {
            return Err(format!(
                "全局 Agent '{}' 的配置格式不支持远程请求头（headers），请取消该目标",
                agent
            ));
        }
        let existing = mcp::read_servers(&target).map_err(|error| {
            format!(
                "读取全局 Agent '{}' 的 MCP 配置失败: {}",
                agent, error
            )
        })?;
        if existing
            .iter()
            .any(|server| server.name == built.server.name)
        {
            conflicts.push(format!("全局 {}", agent));
        }
    }
    for entry in &project_targets {
        let root = roots
            .get(&entry.project_id)
            .ok_or_else(|| format!("项目不存在: {}", entry.project_id))?;
        let target = mcp::project_target(root, &entry.target)
            .ok_or_else(|| format!("项目不支持该 MCP 目标: {}", entry.target))?;
        if !built.server.headers.is_empty() && !mcp::supports_remote_headers(&target.spec) {
            return Err(format!(
                "项目 '{}' 的 '{}' 配置格式不支持远程请求头（headers），请取消该目标",
                entry.project_id, entry.target
            ));
        }
        let existing = mcp::read_servers(&target).map_err(|error| {
            format!(
                "读取项目 '{}' 的 '{}' 配置失败: {}",
                entry.project_id, entry.target, error
            )
        })?;
        if existing
            .iter()
            .any(|server| server.name == built.server.name)
        {
            conflicts.push(format!("项目 {} / {}", entry.project_id, entry.target));
        }
    }
    if !conflicts.is_empty() && !request.overwrite {
        return Err(format!(
            "以下目标已存在同名 MCP server：{}。如需覆盖，请勾选“覆盖同名项”。",
            conflicts.join("、")
        ));
    }

    let mut installed_global = Vec::new();
    let mut installed_projects = Vec::new();

    for agent in &global_targets {
        match apply_server_to_global_target(&built.server, agent) {
            Ok(state_warnings) => {
                installed_global.push(agent.clone());
                warnings.extend(state_warnings);
            }
            Err(error) => warnings.push(format!("全局 {} 安装失败: {}", agent, error)),
        }
    }
    for entry in &project_targets {
        let Some(root) = roots.get(&entry.project_id) else {
            warnings.push(format!("项目 {} 安装失败: 项目不存在", entry.project_id));
            continue;
        };
        match apply_server_to_project_target(
            &entry.project_id,
            root,
            &built.server,
            &entry.target,
        ) {
            Ok(state_warnings) => {
                installed_projects.push(entry.clone());
                warnings.extend(state_warnings);
            }
            Err(error) => warnings.push(format!(
                "项目 {} / {} 安装失败: {}",
                entry.project_id, entry.target, error
            )),
        }
    }

    if installed_global.is_empty() && installed_projects.is_empty() {
        return Err(format!("所有目标都安装失败：{}", warnings.join("；")));
    }

    let record = McpInstallRecord {
        registry_name: registry_name.clone(),
        local_name: built.server.name.clone(),
        title: built.title.clone(),
        description: built.description.clone(),
        version: built.version.clone(),
        option_id: request.option_id.trim().to_string(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        global_targets: installed_global.clone(),
        project_targets: installed_projects.clone(),
    };
    if let Err(error) = registry::record_install(record) {
        warnings.push(format!("安装记录写入失败: {}", error));
    }

    info!(
        "MCP marketplace install '{}' from {} v{} -> global {:?}, projects {}",
        built.server.name,
        registry_name,
        built.version,
        installed_global,
        installed_projects.len()
    );

    Ok(McpInstallResult {
        server: built.server,
        installed_global,
        installed_projects,
        version: built.version,
        warnings,
    })
}
