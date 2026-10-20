//! 回收站单测：归档 / 列取 / 恢复（含冲突） / 彻底删除 / 清空。

#[cfg(test)]
mod tests {
    use crate::trash::{
        empty_trash_in, list_trash_in, move_to_trash_in, purge_trash_entry_in, restore_trash_in,
    };
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn write_skill(dir: &Path, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    fn setup() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        let central = tmp.path().join("skills");
        let trash_root = tmp.path().join("trash");
        (tmp, central, trash_root)
    }

    #[test]
    fn move_to_trash_relocates_dir_and_writes_meta() {
        let (_tmp, central, trash_root) = setup();
        let skill_dir = central.join("demo");
        write_skill(&skill_dir, "---\nname: demo\n---\n");

        let id = move_to_trash_in(
            &trash_root,
            "demo",
            "Demo",
            "delete",
            &[("global".to_string(), skill_dir.clone())],
            None,
        )
        .expect("move to trash should succeed");

        assert!(!skill_dir.exists(), "原始目录应被移走");
        let items = list_trash_in(&trash_root);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].trash_id, id);
        assert_eq!(items[0].skill_id, "demo");
        assert_eq!(items[0].name, "Demo");
        assert_eq!(items[0].reason, "delete");
        assert_eq!(items[0].entries.len(), 1);
        assert_eq!(
            Path::new(&items[0].entries[0].original_path),
            skill_dir.as_path()
        );

        // payload 里应该保留 SKILL.md
        let payload = trash_root.join(&id).join(&items[0].entries[0].payload);
        assert!(payload.join("SKILL.md").exists());
    }

    #[test]
    fn move_to_trash_dedupes_identical_paths() {
        let (_tmp, central, trash_root) = setup();
        let skill_dir = central.join("demo");
        write_skill(&skill_dir, "---\nname: demo\n---\n");

        let id = move_to_trash_in(
            &trash_root,
            "demo",
            "Demo",
            "delete",
            &[
                ("claude".to_string(), skill_dir.clone()),
                ("global".to_string(), skill_dir.clone()),
            ],
            None,
        )
        .unwrap();

        let items = list_trash_in(&trash_root);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].trash_id, id);
        assert_eq!(items[0].entries.len(), 1, "重复路径只应归档一次");
    }

    #[test]
    fn move_to_trash_rejects_when_nothing_exists() {
        let (_tmp, central, trash_root) = setup();
        let err = move_to_trash_in(
            &trash_root,
            "ghost",
            "Ghost",
            "delete",
            &[("global".to_string(), central.join("ghost"))],
            None,
        )
        .unwrap_err();
        assert!(err.contains("ghost"), "错误信息应包含技能 id: {}", err);
    }

    #[test]
    fn restore_puts_dir_back_and_clears_entry() {
        let (_tmp, central, trash_root) = setup();
        let skill_dir = central.join("demo");
        write_skill(&skill_dir, "---\nname: demo\n---\n");

        let id = move_to_trash_in(
            &trash_root,
            "demo",
            "Demo",
            "delete",
            &[("global".to_string(), skill_dir.clone())],
            None,
        )
        .unwrap();

        let report = restore_trash_in(&trash_root, &[id.clone()]);
        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert_eq!(report.restored, vec![id]);
        assert!(skill_dir.join("SKILL.md").exists());
        assert!(list_trash_in(&trash_root).is_empty());
    }

    #[test]
    fn restore_skips_conflict_without_overwriting() {
        let (_tmp, central, trash_root) = setup();
        let skill_dir = central.join("demo");
        write_skill(&skill_dir, "---\nname: demo\n---\nOLD");

        let id = move_to_trash_in(
            &trash_root,
            "demo",
            "Demo",
            "delete",
            &[("global".to_string(), skill_dir.clone())],
            None,
        )
        .unwrap();

        // 删除后用户又在原位置创建了同名技能
        write_skill(&skill_dir, "---\nname: demo\n---\nNEW");

        let report = restore_trash_in(&trash_root, &[id.clone()]);
        assert!(report.restored.is_empty(), "冲突时不应恢复");
        assert_eq!(report.skipped, vec![id.clone()]);
        let content = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        assert!(content.contains("NEW"), "现有目录必须保持不变: {}", content);
        // 条目仍留在回收站
        assert_eq!(list_trash_in(&trash_root).len(), 1);
    }

    #[test]
    fn restore_reports_missing_entry() {
        let (_tmp, _central, trash_root) = setup();
        let report = restore_trash_in(&trash_root, &["nope".to_string()]);
        assert!(report.restored.is_empty());
        assert_eq!(report.errors.len(), 1);
    }

    #[test]
    fn repeated_deletes_create_distinct_entries() {
        let (_tmp, central, trash_root) = setup();
        let skill_dir = central.join("demo");

        write_skill(&skill_dir, "---\nname: demo\n---\nFIRST");
        let first = move_to_trash_in(
            &trash_root,
            "demo",
            "Demo",
            "delete",
            &[("global".to_string(), skill_dir.clone())],
            None,
        )
        .unwrap();

        write_skill(&skill_dir, "---\nname: demo\n---\nSECOND");
        let second = move_to_trash_in(
            &trash_root,
            "demo",
            "Demo",
            "delete",
            &[("global".to_string(), skill_dir.clone())],
            None,
        )
        .unwrap();

        assert_ne!(first, second, "同 id 的两次删除应产生不同条目");
        assert_eq!(list_trash_in(&trash_root).len(), 2);
    }

    #[test]
    fn purge_and_empty_remove_entries() {
        let (_tmp, central, trash_root) = setup();

        let keep = central.join("keep");
        write_skill(&keep, "---\nname: keep\n---\n");
        let keep_id = move_to_trash_in(
            &trash_root,
            "keep",
            "Keep",
            "delete",
            &[("global".to_string(), keep)],
            None,
        )
        .unwrap();

        let gone = central.join("gone");
        write_skill(&gone, "---\nname: gone\n---\n");
        move_to_trash_in(
            &trash_root,
            "gone",
            "Gone",
            "delete",
            &[("global".to_string(), gone)],
            None,
        )
        .unwrap();

        assert_eq!(list_trash_in(&trash_root).len(), 2);
        purge_trash_entry_in(&trash_root, &keep_id).unwrap();
        assert_eq!(list_trash_in(&trash_root).len(), 1);

        let removed = empty_trash_in(&trash_root).unwrap();
        assert_eq!(removed, 1);
        assert!(list_trash_in(&trash_root).is_empty());
    }

    #[test]
    fn list_trash_skips_corrupted_entry() {
        let (_tmp, central, trash_root) = setup();
        let skill_dir = central.join("demo");
        write_skill(&skill_dir, "---\nname: demo\n---\n");
        move_to_trash_in(
            &trash_root,
            "demo",
            "Demo",
            "delete",
            &[("global".to_string(), skill_dir)],
            None,
        )
        .unwrap();

        fs::create_dir_all(trash_root.join("broken-entry")).unwrap();
        fs::write(trash_root.join("broken-entry").join("meta.json"), "{not json").unwrap();

        assert_eq!(list_trash_in(&trash_root).len(), 1, "损坏条目应被跳过");
    }
}
