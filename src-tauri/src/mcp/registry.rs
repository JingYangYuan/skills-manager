//! 官方 MCP Registry 客户端与市场条目映射。
//!
//! 数据源：`https://registry.modelcontextprotocol.io/v0.1`（v0.1 API 已冻结）。
//!
//! 官方 Registry 是“注册表”而不是“应用商店”：没有热度、分类，搜索只匹配名称，
//! 默认按名称排序。因此前端默认页使用 `featured.json` 中的内置精选，搜索/全量列表
//! 再走这里。Registry 返回的 `remotes` / `packages` 会被映射成统一的
//! `McpInstallOption`，安装时再把用户填写的 env/header/参数/URL 变量替换进
//! `McpServerDef`。

use crate::models::{McpServerDef, McpTransport};
use crate::settings::AppSettingsManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const REGISTRY_BASE: &str = "https://registry.modelcontextprotocol.io/v0.1";
const USER_AGENT: &str = concat!(
    "skills-manager/",
    env!("CARGO_PKG_VERSION"),
    " (MCP marketplace)"
);
const FEATURED_JSON: &str = include_str!("featured.json");
const CACHE_MAX_ENTRIES: usize = 128;
/// Registry 列表响应可能很大：除条数外再限制总字节数，避免翻页时内存持续增长。
const CACHE_MAX_BYTES: usize = 8 * 1024 * 1024;
const LIST_TTL: Duration = Duration::from_secs(120);
const DETAIL_TTL: Duration = Duration::from_secs(600);

// ========== 面向前端的数据类型 ==========

