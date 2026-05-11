use std::collections::HashSet;
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;
use tokio::process::Command as TokioCommand;

use crate::commands::cli::get_scan_paths_public;

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
fn is_windows_script_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "cmd" | "bat"))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn needs_cmd_wrapper(cli_path: &str) -> bool {
    let path = Path::new(cli_path);
    if !path.is_absolute() && path.components().count() <= 1 {
        return true;
    }
    is_windows_script_path(path)
}

#[cfg(target_os = "windows")]
fn wrap_for_windows(cli_path: &str, args: &[String]) -> (String, Vec<String>) {
    if !needs_cmd_wrapper(cli_path) {
        return (cli_path.to_string(), args.to_vec());
    }

    crate::logging::write_log(
        "DEBUG",
        "cli_support",
        &format!("Windows 命令包装: {} → cmd.exe /C {} + {} args", cli_path, cli_path, args.len()),
    );

    let mut wrapped_args = vec!["/C".to_string(), cli_path.to_string()];
    wrapped_args.extend(args.iter().cloned());
    ("cmd.exe".to_string(), wrapped_args)
}

fn build_cli_runtime_path(cli_path: &Path) -> Option<OsString> {
    let mut path_entries = Vec::new();
    let mut seen = HashSet::new();

    if let Some(existing_path) = std::env::var_os("PATH") {
        for entry in std::env::split_paths(&existing_path) {
            if entry.as_os_str().is_empty() {
                continue;
            }
            if seen.insert(entry.clone()) {
                path_entries.push(entry);
            }
        }
    }

    if let Some(parent) = cli_path.parent() {
        if !parent.as_os_str().is_empty() && seen.insert(parent.to_path_buf()) {
            path_entries.push(parent.to_path_buf());
        }
    }

    for path in get_scan_paths_public() {
        if path.as_os_str().is_empty() {
            continue;
        }
        if seen.insert(path.clone()) {
            path_entries.push(path);
        }
    }

    std::env::join_paths(path_entries).ok()
}

fn configure_cli_std_command_env(command: &mut Command, cli_path: &Path) {
    if let Some(path_value) = build_cli_runtime_path(cli_path) {
        command.env("PATH", path_value);
    }
}

fn configure_cli_tokio_command_env(command: &mut TokioCommand, cli_path: &Path) {
    if let Some(path_value) = build_cli_runtime_path(cli_path) {
        command.env("PATH", path_value);
    }
}

#[cfg(target_os = "windows")]
pub fn run_cli_command(cli_path: &Path, args: &[&str]) -> std::io::Result<Output> {
    let cli_str = cli_path.to_str().unwrap_or_default();
    if needs_cmd_wrapper(cli_str) {
        let mut command = Command::new("cmd.exe");
        configure_windows_std_command(&mut command);
        configure_cli_std_command_env(&mut command, cli_path);
        command.arg("/C").arg(cli_str).args(args);
        command.output()
    } else {
        let mut command = Command::new(cli_path);
        configure_windows_std_command(&mut command);
        configure_cli_std_command_env(&mut command, cli_path);
        command.args(args);
        command.output()
    }
}

#[cfg(not(target_os = "windows"))]
pub fn run_cli_command(cli_path: &Path, args: &[&str]) -> std::io::Result<Output> {
    let mut command = Command::new(cli_path);
    configure_cli_std_command_env(&mut command, cli_path);
    command.args(args);
    command.output()
}

#[cfg(target_os = "windows")]
pub fn build_tokio_cli_command(cli_path: &str, args: &[String]) -> TokioCommand {
    let (executable, wrapped_args) = wrap_for_windows(cli_path, args);
    let mut command = TokioCommand::new(&executable);
    configure_windows_tokio_command(&mut command);
    configure_cli_tokio_command_env(&mut command, Path::new(cli_path));
    command.args(&wrapped_args);
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

    segments.push(format!("cli_command={cli_path}"));
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
                "Windows 找不到指定命令；请检查 CLI 命令是否已安装并已加入 PATH".to_string(),
            ),
            _ => {}
        }
    }

    segments.join(" | ")
}

#[cfg(not(target_os = "windows"))]
pub fn build_tokio_cli_command(cli_path: &str, args: &[String]) -> TokioCommand {
    let mut command = TokioCommand::new(cli_path);
    configure_cli_tokio_command_env(&mut command, Path::new(cli_path));
    command.args(args);
    command
}

pub fn get_cli_version(cli_path: &Path) -> Option<String> {
    let is_explicit_path = cli_path.is_absolute() || cli_path.components().count() > 1;
    if is_explicit_path && (!cli_path.exists() || cli_path.is_dir()) {
        return None;
    }

    for args in [
        ["--version"].as_slice(),
        ["-v"].as_slice(),
        ["version"].as_slice(),
    ] {
        let output = run_cli_command_with_timeout(cli_path, args);
        let output = match output {
            Some(o) => o,
            None => continue,
        };
        if !output.status.success() {
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            return Some(stdout);
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            return Some(stderr);
        }
    }

    None
}

const VERSION_CHECK_TIMEOUT_SECS: u64 = 5;

fn run_cli_command_with_timeout(cli_path: &Path, args: &[&str]) -> Option<Output> {
    let cli_path_owned = cli_path.to_path_buf();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let args_refs: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
        let result = run_cli_command(&cli_path_owned, &args_refs);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(VERSION_CHECK_TIMEOUT_SECS)) {
        Ok(Ok(output)) => Some(output),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{get_cli_version, normalize_cli_identifier, resolve_cli_name};
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[cfg(unix)]
    #[test]
    fn get_cli_version_supports_env_node_wrappers() {
        let temp_root = std::env::temp_dir().join(format!(
            "easy-agent-pilot-cli-support-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("create temp dir");

        let fake_node_path = temp_root.join("node");
        let fake_cli_path = temp_root.join("codex");

        fs::write(
            &fake_node_path,
            "#!/bin/sh\necho \"codex-cli 9.9.9-test\"\n",
        )
        .expect("write fake node");
        fs::write(&fake_cli_path, "#!/usr/bin/env node\n").expect("write fake cli");

        let executable_mode = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&fake_node_path, executable_mode.clone()).expect("chmod node");
        fs::set_permissions(&fake_cli_path, executable_mode).expect("chmod cli");

        let version = get_cli_version(fake_cli_path.as_path());
        assert_eq!(version.as_deref(), Some("codex-cli 9.9.9-test"));

        let _ = fs::remove_file(&fake_node_path);
        let _ = fs::remove_file(&fake_cli_path);
        let _ = fs::remove_dir(&temp_root);
    }
}
