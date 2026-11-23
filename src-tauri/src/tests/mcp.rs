use crate::mcp::{
    self, json_adapter, opencode_adapter, toml_adapter, yaml_adapter, McpAgentSpec, McpFormat,
    McpTarget,
};
use crate::models::{McpServerDef, McpTransport};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn temp_file(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    (dir, path)
}

fn target(agent: &'static str, path: &Path) -> McpTarget {
    McpTarget {
        spec: McpAgentSpec {
            agent,
            label: agent,
            format: McpFormat::Json(mcp::STANDARD_JSON),
            rel_path: "",
        },
        path: path.to_path_buf(),
    }
}

#[test]
fn json_roundtrip_preserves_unknown_fields() {
    let (_dir, path) = temp_file("mcp.json");
    fs::write(
        &path,
        r#"{"mcpServers":{"demo":{"command":"npx","args":["-y","pkg"],"env":{"TOKEN":"x"},"timeout":30,"type":"stdio"}}}"#,
    )
    .unwrap();

    let defs = json_adapter::read(&path, mcp::STANDARD_JSON).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].command.as_deref(), Some("npx"));
    assert_eq!(defs[0].args, vec!["-y".to_string(), "pkg".to_string()]);
    assert_eq!(defs[0].env.get("TOKEN").map(String::as_str), Some("x"));

    let mut def = defs[0].clone();
    def.args.push("--flag".to_string());
    json_adapter::upsert(&path, &def, mcp::STANDARD_JSON).unwrap();

    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(root["mcpServers"]["demo"]["timeout"], 30);
    assert_eq!(root["mcpServers"]["demo"]["args"][2], "--flag");
}

#[test]
fn json_http_server_uses_url_and_type() {
    let (_dir, path) = temp_file("mcp.json");
    let def = McpServerDef {
        name: "remote".to_string(),
        transport: McpTransport::Http,
        command: None,
        args: vec![],
        env: BTreeMap::new(),
        url: Some("https://example.com/mcp".to_string()),
        headers: BTreeMap::from([("Authorization".to_string(), "Bearer t".to_string())]),
        enabled: true,
        extra: Value::Null,
    };
    json_adapter::upsert(&path, &def, mcp::STANDARD_JSON).unwrap();

    let defs = json_adapter::read(&path, mcp::STANDARD_JSON).unwrap();
    assert_eq!(defs[0].transport, McpTransport::Http);
    assert_eq!(defs[0].url.as_deref(), Some("https://example.com/mcp"));
    assert_eq!(
        defs[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer t")
    );
}

#[test]
fn json_remote_omits_type_when_not_requested() {
    let (_dir, path) = temp_file("cursor.json");
    let def = McpServerDef {
        name: "remote".to_string(),
        transport: McpTransport::Http,
        command: None,
        args: vec![],
        env: BTreeMap::new(),
        url: Some("https://example.com/mcp".to_string()),
        headers: BTreeMap::new(),
        enabled: true,
        extra: Value::Null,
    };
    json_adapter::upsert(&path, &def, mcp::CURSOR_JSON).unwrap();

    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert!(root["mcpServers"]["remote"].get("type").is_none());
    assert_eq!(
        root["mcpServers"]["remote"]["url"],
        "https://example.com/mcp"
    );
}

#[test]
fn antigravity_uses_server_url_and_disabled() {
    let (_dir, path) = temp_file("mcp_config.json");
    fs::write(
        &path,
        r#"{"mcpServers":{"notion":{"serverUrl":"https://mcp.notion.com/mcp","headers":{"Authorization":"Bearer x"},"disabled":true},"local":{"command":"npx","args":["-y","pkg"]}}}"#,
    )
    .unwrap();

    let defs = json_adapter::read(&path, mcp::ANTIGRAVITY_JSON).unwrap();
    let notion = defs.iter().find(|def| def.name == "notion").unwrap();
    assert_eq!(notion.transport, McpTransport::Http);
    assert_eq!(notion.url.as_deref(), Some("https://mcp.notion.com/mcp"));
    assert!(!notion.enabled);

    let local = defs.iter().find(|def| def.name == "local").unwrap();
    assert_eq!(local.transport, McpTransport::Stdio);
    assert!(local.enabled);

    json_adapter::upsert(&path, notion, mcp::ANTIGRAVITY_JSON).unwrap();
    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert!(root["mcpServers"]["notion"].get("url").is_none());
    assert!(root["mcpServers"]["notion"].get("type").is_none());
    assert_eq!(
        root["mcpServers"]["notion"]["serverUrl"],
        "https://mcp.notion.com/mcp"
    );
    assert_eq!(root["mcpServers"]["notion"]["disabled"], true);
    assert_eq!(
        root["mcpServers"]["notion"]["headers"]["Authorization"],
        "Bearer x"
    );
}

#[test]
fn zcode_nested_servers_and_enabled_flag() {
    let (_dir, path) = temp_file("config.json");
    fs::write(
        &path,
        r#"{"provider":{"name":"x"},"mcp":{"servers":{"exa":{"type":"http","url":"https://exa.ai/mcp","enabled":false}}}}"#,
    )
    .unwrap();

    let defs = json_adapter::read(&path, mcp::ZCODE_JSON).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].transport, McpTransport::Http);
    assert_eq!(defs[0].url.as_deref(), Some("https://exa.ai/mcp"));
    assert!(!defs[0].enabled);

    json_adapter::upsert(&path, &defs[0], mcp::ZCODE_JSON).unwrap();
    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(root["mcp"]["servers"]["exa"]["enabled"], false);
    assert_eq!(root["provider"]["name"], "x");
}

