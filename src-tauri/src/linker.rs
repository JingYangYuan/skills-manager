use crate::models::{AgentConfig, LinkStrategy, SkillMetadata};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LinkerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Link creation failed: {0}")]
    LinkFailed(String),

    #[error("Agent path not found: {0}")]
    AgentNotFound(String),
}

/// 链接结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkResult {
    Symlink,
    Copy,
}

/// 把链接相关诊断写入 `~/.skills-manager/linker.log`（GUI 进程没有控制台，
/// eprintln 看不到；链接降级为复制时必须留下可追溯的原因）。
pub(crate) fn linker_log(message: &str) {
    use std::io::Write;

    let Some(home) = dirs::home_dir() else {
        return;
    };
    let dir = home.join(".skills-manager");
    let path = dir.join("linker.log");
    let _ = fs::create_dir_all(&dir);
    if fs::metadata(&path).map(|m| m.len() > 256 * 1024).unwrap_or(false) {
        let _ = fs::remove_file(&path);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "[{}] {}", stamp, message);
    }
}

/// 创建符号链接
#[cfg(unix)]
pub fn create_symlink(source: &Path, target: &Path) -> Result<LinkResult, LinkerError> {
    eprintln!("=== CREATE_SYMLINK DEBUG ===");
    eprintln!("Source path: {:?}", source);
    eprintln!("Source exists: {}", source.exists());
    eprintln!("Source is_dir: {}", source.is_dir());
    eprintln!("Target path: {:?}", target);
    eprintln!("Target exists: {}", target.exists());

    // 检查源路径是否存在
    if !source.exists() {
        eprintln!("ERROR: Source path does not exist!");
        return Err(LinkerError::LinkFailed(format!(
            "Source path does not exist: {:?}",
            source
        )));
    }

    // 确保父目录存在
    if let Some(parent) = target.parent() {
        eprintln!("Target parent: {:?}", parent);
        eprintln!("Parent exists: {}", parent.exists());

        if !parent.exists() {
            eprintln!("Creating parent directory: {:?}", parent);
            fs::create_dir_all(parent)?;
            eprintln!("Parent directory created successfully");
        }
    }

    // 如果目标已存在，先删除
    if target.exists() {
        eprintln!("Target already exists, removing...");
        if target.is_dir() {
            fs::remove_dir_all(target)?;
        } else {
            fs::remove_file(target)?;
        }
        eprintln!("Old target removed");
    }

    // 创建符号链接
    eprintln!("Attempting to create symlink: {:?} -> {:?}", source, target);
    match std::os::unix::fs::symlink(source, target) {
        Ok(_) => {
            eprintln!("Symlink created successfully!");
            eprintln!("=== END CREATE_SYMLINK DEBUG ===");
            Ok(LinkResult::Symlink)
        }
        Err(e) => {
            eprintln!("FAILED to create symlink: {}", e);
            eprintln!("Error kind: {:?}", e.kind());
            eprintln!("=== END CREATE_SYMLINK DEBUG ===");
            Err(LinkerError::Io(e))
        }
    }
}

/// Windows `CreateSymbolicLinkW` 标志：目录链接 + 允许无特权创建（需「开发人员模式」等系统策略）
#[cfg(windows)]
const SYMLINK_FLAG_DIRECTORY: u32 = 0x1;
#[cfg(windows)]
const SYMLINK_FLAG_ALLOW_UNPRIVILEGED_CREATE: u32 = 0x2;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn CreateSymbolicLinkW(
        lp_symlink_file_name: *const u16,
        lp_target_file_name: *const u16,
        dw_flags: u32,
    ) -> u8;

    fn CreateFileW(
        lp_file_name: *const u16,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *mut core::ffi::c_void,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: *mut core::ffi::c_void,
    ) -> isize;

    fn DeviceIoControl(
        h_device: isize,
        dw_io_control_code: u32,
        lp_in_buffer: *const core::ffi::c_void,
        n_in_buffer_size: u32,
        lp_out_buffer: *mut core::ffi::c_void,
        n_out_buffer_size: u32,
        lp_bytes_returned: *mut u32,
        lp_overlapped: *mut core::ffi::c_void,
    ) -> i32;

    fn CloseHandle(h_object: isize) -> i32;
}

