//! 一键归并：把某个 Agent 主技能目录下的真实技能目录移入中央仓库
//! `~/.skills-manager/skills`，**不建立链接**（原位置移除）。
//!
//! 采用「先干跑（plan）再执行（apply）」的两段式流程：
//! - `plan_consolidation` 只读，产出可归并 / 冲突 / 跳过三组结果；
//! - `apply_consolidation` 按冲突决策逐条执行，失败不中断。
//!
//! 所有路径参数显式传入，便于单元测试注入临时目录。

use crate::linker::is_junction_or_symlink;
use crate::models::{AgentConfig, SkillEntry, SOURCE_GLOBAL};
use crate::trash;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ConsolidationItem {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsolidationConflict {
    pub id: String,
    pub name: String,
    pub agent_path: String,
    pub central_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsolidationSkip {
    pub id: String,
    /// already_managed | missing_skill_md | plugin_dir | plugin_cache | extra_path | hidden | not_a_dir
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ConsolidationPlan {
    pub agent: String,
    pub agent_skills_dir: String,
    pub movable: Vec<ConsolidationItem>,
    pub conflicts: Vec<ConsolidationConflict>,
    pub skipped: Vec<ConsolidationSkip>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConflictDecision {
    pub id: String,
    /// skip | overwrite | rename
    pub action: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MovedSkill {
    pub from_id: String,
    pub to_id: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ConsolidationReport {
    pub moved: Vec<MovedSkill>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
    /// 覆盖中央原件时产生的回收站条目
    pub trash_ids: Vec<String>,
}

fn expand_path(path: &str, home: &Path) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else if path == "~" {
        home.to_path_buf()
    } else {
        PathBuf::from(path)
    }
}

/// Agent 的主技能目录（不含 extra_paths 与插件缓存）
pub fn agent_skills_dir(agent: &AgentConfig, home: &Path) -> PathBuf {
    expand_path(&agent.path, home).join(&agent.skills_path)
}

fn read_skill_name(skill_md: &Path, fallback: &str) -> String {
    let Ok(content) = fs::read_to_string(skill_md) else {
        return fallback.to_string();
    };
    let Some(start) = content.find("---") else {
        return fallback.to_string();
    };
    let rest = &content[start + 3..];
    let Some(end) = rest.find("\n---") else {
        return fallback.to_string();
    };
    let yaml = &rest[..end];
    if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(yaml) {
        if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    fallback.to_string()
}

/// 干跑：扫描 Agent 主目录，产出归并计划。只读，不改动任何文件。
pub fn plan_consolidation(
    agent: &AgentConfig,
    central: &Path,
    home: &Path,
) -> ConsolidationPlan {
    let mut plan = ConsolidationPlan {
        agent: agent.name.clone(),
        agent_skills_dir: String::new(),
        ..Default::default()
    };

    let skills_dir = agent_skills_dir(agent, home);
    plan.agent_skills_dir = skills_dir.to_string_lossy().to_string();

    let is_plugin_cache = skills_dir
        .components()
        .any(|c| c.as_os_str().to_string_lossy() == "plugins");

    if let Ok(entries) = fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let id = entry.file_name().to_string_lossy().to_string();

            if id == "plugins" {
                plan.skipped.push(ConsolidationSkip {
                    id,
                    reason: "plugin_dir".to_string(),
                });
                continue;
            }
            if id.starts_with('.') {
                plan.skipped.push(ConsolidationSkip {
                    id,
                    reason: "hidden".to_string(),
                });
                continue;
            }
            if is_plugin_cache {
                plan.skipped.push(ConsolidationSkip {
                    id,
                    reason: "plugin_cache".to_string(),
                });
                continue;
            }
            if is_junction_or_symlink(&path) {
                plan.skipped.push(ConsolidationSkip {
                    id,
                    reason: "already_managed".to_string(),
                });
                continue;
            }
            if !path.is_dir() {
                plan.skipped.push(ConsolidationSkip {
                    id,
                    reason: "not_a_dir".to_string(),
                });
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                plan.skipped.push(ConsolidationSkip {
                    id,
                    reason: "missing_skill_md".to_string(),
                });
                continue;
            }

            let name = read_skill_name(&skill_md, &id);
            let central_target = central.join(&id);
            if central_target.exists() {
                plan.conflicts.push(ConsolidationConflict {
                    id,
                    name,
                    agent_path: path.to_string_lossy().to_string(),
                    central_path: central_target.to_string_lossy().to_string(),
                });
            } else {
                plan.movable.push(ConsolidationItem {
                    id,
                    name,
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }

    // extra_paths 不参与归并，但逐条列出让用户知情（路径语义与扫描器一致：extra_path/skills_path）
    for extra in &agent.extra_paths {
        let dir = expand_path(extra, home).join(&agent.skills_path);
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() || is_junction_or_symlink(&path) {
                    continue;
                }
                if !path.join("SKILL.md").exists() {
                    continue;
                }
                plan.skipped.push(ConsolidationSkip {
                    id: entry.file_name().to_string_lossy().to_string(),
                    reason: "extra_path".to_string(),
                });
            }
        }
    }

    plan.movable.sort_by(|a, b| a.id.cmp(&b.id));
    plan.conflicts.sort_by(|a, b| a.id.cmp(&b.id));
    plan.skipped.sort_by(|a, b| a.id.cmp(&b.id));
    plan
}

fn unique_target(central: &Path, id: &str, agent: &str) -> PathBuf {
    let base = format!("{}-{}", id, agent);
    let mut candidate = base.clone();
    let mut n = 2;
    while central.join(&candidate).exists() {
        candidate = format!("{}-{}", base, n);
        n += 1;
    }
    central.join(candidate)
}

fn move_skill_dir(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建中央目录失败: {}", e))?;
    }
    trash::move_dir(from, to).map_err(|e| format!("移动 {:?} 失败: {}", from, e))
}

/// 执行归并。`conflict_actions` 中未给出的冲突项按「跳过」处理。
pub fn apply_consolidation(
    agent: &AgentConfig,
    plan: &ConsolidationPlan,
    central: &Path,
    conflict_actions: &[ConflictDecision],
    trash_root: &Path,
) -> ConsolidationReport {
    let mut report = ConsolidationReport::default();

    for item in &plan.movable {
        let from = PathBuf::from(&item.path);
        let to = central.join(&item.id);
        match move_skill_dir(&from, &to) {
            Ok(()) => {
                info!("Consolidated {} -> {:?}", item.id, to);
                report.moved.push(MovedSkill {
                    from_id: item.id.clone(),
                    to_id: item.id.clone(),
                });
            }
            Err(e) => report.errors.push(format!("{}: {}", item.id, e)),
        }
    }

    for conflict in &plan.conflicts {
        let action = conflict_actions
            .iter()
            .find(|d| d.id == conflict.id)
            .map(|d| d.action.to_lowercase())
            .unwrap_or_else(|| "skip".to_string());

        let from = PathBuf::from(&conflict.agent_path);

        match action.as_str() {
            "overwrite" => {
                let existing = central.join(&conflict.id);
                if existing.exists() {
                    match trash::move_to_trash_in(
                        trash_root,
                        &conflict.id,
                        &conflict.name,
                        "overwrite",
                        &[(SOURCE_GLOBAL.to_string(), existing.clone())],
                        None,
                    ) {
                        Ok(trash_id) => report.trash_ids.push(trash_id),
                        Err(e) => {
                            report
                                .errors
                                .push(format!("{}: 覆盖前归档中央副本失败: {}", conflict.id, e));
                            continue;
                        }
                    }
                }
                match move_skill_dir(&from, &existing) {
                    Ok(()) => {
                        info!("Consolidated {} (overwrite) -> {:?}", conflict.id, existing);
                        report.moved.push(MovedSkill {
                            from_id: conflict.id.clone(),
                            to_id: conflict.id.clone(),
                        });
                    }
                    Err(e) => report.errors.push(format!("{}: {}", conflict.id, e)),
                }
            }
            "rename" => {
                let target = unique_target(central, &conflict.id, &agent.name);
                let to_id = target
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| conflict.id.clone());
                match move_skill_dir(&from, &target) {
                    Ok(()) => {
                        info!("Consolidated {} (rename) -> {:?}", conflict.id, target);
                        report.moved.push(MovedSkill {
                            from_id: conflict.id.clone(),
                            to_id,
                        });
                    }
                    Err(e) => report.errors.push(format!("{}: {}", conflict.id, e)),
                }
            }
            _ => report.skipped.push(conflict.id.clone()),
        }
    }

    report
}

/// 把归并结果同步进 `skill_states`：
/// - 源从该 Agent 变为 `global`
/// - `primary` 若指向该 Agent 则重新挑选
/// - `open` 移除该 Agent（归并不建立链接）
pub fn apply_states(
    states: &mut HashMap<String, SkillEntry>,
    agent: &str,
    report: &ConsolidationReport,
) {
    for moved in &report.moved {
        if moved.from_id == moved.to_id {
            match states.get_mut(&moved.from_id) {
                Some(entry) => {
                    entry.sources.retain(|s| s != agent);
                    entry.insert_source(SOURCE_GLOBAL);
                    entry.open.retain(|n| n != agent);
                    if entry.primary == agent {
                        entry.primary = String::new();
                    }
                    entry.ensure_valid_primary();
                }
                None => {
                    states.insert(
                        moved.from_id.clone(),
                        SkillEntry {
                            sources: vec![SOURCE_GLOBAL.to_string()],
                            primary: SOURCE_GLOBAL.to_string(),
                            open: Vec::new(),
                        },
                    );
                }
            }
        } else {
            let mut remove_old = false;
            if let Some(old) = states.get_mut(&moved.from_id) {
                old.sources.retain(|s| s != agent);
                old.open.retain(|n| n != agent);
                if old.primary == agent {
                    old.primary = String::new();
                }
                old.ensure_valid_primary();
                remove_old = old.is_orphan();
            }
            if remove_old {
                states.remove(&moved.from_id);
            }
            states.insert(
                moved.to_id.clone(),
                SkillEntry {
                    sources: vec![SOURCE_GLOBAL.to_string()],
                    primary: SOURCE_GLOBAL.to_string(),
                    open: Vec::new(),
                },
            );
        }
    }
}