#[test]
fn workbuddy_uses_streamable_http_type() {
    let (_dir, path) = temp_file("mcp.json");
    let def = McpServerDef {
        name: "notion".to_string(),
        transport: McpTransport::Http,
        command: None,
        args: vec![],
        env: BTreeMap::new(),
        url: Some("https://mcp.notion.com/mcp".to_string()),
        headers: BTreeMap::new(),
        enabled: false,
        extra: Value::Null,
    };
    json_adapter::upsert(&path, &def, mcp::WORKBUDDY_JSON).unwrap();

    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        root["mcpServers"]["notion"]["type"],
        "streamableHttp"
    );
    assert_eq!(root["mcpServers"]["notion"]["disabled"], true);
}

#[test]
fn kimi_uses_transport_and_enabled() {
    let (_dir, path) = temp_file("mcp.json");
    fs::write(
        &path,
        r#"{"mcpServers":{"legacy":{"transport":"sse","url":"https://example.com/sse","enabled":false},"modern":{"url":"https://example.com/mcp"}}}"#,
    )
    .unwrap();

    let defs = json_adapter::read(&path, mcp::KIMI_JSON).unwrap();
    let legacy = defs.iter().find(|def| def.name == "legacy").unwrap();
    assert_eq!(legacy.transport, McpTransport::Sse);
    assert!(!legacy.enabled);
    let modern = defs.iter().find(|def| def.name == "modern").unwrap();
    assert_eq!(modern.transport, McpTransport::Http);

    let http = McpServerDef {
        name: "modern".to_string(),
        transport: McpTransport::Http,
        command: None,
        args: vec![],
        env: BTreeMap::new(),
        url: Some("https://example.com/mcp".to_string()),
        headers: BTreeMap::new(),
        enabled: true,
        extra: Value::Null,
    };
    json_adapter::upsert(&path, &http, mcp::KIMI_JSON).unwrap();
    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert!(root["mcpServers"]["modern"].get("transport").is_none());
    assert_eq!(root["mcpServers"]["modern"]["enabled"], true);
}

