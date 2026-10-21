//! 一键归并单测：干跑分类、执行移动、冲突处置、状态同步。

#[cfg(test)]
mod tests {
    use crate::consolidate::{
        apply_consolidation, apply_states, plan_consolidation, ConflictDecision,
    };
    use crate::models::{AgentConfig, SkillEntry, SOURCE_GLOBAL};
    use crate::trash::list_trash_in;
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn agent_config(home: &Path, name: &str) -> AgentConfig {
        AgentConfig {
            name: name.to_string(),
            display_name: name.to_string(),
            path: home.join(format!(".{}", name)).to_string_lossy().to_string(),
            skills_path: "skills".to_string(),
            enabled: true,
            detected: true,
            extra_paths: Vec::new(),
        }
    }

    fn write_skill(dir: &Path, id: &str, name: &str, body: &str) {
        let target = dir.join(id);
        fs::create_dir_all(&target).unwrap();
        fs::write(
            target.join("SKILL.md"),
            format!("---\nname: {}\ndescription: demo\n---\n{}", name, body),
        )
        .unwrap();
    }

    struct Fixture {
        _tmp: TempDir,
        home: PathBuf,
        central: PathBuf,
        trash_root: PathBuf,
        agent_skills: PathBuf,
        agent: AgentConfig,
    }

    fn fixture() -> Fixture {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        let central = home.join("skills-manager").join("skills");
        let trash_root = home.join("skills-manager").join("trash");
        let agent = agent_config(&home, "claude");
        let agent_skills = PathBuf::from(&agent.path).join(&agent.skills_path);
        fs::create_dir_all(&central).unwrap();
        fs::create_dir_all(&agent_skills).unwrap();
        Fixture {
            _tmp: tmp,
            home,
            central,
            trash_root,
            agent_skills,
            agent,
        }
    }

    #[test]
    fn plan_classifies_movable_conflict_and_skips() {
        let fx = fixture();
        write_skill(&fx.agent_skills, "alpha", "Alpha", "");
        write_skill(&fx.agent_skills, "beta", "Beta", "");
        write_skill(&fx.central, "beta", "Beta", "");
        fs::create_dir_all(fx.agent_skills.join("shell-only")).unwrap();
        fs::create_dir_all(fx.agent_skills.join("plugins")).unwrap();

        let plan = plan_consolidation(&fx.agent, &fx.central, &fx.home);

        assert_eq!(plan.agent, "claude");
        let movable: Vec<&str> = plan.movable.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(movable, vec!["alpha"]);
        assert_eq!(plan.movable[0].name, "Alpha");

        let conflicts: Vec<&str> = plan.conflicts.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(conflicts, vec!["beta"]);
        assert_eq!(
            Path::new(&plan.conflicts[0].central_path),
            fx.central.join("beta").as_path()
        );

        let reasons: HashMap<&str, &str> = plan
            .skipped
            .iter()
            .map(|s| (s.id.as_str(), s.reason.as_str()))
            .collect();
        assert_eq!(reasons.get("shell-only"), Some(&"missing_skill_md"));
        assert_eq!(reasons.get("plugins"), Some(&"plugin_dir"));
    }

    #[test]
    fn plan_skips_plugin_cache_dirs_entirely() {
        let fx = fixture();
        let plugin_root = fx.home.join(".codex");
        let plugin_skills = plugin_root.join("plugins").join("skills");
        write_skill(&plugin_skills, "cached", "Cached", "");

        let mut cached_agent = fx.agent.clone();
        cached_agent.path = plugin_root.to_string_lossy().to_string();
        cached_agent.skills_path = "plugins/skills".to_string();

        let plan = plan_consolidation(&cached_agent, &fx.central, &fx.home);
        assert!(plan.movable.is_empty(), "插件缓存不应被归并");
        assert!(plan.conflicts.is_empty());
        let reasons: HashMap<&str, &str> = plan
            .skipped
            .iter()
            .map(|s| (s.id.as_str(), s.reason.as_str()))
            .collect();
        assert_eq!(reasons.get("cached"), Some(&"plugin_cache"));
    }

