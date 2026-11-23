//! 项目级 Skill 管理。
//!
//! 项目不是 agent，不参与 `skill_states` 的合并；这里只负责：
//! - 注册表（`AppConfig.projects`）的路径规范化与稳定 ID
//! - 项目内约定 skill 目录的扫描
//! - 从 `~/.claude.json` 与常见项目根自动发现候选项目
//!
//! 所有路径都用 `PathBuf` 拼接，避免 Windows 下的分隔符问题。

use crate::linker::is_junction_or_symlink;
use crate::models::{ProjectCandidate, ProjectConfig, ProjectSkill};
use crate::scanner::parse_discovery;
use crate::settings::AppSettingsManager;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// 项目内约定的 skill 目录（相对项目根）与对应的 agent 名。
///
/// 只扫描一层：`<dir>/<skill>/SKILL.md`。项目里出现别的 agent 目录时，
/// 这里补一行即可，前端会按 `agent` 自动分组。
pub const PROJECT_SKILL_DIRS: &[(&str, &str)] = &[
    (".claude/skills", "claude"),
    (".agents/skills", "antigravity"),
    (".cursor/skills", "cursor"),
    (".codex/skills", "codex"),
    (".opencode/skills", "opencode"),
    (".gemini/skills", "antigravity"),
    (".grok/skills", "grok"),
    (".kilocode/skills", "kilo"),
    (".github/skills", "copilot"),
    (".workbuddy/skills", "workbuddy"),
    (".codebuddy/skills", "codebuddy"),
    (".zcode/skills", "zcode"),
    (".openclaw/skills", "openclaw"),
    (".kimi-code/skills", "kimi"),
    (".marscode/skills", "marscode"),
    (".openclaude/skills", "openclaude"),
    (".hermes/skills", "hermes"),
    (".ima/skills", "ima"),
    (".trae/skills", "trae"),
    (".qoder/skills", "qoder"),
    (".kiro/skills", "kiro"),
    (".pi/skills", "pi"),
    (".omp/skills", "omp"),
];

/// 展开 `~` 前缀（与 `commands::settings::open_folder` 的规则保持一致）。
pub fn expand_path(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        dirs::home_dir().unwrap_or_default().join(rest)
    } else if raw == "~" {
        dirs::home_dir().unwrap_or_default()
    } else {
        PathBuf::from(raw)
    }
}

/// 把任意名字压成可用于 ID 的 slug。Unicode 字母（含中文）保留。
pub fn slugify(raw: &str) -> String {
    let mut out = String::new();
    for c in raw.chars() {
        if c.is_alphanumeric() {
            for lower in c.to_lowercase() {
                out.push(lower);
            }
        } else if c == '-' || c == '_' {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed
    }
}

/// FNV-1a：稳定的 64 位哈希。`DefaultHasher` 不保证跨 Rust 版本稳定，
/// 而项目 ID 会写进配置文件，必须长期可复现。
fn stable_hash(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// 项目稳定 ID：`slug(目录名)-<路径哈希 12 位>`。
///
/// 路径本身参与哈希，因此同名项目（如两个 `paper`）不会冲突；
/// 同一路径重复注册会得到相同 ID，天然幂等。
pub fn project_id(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
    format!(
        "{}-{:012x}",
        slugify(name),
        stable_hash(path) & 0x0000_ffff_ffff_ffff
    )
}

/// 规范化为用于比较 / 存储的绝对路径字符串。
///
/// 优先 `canonicalize`（解析符号链接、消除 `..`）；失败时退回展开后的路径。
pub fn normalize_project_path(raw: &str) -> PathBuf {
    let expanded = expand_path(raw);
    fs::canonicalize(&expanded).unwrap_or(expanded)
}

/// 路径是否可以作为项目根注册。
pub fn validate_project_root(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("目录不存在: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("不是目录: {}", path.display()));
    }
    let home = dirs::home_dir().unwrap_or_default();
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let canonical_home = fs::canonicalize(&home).unwrap_or(home);
    if canonical == canonical_home {
        return Err("不能把用户主目录注册为项目".to_string());
    }
    if canonical.parent().is_none() {
        return Err("不能把文件系统根目录注册为项目".to_string());
    }
    Ok(())
}

/// 某个项目下已存在的约定 skill 目录（相对路径）。
pub fn existing_skill_dirs(root: &Path) -> Vec<String> {
    PROJECT_SKILL_DIRS
        .iter()
        .filter(|(rel, _)| root.join(rel).is_dir())
        .map(|(rel, _)| (*rel).to_string())
        .collect()
}

/// 是否是指向 Skills Manager 中央库的托管链接。
fn is_managed_skill_dir(path: &Path) -> bool {
    if !is_junction_or_symlink(path) {
        return false;
    }
    let Ok(central) = fs::canonicalize(AppSettingsManager::get_skills_dir()) else {
        return false;
    };
    fs::canonicalize(path)
        .map(|target| target.starts_with(&central))
        .unwrap_or(false)
}

/// 扫描单个项目的所有约定 skill 目录。
///
/// 与全局扫描不同，这里**保留符号链接**：项目里的托管链接本身就是要展示的条目。
pub fn scan_project_skills(project: &ProjectConfig) -> Vec<ProjectSkill> {
    let mut out = Vec::new();
    if !project.enabled {
        return out;
    }
    let root = expand_path(&project.path);
    if !root.is_dir() {
        return out;
    }

    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (rel, agent) in PROJECT_SKILL_DIRS {
        let base = root.join(rel);
        if !base.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.is_file() {
                continue;
            }
            let Ok(discovery) = parse_discovery(&skill_md, agent) else {
                continue;
            };
            // 同一个 skill 可能通过软链出现在多个目录；按物理路径去重，
            // 保留第一个（PROJECT_SKILL_DIRS 的顺序即优先级）。
            let canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            let key = canonical.to_string_lossy().to_string();
            if !seen.insert(key) {
                continue;
            }
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&discovery.id)
                .to_string();
            out.push(ProjectSkill {
                id: discovery.id,
                name: discovery.name,
                description: discovery.description,
                path: discovery.path,
                rel_dir: format!("{}/{}", rel, dir_name),
                agent: (*agent).to_string(),
                is_symlink: is_junction_or_symlink(&path),
                is_managed: is_managed_skill_dir(&path),
                last_updated: discovery.last_updated,
            });
        }
    }
    out.sort_by(|a, b| a.rel_dir.cmp(&b.rel_dir));
    out
}

