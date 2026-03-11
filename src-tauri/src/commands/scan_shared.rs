use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::commands::scan::{parse_mcp_server_config, McpConfigScope, ScannedMcpServer};

pub(crate) fn read_json_file(path: &Path) -> Option<serde_json::Value> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str::<serde_json::Value>(&content).ok()
}

pub(crate) fn collect_mcp_servers_from_json(
    json: &serde_json::Value,
    scope: McpConfigScope,
    servers: &mut Vec<ScannedMcpServer>,
) {
    if let Some(mcp_servers) = json.get("mcpServers").and_then(|value| value.as_object()) {
        for (name, config) in mcp_servers {
            if servers.iter().any(|server| server.name == *name) {
                continue;
            }

            if let Some(config_obj) = config.as_object() {
                if let Some(server) = parse_mcp_server_config(name, config_obj, scope.clone()) {
                    servers.push(server);
                }
            }
        }
    }
}

pub(crate) fn scan_mcp_source_file(
    path: &Path,
    scope: McpConfigScope,
    servers: &mut Vec<ScannedMcpServer>,
) {
    if !path.exists() {
        return;
    }

    if let Some(json) = read_json_file(path) {
        collect_mcp_servers_from_json(&json, scope, servers);
    }
}

pub(crate) fn parse_author_value(value: Option<&serde_json::Value>) -> Option<String> {
    match value {
        Some(author) if author.is_string() => author.as_str().map(|item| item.to_string()),
        Some(author) if author.is_object() => author
            .as_object()
            .and_then(|object| object.get("name"))
            .and_then(|item| item.as_str())
            .map(|item| item.to_string()),
        _ => None,
    }
}

pub(crate) fn parse_string_map(
    value: Option<&serde_json::Value>,
) -> Option<HashMap<String, String>> {
    value.and_then(|item| item.as_object()).map(|object| {
        object
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|entry| (key.clone(), entry.to_string())))
            .collect()
    })
}
