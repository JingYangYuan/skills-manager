use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AgentConfig, LinkStrategy, CURRENT_SCHEMA_VERSION};

    fn test_agent() -> AgentConfig {
        AgentConfig {
            name: "test-agent".to_string(),
            display_name: "Test Agent".to_string(),
            path: "~/.test".to_string(),
            skills_path: "skills".to_string(),
            enabled: true,
            detected: false,
            extra_paths: vec![],
        }
    }

    #[test]
    fn test_create_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        let config = manager.get_config();
        let agent_names: Vec<&str> = config
            .agents
            .iter()
            .map(|agent| agent.name.as_str())
            .collect();

        assert!(config.agents.len() >= 5);
        assert!(agent_names.contains(&"claude"));
        assert!(agent_names.contains(&"cursor"));
        assert!(agent_names.contains(&"codex"));
        assert!(agent_names.contains(&"openclaw"));
        assert!(agent_names.contains(&"opencode"));
        assert_eq!(
            config
                .agents
                .iter()
                .find(|agent| agent.name == "claude")
                .unwrap()
                .skills_path,
            "skills"
        );
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager =
            crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        let default_count = manager.get_config().agents.len();

        manager.add_agent(test_agent()).unwrap();
        manager.save().unwrap();

        let manager2 = crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        let config = manager2.get_config();

        assert_eq!(config.agents.len(), default_count + 1);
        assert_eq!(config.agents.last().unwrap().name, "test-agent");
    }

    #[test]
    fn test_remove_agent() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager =
            crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        let default_count = manager.get_config().agents.len();

        manager.add_agent(test_agent()).unwrap();
        assert_eq!(manager.get_config().agents.len(), default_count + 1);

        manager.remove_agent("test-agent").unwrap();
        assert_eq!(manager.get_config().agents.len(), default_count);
    }

    #[test]
    fn test_schema_version_mismatch_resets_skill_states() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let legacy = serde_json::json!({
            "schema_version": "v0-legacy",
            "linking_strategy": "symlink",
            "agents": [],
            "skill_states": {
                "stale-skill": { "sources": ["global"], "primary": "global", "open": ["claude"] }
            },
            "language": "zh",
            "skill_hide_prefixes": []
        });
        std::fs::write(&config_path, serde_json::to_string_pretty(&legacy).unwrap()).unwrap();

        let manager = crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        let config = manager.get_config();

        assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(
            config.skill_states.is_empty(),
            "skill_states should be cleared on mismatch"
        );

        let on_disk: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            on_disk["schema_version"].as_str(),
            Some(CURRENT_SCHEMA_VERSION)
        );
    }

    #[test]
    fn test_schema_version_match_preserves_skill_states() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let good = serde_json::json!({
            "schema_version": CURRENT_SCHEMA_VERSION,
            "linking_strategy": "symlink",
            "agents": [],
            "skill_states": {
                "keep-me": { "sources": ["global"], "primary": "global", "open": ["claude"] }
            },
            "language": "zh",
            "skill_hide_prefixes": []
        });
        std::fs::write(&config_path, serde_json::to_string_pretty(&good).unwrap()).unwrap();

        let manager = crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        assert!(manager.get_config().skill_states.contains_key("keep-me"));
    }

    #[test]
    fn test_invalid_json_is_backed_up_and_reset_to_default() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        std::fs::write(&config_path, "{not-json").unwrap();

        let manager = crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        let config = manager.get_config();
        let on_disk: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();

        assert!(!config.agents.is_empty());
        assert_eq!(
            on_disk["schema_version"].as_str(),
            Some(CURRENT_SCHEMA_VERSION)
        );

        let backup_count = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("config.json.invalid-")
            })
            .count();
        assert_eq!(backup_count, 1);
    }

    #[test]
    fn test_update_linking_strategy() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager =
            crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        manager.set_linking_strategy(LinkStrategy::Copy).unwrap();
        manager.save().unwrap();

        let manager2 = crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        assert_eq!(manager2.get_config().linking_strategy, LinkStrategy::Copy);
    }

    #[test]
    fn test_presets_include_pi_and_omp() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        let config = manager.get_config();

        let pi = config
            .agents
            .iter()
            .find(|agent| agent.name == "pi")
            .expect("pi preset missing");
        assert_eq!(pi.path, "~/.pi");
        assert_eq!(pi.skills_path, "agent/skills");

        let omp = config
            .agents
            .iter()
            .find(|agent| agent.name == "omp")
            .expect("omp preset missing");
        assert_eq!(omp.path, "~/.omp");
        assert_eq!(omp.skills_path, "agent/skills");
    }

    #[test]
    fn test_legacy_config_gets_new_presets_on_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // 旧版本 config：不含 pi/omp，加载后应自动补上
        let legacy = serde_json::json!({
            "schema_version": CURRENT_SCHEMA_VERSION,
            "linking_strategy": "symlink",
            "agents": [
                { "name": "claude", "display_name": "Claude Code", "path": "~/.claude",
                  "skills_path": "skills", "enabled": true, "detected": false }
            ],
            "skill_states": {},
            "language": "zh",
            "skill_hide_prefixes": []
        });
        std::fs::write(&config_path, serde_json::to_string_pretty(&legacy).unwrap()).unwrap();

        let manager = crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        let names: Vec<&str> = manager
            .get_config()
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert!(names.contains(&"pi"));
        assert!(names.contains(&"omp"));
    }

    #[test]
    fn test_add_agent_normalizes_name_and_skills_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager =
            crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();

        let mut agent = test_agent();
        agent.name = "  My Custom Agent! " .to_string();
        agent.skills_path = "\\Nested/Dir/\\".to_string();
        agent.path = "  ~/.test  ".to_string();
        manager.add_agent(agent).unwrap();

        let added = manager
            .get_config()
            .agents
            .iter()
            .find(|a| a.name == "my-custom-agent")
            .expect("normalized agent missing");
        assert_eq!(added.skills_path, "Nested/Dir");
        assert_eq!(added.path, "~/.test");
    }

    #[test]
    fn test_add_agent_rejects_invalid_input() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager =
            crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();

        // 保留字
        for reserved in ["global", "all", "GLOBAL"] {
            let mut agent = test_agent();
            agent.name = reserved.to_string();
            assert!(manager.add_agent(agent).is_err(), "{reserved} should be rejected");
        }

        // 非法字符全被过滤后为空
        let mut agent = test_agent();
        agent.name = "!!!".to_string();
        assert!(manager.add_agent(agent).is_err());

        // 空 path / 空 skills_path
        let mut agent = test_agent();
        agent.path = "   ".to_string();
        assert!(manager.add_agent(agent).is_err());

        let mut agent = test_agent();
        agent.skills_path = " / ".to_string();
        assert!(manager.add_agent(agent).is_err());

        // 与内置 preset 重名
        let mut agent = test_agent();
        agent.name = "claude".to_string();
        assert!(manager.add_agent(agent).is_err());
    }

    #[test]
    fn test_remove_agent_cleans_skill_states() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager =
            crate::settings::AppSettingsManager::load_or_create(&config_path).unwrap();
        manager.add_agent(test_agent()).unwrap();

        {
            let config = manager.get_config_mut();
            config.skill_states.insert(
                "demo-skill".to_string(),
                crate::models::SkillEntry {
                    sources: vec!["global".to_string(), "test-agent".to_string()],
                    primary: "global".to_string(),
                    open: vec!["test-agent".to_string()],
                },
            );
        }

        manager.remove_agent("test-agent").unwrap();

        let entry = manager
            .get_config()
            .skill_states
            .get("demo-skill")
            .expect("skill entry should survive");
        assert!(!entry.sources.iter().any(|s| s == "test-agent"));
        assert!(!entry.open.iter().any(|s| s == "test-agent"));
        assert!(entry.sources.iter().any(|s| s == "global"));
    }
}