/// `link` 为新符号链接路径，`point_to` 为链接目标（须存在）。
#[cfg(windows)]
fn path_to_wide_nul(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// 去除 `canonicalize` 产生的 `\\?\` 扩展路径前缀（CreateSymbolicLinkW 对它支持不一致）。
fn strip_extended_prefix(p: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if s.starts_with(r"\\?\") {
            return PathBuf::from(&s[4..]);
        }
    }

    p.to_path_buf()
}

/// 使用与「开发人员模式」兼容的标志创建符号链接；失败时再回退到 `std` 实现。
#[cfg(windows)]
fn create_symlink_windows_api(link: &Path, point_to: &Path, flags: u32) -> std::io::Result<()> {
    let link_w = path_to_wide_nul(link);
    let target_w = path_to_wide_nul(point_to);
    let ok = unsafe { CreateSymbolicLinkW(link_w.as_ptr(), target_w.as_ptr(), flags) };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// 使用 `cmd /c mklink /J` 创建 NTFS Junction（目录连接点）。
/// Junction 无需任何特权，在 Windows 上可替代目录 symlink。
#[cfg(windows)]
fn create_junction(link: &Path, point_to: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;

    let output = std::process::Command::new("cmd")
        .args([
            "/C",
            "mklink",
            "/J",
            &link.to_string_lossy(),
            &point_to.to_string_lossy(),
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("mklink /J failed: {}", stderr.trim()),
        ))
    }
}

/// 直接用 `FSCTL_SET_REPARSE_POINT` 创建 NTFS Junction。
/// 不依赖 cmd.exe，也不依赖「开发人员模式」；`link` 必须是已存在的空目录。
#[cfg(windows)]
fn create_junction_native(link: &Path, point_to: &Path) -> std::io::Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    const GENERIC_WRITE: u32 = 0x4000_0000;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FSCTL_SET_REPARSE_POINT: u32 = 0x0009_00A4;
    const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;

    let target = strip_extended_prefix(point_to);
    let target_str = target.to_string_lossy().to_string();
    let substitute = format!("\\??\\{}", target_str);

    let substitute_w: Vec<u16> = OsStr::new(&substitute).encode_wide().collect();
    let print_w: Vec<u16> = OsStr::new(&target_str).encode_wide().collect();

    // REPARSE_DATA_BUFFER: ULONG tag | USHORT data_len | USHORT reserved |
    //   USHORT sub_off | USHORT sub_len | USHORT print_off | USHORT print_len | WCHAR path_buffer[]
    let path_buffer_bytes = (substitute_w.len() + print_w.len()) * 2;
    let reparse_data_len = 8 + path_buffer_bytes;
    let mut buffer = vec![0u8; 8 + reparse_data_len];
    buffer[0..4].copy_from_slice(&IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
    buffer[4..6].copy_from_slice(&(reparse_data_len as u16).to_le_bytes());
    // buffer[6..8] reserved = 0
    buffer[8..10].copy_from_slice(&0u16.to_le_bytes());
    buffer[10..12].copy_from_slice(&((substitute_w.len() * 2) as u16).to_le_bytes());
    buffer[12..14].copy_from_slice(&((substitute_w.len() * 2) as u16).to_le_bytes());
    buffer[14..16].copy_from_slice(&((print_w.len() * 2) as u16).to_le_bytes());

    let mut offset = 16usize;
    for unit in substitute_w.iter().chain(print_w.iter()) {
        if offset + 2 > buffer.len() {
            break;
        }
        buffer[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        offset += 2;
    }

    let link_wide = path_to_wide_nul(link);
    let handle = unsafe {
        CreateFileW(
            link_wide.as_ptr(),
            GENERIC_WRITE,
            0,
            core::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            core::ptr::null_mut(),
        )
    };
    if handle == -1 {
        return Err(std::io::Error::last_os_error());
    }

    let mut returned = 0u32;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_SET_REPARSE_POINT,
            buffer.as_ptr() as *const core::ffi::c_void,
            buffer.len() as u32,
            core::ptr::null_mut(),
            0,
            &mut returned,
            core::ptr::null_mut(),
        )
    };
    let error = std::io::Error::last_os_error();
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 {
        Err(error)
    } else {
        Ok(())
    }
}