    #[test]
    fn plan_lists_extra_path_skills_as_skipped() {
        let fx = fixture();
        let extra = fx.home.join(".claude-extra").join("skills");
        write_skill(&extra, "extra-skill", "Extra", "");

        let mut agent = fx.agent.clone();
        agent.extra_paths = vec![fx.home.join(".claude-extra").to_string_lossy().to_string()];

        let plan = plan_consolidation(&agent, &fx.central, &fx.home);
        let reasons: HashMap<&str, &str> = plan
            .skipped
            .iter()
            .map(|s| (s.id.as_str(), s.reason.as_str()))
            .collect();
        assert_eq!(reasons.get("extra-skill"), Some(&"extra_path"));
        assert!(plan.movable.is_empty());
    }

    #[test]
    fn apply_moves_skill_into_central_and_syncs_states() {
        let fx = fixture();
        write_skill(&fx.agent_skills, "alpha", "Alpha", "BODY");

        let mut states: HashMap<String, SkillEntry> = HashMap::new();
        states.insert(
            "alpha".to_string(),
            SkillEntry {
                sources: vec!["claude".to_string()],
                primary: "claude".to_string(),
                open: vec!["cursor".to_string()],
            },
        );

        let plan = plan_consolidation(&fx.agent, &fx.central, &fx.home);
        let report = apply_consolidation(&fx.agent, &plan, &fx.central, &[], &fx.trash_root);

        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert_eq!(report.moved.len(), 1);
        assert_eq!(report.moved[0].from_id, "alpha");
        assert_eq!(report.moved[0].to_id, "alpha");
        assert!(!fx.agent_skills.join("alpha").exists());
        assert!(fx.central.join("alpha").join("SKILL.md").exists());

        apply_states(&mut states, "claude", &report);
        let entry = states.get("alpha").expect("state should survive");
        assert_eq!(entry.sources, vec![SOURCE_GLOBAL.to_string()]);
        assert_eq!(entry.primary, SOURCE_GLOBAL);
        assert_eq!(entry.open, vec!["cursor".to_string()]);
    }

    #[test]
    fn apply_creates_state_when_missing() {
        let fx = fixture();
        write_skill(&fx.agent_skills, "fresh", "Fresh", "");

        let mut states: HashMap<String, SkillEntry> = HashMap::new();
        let plan = plan_consolidation(&fx.agent, &fx.central, &fx.home);
        let report = apply_consolidation(&fx.agent, &plan, &fx.central, &[], &fx.trash_root);
        apply_states(&mut states, "claude", &report);

        let entry = states.get("fresh").expect("state should be created");
        assert_eq!(entry.sources, vec![SOURCE_GLOBAL.to_string()]);
        assert_eq!(entry.primary, SOURCE_GLOBAL);
    }

    #[test]
    fn apply_overwrite_trashes_existing_central_copy() {
        let fx = fixture();
        write_skill(&fx.agent_skills, "beta", "Beta", "NEW");
        write_skill(&fx.central, "beta", "Beta", "OLD");

        let plan = plan_consolidation(&fx.agent, &fx.central, &fx.home);
        let report = apply_consolidation(
            &fx.agent,
            &plan,
            &fx.central,
            &[ConflictDecision {
                id: "beta".to_string(),
                action: "overwrite".to_string(),
            }],
            &fx.trash_root,
        );

        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert_eq!(report.moved.len(), 1);
        assert_eq!(report.trash_ids.len(), 1);
        assert!(!fx.agent_skills.join("beta").exists());
        let content = fs::read_to_string(fx.central.join("beta").join("SKILL.md")).unwrap();
        assert!(content.contains("NEW"), "中央副本应被覆盖为新内容: {}", content);

        let trash = list_trash_in(&fx.trash_root);
        assert_eq!(trash.len(), 1);
        assert_eq!(trash[0].reason, "overwrite");
    }

