//! 回收站：把被删除 / 被覆盖的技能目录移入 `~/.skills-manager/trash/`，
//! 支持列出、恢复、彻底删除与清空。
//!
//! 所有函数都接受显式的 `trash_root`，以便单元测试注入临时目录（不触碰真实 HOME）。

use crate::models::SkillEntry;
use crate::settings::AppSettingsManager;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 单条物理目录的归档记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashEntryItem {
    /// 该副本原来的来源（`"global"` 或 agent 名）
    pub source: String,
    /// 原始绝对路径
    pub original_path: String,
    /// 在回收站条目内的相对路径（如 `payload/0`）
    pub payload: String,
}

/// `meta.json` 的结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashMeta {
    pub trash_id: String,
    pub skill_id: String,
    pub name: String,
    /// RFC3339
    pub deleted_at: String,
    /// `delete` | `overwrite`
    pub reason: String,
    pub entries: Vec<TrashEntryItem>,
    /// 删除前的 skill_states 快照，恢复时用于重建配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<SkillEntry>,
}

/// 恢复结果
#[derive(Debug, Default, Serialize)]
pub struct RestoreReport {
    /// 成功恢复的 trash_id
    pub restored: Vec<String>,
    /// 因目标已存在等原因跳过的 trash_id
    pub skipped: Vec<String>,
    /// 失败详情
    pub errors: Vec<String>,
}

/// 恢复成功的条目（供调用方合并 skill_states）
pub struct RestoredEntry {
    pub meta: TrashMeta,
}

/// 默认回收站根目录：`~/.skills-manager/trash`
pub fn trash_root() -> PathBuf {
    AppSettingsManager::get_skills_manager_dir().join("trash")
}

fn timestamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S-%3f").to_string()
}

pub(crate) fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

/// 移动目录：优先 `rename`，跨卷失败时退化为 copy + 删除源。
///
/// 目标已存在时直接报错，绝不「合并」——半合并会同时毁掉源目录和旧数据。
pub(crate) fn move_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    if to.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("目标路径已存在: {}", to.display()),
        ));
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_dir_all(from, to)?;
            fs::remove_dir_all(from).map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!(
                        "内容已复制到 {}，但删除原目录失败: {}（副本已保留，可手动清理源目录）",
                        to.display(),
                        e
                    ),
                )
            })
        }
    }
}

/// 为 `skill_id` 生成不冲突的 trash_id 与目录
fn allocate_entry_dir(trash_root: &Path, skill_id: &str) -> std::io::Result<(String, PathBuf)> {
    fs::create_dir_all(trash_root)?;
    let base = format!("{}-{}", timestamp(), sanitize(skill_id));
    let mut candidate = base.clone();
    let mut n = 2;
    while trash_root.join(&candidate).exists() {
        candidate = format!("{}-{}", base, n);
        n += 1;
    }
    let dir = trash_root.join(&candidate);
    fs::create_dir_all(&dir)?;
    Ok((candidate, dir))
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect()
}

/// 把若干物理目录移入回收站。
///
/// `dirs` 为 `(source, 绝对路径)`；重复路径（canonicalize 相同）只会归档一次。
/// 返回新建的 trash_id。全部移动失败时报错。
pub fn move_to_trash_in(
    trash_root: &Path,
    skill_id: &str,
    name: &str,
    reason: &str,
    dirs: &[(String, PathBuf)],
    state: Option<&SkillEntry>,
) -> Result<String, String> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut unique: Vec<(String, PathBuf)> = Vec::new();
    for (source, path) in dirs {
        if !path.exists() {
            continue;
        }
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        if seen.iter().any(|p| p == &canonical) {
            continue;
        }
        seen.push(canonical);
        unique.push((source.clone(), path.clone()));
    }

    if unique.is_empty() {
        return Err(format!("没有可归档的目录（skill: {}）", skill_id));
    }

    let (trash_id, entry_dir) = allocate_entry_dir(trash_root, skill_id)
        .map_err(|e| format!("创建回收站条目失败: {}", e))?;

    let mut entries: Vec<TrashEntryItem> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for (index, (source, path)) in unique.iter().enumerate() {
        let payload = format!("payload/{}", index);
        let dest = entry_dir.join(&payload);
        match move_dir(path, &dest) {
            Ok(()) => {
                info!("Moved to trash: {:?} -> {:?}", path, dest);
                entries.push(TrashEntryItem {
                    source: source.clone(),
                    original_path: path.to_string_lossy().to_string(),
                    payload,
                });
            }
            Err(e) => {
                warn!("Failed to move {:?} to trash: {}", path, e);
                errors.push(format!("{}: {}", path.display(), e));
            }
        }
    }

    if entries.is_empty() {
        if errors.is_empty() {
            let _ = fs::remove_dir_all(&entry_dir);
            return Err(format!("没有可归档的目录（skill: {}）", skill_id));
        }
        // 内容可能已经复制进 payload、但删除源目录时失败：
        // 保留条目目录（不写 meta，界面上看不到），以免连唯一副本也被销毁。
        warn!("Trash entry kept for manual recovery: {:?}", entry_dir);
        return Err(format!(
            "移入回收站失败: {}（副本已保留在 {}）",
            errors.join("; "),
            entry_dir.display()
        ));
    }

    let meta = TrashMeta {
        trash_id: trash_id.clone(),
        skill_id: skill_id.to_string(),
        name: name.to_string(),
        deleted_at: chrono::Local::now().to_rfc3339(),
        reason: reason.to_string(),
        entries,
        state: state.cloned(),
    };
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("序列化回收站元数据失败: {}", e))?;
    fs::write(entry_dir.join("meta.json"), meta_json)
        .map_err(|e| format!("写入回收站元数据失败: {}", e))?;

    Ok(trash_id)
}

