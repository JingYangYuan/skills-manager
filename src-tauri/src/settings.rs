use crate::linker::{is_junction_or_symlink, remove_link};
use crate::models::{AgentConfig, AppConfig, LinkStrategy, CURRENT_SCHEMA_VERSION};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)
}

fn write_default_config(config_path: &Path) -> Result<AppConfig, AppSettingsError> {
    let mut default = AppConfig::default();
    default.agents = AppSettingsManager::known_agent_presets();

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(&default)?;
    atomic_write(config_path, &content)?;

    Ok(default)
}

fn backup_invalid_config(config_path: &Path, content: &str, error: &serde_json::Error) {
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let backup_path = config_path.with_extension(format!("json.invalid-{timestamp}"));

    eprintln!(
        "[settings] invalid config at {:?}: {}. Backing up to {:?} and recreating defaults.",
        config_path, error, backup_path
    );

    if let Err(write_err) = fs::write(&backup_path, content) {
        eprintln!(
            "[settings] failed to back up invalid config to {:?}: {}",
            backup_path, write_err
        );
    }
}

/// 从 skill_states 中清除对已删除预设的引用（sources / open）。
/// 返回是否发生了变更。
fn prune_deprecated_agent_refs(config: &mut AppConfig, removed: &[&str]) -> bool {
    let mut changed = false;
    for entry in config.skill_states.values_mut() {
        let before = (entry.sources.len(), entry.open.len());
        entry.sources.retain(|s| !removed.contains(&s.as_str()));
        entry.open.retain(|s| !removed.contains(&s.as_str()));
        if (entry.sources.len(), entry.open.len()) != before {
            changed = true;
        }
        entry.ensure_valid_primary();
    }
    changed
}

/// 一次性清理：移除 `~/.agents/skills` 下由本应用创建的托管链接。
///
/// 只删除「本身是 symlink/junction 且指向 `~/.skills-manager/skills`」的条目；
/// 用户自装技能、普通目录、`.skill-lock.json` 等一律保留。
/// 返回 (已删除数量, 错误描述列表)，失败不阻断启动。
pub(crate) fn purge_managed_links_in(home: &Path) -> (u32, Vec<String>) {
    let mut removed = 0u32;
    let mut errors = Vec::new();

    let central = home.join(".skills-manager").join("skills");
    let target_dir = home.join(".agents").join("skills");
    if !target_dir.is_dir() {
        return (removed, errors);
    }

    let Ok(entries) = fs::read_dir(&target_dir) else {
        errors.push(format!("cannot read {}", target_dir.display()));
        return (removed, errors);
    };

    let central_canonical = fs::canonicalize(&central).ok();

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_junction_or_symlink(&path) {
            continue;
        }

        // 目标落在中央仓库内 → 是本应用创建的托管链接；
        // 链接已断（无法 canonicalize）时退而比较同名目录是否存在于中央仓库。
        let points_to_central = match (fs::canonicalize(&path), central_canonical.as_ref()) {
            (Ok(resolved), Some(central_path)) => resolved.starts_with(central_path),
            _ => false,
        };
        let same_name_as_central = central.join(entry.file_name()).is_dir();
        if !points_to_central && !same_name_as_central {
            continue;
        }

        match remove_link(&path) {
            Ok(()) => removed += 1,
            Err(error) => errors.push(format!("{}: {}", path.display(), error)),
        }
    }

    (removed, errors)
}

#[derive(Debug, Error)]
pub enum AppSettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Agent already exists: {0}")]
    AgentAlreadyExists(String),

    #[error("Invalid agent config: {0}")]
    InvalidAgent(String),
}

pub struct AppSettingsManager {
    config_path: PathBuf,
    config: AppConfig,
}