/// 创建符号链接 (Windows)
/// 优先级：Junction（无需权限） > Symlink（需权限） > 返回错误（由外层 fallback 到 Copy）
#[cfg(windows)]
pub fn create_symlink(source: &Path, target: &Path) -> Result<LinkResult, LinkerError> {
    if !source.exists() {
        linker_log(&format!("source missing: {:?}", source));
        return Err(LinkerError::LinkFailed(format!(
            "Source path does not exist: {:?}",
            source
        )));
    }

    if let Some(parent) = target.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    if target.exists() {
        if target.is_dir() {
            fs::remove_dir_all(target)?;
        } else {
            fs::remove_file(target)?;
        }
    }

    let source_abs = strip_extended_prefix(&fs::canonicalize(source).map_err(LinkerError::Io)?);

    if source.is_dir() {
        // 依次尝试三种「真链接」实现，每一种都校验落盘结果：
        //   1) 原生 FSCTL_SET_REPARSE_POINT（不依赖 cmd.exe / 开发人员模式）
        //   2) cmd /C mklink /J
        //   3) CreateSymbolicLinkW / std::symlink_dir
        // 全部失败才返回错误（外层才会降级为复制），并把原因写进 linker.log。
        let mut reasons: Vec<String> = Vec::new();

        // 1) 原生 junction（要求目标已经是空目录）
        match fs::create_dir_all(target) {
            Ok(()) => match create_junction_native(target, &source_abs) {
                Ok(()) if is_junction_or_symlink(target) => {
                    linker_log(&format!(
                        "junction(native) ok: {:?} -> {:?}",
                        target, source_abs
                    ));
                    return Ok(LinkResult::Symlink);
                }
                Ok(()) => reasons.push("native junction: 创建后校验失败".to_string()),
                Err(e) => reasons.push(format!("native junction: {}", e)),
            },
            Err(e) => reasons.push(format!("native junction: 无法创建目标目录: {}", e)),
        }
        let _ = fs::remove_dir(target);

        // 2) mklink /J
        match create_junction(target, &source_abs) {
            Ok(()) if is_junction_or_symlink(target) => {
                linker_log(&format!("junction(mklink) ok: {:?} -> {:?}", target, source_abs));
                return Ok(LinkResult::Symlink);
            }
            Ok(()) => reasons.push("mklink /J: 创建后校验失败".to_string()),
            Err(e) => reasons.push(format!("mklink /J: {}", e)),
        }
        let _ = fs::remove_dir(target);

        // 3) symlink（需要开发人员模式或管理员权限）
        let flags_priv = SYMLINK_FLAG_DIRECTORY | SYMLINK_FLAG_ALLOW_UNPRIVILEGED_CREATE;
        if create_symlink_windows_api(target, &source_abs, flags_priv).is_ok()
            && is_junction_or_symlink(target)
        {
            linker_log(&format!(
                "symlink(CreateSymbolicLinkW) ok: {:?} -> {:?}",
                target, source_abs
            ));
            return Ok(LinkResult::Symlink);
        }
        reasons.push("CreateSymbolicLinkW: 失败或未生成链接".to_string());
        let _ = fs::remove_file(target);
        let _ = fs::remove_dir(target);

        if std::os::windows::fs::symlink_dir(&source_abs, target).is_ok()
            && is_junction_or_symlink(target)
        {
            linker_log(&format!(
                "symlink(std) ok: {:?} -> {:?}",
                target, source_abs
            ));
            return Ok(LinkResult::Symlink);
        }
        reasons.push("std::symlink_dir: 失败或未生成链接".to_string());

        linker_log(&format!(
            "create_symlink FAILED: {:?} -> {:?} | {}",
            target,
            source_abs,
            reasons.join(" ; ")
        ));
        return Err(LinkerError::LinkFailed(format!(
            "无法创建链接: {}",
            reasons.join("; ")
        )));
    } else {
        // 文件级 symlink
        let flags_file = SYMLINK_FLAG_ALLOW_UNPRIVILEGED_CREATE;
        if let Err(e) = create_symlink_windows_api(target, &source_abs, flags_file) {
            linker_log(&format!("file symlink failed ({}), trying std::symlink_file", e));
            std::os::windows::fs::symlink_file(&source_abs, target).map_err(LinkerError::Io)?;
        }
    }

    Ok(LinkResult::Symlink)
}

