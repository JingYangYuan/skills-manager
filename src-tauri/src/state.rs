use crate::models::SkillMetadata;
use crate::settings::AppSettingsManager;
use std::sync::Mutex;

/// 应用全局状态，通过 Tauri 的 managed state 暴露给所有 command。
pub struct AppState {
    pub settings_manager: Mutex<AppSettingsManager>,
    /// 最近一次扫描结果的缓存，避免只读命令（文件树等）重复全盘扫描。
    /// 任何会改变磁盘链接或 skill_states 的写操作后置 None，下次读取时重新扫描。
    pub skills_cache: Mutex<Option<Vec<SkillMetadata>>>,
}