fn read_meta(entry_dir: &Path) -> Option<TrashMeta> {
    let content = fs::read_to_string(entry_dir.join("meta.json")).ok()?;
    match serde_json::from_str::<TrashMeta>(&content) {
        Ok(meta) => Some(meta),
        Err(e) => {
            warn!("回收站条目元数据损坏，已跳过 {:?}: {}", entry_dir, e);
            None
        }
    }
}

/// 列出回收站条目（按删除时间倒序）
pub fn list_trash_in(trash_root: &Path) -> Vec<TrashMeta> {
    let mut out: Vec<TrashMeta> = Vec::new();
    let Ok(entries) = fs::read_dir(trash_root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Some(meta) = read_meta(&path) {
            out.push(meta);
        }
    }
    out.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
    out
}

/// 恢复指定条目到原始路径。目标已存在时整条跳过，绝不覆盖。
pub fn restore_trash_in(trash_root: &Path, trash_ids: &[String]) -> RestoreReport {
    let mut report = RestoreReport::default();

    for id in trash_ids {
        let entry_dir = trash_root.join(sanitize_dir_name(id));
        let entry_dir = if entry_dir.exists() {
            entry_dir
        } else {
            // 兼容外部直接给出真实目录名
            let alt = trash_root.join(id);
            if alt.exists() {
                alt
            } else {
                report.errors.push(format!("回收站条目不存在: {}", id));
                continue;
            }
        };

        let Some(meta) = read_meta(&entry_dir) else {
            report.errors.push(format!("回收站条目损坏: {}", id));
            continue;
        };

        // 先做冲突检查，避免部分恢复
        let mut blocked = false;
        for item in &meta.entries {
            if Path::new(&item.original_path).exists() {
                blocked = true;
                report.skipped.push(meta.trash_id.clone());
                report.errors.push(format!(
                    "{}: 目标路径已存在 {}",
                    meta.trash_id, item.original_path
                ));
                break;
            }
        }
        if blocked {
            continue;
        }

        let mut failed = false;
        for item in &meta.entries {
            let src = entry_dir.join(&item.payload);
            let dest = PathBuf::from(&item.original_path);
            if let Err(e) = move_dir(&src, &dest) {
                report
                    .errors
                    .push(format!("{}: 恢复 {:?} 失败: {}", meta.trash_id, dest, e));
                failed = true;
                break;
            }
        }

        if failed {
            continue;
        }

        if let Err(e) = fs::remove_dir_all(&entry_dir) {
            warn!("恢复后清理回收站条目失败 {:?}: {}", entry_dir, e);
        }
        info!("Restored from trash: {}", meta.trash_id);
        report.restored.push(meta.trash_id.clone());
    }

    report
}

/// 恢复后返回的元数据（供调用方合并 skill_states）
pub fn restore_trash_with_meta_in(
    trash_root: &Path,
    trash_ids: &[String],
) -> (Vec<RestoredEntry>, RestoreReport) {
    let mut restored: Vec<RestoredEntry> = Vec::new();
    let mut report = RestoreReport::default();

    for id in trash_ids {
        let dir = trash_root.join(sanitize_dir_name(id));
        let dir = if dir.exists() { dir } else { trash_root.join(id) };
        let Some(meta) = (if dir.exists() { read_meta(&dir) } else { None }) else {
            report.errors.push(format!("回收站条目不存在或损坏: {}", id));
            continue;
        };

        let single = restore_trash_in(trash_root, &[meta.trash_id.clone()]);
        report.errors.extend(single.errors);
        report.skipped.extend(single.skipped);
        if !single.restored.is_empty() {
            report.restored.extend(single.restored);
            restored.push(RestoredEntry { meta });
        }
    }

    (restored, report)
}

fn sanitize_dir_name(id: &str) -> String {
    sanitize(id)
}

/// 彻底删除单条回收站条目
pub fn purge_trash_entry_in(trash_root: &Path, trash_id: &str) -> Result<(), String> {
    let dir = trash_root.join(sanitize_dir_name(trash_id));
    let dir = if dir.exists() { dir } else { trash_root.join(trash_id) };
    if !dir.exists() {
        return Ok(());
    }
    fs::remove_dir_all(&dir).map_err(|e| format!("删除回收站条目失败: {}", e))
}

/// 清空回收站，返回删除的条目数
pub fn empty_trash_in(trash_root: &Path) -> Result<u32, String> {
    let Ok(entries) = fs::read_dir(trash_root) else {
        return Ok(0);
    };
    let mut removed = 0u32;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match fs::remove_dir_all(&path) {
            Ok(()) => removed += 1,
            Err(e) => warn!("清空回收站条目失败 {:?}: {}", path, e),
        }
    }
    Ok(removed)
}