#[test]
fn openclaw_uses_nested_transport_and_enabled() {
    let (_dir, path) = temp_file("openclaw.json");
    fs::write(
        &path,
        r#"{"mcp":{"servers":{"remote":{"url":"https://example.com/mcp","transport":"streamable-http","enabled":false,"requestTimeoutMs":20000}}}}"#,
    )
    .unwrap();

    let defs = json_adapter::read(&path, mcp::OPENCLAW_JSON).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].transport, McpTransport::Http);
    assert_eq!(defs[0].url.as_deref(), Some("https://example.com/mcp"));
    assert!(!defs[0].enabled);
    assert_eq!(defs[0].extra["requestTimeoutMs"], 20000);

    json_adapter::upsert(&path, &defs[0], mcp::OPENCLAW_JSON).unwrap();
    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        root["mcp"]["servers"]["remote"]["transport"],
        "streamable-http"
    );
    assert_eq!(root["mcp"]["servers"]["remote"]["enabled"], false);
    assert_eq!(root["mcp"]["servers"]["remote"]["requestTimeoutMs"], 20000);
}

#[test]
fn trae_omits_type_for_remote() {
    let (_dir, path) = temp_file("mcp.json");
    let def = McpServerDef {
        name: "remote".to_string(),
        transport: McpTransport::Http,
        command: None,
        args: vec![],
        env: BTreeMap::new(),
        url: Some("https://example.com/mcp".to_string()),
        headers: BTreeMap::new(),
        enabled: true,
        extra: Value::Null,
    };
    json_adapter::upsert(&path, &def, mcp::TRAE_JSON).unwrap();

    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert!(root["mcpServers"]["remote"].get("type").is_none());
    assert_eq!(
        root["mcpServers"]["remote"]["url"],
        "https://example.com/mcp"
    );
}

#[test]
fn kilo_uses_flat_mcp_map() {
    let (_dir, path) = temp_file("kilo.json");
    fs::write(
        &path,
        r#"{"mcp":{"local":{"type":"local","command":["npx","-y","pkg"],"enabled":true}}}"#,
    )
    .unwrap();

    let defs = opencode_adapter::read_flat(&path).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].command.as_deref(), Some("npx"));
    assert_eq!(defs[0].args, vec!["-y".to_string(), "pkg".to_string()]);
    assert!(defs[0].enabled);

    let remote = McpServerDef {
        name: "remote".to_string(),
        transport: McpTransport::Http,
        command: None,
        args: vec![],
        env: BTreeMap::new(),
        url: Some("https://example.com/mcp".to_string()),
        headers: BTreeMap::new(),
        enabled: false,
        extra: Value::Null,
    };
    opencode_adapter::upsert_flat(&path, &remote).unwrap();

    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(root["mcp"]["remote"]["type"], "remote");
    assert_eq!(root["mcp"]["remote"]["enabled"], false);
}

#[test]
fn opencode_local_and_remote_mapping() {
    let (_dir, path) = temp_file("opencode.json");
    fs::write(
        &path,
        r#"{"mcp":{"servers":{"zotero":{"type":"local","command":["npx","-y","zotero"],"environment":{"A":"1"},"enabled":true}}}}"#,
    )
    .unwrap();

    let defs = opencode_adapter::read(&path).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].command.as_deref(), Some("npx"));
    assert_eq!(defs[0].args, vec!["-y".to_string(), "zotero".to_string()]);
    assert_eq!(defs[0].env.get("A").map(String::as_str), Some("1"));

    let remote = McpServerDef {
        name: "notion".to_string(),
        transport: McpTransport::Http,
        command: None,
        args: vec![],
        env: BTreeMap::new(),
        url: Some("https://mcp.notion.com/mcp".to_string()),
        headers: BTreeMap::new(),
        enabled: true,
        extra: Value::Null,
    };
    opencode_adapter::upsert(&path, &remote).unwrap();

    let root: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(root["mcp"]["servers"]["notion"]["type"], "remote");
    assert_eq!(
        root["mcp"]["servers"]["notion"]["url"],
        "https://mcp.notion.com/mcp"
    );
}