impl AppSettingsManager {
    fn known_agent_presets() -> Vec<AgentConfig> {
        vec![
            AgentConfig {
                name: "claude".to_string(),
                display_name: "Claude Code".to_string(),
                path: "~/.claude".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "cursor".to_string(),
                display_name: "Cursor".to_string(),
                path: "~/.cursor".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "antigravity".to_string(),
                display_name: "Google Antigravity".to_string(),
                path: "~/.gemini".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![
                    "~/.antigravity".to_string(),
                    "~/.gemini/config".to_string(),
                    "~/.gemini/antigravity-cli/builtin".to_string(),
                ],
            },
            AgentConfig {
                name: "codex".to_string(),
                display_name: "Codex".to_string(),
                path: "~/.codex".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "copilot".to_string(),
                display_name: "GitHub Copilot".to_string(),
                path: "~/.copilot".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "workbuddy".to_string(),
                display_name: "WorkBuddy".to_string(),
                path: "~/.workbuddy".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec!["~/WorkBuddy".to_string()],
            },
            AgentConfig {
                name: "zcode".to_string(),
                display_name: "ZCode".to_string(),
                path: "~/.zcode".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![
                    "~/ZCodeProject".to_string(),
                    "~/AppData/Roaming/ZCode".to_string(),
                ],
            },
            AgentConfig {
                name: "openclaw".to_string(),
                display_name: "OpenClaw".to_string(),
                path: "~/.openclaw".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "opencode".to_string(),
                display_name: "OpenCode".to_string(),
                path: "~/.config/opencode".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![
                    "~/.opencode".to_string(),
                    "~/AppData/Roaming/ai.opencode.desktop".to_string(),
                ],
            },
            AgentConfig {
                name: "kimi".to_string(),
                display_name: "Kimi Code".to_string(),
                path: "~/.kimi-code".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "marscode".to_string(),
                display_name: "MarsCode".to_string(),
                path: "~/.marscode".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![
                    "~/.marscode/builtin".to_string(),
                    "~/.marscode/builtin_skills".to_string(),
                ],
            },
            AgentConfig {
                name: "openclaude".to_string(),
                display_name: "OpenClaude".to_string(),
                path: "~/.openclaude".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "hermes".to_string(),
                display_name: "Hermes Agent".to_string(),
                path: "~/.hermes".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "grok".to_string(),
                display_name: "Grok".to_string(),
                path: "~/.grok".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "trae".to_string(),
                display_name: "Trae".to_string(),
                path: "~/.trae".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec!["~/.trae-cn".to_string()],
            },
            AgentConfig {
                name: "qoder".to_string(),
                display_name: "Qoder".to_string(),
                path: "~/.qoder".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "kiro".to_string(),
                display_name: "Kiro".to_string(),
                path: "~/.kiro".to_string(),
                skills_path: "skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "pi".to_string(),
                display_name: "Pi".to_string(),
                path: "~/.pi".to_string(),
                skills_path: "agent/skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
            AgentConfig {
                name: "omp".to_string(),
                display_name: "OMP".to_string(),
                path: "~/.omp".to_string(),
                skills_path: "agent/skills".to_string(),
                enabled: true,
                detected: false,
                extra_paths: vec![],
            },
        ]
    }

    pub fn load_or_create(config_path: &Path) -> Result<Self, AppSettingsError> {
        let mut config = if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let mut loaded: AppConfig = match serde_json::from_str(&content) {
                Ok(config) => config,
                Err(error) => {
                    backup_invalid_config(config_path, &content, &error);
                    write_default_config(config_path)?
                }
            };

            if loaded.schema_version != CURRENT_SCHEMA_VERSION {
                eprintln!(
                    "[settings] schema_version mismatch (file={:?}, current={}). Dropping skill_states and rebuilding on next scan.",
                    loaded.schema_version, CURRENT_SCHEMA_VERSION
                );
                loaded.schema_version = CURRENT_SCHEMA_VERSION.to_string();
                loaded.skill_states.clear();

                let content = serde_json::to_string_pretty(&loaded)?;
                atomic_write(config_path, &content)?;
            }

            loaded
        } else {
            write_default_config(config_path)?
        };

        // `agents`（Universal Agents，~/.agents）是所有 CLI 共享的目录，
        // 一旦参与批量启用就会把技能散播到所有工具，因此彻底移除该预设。
        const REMOVED_PRESETS: &[&str] = &["trae-cn", "commandcode", "lingma", "arkcli", "agents"];
        let had_universal_agents = config
            .agents
            .iter()
            .any(|agent| agent.name == "agents");
        let before_len = config.agents.len();
        config
            .agents
            .retain(|agent| !REMOVED_PRESETS.contains(&agent.name.as_str()));
        let mut config_updated = config.agents.len() != before_len;
        if config_updated {
            eprintln!(
                "[settings] removed deprecated agents: {:?}",
                REMOVED_PRESETS
            );
        }
        if prune_deprecated_agent_refs(&mut config, REMOVED_PRESETS) {
            config_updated = true;
        }
        if had_universal_agents {
            let (removed_links, errors) = match dirs::home_dir() {
                Some(home) => purge_managed_links_in(&home),
                None => (
                    0,
                    vec!["cannot resolve home directory".to_string()],
                ),
            };
            if removed_links > 0 {
                eprintln!(
                    "[settings] removed {} managed link(s) from ~/.agents/skills",
                    removed_links
                );
            }
            for error in errors {
                eprintln!("[settings] ~/.agents cleanup skipped {:?}", error);
            }
        }

        for preset in Self::known_agent_presets() {
            if let Some(existing) = config
                .agents
                .iter_mut()
                .find(|agent| agent.name == preset.name)
            {
                if existing.path != preset.path
                    && (existing.name == "antigravity" || existing.name == "opencode")
                {
                    eprintln!(
                        "[settings] updating path for {}: {} -> {}",
                        existing.name, existing.path, preset.path
                    );
                    existing.path = preset.path.clone();
                    config_updated = true;
                }

                if existing.display_name != preset.display_name {
                    existing.display_name = preset.display_name.clone();
                    config_updated = true;
                }

                for ep in &preset.extra_paths {
                    if !existing.extra_paths.contains(ep) {
                        existing.extra_paths.push(ep.clone());
                        config_updated = true;
                    }
                }

                if existing.skills_path != preset.skills_path {
                    eprintln!(
                        "[settings] fixing skills_path for {}: {} -> {}",
                        existing.name, existing.skills_path, preset.skills_path
                    );
                    existing.skills_path = preset.skills_path.clone();
                    config_updated = true;
                }
            } else {
                eprintln!("[settings] adding missing agent: {}", preset.name);
                config.agents.push(preset);
                config_updated = true;
            }
        }

        if config_updated {
            let content = serde_json::to_string_pretty(&config)?;
            atomic_write(config_path, &content)?;
        }

        Ok(Self {
            config_path: config_path.to_path_buf(),
            config,
        })
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    pub fn get_config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    pub fn save(&self) -> Result<(), AppSettingsError> {
        let content = serde_json::to_string_pretty(&self.config)?;
        atomic_write(&self.config_path, &content)?;
        Ok(())
    }

    /// 保留字：与前端 SOURCE 语义冲突的 agent name 不允许使用。
    const RESERVED_AGENT_NAMES: &[&str] = &["global", "all"];

    /// 把任意输入归一化为合法的机器名：trim → 小写 → 空格转 `-` → 仅保留 [a-z0-9_-]。
    fn normalize_agent_name(raw: &str) -> String {
        raw.trim()
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '_')
            .collect()
    }