    #[test]
    fn apply_rename_creates_suffixed_id_and_removes_old_state() {
        let fx = fixture();
        write_skill(&fx.agent_skills, "beta", "Beta", "NEW");
        write_skill(&fx.central, "beta", "Beta", "OLD");

        let mut states: HashMap<String, SkillEntry> = HashMap::new();
        states.insert(
            "beta".to_string(),
            SkillEntry {
                sources: vec!["claude".to_string()],
                primary: "claude".to_string(),
                open: vec![],
            },
        );

        let plan = plan_consolidation(&fx.agent, &fx.central, &fx.home);
        let report = apply_consolidation(
            &fx.agent,
            &plan,
            &fx.central,
            &[ConflictDecision {
                id: "beta".to_string(),
                action: "rename".to_string(),
            }],
            &fx.trash_root,
        );

        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert_eq!(report.moved[0].to_id, "beta-claude");
        assert!(fx.central.join("beta-claude").join("SKILL.md").exists());
        let old_content = fs::read_to_string(fx.central.join("beta").join("SKILL.md")).unwrap();
        assert!(old_content.contains("OLD"), "原中央技能不应被改动");

        apply_states(&mut states, "claude", &report);
        assert!(!states.contains_key("beta"), "旧 id 无源后应被清理");
        let renamed = states.get("beta-claude").expect("新 id 应存在");
        assert_eq!(renamed.sources, vec![SOURCE_GLOBAL.to_string()]);
    }

    #[test]
    fn apply_skip_keeps_everything_untouched() {
        let fx = fixture();
        write_skill(&fx.agent_skills, "beta", "Beta", "NEW");
        write_skill(&fx.central, "beta", "Beta", "OLD");

        let plan = plan_consolidation(&fx.agent, &fx.central, &fx.home);
        let report = apply_consolidation(&fx.agent, &plan, &fx.central, &[], &fx.trash_root);

        assert!(report.moved.is_empty());
        assert_eq!(report.skipped, vec!["beta".to_string()]);
        assert!(fx.agent_skills.join("beta").join("SKILL.md").exists());
        let content = fs::read_to_string(fx.central.join("beta").join("SKILL.md")).unwrap();
        assert!(content.contains("OLD"));
        assert!(list_trash_in(&fx.trash_root).is_empty());
    }

    /// 覆盖归并中途不能出现「旧副本已归档、新副本还没到位」的空窗：
    /// 结束时中央必须是新内容，旧内容可在回收站找回，暂存目录不残留。
    #[test]
    fn overwrite_moves_new_copy_in_without_leaving_staging() {
        let fx = fixture();
        write_skill(&fx.agent_skills, "beta", "Beta", "NEW");
        write_skill(&fx.central, "beta", "Beta", "OLD");

        let plan = plan_consolidation(&fx.agent, &fx.central, &fx.home);
        let report = apply_consolidation(
            &fx.agent,
            &plan,
            &fx.central,
            &[ConflictDecision {
                id: "beta".to_string(),
                action: "overwrite".to_string(),
            }],
            &fx.trash_root,
        );

        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert_eq!(report.moved.len(), 1);
        assert_eq!(report.trash_ids.len(), 1);
        // 中央只有新内容，没有暂存残留
        let content = fs::read_to_string(fx.central.join("beta").join("SKILL.md")).unwrap();
        assert!(content.contains("NEW"), "中央应为新副本: {}", content);
        let staging = fx.home.join("skills-manager").join(".staging");
        assert!(
            !staging.join("beta").exists(),
            ".staging 不应残留待转正的副本"
        );
        // 旧内容完整进了回收站
        let trash = list_trash_in(&fx.trash_root);
        assert_eq!(trash.len(), 1);
        assert_eq!(trash[0].reason, "overwrite");
        let payload = fx
            .trash_root
            .join(&trash[0].trash_id)
            .join(&trash[0].entries[0].payload)
            .join("SKILL.md");
        let trashed = fs::read_to_string(payload).unwrap();
        assert!(trashed.contains("OLD"), "回收站应保存旧副本: {}", trashed);
    }

    /// 上次归并/删除中断留下的空壳目录（无 SKILL.md 且为空）不应被当成冲突
    #[test]
    fn stale_empty_central_shell_is_movable() {
        let fx = fixture();
        write_skill(&fx.agent_skills, "beta", "Beta", "NEW");
        fs::create_dir_all(fx.central.join("beta")).unwrap();

        let plan = plan_consolidation(&fx.agent, &fx.central, &fx.home);
        assert!(
            plan.conflicts.is_empty(),
            "空壳不应进冲突: {:?}",
            plan.conflicts
        );
        assert_eq!(plan.movable.len(), 1);

        let report = apply_consolidation(&fx.agent, &plan, &fx.central, &[], &fx.trash_root);
        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert!(fx.central.join("beta").join("SKILL.md").exists());
        assert!(!fx.agent_skills.join("beta").exists());
    }
}