/// 创建文件副本
pub fn create_copy(source: &Path, target: &Path) -> Result<LinkResult, LinkerError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    if target.exists() {
        if target.is_dir() {
            fs::remove_dir_all(target)?;
        } else {
            fs::remove_file(target)?;
        }
    }

    if source.is_dir() {
        recursive_copy(source, target)?;
    } else {
        fs::copy(source, target)?;
    }

    Ok(LinkResult::Copy)
}

/// 递归复制目录
fn recursive_copy(source: &Path, target: &Path) -> Result<(), LinkerError> {
    fs::create_dir_all(target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            recursive_copy(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)?;
        }
    }

    Ok(())
}

/// 检查路径是否为 NTFS Junction（reparse point 且非 symlink）或 symlink
/// 重要：使用 symlink_metadata() 而不是 is_symlink()，因为后者对断链的 symlink 可能返回 false
pub fn is_junction_or_symlink(path: &Path) -> bool {
    eprintln!("=== IS_JUNCTION_OR_SYMLINK DEBUG ===");
    eprintln!("Checking path: {:?}", path);

    // 先尝试标准检查
    let is_symlink_std = path.is_symlink();
    eprintln!("path.is_symlink(): {}", is_symlink_std);

    // 使用 symlink_metadata() 可以检测断链的 symlink
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            eprintln!("symlink_metadata OK");
            let file_type = metadata.file_type();
            eprintln!("file_type.is_symlink(): {}", file_type.is_symlink());
            eprintln!("file_type.is_dir(): {}", file_type.is_dir());
            eprintln!("file_type.is_file(): {}", file_type.is_file());

            // 检查 file_type 是否是 symlink
            if file_type.is_symlink() {
                eprintln!("✅ Detected as symlink via symlink_metadata");
                eprintln!("=== END IS_JUNCTION_OR_SYMLINK ===");
                return true;
            }
        }
        Err(e) => {
            eprintln!("symlink_metadata failed: {}", e);
        }
    }

    // Junction 在 Windows 上 is_symlink() 返回 false，但 metadata 的 file_attributes 带 REPARSE_POINT
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if let Ok(meta) = fs::symlink_metadata(path) {
            let is_reparse = meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
            eprintln!("Windows reparse point: {}", is_reparse);
            if is_reparse {
                eprintln!("=== END IS_JUNCTION_OR_SYMLINK ===");
                return true;
            }
        }
    }

    eprintln!("❌ Not detected as junction or symlink");
    eprintln!("=== END IS_JUNCTION_OR_SYMLINK ===");
    false
}