    /// 归一化 skills 子路径：trim、去首尾斜杠、反斜杠统一为 `/`。
    fn normalize_skills_path(raw: &str) -> String {
        raw.trim().replace('\\', "/").trim_matches('/').to_string()
    }

    pub fn add_agent(&mut self, mut agent: AgentConfig) -> Result<(), AppSettingsError> {
        agent.name = Self::normalize_agent_name(&agent.name);
        agent.path = agent.path.trim().to_string();
        agent.skills_path = Self::normalize_skills_path(&agent.skills_path);

        if agent.name.is_empty() {
            return Err(AppSettingsError::InvalidAgent(
                "Agent name is empty or contains no valid characters [a-z0-9_-]".to_string(),
            ));
        }
        if Self::RESERVED_AGENT_NAMES.contains(&agent.name.as_str()) {
            return Err(AppSettingsError::InvalidAgent(format!(
                "Agent name '{}' is reserved",
                agent.name
            )));
        }
        if agent.path.is_empty() {
            return Err(AppSettingsError::InvalidAgent(
                "Agent path is empty".to_string(),
            ));
        }
        if agent.skills_path.is_empty() {
            return Err(AppSettingsError::InvalidAgent(
                "Agent skills_path is empty".to_string(),
            ));
        }

        if self
            .config
            .agents
            .iter()
            .any(|existing| existing.name == agent.name)
        {
            return Err(AppSettingsError::AgentAlreadyExists(format!(
                "Agent '{}' already exists",
                agent.name
            )));
        }

        self.config.agents.push(agent);
        Ok(())
    }

