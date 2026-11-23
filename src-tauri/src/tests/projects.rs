use crate::models::ProjectConfig;
use crate::projects;
use std::fs;
use std::path::{Path, PathBuf};

fn unique_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "skills-manager-projects-{}-{}-{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_skill(root: &Path, rel: &str, name: &str, description: &str) {
    let dir = root.join(rel).join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {}\ndescription: {}\n---\n", name, description),
    )
    .unwrap();
}

fn project_for(root: &Path) -> ProjectConfig {
    let path = root.to_string_lossy().to_string();
    ProjectConfig {
        id: projects::project_id(&path),
        name: "demo".to_string(),
        path,
        enabled: true,
        auto_detected: false,
    }
}

#[test]
fn scan_project_skills_reads_known_dirs() {
    let root = unique_dir("scan");
    write_skill(&root, ".claude/skills", "alpha", "first");
    write_skill(&root, ".agents/skills", "beta", "second");
    // 非约定目录不扫描
    write_skill(&root, "skills", "ignored", "nope");

    let skills = projects::scan_project_skills(&project_for(&root));
    assert_eq!(skills.len(), 2);
    assert!(skills.iter().any(|s| s.id == "alpha" && s.agent == "claude"));
    assert!(skills.iter().any(|s| s.id == "beta" && s.agent == "agents"));
    assert!(skills.iter().all(|s| s.rel_dir.contains("/skills/")));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn scan_project_skills_skips_disabled_project() {
    let root = unique_dir("disabled");
    write_skill(&root, ".claude/skills", "alpha", "first");
    let mut project = project_for(&root);
    project.enabled = false;
    assert!(projects::scan_project_skills(&project).is_empty());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn existing_skill_dirs_reports_only_present_dirs() {
    let root = unique_dir("dirs");
    fs::create_dir_all(root.join(".claude/skills")).unwrap();
    fs::create_dir_all(root.join(".grok/skills")).unwrap();
    let dirs = projects::existing_skill_dirs(&root);
    assert_eq!(
        dirs,
        vec![".claude/skills".to_string(), ".grok/skills".to_string()]
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn validate_project_root_rejects_home_and_filesystem_root() {
    let home = dirs::home_dir().unwrap();
    assert!(projects::validate_project_root(&home).is_err());
    assert!(projects::validate_project_root(Path::new("/")).is_err());

    let root = unique_dir("valid");
    assert!(projects::validate_project_root(&root).is_ok());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn normalize_project_path_resolves_trailing_segments() {
    let root = unique_dir("normalize");
    let raw = format!("{}/sub/..", root.to_string_lossy());
    fs::create_dir_all(root.join("sub")).unwrap();
    let normalized = projects::normalize_project_path(&raw);
    assert_eq!(normalized, fs::canonicalize(&root).unwrap());
    fs::remove_dir_all(&root).ok();
}

#[cfg(unix)]
#[test]
fn scan_project_skills_dedupes_symlinked_copies() {
    use std::os::unix::fs::symlink;

    let root = unique_dir("symlink");
    write_skill(&root, ".claude/skills", "alpha", "first");
    fs::create_dir_all(root.join(".agents/skills")).unwrap();
    symlink(
        root.join(".claude/skills/alpha"),
        root.join(".agents/skills/alpha"),
    )
    .unwrap();

    let skills = projects::scan_project_skills(&project_for(&root));
    assert_eq!(skills.len(), 1, "同一物理目录只应出现一次");
    assert_eq!(skills[0].agent, "claude");

    fs::remove_dir_all(&root).ok();
}