/// 移除链接（安全版本，只删除 symlink / junction，不删真实目录）
pub fn remove_link(target: &Path) -> Result<(), LinkerError> {
    eprintln!("=== REMOVE_LINK START ===");
    eprintln!("Target path: {:?}", target);
    eprintln!("Target exists: {}", target.exists());

    // 先检查是否是 symlink/junction（即使断链也能检测到）
    if is_junction_or_symlink(target) {
        eprintln!("✅ Confirmed symlink/junction, proceeding with removal");

        #[cfg(windows)]
        {
            // Windows 特殊处理：
            // - 目录 Junction / 目录 symlink 必须用 remove_dir
            // - 文件级 symlink 必须用 remove_file（用 remove_dir 会报 ERROR_DIRECTORY 并残留链接）
            let is_dir_link = fs::symlink_metadata(target)
                .map(|m| m.file_type().is_dir())
                .unwrap_or(false);
            eprintln!(
                "Windows: removing {} link",
                if is_dir_link { "directory" } else { "file" }
            );
            let result = if is_dir_link {
                fs::remove_dir(target).or_else(|_| fs::remove_dir_all(target))
            } else {
                fs::remove_file(target).or_else(|_| fs::remove_dir(target))
            };
            match result {
                Ok(_) => {
                    eprintln!("✅ Successfully removed: {:?}", target);
                    eprintln!("=== REMOVE_LINK SUCCESS ===");
                    return Ok(());
                }
                Err(e) => {
                    eprintln!(
                        "⚠️  Warning: Failed to remove {:?}: {}, but continuing",
                        target, e
                    );
                    eprintln!("=== REMOVE_LINK DONE (WITH WARNING) ===");
                    return Ok(());
                }
            }
        }

        #[cfg(unix)]
        {
            // Unix/macOS 特殊处理：Symlink 可能需要 remove_file
            eprintln!("Unix: Determining removal method for symlink");

            // 获取文件类型信息
            let file_type = fs::symlink_metadata(target)
                .map(|m| {
                    eprintln!(
                        "Metadata: is_dir={}, is_file={}, is_symlink={}",
                        m.file_type().is_dir(),
                        m.file_type().is_file(),
                        m.file_type().is_symlink()
                    );
                    m.file_type()
                })
                .ok();

            // Unix 上，symlink 通常需要用 remove_file 删除（即使指向目录）
            let result = if file_type.map_or(false, |ft| ft.is_file()) {
                eprintln!("Using remove_file (confirmed file)");
                fs::remove_file(target)
            } else {
                // 对于目录类型或不确定类型（断链），先尝试 remove_file
                // 因为 Unix 上 symlink（即使指向目录）应该用 remove_file 删除
                eprintln!("Using remove_file for symlink (directory or broken)");
                fs::remove_file(target)
            };

            match result {
                Ok(_) => {
                    eprintln!("✅ Successfully removed: {:?}", target);
                    eprintln!("=== REMOVE_LINK SUCCESS ===");
                    Ok(())
                }
                Err(e) => {
                    eprintln!(
                        "⚠️  Warning: Failed to remove {:?}: {}, but continuing",
                        target, e
                    );
                    eprintln!("=== REMOVE_LINK DONE (WITH WARNING) ===");
                    Ok(())
                }
            }
        }
    } else if target.exists() {
        // 存在但不是 symlink/junction，拒绝删除以防误删真实数据
        eprintln!(
            "⚠️  Warning: Target exists but is not a symlink/junction, refusing to delete: {:?}",
            target
        );
        eprintln!("=== REMOVE_LINK ABORTED (NOT A SYMLINK) ===");
        Ok(())
    } else {
        // 不存在且不是 symlink/junction，可能已经被删除了
        eprintln!(
            "ℹ️  Target does not exist (may already be removed): {:?}",
            target
        );
        eprintln!("=== REMOVE_LINK DONE (ALREADY GONE) ===");
        Ok(())
    }
}

/// 验证链接是否有效
#[allow(dead_code)]
pub fn verify_link(skill_path: &Path, link_path: &Path) -> bool {
    if !link_path.exists() {
        return false;
    }

    // 检查是否是符号链接且指向正确目标
    match link_path.read_link() {
        Ok(target) => paths_refer_to_same_location(&target, skill_path),
        Err(_) => {
            // 不是符号链接，检查文件是否相同
            same_file::is_same_file(skill_path, link_path).unwrap_or(false)
        }
    }
}

/// 判断两个路径是否指向同一位置。
/// create_symlink 会先把源路径规范化，而 Windows 上的 8.3 短路径（如 RUNNER~1）、
/// 大小写差异或残留的 `\\?\` 前缀都会让字面量比较失败，因此这里比较规范化后的结果。
fn paths_refer_to_same_location(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }

    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => strip_extended_prefix(&ca) == strip_extended_prefix(&cb),
        _ => false,
    }
}

