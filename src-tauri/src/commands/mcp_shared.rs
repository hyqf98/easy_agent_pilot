use serde_json::{json, Map, Value};
use std::fs;
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command as TokioCommand;

/// 在 Windows 上构建兼容的命令。
/// 对于 npx、npm 等 Node.js 脚本命令，在 Windows 上需要使用 cmd.exe /C 来执行。
pub fn build_platform_command(command: &str, args: &[String]) -> TokioCommand {
    #[cfg(target_os = "windows")]
    {
        let script_commands = ["npx", "npm", "yarn", "pnpm", "bun"];

        if script_commands.contains(&command) {
            let full_command = if args.is_empty() {
                command.to_string()
            } else {
                format!("{} {}", command, args.join(" "))
            };
            let mut cmd = TokioCommand::new("cmd");
            cmd.arg("/C").arg(&full_command);
            return cmd;
        }
    }

    let mut cmd = TokioCommand::new(command);
    cmd.args(args);
    cmd
}

pub fn parse_args_string(args: Option<&str>) -> Vec<String> {
    args.map(|value| value.split_whitespace().map(|item| item.to_string()).collect())
        .unwrap_or_default()
}

pub fn parse_string_map_json(raw: Option<&str>) -> HashMap<String, String> {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .and_then(|value| {
            value.as_object().map(|object| {
                object
                    .iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|value| (key.clone(), value.to_string()))
                    })
                    .collect()
            })
        })
        .unwrap_or_default()
}

pub fn build_stdio_command(
    command: &str,
    args: Option<&str>,
    env: Option<&str>,
) -> TokioCommand {
    let args_vec = parse_args_string(args);
    let env_map = parse_string_map_json(env);
    let mut cmd = build_platform_command(command, &args_vec);

    for (key, value) in env_map {
        cmd.env(key, value);
    }

    cmd
}

pub fn read_json_config_or_default(path: &Path, default_value: Value) -> Result<Value, String> {
    if !path.exists() {
        return Ok(default_value);
    }

    let content = fs::read_to_string(path).map_err(|e| format!("读取配置文件失败: {}", e))?;
    Ok(serde_json::from_str(&content).unwrap_or(default_value))
}

pub fn ensure_mcp_servers_object(
    settings: &mut Value,
) -> Result<&mut Map<String, Value>, String> {
    if !settings.is_object() {
        *settings = json!({});
    }

    let root = settings
        .as_object_mut()
        .ok_or_else(|| "配置根节点不是对象".to_string())?;

    let needs_reset = !root
        .get("mcpServers")
        .is_some_and(|value| value.is_object());
    if needs_reset {
        root.insert("mcpServers".to_string(), json!({}));
    }

    root.get_mut("mcpServers")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "无法获取 mcpServers 配置".to_string())
}

pub fn write_json_config_pretty(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
        }
    }

    let content =
        serde_json::to_string_pretty(value).map_err(|e| format!("序列化配置失败: {}", e))?;
    fs::write(path, content).map_err(|e| format!("写入配置文件失败: {}", e))?;
    Ok(())
}

/// 格式化 MCP 工具调用结果，使其更易读。
pub fn format_call_tool_result(call_result: &rmcp::model::CallToolResult) -> Value {
    use rmcp::model::RawContent;

    let content: Vec<Value> = call_result
        .content
        .iter()
        .map(|content| match &content.raw {
            RawContent::Text(text_content) => {
                if let Ok(parsed) = serde_json::from_str::<Value>(&text_content.text) {
                    parsed
                } else {
                    json!({
                        "type": "text",
                        "text": text_content.text
                    })
                }
            }
            RawContent::Image(image_content) => {
                json!({
                    "type": "image",
                    "mime_type": image_content.mime_type,
                    "data": image_content.data
                })
            }
            RawContent::Resource(resource_content) => {
                json!({
                    "type": "resource",
                    "resource": resource_content.resource
                })
            }
            RawContent::Audio(audio_content) => {
                json!({
                    "type": "audio",
                    "mime_type": audio_content.mime_type,
                    "data": audio_content.data
                })
            }
            RawContent::ResourceLink(resource_link) => {
                json!({
                    "type": "resource_link",
                    "uri": resource_link.uri,
                    "name": resource_link.name
                })
            }
        })
        .collect();

    let mut result = json!({ "content": content });

    if let Some(is_error) = call_result.is_error {
        result["is_error"] = json!(is_error);
    }

    if let Some(ref structured) = call_result.structured_content {
        result["structured_content"] = structured.clone();
    }

    result
}
