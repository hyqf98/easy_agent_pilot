use std::collections::HashSet;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tokio::process::Command as TokioCommand;

const WINDOWS_EXEC_EXTENSIONS: &[&str] = &[".exe", ".cmd", ".bat", ".com"];
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
pub fn configure_windows_std_command(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn configure_windows_std_command(_command: &mut Command) {}

#[cfg(target_os = "windows")]
pub fn configure_windows_tokio_command(command: &mut TokioCommand) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
pub fn configure_windows_tokio_command(_command: &mut TokioCommand) {}

fn split_path_tail(value: &str) -> &str {
    value.rsplit(['/', '\\']).next().unwrap_or(value)
}

pub fn normalize_cli_identifier(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }

    let lower = split_path_tail(trimmed).to_lowercase();
    let normalized = WINDOWS_EXEC_EXTENSIONS
        .iter()
        .find_map(|ext| lower.strip_suffix(ext))
        .unwrap_or(lower.as_str());

    if normalized.is_empty() {
        return None;
    }

    let canonical = match normalized {
        "claude" | "claude-code" => "claude",
        "codex" => "codex",
        "opencode" => "opencode",
        "qwen" | "qwen-code" => "qwen",
        other => other,
    };

    Some(canonical.to_string())
}

pub fn resolve_cli_name(
    cli_path: Option<&str>,
    cli_type_hint: Option<&str>,
    default_name: &str,
) -> String {
    cli_type_hint
        .and_then(normalize_cli_identifier)
        .or_else(|| cli_path.and_then(normalize_cli_identifier))
        .unwrap_or_else(|| default_name.to_string())
}

#[cfg(target_os = "windows")]
pub fn get_executable_extensions() -> Vec<String> {
    let mut seen = HashSet::new();
    let mut extensions = Vec::new();

    if let Some(path_ext) = std::env::var_os("PATHEXT") {
        for ext in path_ext.to_string_lossy().split(';') {
            let trimmed = ext.trim().to_lowercase();
            if trimmed.is_empty() {
                continue;
            }

            let normalized = if trimmed.starts_with('.') {
                trimmed
            } else {
                format!(".{}", trimmed)
            };

            if seen.insert(normalized.clone()) {
                extensions.push(normalized);
            }
        }
    }

    for ext in WINDOWS_EXEC_EXTENSIONS {
        let ext = ext.to_string();
        if seen.insert(ext.clone()) {
            extensions.push(ext);
        }
    }

    extensions
}

#[cfg(not(target_os = "windows"))]
pub fn get_executable_extensions() -> Vec<String> {
    vec![String::new()]
}

pub fn collect_search_dirs(extra_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut dirs = Vec::new();

    if let Some(path_value) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_value) {
            if dir.as_os_str().is_empty() {
                continue;
            }
            if seen.insert(dir.clone()) {
                dirs.push(dir);
            }
        }
    }

    for dir in extra_dirs {
        if dir.as_os_str().is_empty() {
            continue;
        }
        if seen.insert(dir.clone()) {
            dirs.push(dir.clone());
        }
    }

    dirs
}

fn resolve_cli_candidate_paths(cli_name: &str, dir: &Path) -> Vec<PathBuf> {
    let cli_path = Path::new(cli_name);
    let raw_name = cli_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(cli_name);
    let normalized_name =
        normalize_cli_identifier(raw_name).unwrap_or_else(|| raw_name.to_lowercase());
    let has_known_extension = raw_name
        .rsplit('.')
        .next()
        .map(|ext| WINDOWS_EXEC_EXTENSIONS.contains(&format!(".{}", ext.to_lowercase()).as_str()))
        .unwrap_or(false);

    let mut candidates = Vec::new();

    if has_known_extension {
        candidates.push(dir.join(raw_name));
        return candidates;
    }

    for ext in get_executable_extensions() {
        let file_name = if ext.is_empty() {
            normalized_name.clone()
        } else {
            format!("{}{}", normalized_name, ext)
        };
        candidates.push(dir.join(file_name));
    }

    candidates
}

#[cfg(target_os = "windows")]
fn find_cli_executables_via_shell(cli_name: &str) -> Vec<PathBuf> {
    let mut command = Command::new("where.exe");
    configure_windows_std_command(&mut command);
    let output = command.arg(cli_name).output();
    parse_shell_lookup_output(output)
}

#[cfg(not(target_os = "windows"))]
fn find_cli_executables_via_shell(cli_name: &str) -> Vec<PathBuf> {
    let output = Command::new("which").arg("-a").arg(cli_name).output();
    parse_shell_lookup_output(output)
}

fn parse_shell_lookup_output(output: std::io::Result<Output>) -> Vec<PathBuf> {
    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(|line| line.trim().trim_matches('"'))
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .collect()
}

pub fn find_cli_executables(cli_name: &str, extra_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let path = Path::new(cli_name);
    if path.components().count() > 1 || path.is_absolute() {
        if path.exists() {
            return vec![path.to_path_buf()];
        }
        return Vec::new();
    }

    let dirs = collect_search_dirs(extra_dirs);
    let mut seen = HashSet::new();
    let mut matches = Vec::new();

    for dir in dirs {
        if !dir.exists() || !dir.is_dir() {
            continue;
        }

        for candidate in resolve_cli_candidate_paths(cli_name, &dir) {
            if candidate.exists() && seen.insert(candidate.clone()) {
                matches.push(candidate);
            }
        }
    }

    for candidate in find_cli_executables_via_shell(cli_name) {
        if seen.insert(candidate.clone()) {
            matches.push(candidate);
        }
    }

    matches
}

pub fn find_cli_executable(cli_name: &str, extra_dirs: &[PathBuf]) -> Option<PathBuf> {
    find_cli_executables(cli_name, extra_dirs)
        .into_iter()
        .next()
}

