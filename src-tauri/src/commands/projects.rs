//! 项目级 Skill 管理的 Tauri 命令。

use crate::commands::skills::{invalidate_skills_cache, scan_only};
use crate::linker::{is_junction_or_symlink, remove_link, LinkManager};
use crate::models::{ProjectCandidate, ProjectConfig, ProjectSkill};
use crate::projects;
use crate::settings::AppSettingsManager;
use crate::state::AppState;
use crate::trash;
use log::info;
use std::path::PathBuf;
use tauri::State;

fn find_project(settings: &AppSettingsManager, id: &str) -> Result<ProjectConfig, String> {
    settings
        .get_config()
        .projects
        .iter()
        .find(|project| project.id == id)
        .cloned()
        .ok_or_else(|| format!("项目不存在: {}", id))
}

/// 校验 `target_dir` 是已知的项目 skill 目录，避免写入任意路径。
fn normalize_target_dir(target_dir: Option<String>) -> Result<String, String> {
    let rel = target_dir.unwrap_or_else(|| ".claude/skills".to_string());
    let rel = rel.trim_matches('/').replace('\\', "/");
    if projects::PROJECT_SKILL_DIRS
        .iter()
        .any(|(known, _)| *known == rel.as_str())
    {
        Ok(rel)
    } else {
        Err(format!(
            "不支持的目标目录: {}（可选：{}）",
            rel,
            projects::PROJECT_SKILL_DIRS
                .iter()
                .map(|(dir, _)| *dir)
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

/// 校验 skill 路径确实位于项目的某个约定目录内。
fn resolve_project_skill_path(project: &ProjectConfig, skill_path: &str) -> Result<PathBuf, String> {
    let root = projects::normalize_project_path(&project.path);
    let candidate = PathBuf::from(skill_path);
    if !candidate.is_dir() {
        return Err(format!("技能目录不存在: {}", skill_path));
    }
    let canonical = std::fs::canonicalize(&candidate).unwrap_or(candidate);
    let in_known_dir = projects::PROJECT_SKILL_DIRS.iter().any(|(rel, _)| {
        let base = std::fs::canonicalize(root.join(rel)).unwrap_or_else(|_| root.join(rel));
        canonical.starts_with(&base)
    });
    if !in_known_dir {
        return Err("只能操作项目内约定目录下的技能".to_string());
    }
    Ok(canonical)
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectConfig>, String> {
    let settings = state
        .settings_manager
        .lock()
        .map_err(|e| format!("Failed to acquire lock: {}", e))?;
    Ok(settings.get_config().projects.clone())
}

#[tauri::command]
pub async fn add_project(
    path: String,
    name: Option<String>,
    state: State<'_, AppState>,
) -> Result<ProjectConfig, String> {
    let normalized = projects::normalize_project_path(&path);
    projects::validate_project_root(&normalized)?;
    let path_string = normalized.to_string_lossy().to_string();

    let mut settings = state
        .settings_manager
        .lock()
        .map_err(|e| format!("Failed to acquire lock: {}", e))?;

    if let Some(existing) = settings
        .get_config()
        .projects
        .iter()
        .find(|project| projects::normalize_project_path(&project.path) == normalized)
    {
        return Err(format!(
            "项目已注册: {}（{}）",
            existing.name, existing.path
        ));
    }

    let mut id = projects::project_id(&path_string);
    if settings
        .get_config()
        .projects
        .iter()
        .any(|project| project.id == id)
    {
        // 哈希碰撞的兜底：理论上不会发生，但不能让 ID 冲突覆盖别的项目。
        id = format!("{}-{}", id, settings.get_config().projects.len() + 1);
    }

    let display_name = name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| {
            normalized
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project")
                .to_string()
        });

    let project = ProjectConfig {
        id,
        name: display_name,
        path: path_string,
        enabled: true,
        auto_detected: false,
    };
    settings.get_config_mut().projects.push(project.clone());
    settings
        .save()
        .map_err(|e| format!("保存配置失败: {}", e))?;
    info!("Project registered: {} ({})", project.name, project.path);
    Ok(project)
}

#[tauri::command]
pub async fn remove_project(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut settings = state
        .settings_manager
        .lock()
        .map_err(|e| format!("Failed to acquire lock: {}", e))?;
    let before = settings.get_config().projects.len();
    settings
        .get_config_mut()
        .projects
        .retain(|project| project.id != id);
    if settings.get_config().projects.len() == before {
        return Err(format!("项目不存在: {}", id));
    }
    settings
        .save()
        .map_err(|e| format!("保存配置失败: {}", e))?;
    info!("Project unregistered: {}", id);
    Ok(())
}

#[tauri::command]
pub async fn detect_project_candidates(
    state: State<'_, AppState>,
) -> Result<Vec<ProjectCandidate>, String> {
    let registered = {
        let settings = state
            .settings_manager
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        settings.get_config().projects.clone()
    };
    Ok(projects::detect_candidates(&registered))
}

#[tauri::command]
pub async fn list_project_skills(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProjectSkill>, String> {
    let project = {
        let settings = state
            .settings_manager
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        find_project(&settings, &project_id)?
    };
    Ok(projects::scan_project_skills(&project))
}

/// 把中央库 / 任意已扫描到的技能链接（或复制）到项目目录。
#[tauri::command]
pub async fn install_skill_to_project(
    project_id: String,
    skill_id: String,
    target_dir: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let target_rel = normalize_target_dir(target_dir)?;

    let (project, skill, strategy) = {
        let mut settings = state
            .settings_manager
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        let project = find_project(&settings, &project_id)?;
        let strategy = settings.get_config().linking_strategy;
        let skills = scan_only(&mut settings)?;
        let skill = skills
            .iter()
            .find(|s| s.id == skill_id)
            .cloned()
            .ok_or_else(|| format!("技能不存在: {}", skill_id))?;
        (project, skill, strategy)
    };

    let root = projects::normalize_project_path(&project.path);
    let target_parent = root.join(&target_rel);
    let target_path = target_parent.join(&skill.id);
    if target_path.exists() {
        return Err(format!(
            "项目目录中已存在技能 '{}'（{}）",
            skill.name, target_rel
        ));
    }
    std::fs::create_dir_all(&target_parent)
        .map_err(|e| format!("创建目录失败: {} ({})", target_parent.display(), e))?;

    let agent = projects::project_as_agent(&project, &target_rel);
    let linker = LinkManager::new(strategy);
    linker
        .link_skill_to_agent(&skill, &agent, &AppSettingsManager::get_skills_dir())
        .map_err(|e| format!("安装到项目失败: {}", e))?;

    invalidate_skills_cache(&state);
    info!(
        "Skill '{}' installed to project '{}' ({})",
        skill.id, project.name, target_rel
    );
    Ok(target_path.to_string_lossy().to_string())
}

/// 把项目内原生技能复制到中央库（之后可在全局列表中管理）。
#[tauri::command]
pub async fn import_project_skill(
    project_id: String,
    skill_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let project = {
        let settings = state
            .settings_manager
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        find_project(&settings, &project_id)?
    };
    let resolved = resolve_project_skill_path(&project, &skill_path)?;
    if !resolved.join("SKILL.md").is_file() {
        return Err("该目录不是有效技能（缺少 SKILL.md）".to_string());
    }

    let name = crate::commands::skills::import_skill_folder(
        resolved.to_string_lossy().to_string(),
    )
    .await?;
    invalidate_skills_cache(&state);
    info!(
        "Imported project skill '{}' from {:?} to central storage",
        name, resolved
    );
    Ok(name)
}

/// 删除项目内技能。
///
/// - `mode = "link"`：只删除托管链接（符号链接 / junction）
/// - `mode = "trash"`：把原生目录移入 Skills Manager 回收站（可恢复）
#[tauri::command]
pub async fn delete_project_skill(
    project_id: String,
    skill_path: String,
    mode: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let project = {
        let settings = state
            .settings_manager
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        find_project(&settings, &project_id)?
    };
    let resolved = resolve_project_skill_path(&project, &skill_path)?;

    match mode.as_str() {
        "link" => {
            if !is_junction_or_symlink(&resolved) {
                return Err("该技能不是托管链接，请使用移入回收站".to_string());
            }
            remove_link(&resolved).map_err(|e| format!("删除链接失败: {}", e))?;
            info!("Removed project skill link: {:?}", resolved);
        }
        "trash" => {
            let skill_id = resolved
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project-skill")
                .to_string();
            let rel = resolved
                .strip_prefix(projects::normalize_project_path(&project.path))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| skill_id.clone());
            trash::move_to_trash_in(
                &trash::trash_root(),
                &skill_id,
                &skill_id,
                &format!("project:{} ({})", project.name, rel),
                &[(project.name.clone(), resolved.clone())],
                None,
            )?;
            info!("Moved project skill to trash: {:?}", resolved);
        }
        other => return Err(format!("未知的删除模式: {}", other)),
    }
    invalidate_skills_cache(&state);
    Ok(())
}

/// 在文件管理器中打开项目根目录或其中某个子路径。
#[tauri::command]
pub async fn open_project_path(
    project_id: String,
    rel: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let project = {
        let settings = state
            .settings_manager
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?;
        find_project(&settings, &project_id)?
    };
    let root = projects::normalize_project_path(&project.path);
    let target = match rel {
        Some(rel) if !rel.trim().is_empty() => {
            let candidate = root.join(rel.trim().trim_matches('/'));
            // 只允许打开项目内的路径，避免前端传入任意绝对路径或 `..` 逃逸。
            let canonical = std::fs::canonicalize(&candidate).unwrap_or(candidate);
            if !canonical.starts_with(&root) {
                return Err("只能打开项目内的路径".to_string());
            }
            canonical
        }
        _ => root,
    };
    crate::commands::settings::open_folder(target.to_string_lossy().to_string()).await
}
