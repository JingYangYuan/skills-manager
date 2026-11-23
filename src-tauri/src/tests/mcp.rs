use crate::mcp::{
    self, json_adapter, opencode_adapter, toml_adapter, McpAgentSpec, McpFormat, McpTarget,
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
            format: McpFormat::Json,
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

    let defs = json_adapter::read(&path).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].command.as_deref(), Some("npx"));
    assert_eq!(
        defs[0].args,
        vec!["-y".to_string(), "pkg".to_string()]
    );
    assert_eq!(defs[0].env.get("TOKEN").map(String::as_str), Some("x"));

    let mut def = defs[0].clone();
    def.args.push("--flag".to_string());
    json_adapter::upsert(&path, &def, false).unwrap();

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
    json_adapter::upsert(&path, &def, false).unwrap();

    let defs = json_adapter::read(&path).unwrap();
    assert_eq!(defs[0].transport, McpTransport::Http);
    assert_eq!(defs[0].url.as_deref(), Some("https://example.com/mcp"));
    assert_eq!(
        defs[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer t")
    );
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
    assert_eq!(
        defs[0].args,
        vec!["-y".to_string(), "zotero".to_string()]
    );
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