    pub fn remove_agent(&mut self, name: &str) -> Result<(), AppSettingsError> {
        let original_len = self.config.agents.len();
        self.config.agents.retain(|agent| agent.name != name);

        if self.config.agents.len() == original_len {
            return Err(AppSettingsError::AgentNotFound(name.to_string()));
        }

        // 清理 skill_states 中对该 agent 的引用：磁盘上的链接不动，由 scanner 自愈。
        for entry in self.config.skill_states.values_mut() {
            entry.open.retain(|s| s != name);
            entry.sources.retain(|s| s != name);
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_agent(
        &mut self,
        name: &str,
        updated: AgentConfig,
    ) -> Result<(), AppSettingsError> {
        let pos = self
            .config
            .agents
            .iter()
            .position(|agent| agent.name == name)
            .ok_or_else(|| AppSettingsError::AgentNotFound(name.to_string()))?;

        self.config.agents[pos] = updated;
        Ok(())
    }

    pub fn set_linking_strategy(&mut self, strategy: LinkStrategy) -> Result<(), AppSettingsError> {
        self.config.linking_strategy = strategy;
        Ok(())
    }

    pub fn update_language(&mut self, lang: &str) -> Result<(), AppSettingsError> {
        self.config.language = lang.to_string();
        self.save()?;
        Ok(())
    }

    pub fn set_skill_hide_prefixes(
        &mut self,
        prefixes: Vec<String>,
    ) -> Result<(), AppSettingsError> {
        let mut seen = std::collections::HashSet::new();
        let normalized: Vec<String> = prefixes
            .into_iter()
            .map(|prefix| prefix.trim().to_string())
            .filter(|prefix| !prefix.is_empty() && seen.insert(prefix.clone()))
            .collect();
        self.config.skill_hide_prefixes = normalized;
        self.save()?;
        Ok(())
    }

    pub fn get_skills_manager_dir() -> PathBuf {
        dirs::home_dir().unwrap().join(".skills-manager")
    }

    pub fn get_skills_dir() -> PathBuf {
        Self::get_skills_manager_dir().join("skills")
    }

    pub fn get_config_path() -> PathBuf {
        Self::get_skills_manager_dir().join("config.json")
    }

    pub fn detect_agents(&mut self) -> Result<usize, AppSettingsError> {
        let mut detected_count = 0;
        let home_dir = dirs::home_dir().ok_or_else(|| {
            AppSettingsError::AgentNotFound("Cannot find home directory".to_string())
        })?;

        for agent in &mut self.config.agents {
            let agent_path = if agent.path.starts_with("~/") {
                home_dir.join(&agent.path[2..])
            } else if agent.path.starts_with('~') {
                home_dir.join(&agent.path[1..])
            } else {
                home_dir.join(&agent.path)
            };

            let extra_exists = agent.extra_paths.iter().any(|extra_path| {
                let path = if extra_path.starts_with("~/") {
                    home_dir.join(&extra_path[2..])
                } else if extra_path.starts_with('~') {
                    home_dir.join(&extra_path[1..])
                } else {
                    home_dir.join(extra_path.as_str())
                };
                path.exists()
            });

            eprintln!(
                "Checking agent path: {:?}, exists: {}",
                agent_path,
                agent_path.exists()
            );
            agent.detected = agent_path.exists() || extra_exists;

            if agent.detected {
                detected_count += 1;
            }
        }

        eprintln!(
            "Detected {} out of {} agents",
            detected_count,
            self.config.agents.len()
        );
        Ok(detected_count)
    }
}