/// 市场列表 / 精选 / 详情共用的条目。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpMarketplaceItem {
    pub registry_name: String,
    pub title: String,
    pub description: String,
    pub version: String,
    pub icon_url: Option<String>,
    pub repository_url: Option<String>,
    pub website_url: Option<String>,
    pub categories: Vec<String>,
    /// 例如 `["remote", "npm", "oci"]`，列表页用于展示安装方式。
    pub install_kinds: Vec<String>,
    /// 精选列表通常为空，打开详情时再拉取完整安装方式。
    pub install_options: Vec<McpInstallOption>,
    pub source: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInstallOption {
    pub id: String,
    /// `remote` | `package`
    pub kind: String,
    pub label: String,
    pub transport: String,
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub inputs: Vec<McpInstallInput>,
    pub supported: bool,
    pub unsupported_reason: Option<String>,
    /// npm / pypi / oci / mcpb / cargo / nuget
    pub package_type: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInstallInput {
    /// 前端提交 values 时使用的唯一键，例如 `env:API_KEY`、`var:tenant`。
    pub key: String,
    pub label: String,
    pub description: Option<String>,
    pub required: bool,
    pub secret: bool,
    pub default_value: Option<String>,
    pub placeholder: Option<String>,
    pub choices: Vec<String>,
    /// `env` | `header` | `argument` | `variable`
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpMarketplacePage {
    pub items: Vec<McpMarketplaceItem>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpMarketplaceDetail {
    #[serde(flatten)]
    pub item: McpMarketplaceItem,
    pub published_at: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpProjectTarget {
    pub project_id: String,
    pub target: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInstallRequest {
    pub registry_name: String,
    #[serde(default)]
    pub version: String,
    pub option_id: String,
    pub local_name: String,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub global_targets: Vec<String>,
    #[serde(default)]
    pub project_targets: Vec<McpProjectTarget>,
    #[serde(default)]
    pub overwrite: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone)]
pub struct McpInstallBuilt {
    pub server: McpServerDef,
    pub version: String,
    pub title: String,
    pub description: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInstallResult {
    pub server: McpServerDef,
    pub installed_global: Vec<String>,
    pub installed_projects: Vec<McpProjectTarget>,
    pub version: String,
    pub warnings: Vec<String>,
}

/// 安装记录。**不包含 env/header 的值**，避免密钥落盘到市场记录里。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInstallRecord {
    pub registry_name: String,
    pub local_name: String,
    pub title: String,
    pub description: String,
    pub version: String,
    pub option_id: String,
    pub installed_at: String,
    #[serde(default)]
    pub global_targets: Vec<String>,
    #[serde(default)]
    pub project_targets: Vec<McpProjectTarget>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct McpInstallStore {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    installs: Vec<McpInstallRecord>,
}

// ========== Registry 原始类型 ==========

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryServer {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    version: String,
    #[serde(default)]
    remotes: Option<Vec<RegistryTransport>>,
    #[serde(default)]
    packages: Option<Vec<RegistryPackage>>,
    #[serde(default)]
    repository: Option<RegistryRepository>,
    #[serde(default)]
    website_url: Option<String>,
    #[serde(default)]
    icons: Option<Vec<RegistryIcon>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryTransport {
    #[serde(default, rename = "type")]
    transport_type: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    headers: Option<Vec<RegistryInput>>,
    #[serde(default)]
    variables: Option<BTreeMap<String, RegistryInput>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryPackage {
    #[serde(default)]
    registry_type: String,
    #[serde(default)]
    identifier: String,
    #[serde(default)]
    runtime_arguments: Option<Vec<RegistryArgument>>,
    #[serde(default)]
    package_arguments: Option<Vec<RegistryArgument>>,
    #[serde(default)]
    environment_variables: Option<Vec<RegistryInput>>,
    #[serde(default)]
    transport: Option<RegistryTransport>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RegistryInput {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    is_required: bool,
    #[serde(default)]
    is_secret: bool,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    placeholder: Option<String>,
    #[serde(default)]
    choices: Option<Vec<String>>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    variables: Option<BTreeMap<String, RegistryInput>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryArgument {
    #[serde(default, rename = "type")]
    argument_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    is_required: bool,
    #[serde(default)]
    is_secret: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    choices: Option<Vec<String>>,
    #[serde(default)]
    placeholder: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    is_repeated: bool,
    #[serde(default)]
    variables: Option<BTreeMap<String, RegistryInput>>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryRepository {
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryIcon {
    #[serde(default)]
    src: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryEntry {
    server: RegistryServer,
    #[serde(default, rename = "_meta")]
    meta: Option<RegistryMeta>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryMeta {
    #[serde(default, rename = "io.modelcontextprotocol.registry/official")]
    official: Option<RegistryOfficialMeta>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryOfficialMeta {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    is_latest: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryListResponse {
    /// 先按 `Value` 接收，逐条解析；Registry 里存在个别脏数据，
    /// 不能让一条坏记录导致整页列表解析失败。
    #[serde(default)]
    servers: Option<Vec<Value>>,
    #[serde(default)]
    metadata: Option<RegistryListMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryListMetadata {
    #[serde(default)]
    next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FeaturedEntry {
    registry_name: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    repository_url: Option<String>,
    #[serde(default)]
    website_url: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    install_kinds: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct FeaturedFile {
    #[serde(default)]
    items: Vec<FeaturedEntry>,
}

// ========== HTTP + 缓存 ==========

struct CacheEntry {
    at: Instant,
    body: String,
}

static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
static AGENT: OnceLock<ureq::Agent> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_get(key: &str, ttl: Duration) -> Option<String> {
    let guard = cache().lock().ok()?;
    let entry = guard.get(key)?;
    if entry.at.elapsed() <= ttl {
        Some(entry.body.clone())
    } else {
        None
    }
}

fn cache_put(key: &str, body: &str) {
    if let Ok(mut guard) = cache().lock() {
        let total: usize = guard.values().map(|entry| entry.body.len()).sum();
        if guard.len() >= CACHE_MAX_ENTRIES || total + body.len() > CACHE_MAX_BYTES {
            guard.clear();
        }
        guard.insert(
            key.to_string(),
            CacheEntry {
                at: Instant::now(),
                body: body.to_string(),
            },
        );
    }
}

fn http_agent() -> &'static ureq::Agent {
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(20))
            .user_agent(USER_AGENT)
            .build()
    })
}

fn truncate_chars(value: &str, max: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

fn http_get(url: &str, cache_key: &str, ttl: Duration) -> Result<String, String> {
    if let Some(body) = cache_get(cache_key, ttl) {
        return Ok(body);
    }

    let mut last_error = String::new();
    for attempt in 0..2 {
        match http_agent()
            .get(url)
            .set("Accept", "application/json")
            .call()
        {
            Ok(response) => {
                let body = response
                    .into_string()
                    .map_err(|error| format!("读取 MCP Registry 响应失败: {}", error))?;
                cache_put(cache_key, &body);
                return Ok(body);
            }
            Err(ureq::Error::Status(code, response)) => {
                let body = response.into_string().unwrap_or_default();
                last_error = format!(
                    "MCP Registry 请求失败（HTTP {}）: {}",
                    code,
                    truncate_chars(&body, 300)
                );
                if (code == 429 || code >= 500) && attempt == 0 {
                    std::thread::sleep(Duration::from_millis(400));
                    continue;
                }
                return Err(last_error);
            }
            Err(error) => {
                last_error = format!("无法连接 MCP Registry: {}", error);
                if attempt == 0 {
                    std::thread::sleep(Duration::from_millis(300));
                    continue;
                }
                return Err(last_error);
            }
        }
    }
    Err(last_error)
}

fn encode_component(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

// ========== 拉取与映射 ==========

/// 内置精选。不访问网络，保证市场默认页可用。
pub fn fetch_featured() -> Result<Vec<McpMarketplaceItem>, String> {
    let file: FeaturedFile = serde_json::from_str(FEATURED_JSON)
        .map_err(|error| format!("内置 MCP 精选数据无效: {}", error))?;

    Ok(file
        .items
        .into_iter()
        .map(|entry| {
            let title = if entry.title.trim().is_empty() {
                display_title(&entry.registry_name)
            } else {
                entry.title
            };
            McpMarketplaceItem {
                registry_name: entry.registry_name,
                title,
                description: entry.description,
                version: entry.version,
                icon_url: entry.icon_url.filter(|url| is_https_url(url)),
                repository_url: entry.repository_url.filter(|url| is_https_url(url)),
                website_url: entry.website_url.filter(|url| is_https_url(url)),
                categories: entry.categories,
                install_kinds: entry.install_kinds,
                install_options: Vec::new(),
                source: "featured".to_string(),
                updated_at: None,
            }
        })
        .collect())
}

/// 拉取 Registry 列表或按名称搜索。
///
/// - 无搜索词：返回一页 A-Z 列表，带 `nextCursor`。
/// - 有搜索词：Registry 只支持名称子串匹配，要求至少 3 个字符；一次取 100 条，
///   本地按相关度排序后返回（不再分页）。
pub fn fetch_servers(
    query: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<McpMarketplacePage, String> {
    let query = query
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(value) = &query {
        if value.chars().count() < 3 {
            return Err("MCP Registry 搜索词至少需要 3 个字符".to_string());
        }
    }

    let limit = limit.unwrap_or(40).clamp(1, 100);
    let url = if let Some(value) = &query {
        format!(
            "{}/servers?version=latest&limit=100&search={}",
            REGISTRY_BASE,
            encode_component(value)
        )
    } else {
        let mut url = format!("{}/servers?version=latest&limit={}", REGISTRY_BASE, limit);
        if let Some(value) = &cursor {
            url.push_str("&cursor=");
            url.push_str(&encode_component(value));
        }
        url
    };

    let body = http_get(&url, &url, LIST_TTL)?;
    let response: RegistryListResponse =
        serde_json::from_str(&body).map_err(|error| format!("解析 MCP Registry 列表失败: {}", error))?;

    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for raw in response.servers.unwrap_or_default() {
        let Ok(entry) = serde_json::from_value::<RegistryEntry>(raw) else {
            continue;
        };
        if entry.server.name.trim().is_empty() || !entry_is_visible(&entry) {
            continue;
        }
        if !seen.insert(entry.server.name.clone()) {
            continue;
        }
        items.push(map_item(
            &entry.server,
            "registry",
            Vec::new(),
            entry.meta.as_ref(),
        ));
    }

    if let Some(value) = &query {
        items.sort_by(|a, b| {
            search_score(b, value)
                .cmp(&search_score(a, value))
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                .then_with(|| a.registry_name.cmp(&b.registry_name))
        });
    }

    let next_cursor = if query.is_some() {
        None
    } else {
        response.metadata.and_then(|metadata| metadata.next_cursor)
    };

    Ok(McpMarketplacePage { items, next_cursor })
}

/// 拉取单个 server 的最新版本详情。
pub fn fetch_detail(registry_name: &str) -> Result<McpMarketplaceDetail, String> {
    let entry = fetch_entry(registry_name)?;
    let official = entry
        .meta
        .as_ref()
        .and_then(|meta| meta.official.as_ref());
    let status = official.and_then(|meta| meta.status.clone());
    let published_at = official.and_then(|meta| meta.published_at.clone());
    let item = map_item(&entry.server, "registry", Vec::new(), entry.meta.as_ref());

    Ok(McpMarketplaceDetail {
        item,
        published_at,
        status,
    })
}

fn fetch_entry(registry_name: &str) -> Result<RegistryEntry, String> {
    let registry_name = registry_name.trim();
    if registry_name.is_empty() {
        return Err("MCP Registry 名称不能为空".to_string());
    }
    let url = format!(
        "{}/servers/{}/versions/latest",
        REGISTRY_BASE,
        encode_component(registry_name)
    );
    let body = http_get(&url, &url, DETAIL_TTL)?;
    let entry: RegistryEntry =
        serde_json::from_str(&body).map_err(|error| format!("解析 MCP Registry 详情失败: {}", error))?;

    let status = entry
        .meta
        .as_ref()
        .and_then(|meta| meta.official.as_ref())
        .and_then(|meta| meta.status.as_deref());
    if status == Some("deleted") {
        return Err(format!("MCP server '{}' 已从 Registry 删除", registry_name));
    }
    Ok(entry)
}

fn entry_is_visible(entry: &RegistryEntry) -> bool {
    let Some(official) = entry
        .meta
        .as_ref()
        .and_then(|meta| meta.official.as_ref())
    else {
        return true;
    };
    if matches!(official.status.as_deref(), Some("deleted") | Some("deprecated")) {
        return false;
    }
    if official.is_latest == Some(false) {
        return false;
    }
    true
}

fn map_item(
    server: &RegistryServer,
    source: &str,
    categories: Vec<String>,
    meta: Option<&RegistryMeta>,
) -> McpMarketplaceItem {
    let install_options = build_options(server);
    let mut install_kinds = Vec::new();
    for option in &install_options {
        let kind = if option.kind == "remote" {
            "remote".to_string()
        } else {
            option
                .package_type
                .clone()
                .unwrap_or_else(|| "package".to_string())
        };
        if !install_kinds.contains(&kind) {
            install_kinds.push(kind);
        }
    }

    let title = server
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| display_title(&server.name));

    McpMarketplaceItem {
        registry_name: server.name.clone(),
        title,
        description: server.description.trim().to_string(),
        version: server.version.clone(),
        icon_url: first_icon(server),
        repository_url: server
            .repository
            .as_ref()
            .and_then(|repository| repository.url.clone())
            .filter(|url| is_https_url(url)),
        website_url: server.website_url.clone().filter(|url| is_https_url(url)),
        categories,
        install_kinds,
        install_options,
        source: source.to_string(),
        updated_at: meta
            .and_then(|meta| meta.official.as_ref())
            .and_then(|meta| meta.updated_at.clone()),
    }
}

fn first_icon(server: &RegistryServer) -> Option<String> {
    server
        .icons
        .as_ref()?
        .iter()
        .find_map(|icon| icon.src.starts_with("https://").then(|| icon.src.clone()))
}

fn is_https_url(url: &str) -> bool {
    url.starts_with("https://")
}

fn search_score(item: &McpMarketplaceItem, query: &str) -> i32 {
    let query = query.to_lowercase();
    let title = item.title.to_lowercase();
    let name = item.registry_name.to_lowercase();
    let description = item.description.to_lowercase();

    let mut score = 0;
    if title == query {
        score += 100;
    } else if title.contains(&query) {
        score += 60;
    }
    if name.contains(&query) {
        score += 40;
    }
    if description.contains(&query) {
        score += 20;
    }
    if item.repository_url.is_some() {
        score += 10;
    }
    if item.icon_url.is_some() {
        score += 5;
    }
    if item.install_options.iter().any(|option| option.supported) {
        score += 10;
    }
    if item.registry_name.len() > 80 {
        score -= 10;
    }
    score
}

// ========== 名称推导 ==========

/// 从 Registry 名称/标题推导一个可编辑的本地名称。
pub fn suggest_local_name(registry_name: &str, title: &str) -> String {
    let from_title = slugify(title);
    if !from_title.is_empty() {
        return from_title;
    }
    slugify(&display_title(registry_name))
}

/// 校验本地名称，避免斜杠/控制字符写进配置文件键。
pub fn validate_local_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("MCP server 名称不能为空".to_string());
    }
    if trimmed.chars().count() > 100 {
        return Err("MCP server 名称过长".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("MCP server 名称不能包含斜杠".to_string());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("MCP server 名称包含非法字符".to_string());
    }
    Ok(())
}

fn display_title(registry_name: &str) -> String {
    let (namespace, server) = registry_name
        .split_once('/')
        .unwrap_or(("", registry_name));
    let raw = if !is_generic_segment(server) {
        server
    } else {
        namespace
            .split('.')
            .rev()
            .find(|label| !is_generic_segment(*label))
            .unwrap_or(server)
    };

    let mut value = raw.to_string();
    for prefix in ["mcp-server-", "mcp-", "server-"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            if !rest.is_empty() {
                value = rest.to_string();
                break;
            }
        }
    }
    for suffix in ["-mcp-server", "-mcp", "-server"] {
        if let Some(rest) = value.strip_suffix(suffix) {
            if !rest.is_empty() {
                value = rest.to_string();
                break;
            }
        }
    }

    let words: Vec<String> = value
        .split(['-', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .filter(|word| !word.is_empty())
        .collect();

    if words.is_empty() {
        registry_name.to_string()
    } else {
        words.join(" ")
    }
}

fn is_generic_segment(value: &str) -> bool {
    matches!(
        value,
        "" | "mcp"
            | "mcp-server"
            | "server"
            | "api"
            | "ai"
            | "com"
            | "io"
            | "app"
            | "co"
            | "dev"
            | "org"
            | "cloud"
            | "www"
    )
}

fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in raw.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

// ========== 安装计划与解析 ==========

#[derive(Debug, Clone)]
enum ArgPart {
    Literal(String),
    Input(String),
    NamedInput(String, String),
    /// 命名参数 + 模板值：两者必须同时输出或同时省略，
    /// 避免可选变量缺失时留下孤立的 `--flag`。
    NamedTemplate(String, String),
    /// Docker 的 `-e NAME`：仅在最终 env 中存在该键时输出。
    EnvForward(String),
}

struct OptionPlan {
    public: McpInstallOption,
    transport: McpTransport,
    url_template: Option<String>,
    header_templates: Vec<(String, String)>,
    header_inputs: Vec<(String, String)>,
    command: Option<String>,
    args: Vec<ArgPart>,
    env_templates: Vec<(String, String)>,
    env_inputs: Vec<(String, String)>,
}

#[derive(Default)]
struct VarCollector {
    inputs: BTreeMap<String, McpInstallInput>,
}

impl VarCollector {
    fn add_template(
        &mut self,
        template: &str,
        variables: Option<&BTreeMap<String, RegistryInput>>,
        fallback: Option<&RegistryInput>,
        default_required: bool,
    ) {
        for name in template_vars(template) {
            if let Some(source) = variables.and_then(|map| map.get(&name)) {
                self.add_metadata(&name, source, "variable", default_required);
            } else if let Some(fallback) = fallback {
                self.add_metadata(&name, fallback, "variable", default_required);
            } else {
                self.add_metadata(
                    &name,
                    &RegistryInput::default(),
                    "variable",
                    default_required,
                );
            }
        }
    }

    fn add_metadata(
        &mut self,
        name: &str,
        source: &RegistryInput,
        kind: &str,
        default_required: bool,
    ) {
        let key = if kind == "variable" {
            format!("var:{}", name)
        } else {
            name.to_string()
        };
        self.merge(McpInstallInput {
            key,
            label: name.to_string(),
            description: source.description.clone(),
            required: source.is_required || default_required,
            secret: source.is_secret,
            default_value: source.default.clone(),
            placeholder: source.placeholder.clone(),
            choices: source.choices.clone().unwrap_or_default(),
            kind: kind.to_string(),
        });
    }

    fn add_direct(&mut self, key: String, label: &str, source: &RegistryInput, kind: &str) {
        self.merge(McpInstallInput {
            key,
            label: if label.trim().is_empty() {
                "参数".to_string()
            } else {
                label.to_string()
            },
            description: source.description.clone(),
            required: source.is_required,
            secret: source.is_secret,
            default_value: source.default.clone(),
            placeholder: source.placeholder.clone(),
            choices: source.choices.clone().unwrap_or_default(),
            kind: kind.to_string(),
        });
    }

    fn merge(&mut self, input: McpInstallInput) {
        match self.inputs.get_mut(&input.key) {
            Some(existing) => {
                existing.required |= input.required;
                existing.secret |= input.secret;
                if existing.description.is_none() {
                    existing.description = input.description;
                }
                if existing.default_value.is_none() {
                    existing.default_value = input.default_value;
                }
                if existing.placeholder.is_none() {
                    existing.placeholder = input.placeholder;
                }
                for choice in input.choices {
                    if !existing.choices.contains(&choice) {
                        existing.choices.push(choice);
                    }
                }
            }
            None => {
                self.inputs.insert(input.key.clone(), input);
            }
        }
    }

    fn into_inputs(self) -> Vec<McpInstallInput> {
        self.inputs.into_values().collect()
    }
}

fn empty_option(id: String, kind: &str, label: &str, transport: McpTransport) -> McpInstallOption {
    McpInstallOption {
        id,
        kind: kind.to_string(),
        label: label.to_string(),
        transport: transport_name(transport).to_string(),
        url: None,
        headers: BTreeMap::new(),
        command: None,
        args: Vec::new(),
        env: BTreeMap::new(),
        inputs: Vec::new(),
        supported: true,
        unsupported_reason: None,
        package_type: None,
        warnings: Vec::new(),
    }
}

fn transport_name(transport: McpTransport) -> &'static str {
    match transport {
        McpTransport::Stdio => "stdio",
        McpTransport::Sse => "sse",
        McpTransport::Http => "http",
    }
}

fn build_options(server: &RegistryServer) -> Vec<McpInstallOption> {
    build_plans(server)
        .into_iter()
        .map(|plan| plan.public)
        .collect()
}

fn build_plans(server: &RegistryServer) -> Vec<OptionPlan> {
    let mut plans = Vec::new();
    for (index, remote) in server.remotes.as_deref().unwrap_or(&[]).iter().enumerate() {
        plans.push(remote_plan(index, remote));
    }
    for (index, package) in server.packages.as_deref().unwrap_or(&[]).iter().enumerate() {
        plans.push(package_plan(index, package));
    }
    plans
}

fn remote_plan(index: usize, remote: &RegistryTransport) -> OptionPlan {
    let transport = match remote.transport_type.as_str() {
        "streamable-http" => McpTransport::Http,
        "sse" => McpTransport::Sse,
        _ => McpTransport::Http,
    };
    let label = match remote.transport_type.as_str() {
        "streamable-http" => "Streamable HTTP",
        "sse" => "SSE",
        other => other,
    };
    let mut public = empty_option(format!("remote:{}", index), "remote", label, transport);
    let mut vars = VarCollector::default();

    if !matches!(remote.transport_type.as_str(), "streamable-http" | "sse") {
        public.supported = false;
        public.unsupported_reason = Some(format!("暂不支持传输类型: {}", remote.transport_type));
    }

    let url_template = remote.url.clone().filter(|url| !url.trim().is_empty());
    if let Some(url) = &url_template {
        if let Some(variables) = &remote.variables {
            for (name, input) in variables {
                vars.add_metadata(name, input, "variable", false);
            }
        }
        vars.add_template(url, None, None, true);
        if !url.starts_with("https://") && !url.starts_with("http://") {
            public.supported = false;
            public.unsupported_reason = Some("该服务地址不是 HTTP(S)，无法自动安装".to_string());
        } else if url.starts_with("http://") {
            public.warnings.push("该服务使用非 HTTPS 地址".to_string());
        }
    } else {
        public.supported = false;
        public.unsupported_reason = Some("该服务未提供 URL".to_string());
    }

    let mut header_templates = Vec::new();
    let mut header_inputs = Vec::new();
    for header in remote.headers.as_deref().unwrap_or(&[]) {
        let name = header.name.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(value) = &header.value {
            vars.add_template(
                value,
                header.variables.as_ref(),
                Some(header),
                header.is_required,
            );
            public.headers.insert(name.to_string(), value.clone());
            header_templates.push((name.to_string(), value.clone()));
        } else {
            let key = format!("header:{}", name);
            vars.add_direct(key.clone(), name, header, "header");
            public.headers.insert(name.to_string(), String::new());
            header_inputs.push((name.to_string(), key));
        }
    }

    public.url = url_template.clone();
    public.inputs = vars.into_inputs();

    OptionPlan {
        public,
        transport,
        url_template,
        header_templates,
        header_inputs,
        command: None,
        args: Vec::new(),
        env_templates: Vec::new(),
        env_inputs: Vec::new(),
    }
}

fn package_plan(index: usize, package: &RegistryPackage) -> OptionPlan {
    let registry_type = package.registry_type.trim();
    let (label, package_type) = match registry_type {
        "npm" => ("npm · npx", "npm"),
        "pypi" => ("PyPI · uvx", "pypi"),
        "oci" => ("Docker", "oci"),
        "mcpb" => ("MCPB", "mcpb"),
        "cargo" => ("Cargo", "cargo"),
        "nuget" => ("NuGet", "nuget"),
        other => (other, other),
    };

    let mut public = empty_option(
        format!("package:{}", index),
        "package",
        label,
        McpTransport::Stdio,
    );
    public.package_type = Some(package_type.to_string());

    let transport_type = package
        .transport
        .as_ref()
        .map(|transport| transport.transport_type.as_str())
        .unwrap_or("stdio");
    if transport_type != "stdio" {
        public.supported = false;
        public.unsupported_reason = Some("该包以远程方式运行，暂不支持自动安装".to_string());
    }
    if package.identifier.trim().is_empty() {
        public.supported = false;
        public.unsupported_reason = Some("该包缺少 identifier".to_string());
    }

    let default_command = match registry_type {
        "npm" => Some("npx"),
        "pypi" => Some("uvx"),
        "oci" => Some("docker"),
        _ => None,
    };
    let Some(command) = default_command else {
        public.supported = false;
        public.unsupported_reason = Some(format!("暂不支持 {} 包自动安装", registry_type));
        return OptionPlan {
            public,
            transport: McpTransport::Stdio,
            url_template: None,
            header_templates: Vec::new(),
            header_inputs: Vec::new(),
            command: None,
            args: Vec::new(),
            env_templates: Vec::new(),
            env_inputs: Vec::new(),
        };
    };

    let mut vars = VarCollector::default();
    let mut runtime_parts = argument_parts(
        package.runtime_arguments.as_deref().unwrap_or(&[]),
        "runtime",
        &mut vars,
    );
    let env_names: Vec<String> = package
        .environment_variables
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter_map(|env| {
            let name = env.name.trim();
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect();
    let mut args = Vec::new();

    match registry_type {
        "npm" => {
            if !(has_flag(&runtime_parts, "-y") || has_flag(&runtime_parts, "--yes")) {
                args.push(ArgPart::Literal("-y".to_string()));
            }
            let has_package_value = runtime_parts.iter().any(|part| match part {
                ArgPart::Literal(value) => {
                    value == &package.identifier
                        || value == &format!("--package={}", package.identifier)
                }
                ArgPart::NamedInput(name, _) => name == "--package",
                ArgPart::NamedTemplate(name, _) => name == "--package",
                _ => false,
            });
            args.append(&mut runtime_parts);
            if !has_package_value {
                args.push(ArgPart::Literal(package.identifier.clone()));
            }
        }
        "oci" => {
            if !has_flag(&runtime_parts, "run") {
                args.push(ArgPart::Literal("run".to_string()));
            }
            if !(has_flag(&runtime_parts, "-i") || has_flag(&runtime_parts, "--interactive")) {
                args.push(ArgPart::Literal("-i".to_string()));
            }
            if !has_flag(&runtime_parts, "--rm") {
                args.push(ArgPart::Literal("--rm".to_string()));
            }
            args.append(&mut runtime_parts);
            // `docker run` 不会自动把宿主环境变量传入容器；`-e NAME`
            // 让 docker 从进程环境读取实际值，避免密钥出现在命令行里。
            for name in &env_names {
                args.push(ArgPart::EnvForward(name.clone()));
            }
            args.push(ArgPart::Literal(package.identifier.clone()));
        }
        _ => {
            args.append(&mut runtime_parts);
            args.push(ArgPart::Literal(package.identifier.clone()));
        }
    }

    args.append(&mut argument_parts(
        package.package_arguments.as_deref().unwrap_or(&[]),
        "package",
        &mut vars,
    ));

    let mut env_templates = Vec::new();
    let mut env_inputs = Vec::new();
    for env in package.environment_variables.as_deref().unwrap_or(&[]) {
        let name = env.name.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(value) = &env.value {
            vars.add_template(value, env.variables.as_ref(), Some(env), env.is_required);
            public.env.insert(name.to_string(), value.clone());
            env_templates.push((name.to_string(), value.clone()));
        } else {
            let key = format!("env:{}", name);
            vars.add_direct(key.clone(), name, env, "env");
            public.env.insert(name.to_string(), String::new());
            env_inputs.push((name.to_string(), key));
        }
    }

    if package
        .runtime_arguments
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .chain(package.package_arguments.as_deref().unwrap_or(&[]).iter())
        .any(|argument| argument.is_repeated)
    {
        public
            .warnings
            .push("部分参数支持重复，当前只填写一次".to_string());
    }

    public.command = Some(command.to_string());
    public.args = preview_args(&args);
    public.inputs = vars.into_inputs();

    OptionPlan {
        public,
        transport: McpTransport::Stdio,
        url_template: None,
        header_templates: Vec::new(),
        header_inputs: Vec::new(),
        command: Some(command.to_string()),
        args,
        env_templates,
        env_inputs,
    }
}

fn argument_parts(
    arguments: &[RegistryArgument],
    prefix: &str,
    vars: &mut VarCollector,
) -> Vec<ArgPart> {
    let mut parts = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        let named = argument.argument_type == "named";
        let name = argument.name.clone().unwrap_or_default();
        let key = if named {
            format!("argname:{}:{}", prefix, name)
        } else {
            format!("arg:{}:{}", prefix, index)
        };

        if let Some(value) = &argument.value {
            let fallback = argument_as_input(argument);
            vars.add_template(
                value,
                argument.variables.as_ref(),
                Some(&fallback),
                argument.is_required,
            );
            if named && !name.is_empty() {
                parts.push(ArgPart::NamedTemplate(name.clone(), value.clone()));
            } else {
                parts.push(ArgPart::Literal(value.clone()));
            }
        } else {
            let label = if name.is_empty() {
                format!("参数 {}", index + 1)
            } else {
                name.clone()
            };
            let input = argument_as_input(argument);
            vars.add_direct(key.clone(), &label, &input, "argument");
            if named && !name.is_empty() {
                parts.push(ArgPart::NamedInput(name.clone(), key));
            } else {
                parts.push(ArgPart::Input(key));
            }
        }
    }
    parts
}

fn argument_as_input(argument: &RegistryArgument) -> RegistryInput {
    RegistryInput {
        name: argument.name.clone().unwrap_or_default(),
        description: argument.description.clone(),
        is_required: argument.is_required,
        is_secret: argument.is_secret,
        default: argument.default.clone(),
        placeholder: argument.placeholder.clone(),
        choices: argument.choices.clone(),
        format: argument.format.clone(),
        value: None,
        variables: argument.variables.clone(),
    }
}

fn has_flag(parts: &[ArgPart], flag: &str) -> bool {
    parts.iter().any(|part| match part {
        ArgPart::Literal(value) => value == flag,
        ArgPart::NamedTemplate(name, _) => name == flag,
        _ => false,
    })
}

fn preview_args(parts: &[ArgPart]) -> Vec<String> {
    let mut out = Vec::new();
    for part in parts {
        match part {
            ArgPart::Literal(value) => out.push(value.clone()),
            ArgPart::Input(_) => out.push("<value>".to_string()),
            ArgPart::NamedInput(name, _) => {
                out.push(name.clone());
                out.push("<value>".to_string());
            }
            ArgPart::NamedTemplate(name, template) => {
                out.push(name.clone());
                out.push(template.clone());
            }
            ArgPart::EnvForward(name) => {
                out.push("-e".to_string());
                out.push(name.clone());
            }
        }
    }
    out
}

fn template_vars(template: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '{' {
            continue;
        }
        let mut name = String::new();
        let mut closed = false;
        for ch in chars.by_ref() {
            if ch == '}' {
                closed = true;
                break;
            }
            if ch == '{' {
                name.clear();
                continue;
            }
            name.push(ch);
        }
        if closed {
            let name = name.trim();
            if !name.is_empty() && !out.iter().any(|value| value.as_str() == name) {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// 替换 `{var}` 模板。存在未解析变量时返回 `None`，
/// 让调用方决定省略该项或报错，避免写入 `Bearer ` / 孤立 `--flag` 这类残缺配置。
fn substitute(template: &str, resolved: &BTreeMap<String, String>) -> Option<String> {
    let mut out = String::new();
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '{' {
            out.push(ch);
            continue;
        }
        let mut name = String::new();
        let mut closed = false;
        for ch in chars.by_ref() {
            if ch == '}' {
                closed = true;
                break;
            }
            if ch == '{' {
                name.clear();
                continue;
            }
            name.push(ch);
        }
        if closed {
            let key = format!("var:{}", name.trim());
            out.push_str(resolved.get(&key)?);
        } else {
            out.push('{');
            out.push_str(&name);
        }
    }
    Some(out)
}

fn resolve_plan(
    plan: &OptionPlan,
    local_name: &str,
    values: &BTreeMap<String, String>,
    enabled: bool,
) -> Result<McpServerDef, String> {
    let mut resolved = BTreeMap::new();
    for input in &plan.public.inputs {
        let value = values
            .get(&input.key)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                input
                    .default_value
                    .clone()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or_default();
        if input.required && value.is_empty() {
            return Err(format!("请填写 {}", input.label));
        }
        if !value.is_empty()
            && !input.choices.is_empty()
            && !input.choices.iter().any(|choice| choice == &value)
        {
            return Err(format!(
                "{} 的值必须是以下之一: {}",
                input.label,
                input.choices.join(" / ")
            ));
        }
        if !value.is_empty() {
            resolved.insert(input.key.clone(), value);
        }
    }

    let mut env = BTreeMap::new();
    for (name, template) in &plan.env_templates {
        if let Some(value) = substitute(template, &resolved) {
            if !value.trim().is_empty() {
                env.insert(name.clone(), value);
            }
        }
    }
    for (name, key) in &plan.env_inputs {
        if let Some(value) = resolved.get(key) {
            env.insert(name.clone(), value.clone());
        }
    }

    let mut headers = BTreeMap::new();
    for (name, template) in &plan.header_templates {
        if let Some(value) = substitute(template, &resolved) {
            if !value.trim().is_empty() {
                headers.insert(name.clone(), value);
            }
        }
    }
    for (name, key) in &plan.header_inputs {
        if let Some(value) = resolved.get(key) {
            headers.insert(name.clone(), value.clone());
        }
    }

    let mut args = Vec::new();
    for part in &plan.args {
        match part {
            ArgPart::Literal(template) => {
                if let Some(value) = substitute(template, &resolved) {
                    if !value.is_empty() {
                        args.push(value);
                    }
                }
            }
            ArgPart::Input(key) => {
                if let Some(value) = resolved.get(key) {
                    args.push(value.clone());
                }
            }
            ArgPart::NamedInput(name, key) => {
                if let Some(value) = resolved.get(key) {
                    args.push(name.clone());
                    args.push(value.clone());
                }
            }
            ArgPart::NamedTemplate(name, template) => {
                if let Some(value) = substitute(template, &resolved) {
                    if !value.is_empty() {
                        args.push(name.clone());
                        args.push(value);
                    }
                }
            }
            ArgPart::EnvForward(name) => {
                if env.contains_key(name) {
                    args.push("-e".to_string());
                    args.push(name.clone());
                }
            }
        }
    }

    let command = plan.command.clone();
    if plan.transport == McpTransport::Stdio
        && command.as_deref().unwrap_or("").trim().is_empty()
    {
        return Err("该安装方式缺少启动命令".to_string());
    }

    let url = match &plan.url_template {
        Some(template) => Some(
            substitute(template, &resolved)
                .ok_or_else(|| "URL 中存在未填写的变量".to_string())?,
        ),
        None => None,
    };
    if plan.transport != McpTransport::Stdio && url.as_deref().unwrap_or("").trim().is_empty() {
        return Err("该安装方式缺少 URL".to_string());
    }

    Ok(McpServerDef {
        name: local_name.trim().to_string(),
        transport: plan.transport,
        command: if plan.transport == McpTransport::Stdio {
            command
        } else {
            None
        },
        args: if plan.transport == McpTransport::Stdio {
            args
        } else {
            Vec::new()
        },
        env: if plan.transport == McpTransport::Stdio {
            env
        } else {
            BTreeMap::new()
        },
        url: if plan.transport == McpTransport::Stdio {
            None
        } else {
            url
        },
        headers: if plan.transport == McpTransport::Stdio {
            BTreeMap::new()
        } else {
            headers
        },
        enabled,
        extra: Value::Null,
    })
}

/// 为安装命令构建最终的 `McpServerDef`。会重新拉取 Registry 最新详情，
/// 保证安装的是最新版本的配置，而不是列表页缓存的旧数据。
pub fn build_server_for_install(
    registry_name: &str,
    option_id: &str,
    local_name: &str,
    values: &BTreeMap<String, String>,
    enabled: bool,
) -> Result<McpInstallBuilt, String> {
    validate_local_name(local_name)?;
    let entry = fetch_entry(registry_name)?;
    let plans = build_plans(&entry.server);
    let plan = plans
        .iter()
        .find(|plan| plan.public.id == option_id)
        .ok_or_else(|| format!("安装方式不存在或已失效: {}", option_id))?;
    if !plan.public.supported {
        return Err(plan
            .public
            .unsupported_reason
            .clone()
            .unwrap_or_else(|| "该安装方式暂不支持".to_string()));
    }
    let server = resolve_plan(plan, local_name, values, enabled)?;
    let title = entry
        .server
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| display_title(&entry.server.name));

    Ok(McpInstallBuilt {
        server,
        version: entry.server.version.clone(),
        title,
        description: entry.server.description.trim().to_string(),
        warnings: plan.public.warnings.clone(),
    })
}

// ========== 安装记录 ==========

fn install_store_path() -> PathBuf {
    AppSettingsManager::get_skills_manager_dir().join("mcp-installs.json")
}

/// 安装记录是 read-modify-write；市场允许快速连续安装，用进程内锁避免互相覆盖。
static INSTALL_STORE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn install_store_lock() -> &'static Mutex<()> {
    INSTALL_STORE_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn list_installs() -> Vec<McpInstallRecord> {
    read_install_store_at(&install_store_path()).installs
}

pub fn record_install(record: McpInstallRecord) -> Result<(), String> {
    record_install_at(&install_store_path(), record)
}

/// 仅从“已安装”列表移除记录，不修改任何 Agent 配置。
pub fn remove_install(registry_name: &str, local_name: &str) -> Result<(), String> {
    remove_install_at(&install_store_path(), registry_name, local_name)
}

fn remove_install_at(
    path: &Path,
    registry_name: &str,
    local_name: &str,
) -> Result<(), String> {
    let _guard = install_store_lock()
        .lock()
        .map_err(|_| "MCP 安装记录锁已损坏".to_string())?;
    let mut store = read_install_store_at(path);
    let before = store.installs.len();
    store
        .installs
        .retain(|item| item.registry_name != registry_name || item.local_name != local_name);
    if store.installs.len() == before {
        return Ok(());
    }
    store.version = 1;
    write_install_store(path, &store)
}

fn record_install_at(path: &Path, record: McpInstallRecord) -> Result<(), String> {
    let _guard = install_store_lock()
        .lock()
        .map_err(|_| "MCP 安装记录锁已损坏".to_string())?;
    let mut store = read_install_store_at(path);
    if let Some(existing) = store
        .installs
        .iter_mut()
        .find(|item| item.registry_name == record.registry_name && item.local_name == record.local_name)
    {
        for target in record.global_targets {
            if !existing.global_targets.contains(&target) {
                existing.global_targets.push(target);
            }
        }
        for target in record.project_targets {
            if !existing.project_targets.contains(&target) {
                existing.project_targets.push(target);
            }
        }
        existing.version = record.version;
        existing.option_id = record.option_id;
        existing.title = record.title;
        existing.description = record.description;
        existing.installed_at = record.installed_at;
    } else {
        store.installs.push(record);
    }
    store.version = 1;
    write_install_store(path, &store)
}

fn read_install_store_at(path: &Path) -> McpInstallStore {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_install_store(path: &Path, store: &McpInstallStore) -> Result<(), String> {
    let text = serde_json::to_string_pretty(store)
        .map_err(|error| format!("序列化 MCP 安装记录失败: {}", error))?;
    crate::mcp::write_atomic(path, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> RegistryServer {
        serde_json::from_str(json).unwrap()
    }

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn encode_component_escapes_slash_and_unicode() {
        assert_eq!(encode_component("com.notion/mcp"), "com.notion%2Fmcp");
        assert_eq!(encode_component("论文"), "%E8%AE%BA%E6%96%87");
        assert_eq!(encode_component("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn template_vars_dedupes_and_ignores_empty() {
        assert_eq!(
            template_vars("https://{tenant}/mcp/{token}/{tenant}"),
            vec!["tenant".to_string(), "token".to_string()]
        );
        assert_eq!(template_vars("{}"), Vec::<String>::new());
        assert_eq!(template_vars("plain"), Vec::<String>::new());
    }

    #[test]
    fn suggest_local_name_uses_title_then_namespace() {
        assert_eq!(suggest_local_name("com.notion/mcp", ""), "notion");
        assert_eq!(
            suggest_local_name("io.github.github/github-mcp-server", "GitHub"),
            "github"
        );
        assert_eq!(
            suggest_local_name("io.github.tavily-ai/tavily-mcp", ""),
            "tavily"
        );
        assert_eq!(
            suggest_local_name("com.figma.mcp/mcp", ""),
            "figma"
        );
    }

    #[test]
    fn remote_header_template_becomes_secret_input() {
        let server = parse(
            r#"{
                "name": "com.example/remote",
                "description": "demo",
                "version": "1.0.0",
                "remotes": [{
                    "type": "streamable-http",
                    "url": "https://api.example.com/{tenant}/mcp",
                    "headers": [{
                        "name": "Authorization",
                        "value": "Bearer {token}",
                        "isRequired": true,
                        "isSecret": true
                    }]
                }]
            }"#,
        );
        let plans = build_plans(&server);
        assert_eq!(plans.len(), 1);
        let option = &plans[0].public;
        assert!(option.supported);
        assert_eq!(option.transport, "http");
        let tenant = option.inputs.iter().find(|input| input.key == "var:tenant").unwrap();
        assert!(tenant.required);
        let token = option.inputs.iter().find(|input| input.key == "var:token").unwrap();
        assert!(token.required);
        assert!(token.secret);

        let def = resolve_plan(
            &plans[0],
            "remote",
            &values(&[("var:tenant", "acme"), ("var:token", "s3cret")]),
            true,
        )
        .unwrap();
        assert_eq!(def.url.as_deref(), Some("https://api.example.com/acme/mcp"));
        assert_eq!(
            def.headers.get("Authorization").map(String::as_str),
            Some("Bearer s3cret")
        );
    }

    #[test]
    fn npm_runtime_package_flag_inserts_identifier() {
        let server = parse(
            r#"{
                "name": "ai.aquex/stage1",
                "description": "demo",
                "version": "1.0.0",
                "packages": [{
                    "registryType": "npm",
                    "identifier": "@aquex/stage1",
                    "runtimeHint": "npx",
                    "runtimeArguments": [
                        {"type": "positional", "value": "-y"},
                        {"type": "positional", "value": "--package"}
                    ],
                    "packageArguments": [{"type": "positional", "value": "stage1-mcp"}]
                }]
            }"#,
        );
        let plans = build_plans(&server);
        let option = &plans[0].public;
        assert_eq!(option.command.as_deref(), Some("npx"));
        assert_eq!(
            option.args,
            vec![
                "-y".to_string(),
                "--package".to_string(),
                "@aquex/stage1".to_string(),
                "stage1-mcp".to_string()
            ]
        );
    }

    #[test]
    fn npm_runtime_package_value_does_not_duplicate_identifier() {
        let server = parse(
            r#"{
                "name": "ai.clize/domains",
                "description": "demo",
                "version": "1.0.0",
                "packages": [{
                    "registryType": "npm",
                    "identifier": "@clize/domains",
                    "runtimeHint": "npx",
                    "runtimeArguments": [
                        {"type": "named", "name": "--package", "value": "@clize/domains"},
                        {"type": "positional", "value": "clize-mcp"}
                    ],
                    "packageArguments": [{"type": "named", "name": "--profile", "value": "domains"}]
                }]
            }"#,
        );
        let option = &build_plans(&server)[0].public;
        assert_eq!(
            option.args,
            vec![
                "-y".to_string(),
                "--package".to_string(),
                "@clize/domains".to_string(),
                "clize-mcp".to_string(),
                "--profile".to_string(),
                "domains".to_string()
            ]
        );
    }

    #[test]
    fn oci_uses_docker_run_and_keeps_runtime_args() {
        let server = parse(
            r#"{
                "name": "com.example/image",
                "description": "demo",
                "version": "1.0.0",
                "packages": [{
                    "registryType": "oci",
                    "identifier": "ghcr.io/example/image:1",
                    "runtimeArguments": [
                        {"type": "named", "name": "-p", "value": "3030:3030"}
                    ]
                }]
            }"#,
        );
        let option = &build_plans(&server)[0].public;
        assert_eq!(option.command.as_deref(), Some("docker"));
        assert_eq!(
            option.args,
            vec![
                "run".to_string(),
                "-i".to_string(),
                "--rm".to_string(),
                "-p".to_string(),
                "3030:3030".to_string(),
                "ghcr.io/example/image:1".to_string()
            ]
        );
    }

    #[test]
    fn required_env_input_blocks_install_until_filled() {
        let server = parse(
            r#"{
                "name": "io.github.tavily-ai/tavily-mcp",
                "description": "demo",
                "version": "1.0.0",
                "packages": [{
                    "registryType": "npm",
                    "identifier": "tavily-mcp",
                    "environmentVariables": [{
                        "name": "TAVILY_API_KEY",
                        "description": "API key",
                        "isRequired": true,
                        "isSecret": true
                    }]
                }]
            }"#,
        );
        let plan = &build_plans(&server)[0];
        let error = resolve_plan(plan, "tavily", &BTreeMap::new(), true).unwrap_err();
        assert!(error.contains("TAVILY_API_KEY"));

        let def = resolve_plan(
            plan,
            "tavily",
            &values(&[("env:TAVILY_API_KEY", "tvly-test")]),
            true,
        )
        .unwrap();
        assert_eq!(def.command.as_deref(), Some("npx"));
        assert_eq!(
            def.env.get("TAVILY_API_KEY").map(String::as_str),
            Some("tvly-test")
        );
    }

    #[test]
    fn oci_forwards_env_names_without_secret_values() {
        let server = parse(
            r#"{
                "name": "com.example/db",
                "description": "demo",
                "version": "1.0.0",
                "packages": [{
                    "registryType": "oci",
                    "identifier": "ghcr.io/example/db:1",
                    "environmentVariables": [{
                        "name": "DB_PASSWORD",
                        "isRequired": true,
                        "isSecret": true
                    }]
                }]
            }"#,
        );
        let plan = &build_plans(&server)[0];
        assert!(plan.public.args.iter().any(|arg| arg == "-e"));
        assert!(plan.public.args.iter().any(|arg| arg == "DB_PASSWORD"));

        let def = resolve_plan(
            plan,
            "db",
            &values(&[("env:DB_PASSWORD", "pw")]),
            true,
        )
        .unwrap();
        let env_pos = def
            .args
            .iter()
            .position(|arg| arg == "DB_PASSWORD")
            .unwrap();
        assert_eq!(def.args[env_pos - 1], "-e");
        assert!(!def.args.iter().any(|arg| arg == "pw"));
        let image_pos = def
            .args
            .iter()
            .position(|arg| arg == "ghcr.io/example/db:1")
            .unwrap();
        assert!(env_pos < image_pos);
    }

    #[test]
    fn optional_named_template_is_omitted_entirely() {
        let server = parse(
            r#"{
                "name": "com.example/opt",
                "description": "demo",
                "version": "1.0.0",
                "packages": [{
                    "registryType": "npm",
                    "identifier": "opt-mcp",
                    "packageArguments": [{
                        "type": "named",
                        "name": "--token",
                        "value": "{token}"
                    }]
                }]
            }"#,
        );
        let plan = &build_plans(&server)[0];
        let def = resolve_plan(plan, "opt", &BTreeMap::new(), true).unwrap();
        assert!(!def.args.iter().any(|arg| arg == "--token"));
        assert!(!def.args.iter().any(|arg| arg.contains("token")));

        let def = resolve_plan(
            plan,
            "opt",
            &values(&[("var:token", "abc")]),
            true,
        )
        .unwrap();
        let flag_pos = def.args.iter().position(|arg| arg == "--token").unwrap();
        assert_eq!(def.args[flag_pos + 1], "abc");
    }

    #[test]
    fn install_record_roundtrip_has_no_secret_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp-installs.json");
        let record = McpInstallRecord {
            registry_name: "com.example/demo".to_string(),
            local_name: "demo".to_string(),
            title: "Demo".to_string(),
            description: "demo".to_string(),
            version: "1.0.0".to_string(),
            option_id: "remote:0".to_string(),
            installed_at: "2026-11-23T00:00:00Z".to_string(),
            global_targets: vec!["claude".to_string()],
            project_targets: vec![McpProjectTarget {
                project_id: "proj-1".to_string(),
                target: "claude".to_string(),
            }],
        };
        record_install_at(&path, record).unwrap();

        let store = read_install_store_at(&path);
        assert_eq!(store.installs.len(), 1);
        assert_eq!(store.installs[0].local_name, "demo");
        assert_eq!(store.installs[0].global_targets, vec!["claude"]);
        assert_eq!(store.installs[0].project_targets.len(), 1);

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("tvly-test"));
        assert!(!text.contains("Bearer "));
    }
}