#[test]
fn hermes_yaml_roundtrip_preserves_other_sections() {
    let (_dir, path) = temp_file("config.yaml");
    fs::write(
        &path,
        "# top comment\nmodel: test\n\n# mcp servers\nmcp_servers:\n  demo:\n    command: npx\n    args: [\"-y\", \"pkg\"]\n    timeout: 30\n    enabled: true\n\nmemory:\n  memory_enabled: false\n",
    )
    .unwrap();

    let defs = yaml_adapter::read(&path).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].command.as_deref(), Some("npx"));
    assert_eq!(defs[0].args, vec!["-y".to_string(), "pkg".to_string()]);
    assert!(defs[0].enabled);
    assert_eq!(defs[0].extra["timeout"], 30);

    let mut def = defs[0].clone();
    def.enabled = false;
    def.args.push("--flag".to_string());
    yaml_adapter::upsert(&path, &def).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("# top comment"));
    assert!(text.contains("memory:\n  memory_enabled: false"));

    let defs = yaml_adapter::read(&path).unwrap();
    assert_eq!(defs.len(), 1);
    assert!(!defs[0].enabled);
    assert_eq!(
        defs[0].args,
        vec!["-y".to_string(), "pkg".to_string(), "--flag".to_string()]
    );
    assert_eq!(defs[0].extra["timeout"], 30);

    yaml_adapter::delete(&path, "demo").unwrap();
    assert!(yaml_adapter::read(&path).unwrap().is_empty());
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("mcp_servers: {}"));
    assert!(text.contains("memory:"));
}

#[test]
fn toml_roundtrip_preserves_comments_and_other_tables() {
    let (_dir, path) = temp_file("config.toml");
    fs::write(
        &path,
        "# keep me\nmodel = \"gpt\"\n\n[mcp_servers.demo]\ncommand = \"npx\"\nargs = [\"-y\", \"pkg\"]\nenabled = true\n",
    )
    .unwrap();

    let defs = toml_adapter::read(&path).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].command.as_deref(), Some("npx"));
    assert!(defs[0].enabled);

    let mut def = defs[0].clone();
    def.enabled = false;
    def.args.push("--flag".to_string());
    toml_adapter::upsert(&path, &def).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("# keep me"));
    assert!(text.contains("model = \"gpt\""));

    let defs = toml_adapter::read(&path).unwrap();
    assert!(!defs[0].enabled);
    assert_eq!(
        defs[0].args,
        vec!["-y".to_string(), "pkg".to_string(), "--flag".to_string()]
    );
}

#[test]
fn toml_creates_missing_tables() {
    let (_dir, path) = temp_file("config.toml");
    fs::write(&path, "model = \"gpt\"\n").unwrap();

    let def = McpServerDef {
        name: "new".to_string(),
        transport: McpTransport::Stdio,
        command: Some("uvx".to_string()),
        args: vec!["mcp-server".to_string()],
        env: BTreeMap::new(),
        url: None,
        headers: BTreeMap::new(),
        enabled: true,
        extra: Value::Null,
    };
    toml_adapter::upsert(&path, &def).unwrap();

    let defs = toml_adapter::read(&path).unwrap();
    assert_eq!(defs[0].name, "new");
    assert_eq!(defs[0].command.as_deref(), Some("uvx"));
}

#[test]
fn raw_section_redacts_env_and_headers() {
    let (_dir, path) = temp_file("mcp.json");
    fs::write(
        &path,
        r#"{"mcpServers":{"demo":{"command":"npx","env":{"TOKEN":"supersecret"},"headers":{"Authorization":"Bearer x"}}}}"#,
    )
    .unwrap();

    let text = mcp::raw_section(&target("claude", &path)).unwrap();
    assert!(!text.contains("supersecret"));
    assert!(!text.contains("Bearer x"));
    assert!(text.contains("TOKEN"));
}

#[test]
fn overview_detects_conflicts_between_agents() {
    let (_dir, path_a) = temp_file("a.json");
    let (_dir_b, path_b) = temp_file("b.json");
    fs::write(
        &path_a,
        r#"{"mcpServers":{"demo":{"command":"npx","args":["a"]}}}"#,
    )
    .unwrap();
    fs::write(
        &path_b,
        r#"{"mcpServers":{"demo":{"command":"npx","args":["b"]}}}"#,
    )
    .unwrap();

    let overview = mcp::collect_overview(
        &[target("claude", &path_a), target("cursor", &path_b)],
        "test-conflict",
    );
    assert_eq!(overview.servers.len(), 1);
    assert!(overview.servers[0].conflict);
    assert_eq!(overview.servers[0].bindings.len(), 2);
}