#[cfg(target_os = "windows")]
pub fn run_cli_command(cli_path: &Path, args: &[&str]) -> std::io::Result<Output> {
    let resolved_cli_path = if !cli_path.is_absolute() && cli_path.components().count() == 1 {
        find_cli_executable(cli_path.to_string_lossy().as_ref(), &[])
            .unwrap_or_else(|| cli_path.to_path_buf())
    } else {
        cli_path.to_path_buf()
    };
    let extension = cli_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let resolved_extension = resolved_cli_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if matches!(extension.as_str(), "cmd" | "bat")
        || matches!(resolved_extension.as_str(), "cmd" | "bat")
    {
        let mut command = Command::new("cmd");
        configure_windows_std_command(&mut command);
        command.arg("/C").arg(&resolved_cli_path);
        command.args(args);
        command.output()
    } else {
        let mut command = Command::new(&resolved_cli_path);
        configure_windows_std_command(&mut command);
        command.args(args);
        command.output()
    }
}

#[cfg(not(target_os = "windows"))]
pub fn run_cli_command(cli_path: &Path, args: &[&str]) -> std::io::Result<Output> {
    let mut command = Command::new(cli_path);
    command.args(args);
    command.output()
}

#[cfg(target_os = "windows")]
pub fn build_tokio_cli_command(cli_path: &str, args: &[String]) -> TokioCommand {
    let raw_path = Path::new(cli_path);
    let resolved_cli_path = if !raw_path.is_absolute() && raw_path.components().count() == 1 {
        find_cli_executable(cli_path, &[])
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| cli_path.to_string())
    } else {
        cli_path.to_string()
    };
    let extension = raw_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let resolved_extension = Path::new(&resolved_cli_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if matches!(extension.as_str(), "cmd" | "bat")
        || matches!(resolved_extension.as_str(), "cmd" | "bat")
    {
        let mut command = TokioCommand::new("cmd");
        configure_windows_tokio_command(&mut command);
        command.arg("/C").arg(&resolved_cli_path);
        command.args(args);
        return command;
    }

    let mut command = TokioCommand::new(&resolved_cli_path);
    configure_windows_tokio_command(&mut command);
    command.args(args);
    command
}

pub fn build_cli_launch_error_message(
    cli_name: &str,
    cli_path: &str,
    error: &std::io::Error,
    working_directory: Option<&str>,
    arg_count: usize,
    stdin_payload_len: usize,
    prompt_via_stdin: bool,
) -> String {
    let mut segments = vec![format!("{cli_name} CLI 启动失败: {error}")];

    if let Some(os_code) = error.raw_os_error() {
        segments.push(format!("os error {os_code}"));
    }

    segments.push(format!("cli_path={cli_path}"));
    if let Some(cwd) = working_directory
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        segments.push(format!("cwd={cwd}"));
    }
    segments.push(format!("arg_count={arg_count}"));
    segments.push(format!(
        "stdin_prompt={}",
        if prompt_via_stdin { "yes" } else { "no" }
    ));

    if stdin_payload_len > 0 {
        segments.push(format!("prompt_chars={stdin_payload_len}"));
    }

    #[cfg(target_os = "windows")]
    {
        let extension = Path::new(cli_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if matches!(extension.as_str(), "cmd" | "bat") {
            segments.push(
                "当前 CLI 通过 cmd/bat 启动，Windows 命令行长度限制会比 .exe 更严格".to_string(),
            );
        }

        match error.raw_os_error() {
            Some(206) => segments.push(
                "Windows 检测到命令行长度限制；请避免把长提示词或大 JSON 直接放进命令参数"
                    .to_string(),
            ),
            Some(2) => segments.push(
                "Windows 找不到指定文件；请检查 CLI 路径是否存在并优先使用可执行 .exe 路径"
                    .to_string(),
            ),
            _ => {}
        }
    }

    segments.join(" | ")
}

#[cfg(not(target_os = "windows"))]
pub fn build_tokio_cli_command(cli_path: &str, args: &[String]) -> TokioCommand {
    let mut command = TokioCommand::new(cli_path);
    command.args(args);
    command
}

pub fn get_cli_version(cli_path: &Path) -> Option<String> {
    let output = run_cli_command(cli_path, &["--version"]).ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return Some(stdout);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        None
    } else {
        Some(stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_cli_identifier, resolve_cli_name};

    #[test]
    fn normalize_cli_identifier_supports_windows_wrappers() {
        assert_eq!(
            normalize_cli_identifier(r"C:\Users\dev\AppData\Roaming\npm\claude.cmd"),
            Some("claude".to_string())
        );
        assert_eq!(
            normalize_cli_identifier(r"C:\Users\dev\AppData\Roaming\npm\codex.CMD"),
            Some("codex".to_string())
        );
    }

    #[test]
    fn normalize_cli_identifier_supports_aliases() {
        assert_eq!(
            normalize_cli_identifier("/usr/local/bin/claude-code"),
            Some("claude".to_string())
        );
        assert_eq!(
            normalize_cli_identifier("/usr/local/bin/qwen-code"),
            Some("qwen".to_string())
        );
    }

    #[test]
    fn resolve_cli_name_prefers_hint_then_path() {
        assert_eq!(
            resolve_cli_name(
                Some(r"C:\Users\dev\AppData\Roaming\npm\codex.cmd"),
                Some("claude"),
                "claude"
            ),
            "claude"
        );
        assert_eq!(
            resolve_cli_name(
                Some(r"C:\Users\dev\AppData\Roaming\npm\codex.cmd"),
                None,
                "claude"
            ),
            "codex"
        );
    }
}