/// 复制技能到中央存储
pub fn copy_to_central_storage(
    skill_source: &Path,
    skill_id: &str,
) -> Result<PathBuf, LinkerError> {
    eprintln!("=== COPY_TO_CENTRAL_STORAGE START ===");
    eprintln!("Skill source: {:?}", skill_source);
    eprintln!("Skill ID: {}", skill_id);

    let home_dir = dirs::home_dir()
        .ok_or_else(|| LinkerError::LinkFailed("Cannot find home directory".to_string()))?;

    let central_storage = home_dir.join(".skills-manager/skills");
    let skill_target = central_storage.join(skill_id);

    eprintln!("Central storage: {:?}", central_storage);
    eprintln!("Target path: {:?}", skill_target);

    // 确保中央存储目录存在
    if !central_storage.exists() {
        eprintln!("Creating central storage directory");
        fs::create_dir_all(&central_storage)?;
    }

    // 如果目标已存在，先删除
    if skill_target.exists() {
        eprintln!("Target already exists, removing...");
        if skill_target.is_dir() {
            fs::remove_dir_all(&skill_target)?;
        } else {
            fs::remove_file(&skill_target)?;
        }
    }

    // 复制技能到中央存储
    eprintln!("Copying skill to central storage...");
    if skill_source.is_dir() {
        recursive_copy(skill_source, &skill_target)?;
    } else {
        fs::copy(skill_source, &skill_target)?;
    }

    eprintln!("✅ Skill copied to central storage: {:?}", skill_target);
    eprintln!("=== COPY_TO_CENTRAL_STORAGE SUCCESS ===");

    Ok(skill_target)
}

/// 链接管理器
pub struct LinkManager {
    strategy: LinkStrategy,
}

impl LinkManager {
    pub fn new(strategy: LinkStrategy) -> Self {
        Self { strategy }
    }

    /// 将技能链接到指定 Agent
    pub fn link_skill_to_agent(
        &self,
        skill: &SkillMetadata,
        agent: &AgentConfig,
        _skills_base: &Path,
    ) -> Result<LinkResult, LinkerError> {
        eprintln!("=== LINK_SKILL_TO_AGENT START ===");
        eprintln!("Skill ID: {}", skill.id);
        eprintln!("Agent name: {}", agent.name);
        eprintln!("Skill primary: {}", skill.primary);

        let skill_path = skill.source_paths.get(&skill.primary).ok_or_else(|| {
            LinkerError::LinkFailed(format!("Skill {} has no primary path", skill.id))
        })?;
        let skill_source = Path::new(skill_path);

        eprintln!("Skill source path: {:?}", skill_source);
        eprintln!("Skill source exists: {}", skill_source.exists());

        if !skill_source.exists() {
            eprintln!("ERROR: Skill source path does not exist!");
            return Err(LinkerError::LinkFailed(format!(
                "Skill path does not exist: {:?}",
                skill_source
            )));
        }

        // 检查技能是否已经在中央存储
        let home_dir = dirs::home_dir()
            .ok_or_else(|| LinkerError::AgentNotFound("Cannot find home directory".to_string()))?;
        let central_storage = home_dir.join(".skills-manager/skills");
        let central_skill_path = central_storage.join(&skill.id);

        eprintln!("Central storage: {:?}", central_storage);
        eprintln!("Central skill path: {:?}", central_skill_path);
        eprintln!("Central skill exists: {}", central_skill_path.exists());

        // 如果技能不在中央存储，复制过去
        let link_source = if central_skill_path.exists() {
            eprintln!("✅ Skill already in central storage");
            central_skill_path
        } else {
            eprintln!("📋 Copying skill to central storage...");
            copy_to_central_storage(skill_source, &skill.id)?
        };

        // 构建 Agent 的完整路径
        eprintln!("Home directory: {:?}", home_dir);

        // 正确处理 ~/ 前缀
        let agent_path = if agent.path.starts_with("~/") {
            eprintln!("Agent path starts with ~/");
            home_dir.join(&agent.path[2..])
        } else if agent.path.starts_with("~") {
            eprintln!("Agent path starts with ~");
            home_dir.join(&agent.path[1..])
        } else {
            eprintln!("Agent path does not start with ~");
            home_dir.join(&agent.path)
        };

        eprintln!("Agent path config: {}", agent.path);
        eprintln!("Agent full path: {:?}", agent_path);
        eprintln!("Agent path exists: {}", agent_path.exists());

        if !agent_path.exists() {
            eprintln!("ERROR: Agent path does not exist!");
            return Err(LinkerError::AgentNotFound(agent.path.clone()));
        }

        let skill_target = agent_path.join(&agent.skills_path).join(&skill.id);

        eprintln!("Agent skills_path: {}", agent.skills_path);
        eprintln!("Link source (central storage): {:?}", link_source);
        eprintln!("Skill target: {:?}", skill_target);

        match self.strategy {
            LinkStrategy::Symlink => {
                eprintln!("Using symlink strategy");
                match create_symlink(&link_source, &skill_target) {
                    Ok(result) => {
                        eprintln!("=== LINK_SKILL_TO_AGENT SUCCESS ===");
                        Ok(result)
                    }
                    Err(e) => {
                        eprintln!("Symlink failed with error: {}", e);
                        // 符号链接失败，降级到复制（GUI 无控制台，必须落盘记录）
                        linker_log(&format!(
                            "FALLBACK TO COPY: skill={} agent={} target={:?} reason={}",
                            skill.id, agent.name, skill_target, e
                        ));
                        eprintln!("Falling back to copy strategy for {}", skill.id);
                        let copy_result = create_copy(&link_source, &skill_target);
                        match &copy_result {
                            Ok(_) => {
                                eprintln!("=== LINK_SKILL_TO_AGENT SUCCESS (COPY FALLBACK) ===")
                            }
                            Err(e2) => eprintln!(
                                "=== LINK_SKILL_TO_AGENT FAILED (COPY ALSO FAILED): {} ===",
                                e2
                            ),
                        }
                        copy_result
                    }
                }
            }
            LinkStrategy::Copy => {
                eprintln!("Using copy strategy");
                let copy_result = create_copy(&link_source, &skill_target);
                match &copy_result {
                    Ok(_) => eprintln!("=== LINK_SKILL_TO_AGENT SUCCESS (COPY) ==="),
                    Err(e) => eprintln!("=== LINK_SKILL_TO_AGENT FAILED (COPY): {} ===", e),
                }
                copy_result
            }
        }
    }

