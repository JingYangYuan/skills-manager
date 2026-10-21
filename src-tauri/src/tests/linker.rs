use std::fs;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("target.txt");

        // 创建源文件
        fs::write(&source, b"test content").unwrap();

        // 创建符号链接
        let result = crate::linker::create_symlink(&source, &target);

        assert!(result.is_ok());
        assert!(target.exists());
        // Windows 上 create_symlink 会先把源路径规范化（8.3 短路径 → 长路径），
        // 因此比较规范化后的路径，而不是字面量
        let linked_target = target.read_link().unwrap();
        assert_eq!(
            fs::canonicalize(&linked_target).unwrap(),
            fs::canonicalize(&source).unwrap()
        );

        // 验证内容一致
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_create_copy_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("target.txt");

        fs::write(&source, b"test content").unwrap();

        // 使用复制方式
        let result = crate::linker::create_copy(&source, &target);

        assert!(result.is_ok());
        assert!(target.exists());

        // 验证是副本而不是链接
        assert!(target.read_link().is_err());

        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_remove_link() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("target.txt");

        fs::write(&source, b"test content").unwrap();
        crate::linker::create_symlink(&source, &target).unwrap();

        assert!(target.exists());

        let result = crate::linker::remove_link(&target);

        assert!(result.is_ok());
        assert!(!target.exists());
    }

    #[test]
    fn test_verify_link() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("target.txt");

        fs::write(&source, b"test content").unwrap();
        crate::linker::create_symlink(&source, &target).unwrap();

        assert!(crate::linker::verify_link(&source, &target));

        // 删除源文件，链接应该失效
        fs::remove_file(&source).unwrap();
        assert!(!crate::linker::verify_link(&source, &target));
    }

    /// 目录必须建立真正的链接（junction / symlink），不能退化成复制——
    /// 否则会在 Agent 目录里留下副本，技能就变成不可关闭的 native 项。
    #[test]
    fn test_create_symlink_directory_creates_real_link() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("central").join("demo-skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), b"---\nname: demo\n---\n").unwrap();
        let target = temp_dir.path().join("agent").join("skills").join("demo-skill");

        let result = crate::linker::create_symlink(&source, &target).unwrap();
        assert_eq!(result, crate::linker::LinkResult::Symlink);
        assert!(
            crate::linker::is_junction_or_symlink(&target),
            "目录链接必须被识别为 junction/symlink"
        );
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "---\nname: demo\n---\n"
        );

        // 删除链接不能影响中央库内容
        crate::linker::remove_link(&target).unwrap();
        assert!(source.join("SKILL.md").exists());
    }

    /// Agent 配置里的路径可能带正斜杠（`.config/opencode`、`.pi/agent`、`.omp/agent`）。
    /// 拼进 `mklink` 命令行后，`/opencode` 会被 cmd 当成开关，链接失败后静默降级成副本，
    /// 技能在界面上就再也关不掉。本测试用带 `/` 的路径组件复现该形态。
    #[test]
    fn test_create_symlink_with_mixed_separators() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("central").join("ppt-master");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), b"---\nname: ppt-master\n---\n").unwrap();

        // 目标路径带正斜杠，与真实配置（~/.config/opencode、~/.pi/agent、~/.omp/agent）一致
        let target = temp_dir
            .path()
            .join("home")
            .join(".config/opencode")
            .join("skills")
            .join("ppt-master");

        let result = crate::linker::create_symlink(&source, &target).unwrap();
        assert_eq!(result, crate::linker::LinkResult::Symlink);
        assert!(
            crate::linker::is_junction_or_symlink(&target),
            "含正斜杠的路径也必须建成真链接，不能退化为副本"
        );
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "---\nname: ppt-master\n---\n"
        );

        crate::linker::remove_link(&target).unwrap();
        assert!(source.join("SKILL.md").exists());
    }

    #[test]
    fn test_dirs_identical_and_missing() {
        let temp_dir = TempDir::new().unwrap();
        let a = temp_dir.path().join("a");
        let b = temp_dir.path().join("b");
        fs::create_dir_all(a.join("nested")).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("SKILL.md"), b"same").unwrap();
        fs::write(a.join("nested").join("x.md"), b"nested").unwrap();

        assert!(!crate::linker::dirs_identical(&a, &b));
        assert!(!crate::linker::dirs_identical(&a, &temp_dir.path().join("missing")));

        crate::linker::create_copy(&a, &b).unwrap();
        assert!(crate::linker::dirs_identical(&a, &b));

        fs::write(b.join("nested").join("x.md"), b"changed").unwrap();
        assert!(!crate::linker::dirs_identical(&a, &b));
    }

    #[test]
    fn test_agent_entry_state_detects_identical_copy_and_converts() {
        let temp_dir = TempDir::new().unwrap();
        let central = temp_dir.path().join("central").join("demo");
        fs::create_dir_all(&central).unwrap();
        fs::write(central.join("SKILL.md"), b"content").unwrap();

        let agent_dir = temp_dir.path().join("agent").join("skills").join("demo");
        crate::linker::create_copy(&central, &agent_dir).unwrap();
        assert_eq!(
            crate::linker::agent_entry_state(&central, &agent_dir),
            crate::linker::AgentEntryState::IdenticalCopy
        );

        // 内容不同 → 用户自维护的原生版本，绝不能动
        fs::write(agent_dir.join("SKILL.md"), b"other").unwrap();
        assert_eq!(
            crate::linker::agent_entry_state(&central, &agent_dir),
            crate::linker::AgentEntryState::DifferentCopy
        );
        assert!(!crate::linker::convert_copy_to_link(&central, &agent_dir).unwrap());
        assert!(agent_dir.is_dir());

        // 恢复一致后应能原地转成链接
        fs::write(agent_dir.join("SKILL.md"), b"content").unwrap();
        assert!(crate::linker::convert_copy_to_link(&central, &agent_dir).unwrap());
        assert!(crate::linker::is_junction_or_symlink(&agent_dir));
        assert_eq!(
            crate::linker::agent_entry_state(&central, &agent_dir),
            crate::linker::AgentEntryState::Link
        );
        assert_eq!(
            fs::read_to_string(agent_dir.join("SKILL.md")).unwrap(),
            "content"
        );
    }

    #[test]
    fn test_agent_entry_state_missing() {
        let temp_dir = TempDir::new().unwrap();
        let central = temp_dir.path().join("central").join("demo");
        fs::create_dir_all(&central).unwrap();
        assert_eq!(
            crate::linker::agent_entry_state(&central, &temp_dir.path().join("nope")),
            crate::linker::AgentEntryState::Missing
        );
    }
}