/// 读取 `~/.claude.json` 中记录的项目路径（只读，不修改该文件）。
fn claude_json_projects(home: &Path) -> Vec<PathBuf> {
    let Ok(content) = fs::read_to_string(home.join(".claude.json")) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };
    value
        .get("projects")
        .and_then(|projects| projects.as_object())
        .map(|map| map.keys().map(PathBuf::from).collect())
        .unwrap_or_default()
}

/// 常见项目根下的一层子目录。
fn common_root_children(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in ["Documents", "Desktop", "Projects"] {
        let dir = home.join(root);
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.push(path);
            }
        }
    }
    out
}

/// 过滤掉用户主目录下的隐藏目录（agent 数据目录、`.skills-manager` 等）。
fn is_internal_path(path: &Path, home: &Path) -> bool {
    if path == home {
        return true;
    }
    let Ok(rel) = path.strip_prefix(home) else {
        return false;
    };
    rel.components()
        .next()
        .map(|first| first.as_os_str().to_string_lossy().starts_with('.'))
        .unwrap_or(false)
}

/// 自动发现项目候选。
///
/// 来源：`~/.claude.json` 的 `projects` 键 + `~/Documents`、`~/Desktop`、`~/Projects`
/// 的一层子目录。已注册的路径会被标记 `registered: true`，由前端决定是否展示。
pub fn detect_candidates(registered: &[ProjectConfig]) -> Vec<ProjectCandidate> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut paths: BTreeSet<PathBuf> = BTreeSet::new();

    for path in claude_json_projects(&home)
        .into_iter()
        .chain(common_root_children(&home))
    {
        if path.is_dir() {
            paths.insert(path);
        }
    }

    let registered_canonical: BTreeSet<PathBuf> = registered
        .iter()
        .map(|project| normalize_project_path(&project.path))
        .collect();

    let mut out = Vec::new();
    for path in paths {
        if is_internal_path(&path, &home) {
            continue;
        }
        let canonical = normalize_project_path(&path.to_string_lossy());
        let registered = registered_canonical.contains(&canonical);
        let name = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .or_else(|| path.file_name().and_then(|n| n.to_str()))
            .unwrap_or("project")
            .to_string();
        out.push(ProjectCandidate {
            path: canonical.to_string_lossy().to_string(),
            name,
            skill_dirs: existing_skill_dirs(&canonical),
            registered,
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// 构造用于复用 `LinkManager::link_skill_to_agent` 的伪 AgentConfig。
///
/// 项目在磁盘上就是一个 agent 根目录，`skills_path` 是相对路径（如 `.claude/skills`）。
pub fn project_as_agent(project: &ProjectConfig, target_dir: &str) -> crate::models::AgentConfig {
    crate::models::AgentConfig {
        name: format!("project:{}", project.id),
        display_name: project.name.clone(),
        path: project.path.clone(),
        skills_path: target_dir.to_string(),
        enabled: true,
        detected: true,
        extra_paths: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_handles_unicode_and_symbols() {
        assert_eq!(slugify("My Project"), "my-project");
        assert_eq!(slugify("论文项目 A"), "论文项目-a");
        assert_eq!(slugify("!!!"), "project");
    }

    #[test]
    fn project_id_is_stable_and_path_specific() {
        let a = project_id("/tmp/demo");
        let b = project_id("/tmp/demo");
        let c = project_id("/tmp/other/demo");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("demo-"));
    }

    #[test]
    fn expand_path_expands_tilde() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_path("~"), home);
        assert_eq!(expand_path("~/foo"), home.join("foo"));
        assert_eq!(expand_path("/abs/foo"), PathBuf::from("/abs/foo"));
    }
}