    /// 从 Agent 移除技能链接
    pub fn unlink_skill_from_agent(
        &self,
        skill: &SkillMetadata,
        agent: &AgentConfig,
    ) -> Result<(), LinkerError> {
        self.unlink_skill_id_from_agent(&skill.id, agent)
    }

    /// 仅通过 skill ID 移除链接（无需完整 SkillMetadata）
    pub fn unlink_skill_id_from_agent(
        &self,
        skill_id: &str,
        agent: &AgentConfig,
    ) -> Result<(), LinkerError> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| LinkerError::AgentNotFound("Cannot find home directory".to_string()))?;

        // 正确处理 ~/ 前缀
        let agent_path = if agent.path.starts_with("~/") {
            home_dir.join(&agent.path[2..])
        } else if agent.path.starts_with("~") {
            home_dir.join(&agent.path[1..])
        } else {
            home_dir.join(&agent.path)
        };

        let skill_target = agent_path.join(&agent.skills_path).join(skill_id);
        remove_link(&skill_target)
    }

    /// 更新技能在所有 Agent 中的链接状态
    #[allow(dead_code)]
    pub fn update_skill_links(
        &self,
        skill: &SkillMetadata,
        agents: &[AgentConfig],
        skills_base: &Path,
    ) -> Result<Vec<(String, LinkResult)>, LinkerError> {
        let mut results = Vec::new();

        for agent in agents {
            if !agent.enabled {
                continue;
            }

            let should_link = skill
                .agent_enabled
                .get(&agent.name)
                .copied()
                .unwrap_or(false);

            if should_link {
                match self.link_skill_to_agent(skill, agent, skills_base) {
                    Ok(result) => results.push((agent.name.clone(), result)),
                    Err(e) => {
                        eprintln!("Failed to link {} to {}: {}", skill.id, agent.name, e);
                    }
                }
            } else {
                // 移除链接
                let _ = self.unlink_skill_from_agent(skill, agent);
            }
        }

        Ok(results)
    }
}

// ---------- 副本识别与转换（修复历史「链接降级为复制」留下的脏数据） ----------

