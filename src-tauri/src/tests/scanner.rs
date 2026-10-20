use std::fs;
use std::path::Path;

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::{pick_default_primary, SkillEntry, SOURCE_GLOBAL};
    use std::collections::HashMap;

    fn create_test_skill(dir: &Path, name: &str, content: &str) {
        let skill_dir = dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_md = skill_dir.join("SKILL.md");
        fs::write(&skill_md, content).unwrap();
    }

    #[test]
    fn test_pick_default_primary_prefers_global() {
        let sources = vec![
            "cursor".to_string(),
            SOURCE_GLOBAL.to_string(),
            "claude".to_string(),
        ];
        assert_eq!(
            pick_default_primary(&sources),
            Some(SOURCE_GLOBAL.to_string())
        );
    }

    #[test]
    fn test_skill_entry_insert_source_is_idempotent() {
        let mut e = SkillEntry::default();
        e.insert_source(SOURCE_GLOBAL);
        e.insert_source(SOURCE_GLOBAL);
        e.insert_source("claude");
        assert_eq!(
            e.sources,
            vec![SOURCE_GLOBAL.to_string(), "claude".to_string()]
        );
    }

    #[test]
    fn test_skill_entry_derive_native_plus_open() {
        let e = SkillEntry {
            sources: vec![SOURCE_GLOBAL.to_string(), "openclaw".to_string()],
            primary: SOURCE_GLOBAL.to_string(),
            open: vec!["cursor".to_string()],
        };
        let agents = vec![
            "cursor".to_string(),
            "openclaw".to_string(),
            "claude".to_string(),
        ];
        let m = e.derive_agent_enabled(&agents);
        assert_eq!(m.get("cursor"), Some(&true)); // open
        assert_eq!(m.get("openclaw"), Some(&true)); // native
        assert_eq!(m.get("claude"), Some(&false));
    }

    #[test]
    fn test_touch_fs_helpers_dont_panic() {
        // 仅确认 create_test_skill 工具函数工作；扫描本身依赖固定绝对路径，不便于在单测中断言。
        let tmp = tempfile::TempDir::new().unwrap();
        create_test_skill(
            tmp.path(),
            "foo",
            "---\nname: foo\ndescription: d\ncategory: Test\n---\nBody",
        );
        assert!(tmp.path().join("foo").join("SKILL.md").exists());
        let mut _states: HashMap<String, SkillEntry> = HashMap::new();
        // 只调用到 models 层，避免扫描全局目录。
    }

    #[test]
    fn test_scan_and_merge_prunes_stale_open() {
        let tmp = tempfile::TempDir::new().unwrap();
        let skills_dir = tmp.path().join("skills");
        create_test_skill(
            &skills_dir,
            "demo",
            "---\nname: demo\ndescription: d\ncategory: Test\n---\nBody",
        );

        // 绝对路径不会被 ~ 展开，直接指向临时目录
        let agents = vec![crate::models::AgentConfig {
            name: "pi".to_string(),
            display_name: "Pi".to_string(),
            path: tmp.path().to_string_lossy().to_string(),
            skills_path: "skills".to_string(),
            enabled: true,
            detected: true,
            extra_paths: vec![],
        }];

        let mut states: HashMap<String, SkillEntry> = HashMap::new();
        states.insert(
            "demo".to_string(),
            SkillEntry {
                sources: vec!["global".to_string()],
                primary: "global".to_string(),
                // removed-agent 不在 agents 列表；omp 不在 agents 列表；
                // pi 有真实物理副本 → 扫描后成为 native，open 应被裁剪
                open: vec![
                    "removed-agent".to_string(),
                    "omp".to_string(),
                    "pi".to_string(),
                ],
            },
        );

        let out = crate::scanner::scan_and_merge(&mut states, &agents).unwrap();
        let entry = states.get("demo").unwrap();
        assert!(entry.open.is_empty(), "stale open entries should be pruned");

        let meta = out.iter().find(|s| s.id == "demo").unwrap();
        // pi 变为 native（物理副本），仍应通过 sources 派生为启用
        assert_eq!(meta.agent_enabled.get("pi"), Some(&true));
    }
}