/// 某个技能在某个 Agent 目录里的落地形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentEntryState {
    /// 不存在
    Missing,
    /// 是链接（junction / symlink）
    Link,
    /// 真实目录，但内容与中央库完全一致 → 属于软件托管的副本，可以安全替换成链接
    IdenticalCopy,
    /// 真实目录且内容与中央库不同 → 用户自己维护的原生版本，不能动
    DifferentCopy,
}

/// 判定 Agent 目录下的条目形态。
pub fn agent_entry_state(central_skill: &Path, agent_skill: &Path) -> AgentEntryState {
    if is_junction_or_symlink(agent_skill) {
        return AgentEntryState::Link;
    }
    if !agent_skill.is_dir() {
        return AgentEntryState::Missing;
    }
    if !central_skill.is_dir() {
        return AgentEntryState::DifferentCopy;
    }
    if dirs_identical(central_skill, agent_skill) {
        AgentEntryState::IdenticalCopy
    } else {
        AgentEntryState::DifferentCopy
    }
}

/// 收集目录下全部文件的「相对路径 → 大小」。
/// 文件数超过 `max_files` 或读取失败时返回 None（宁可判成不同，也不误删）。
fn file_fingerprint(
    root: &Path,
    max_files: usize,
) -> Option<std::collections::BTreeMap<String, u64>> {
    let mut map = std::collections::BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries {
            let entry = entry.ok()?;
            let file_type = entry.file_type().ok()?;
            let path = entry.path();
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            if map.len() >= max_files {
                return None;
            }
            map.insert(rel, entry.metadata().ok()?.len());
        }
    }
    Some(map)
}

/// 有界地比较两个目录的内容：先比「相对路径集合 + 文件大小」，
/// 再对 ≤ 64KB 的文件逐字节校验，避免仅凭大小误判。
pub fn dirs_identical(a: &Path, b: &Path) -> bool {
    const MAX_FILES: usize = 200;
    const MAX_COMPARE_BYTES: u64 = 64 * 1024;

    let Some(left) = file_fingerprint(a, MAX_FILES) else {
        return false;
    };
    let Some(right) = file_fingerprint(b, MAX_FILES) else {
        return false;
    };
    if left.len() != right.len() {
        return false;
    }
    for (rel, size) in &left {
        match right.get(rel) {
            Some(other) if other == size => {}
            _ => return false,
        }
        if *size <= MAX_COMPARE_BYTES {
            let (Ok(a_bytes), Ok(b_bytes)) = (fs::read(a.join(rel)), fs::read(b.join(rel))) else {
                return false;
            };
            if a_bytes != b_bytes {
                return false;
            }
        }
    }
    true
}

/// 删除一个路径（目录链接用 remove_dir，不跟随链接内容）
fn remove_path(path: &Path) -> std::io::Result<()> {
    if !path.exists() && !is_junction_or_symlink(path) {
        return Ok(());
    }
    if is_junction_or_symlink(path) {
        fs::remove_dir(path).or_else(|_| fs::remove_file(path))
    } else if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// 把「与中央库完全一致的真实副本」替换成链接。
/// 返回 Ok(true) = 已转换，Ok(false) = 无需/不能转换（已是链接、内容不同、不存在）。
/// 转换前先把副本改名为临时目录，链接失败时原样回滚，不会丢数据。
pub fn convert_copy_to_link(central_skill: &Path, agent_skill: &Path) -> Result<bool, LinkerError> {
    if agent_entry_state(central_skill, agent_skill) != AgentEntryState::IdenticalCopy {
        return Ok(false);
    }

    let backup = agent_skill.with_file_name(format!(
        ".{}.copy-backup",
        agent_skill
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("skill")
    ));
    let _ = remove_path(&backup);
    fs::rename(agent_skill, &backup)?;

    match create_symlink(central_skill, agent_skill) {
        Ok(_) => {
            let _ = remove_path(&backup);
            linker_log(&format!("converted copy -> link: {:?}", agent_skill));
            Ok(true)
        }
        Err(e) => {
            let _ = remove_path(agent_skill);
            let _ = fs::rename(&backup, agent_skill);
            linker_log(&format!(
                "convert copy -> link FAILED, rolled back: {:?} | {}",
                agent_skill, e
            ));
            Err(e)
        }
    }
}
