//! CLI 安装器模�?
//!

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "windows")]
use crate::commands::cli_support::configure_windows_std_command;
use crate::commands::cli_support::{find_cli_executable, get_cli_version};
use crate::commands::cli::get_scan_paths_public;

/// 当前正在执行安装或升级的 CLI。
static ACTIVE_CLI_OPERATIONS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn create_command<S: AsRef<OsStr>>(program: S) -> Command {
    let program_ref = program.as_ref();
    let resolved = {
        let program_str = program_ref.to_str().unwrap_or_default();
        let path = std::path::Path::new(program_str);
        if path.is_absolute() || path.components().count() > 1 {
            program_ref.to_os_string()
        } else {
            let scan_paths = get_scan_paths_public();
            find_cli_executable(program_str, &scan_paths)
                .map(|p| p.into_os_string())
                .unwrap_or_else(|| program_ref.to_os_string())
        }
    };

    let mut command = Command::new(&resolved);
    #[cfg(target_os = "windows")]
    {
        configure_windows_std_command(&mut command);
    }

    let scan_paths = get_scan_paths_public();
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

    for path in scan_paths {
        if path.as_os_str().is_empty() {
            continue;
        }
        if seen.insert(path.clone()) {
            path_entries.push(path);
        }
    }

    if !path_entries.is_empty() {
        let new_path = std::env::join_paths(&path_entries).unwrap_or_default();
        command.env("PATH", new_path);
    }

    command
}

#[cfg(windows)]
const WINDOWS_CLAUDE_NATIVE_INSTALL_COMMAND: &str =
    "& ([ScriptBlock]::Create((Invoke-RestMethod -Uri 'https://claude.ai/install.ps1'))) -Target latest";

/// 包管理器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManager {
    /// 包管理器名称 (npm, homebrew, curl)
    pub name: String,
    pub available: bool,
    /// 版本信息
    pub version: Option<String>,
}

/// 安装选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOption {
    /// 安装方式 (native, npm, homebrew)
    pub method: String,
    /// 完整安装命令
    pub command: String,
    /// 是否推荐
    pub recommended: bool,
    /// 依赖的包管理器是否可�?
    pub available: bool,
    /// 方式显示名称
    pub display_name: String,
}

/// CLI 安装信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliInstallerInfo {
    /// CLI 名称
    pub cli_name: String,
    /// 是否已安�?
    pub installed: bool,
    pub current_version: Option<String>,
    pub install_options: Vec<InstallOption>,
}

/// 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current: Option<String>,
    pub latest: Option<String>,
    pub has_update: bool,
    /// 更新说明
    pub release_notes: Option<String>,
}

/// 安装日志事件
#[derive(Debug, Clone, Serialize)]
pub struct InstallLogEvent {
    /// CLI 名称
    pub cli_name: String,
    /// 日志消息
    pub message: String,
    /// 时间�?
    pub timestamp: String,
}

/// 安装完成事件
#[derive(Debug, Clone, Serialize)]
pub struct InstallCompleteEvent {
    /// CLI 名称
    pub cli_name: String,
    /// 是否成功
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn detect_package_managers() -> Result<Vec<PackageManager>, String> {
    tokio::task::spawn_blocking(|| {
        let mut managers = Vec::new();
        let scan_paths = get_scan_paths_public();

        let npm_path = find_cli_executable("npm", &scan_paths);
        managers.push(PackageManager {
            name: "npm".to_string(),
            available: npm_path.is_some(),
            version: npm_path
                .as_deref()
                .and_then(|path| get_cli_version(path)),
        });

        #[cfg(target_os = "macos")]
        {
            let brew_path = find_cli_executable("brew", &scan_paths);
            managers.push(PackageManager {
                name: "homebrew".to_string(),
                available: true,
                version: brew_path
                    .as_deref()
                    .and_then(|path| get_cli_version(path)),
            });
        }

        #[cfg(not(windows))]
        {
            let curl_path = find_cli_executable("curl", &scan_paths);
            managers.push(PackageManager {
                name: "curl".to_string(),
                available: curl_path.is_some(),
                version: curl_path
                    .as_deref()
                    .and_then(|path| get_cli_version(path)),
            });
        }

        Ok(managers)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn detect_package_managers_sync() -> Vec<PackageManager> {
    let mut managers = Vec::new();
    let scan_paths = get_scan_paths_public();

    let npm_path = find_cli_executable("npm", &scan_paths);
    managers.push(PackageManager {
        name: "npm".to_string(),
        available: npm_path.is_some(),
        version: npm_path
            .as_deref()
            .and_then(|path| get_cli_version(path)),
    });

    #[cfg(target_os = "macos")]
    {
        let brew_path = find_cli_executable("brew", &scan_paths);
        managers.push(PackageManager {
            name: "homebrew".to_string(),
            available: true,
            version: brew_path
                .as_deref()
                .and_then(|path| get_cli_version(path)),
        });
    }

    #[cfg(not(windows))]
    {
        let curl_path = find_cli_executable("curl", &scan_paths);
        managers.push(PackageManager {
            name: "curl".to_string(),
            available: curl_path.is_some(),
            version: curl_path
                .as_deref()
                .and_then(|path| get_cli_version(path)),
        });
    }

    managers
}

fn detect_cli_installed(cli_name: &str) -> (bool, Option<String>) {
    let scan_paths = get_scan_paths_public();
    let executable = find_cli_executable(cli_name, &scan_paths);
    let version = executable
        .as_deref()
        .and_then(|path| get_cli_version(path));
    (executable.is_some(), version)
}

/// 获取 CLI 的安装��项
#[tauri::command]
pub async fn get_cli_install_options(cli_name: String) -> Result<CliInstallerInfo, String> {
    let cli_name_clone = cli_name.clone();
    let managers = tokio::task::spawn_blocking(move || {
        let scan_paths = get_scan_paths_public();
        let executable = find_cli_executable(&cli_name_clone, &scan_paths);
        let version = executable
            .as_deref()
            .and_then(|path| get_cli_version(path));
        let managers = detect_package_managers_sync();
        (executable.is_some(), version, managers)
    })
    .await
    .map_err(|e| e.to_string())?;

    let (installed, current_version, manager_list) = managers;
    let manager_map: HashMap<&str, bool> = manager_list
        .iter()
        .map(|m| (m.name.as_str(), m.available))
        .collect();

    let mut options = Vec::new();

    match cli_name.as_str() {
        "claude" => {
            // 原生安装 - macOS/Linux
            #[cfg(not(windows))]
            {
                options.push(InstallOption {
                    method: "native".to_string(),
                    command: "curl -fsSL https://claude.ai/install.sh | bash".to_string(),
                    recommended: true,
                    available: *manager_map.get("curl").unwrap_or(&false),
                    display_name: "Native Install".to_string(),
                });
            }

            // 原生安装 - Windows
            #[cfg(windows)]
            {
                options.push(InstallOption {
                    method: "native".to_string(),
                    command: WINDOWS_CLAUDE_NATIVE_INSTALL_COMMAND.to_string(),
                    recommended: true,
                    available: true, // PowerShell 默认可用
                    display_name: "Native Install (PowerShell)".to_string(),
                });
            }

            // Homebrew - macOS
            #[cfg(target_os = "macos")]
            {
                options.push(InstallOption {
                    method: "homebrew".to_string(),
                    command: "brew install claude-code".to_string(),
                    recommended: false,
                    available: true,
                    display_name: "Homebrew".to_string(),
                });
            }

            if *manager_map.get("npm").unwrap_or(&false) {
                options.push(InstallOption {
                    method: "npm".to_string(),
                    command: "npm install -g @anthropic-ai/claude-code".to_string(),
                    recommended: false,
                    available: true,
                    display_name: "npm".to_string(),
                });
            }
        }
        "codex" => {
            // npm - 推荐方式
            if *manager_map.get("npm").unwrap_or(&false) {
                options.push(InstallOption {
                    method: "npm".to_string(),
                    command: "npm install -g @openai/codex".to_string(),
                    recommended: true,
                    available: true,
                    display_name: "npm".to_string(),
                });
            }

            // Homebrew - macOS
            #[cfg(target_os = "macos")]
            {
                options.push(InstallOption {
                    method: "homebrew".to_string(),
                    command: "brew install codex".to_string(),
                    recommended: false,
                    available: true,
                    display_name: "Homebrew".to_string(),
                });
            }
        }
        "opencode" => {
            #[cfg(not(windows))]
            {
                options.push(InstallOption {
                    method: "native".to_string(),
                    command: "curl -fsSL https://opencode.ai/install | bash".to_string(),
                    recommended: true,
                    available: *manager_map.get("curl").unwrap_or(&false),
                    display_name: "Native Install".to_string(),
                });
            }

            #[cfg(target_os = "macos")]
            {
                options.push(InstallOption {
                    method: "homebrew".to_string(),
                    command: "brew install anomalyco/tap/opencode".to_string(),
                    recommended: false,
                    available: true,
                    display_name: "Homebrew".to_string(),
                });
            }

            if *manager_map.get("npm").unwrap_or(&false) {
                options.push(InstallOption {
                    method: "npm".to_string(),
                    command: "npm install -g opencode-ai@latest".to_string(),
                    recommended: false,
                    available: true,
                    display_name: "npm".to_string(),
                });
            }
        }
        _ => {
            return Err(format!("Unsupported CLI: {}", cli_name));
        }
    }

    Ok(CliInstallerInfo {
        cli_name,
        installed,
        current_version,
        install_options: options,
    })
}

/// 获取 npm 包名
fn get_npm_package(cli_name: &str) -> &'static str {
    match cli_name {
        "claude" => "@anthropic-ai/claude-code",
        "codex" => "@openai/codex",
        "opencode" => "opencode-ai",
        _ => "",
    }
}

/// 获取 brew 包名
fn get_brew_package(cli_name: &str) -> &'static str {
    match cli_name {
        "claude" => "claude-code",
        "codex" => "codex",
        "opencode" => "anomalyco/tap/opencode",
        _ => "",
    }
}

fn extract_version_token(value: &str) -> Option<String> {
    let mut started = false;
    let mut token = String::new();

    for ch in value.chars() {
        if ch.is_ascii_digit() {
            started = true;
            token.push(ch);
            continue;
        }

        if started && ch == '.' {
            token.push(ch);
            continue;
        }

        if started {
            break;
        }
    }

    let normalized = token.trim_matches('.').to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_version_identity(value: Option<String>) -> Option<String> {
    value.and_then(|raw| extract_version_token(&raw).or(Some(raw)))
}

fn resolve_version_state(current_raw: Option<String>, latest_raw: Option<String>) -> VersionInfo {
    let current = normalize_version_identity(current_raw);
    let latest = normalize_version_identity(latest_raw);
    let has_update = match (&current, &latest) {
        (Some(curr), Some(lat)) => curr != lat,
        _ => false,
    };

    VersionInfo {
        current,
        latest,
        has_update,
        release_notes: None,
    }
}

fn emit_log_event(app: &AppHandle, cli_name: &str, message: &str) {
    let _ = app.emit(
        "cli-install-log",
        InstallLogEvent {
            cli_name: cli_name.to_string(),
            message: message.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        },
    );
}

fn begin_cli_operation(cli_name: &str) -> Result<(), String> {
    let mut operations = ACTIVE_CLI_OPERATIONS
        .lock()
        .map_err(|_| "CLI operation state lock poisoned".to_string())?;

    if operations.contains(cli_name) {
        return Err(format!("{} 正在执行安装或升级，请稍后再试", cli_name));
    }

    operations.insert(cli_name.to_string());
    Ok(())
}

fn finish_cli_operation(cli_name: &str) {
    if let Ok(mut operations) = ACTIVE_CLI_OPERATIONS.lock() {
        operations.remove(cli_name);
    }
}

#[tauri::command]
pub async fn install_cli(cli_name: String, method: String, app: AppHandle) -> Result<(), String> {
    begin_cli_operation(&cli_name)?;

    emit_log_event(
        &app,
        &cli_name,
        &format!("🚀 Starting installation of {}...", cli_name),
    );

    let options = get_cli_install_options(cli_name.clone()).await?;
    let option = options
        .install_options
        .iter()
        .find(|o| o.method == method)
        .ok_or("Installation method not found")?;

    if !option.available {
        finish_cli_operation(&cli_name);
        return Err(format!(
            "Required package manager for '{}' method is not available",
            method
        ));
    }

    emit_log_event(
        &app,
        &cli_name,
        &format!("📝 Executing: {}", option.command),
    );

    let result = execute_install_command(&app, &cli_name, &method, &option.command);
    finish_cli_operation(&cli_name);

    match result {
        Ok(_) => {
            emit_log_event(
                &app,
                &cli_name,
                &format!("�?{} installed successfully!", cli_name),
            );
            let _ = app.emit(
                "cli-install-complete",
                InstallCompleteEvent {
                    cli_name: cli_name.clone(),
                    success: true,
                    error: None,
                },
            );
            Ok(())
        }
        Err(e) => {
            emit_log_event(&app, &cli_name, &format!("�?Installation failed: {}", e));
            let _ = app.emit(
                "cli-install-complete",
                InstallCompleteEvent {
                    cli_name: cli_name.clone(),
                    success: false,
                    error: Some(e.clone()),
                },
            );
            Err(e)
        }
    }
}

fn execute_install_command(
    app: &AppHandle,
    cli_name: &str,
    method: &str,
    command: &str,
) -> Result<(), String> {
    let mut child = match method {
        "native" => {
            #[cfg(not(windows))]
            {
                create_command("bash")
                    .arg("-c")
                    .arg(command)
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
            }
            #[cfg(windows)]
            {
                create_command("powershell.exe")
                    .args([
                        "-NoLogo",
                        "-NoProfile",
                        "-NonInteractive",
                        "-ExecutionPolicy",
                        "Bypass",
                        "-Command",
                        command,
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
            }
        }
        "npm" => {
            let package = get_npm_package(cli_name);
            create_command("npm")
                .args(["install", "-g", package])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        }
        "homebrew" => {
            let package = get_brew_package(cli_name);
            create_command("brew")
                .args(["install", package])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        }
        _ => return Err(format!("Unsupported installation method: {}", method)),
    }
    .map_err(|e| format!("Failed to start command: {}", e))?;

    let app_clone = app.clone();
    let cli_name_clone = cli_name.to_string();

    // 实时读取 stdout
    if let Some(stdout) = child.stdout.take() {
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                emit_log_event(&app_clone, &cli_name_clone, &line);
            }
        });
    }

    // �?stderr 准备克隆
    let app_clone = app.clone();
    let cli_name_clone = cli_name.to_string();

    // 实时读取 stderr
    if let Some(stderr) = child.stderr.take() {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                emit_log_event(&app_clone, &cli_name_clone, &line);
            }
        });
    }

    // 等待命令完成
    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for command: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Command failed with exit code: {:?}",
            status.code()
        ))
    }
}

#[tauri::command]
pub async fn check_cli_update(cli_name: String) -> Result<VersionInfo, String> {
    let cli_name_clone = cli_name.clone();
    let current_raw = tokio::task::spawn_blocking(move || {
        let (_, version) = detect_cli_installed(&cli_name_clone);
        version
    })
    .await
    .map_err(|e| e.to_string())?;

    if current_raw.is_none() {
        return Ok(resolve_version_state(None, None));
    }

    let latest_raw = fetch_npm_version(cli_name.as_str()).await;
    Ok(resolve_version_state(current_raw, latest_raw))
}

async fn fetch_npm_version(cli_name: &str) -> Option<String> {
    let url = match cli_name {
        "claude" => "https://registry.npmjs.org/@anthropic-ai/claude-code/latest",
        "codex" => "https://registry.npmjs.org/@openai/codex/latest",
        "opencode" => "https://registry.npmjs.org/opencode-ai/latest",
        _ => return None,
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let response = client.get(url).send().await.ok()?;

    if !response.status().is_success() {
        return None;
    }

    let json: serde_json::Value = response.json().await.ok()?;
    json.get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn detect_install_method(cli_name: &str) -> Option<String> {
    if verify_npm_owns(cli_name) {
        return Some("npm".to_string());
    }

    #[cfg(unix)]
    if verify_brew_owns(cli_name) {
        return Some("homebrew".to_string());
    }

    None
}

fn verify_npm_owns(cli_name: &str) -> bool {
    let package = get_npm_package(cli_name);
    if package.is_empty() {
        return false;
    }
    create_command("npm")
        .args(["list", "-g", package, "--depth=0"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(unix)]
fn verify_brew_owns(cli_name: &str) -> bool {
    let package = get_brew_package(cli_name);
    if package.is_empty() {
        return false;
    }
    // Check cask first (claude-code etc.), then formula
    create_command("brew")
        .args(["list", "--cask", package])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        || create_command("brew")
            .args(["list", "--formula", package])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

fn build_upgrade_method_order_for_context(
    _cli_name: &str,
    install_info: &CliInstallerInfo,
    detected: Option<String>,
    prefer_native_last: bool,
) -> Vec<String> {
    let mut methods = Vec::new();
    let mut push_method = |method: &str| {
        if methods.iter().any(|existing| existing == method) {
            return;
        }

        if !install_info
            .install_options
            .iter()
            .any(|option| option.available && option.method == method)
        {
            return;
        }

        methods.push(method.to_string());
    };

    if let Some(ref m) = detected {
        let should_deprioritize_detected_native = prefer_native_last && m == "native";
        if !should_deprioritize_detected_native {
            push_method(m);
        }
    }

    if prefer_native_last {
        for option in &install_info.install_options {
            if option.available && option.method != "native" {
                push_method(&option.method);
            }
        }
    }

    for option in &install_info.install_options {
        if option.available {
            push_method(&option.method);
        }
    }

    methods
}

fn build_upgrade_method_order(cli_name: &str, install_info: &CliInstallerInfo) -> Vec<String> {
    let detected = detect_install_method(cli_name);

    #[cfg(windows)]
    let prefer_native_last = cli_name == "claude";
    #[cfg(not(windows))]
    let prefer_native_last = false;

    build_upgrade_method_order_for_context(cli_name, install_info, detected, prefer_native_last)
}

async fn execute_upgrade_by_method(
    app: &AppHandle,
    cli_name: &str,
    method: &str,
    install_info: &CliInstallerInfo,
) -> Result<(), String> {
    match method {
        "npm" => {
            let package = get_npm_package(cli_name);
            execute_upgrade_command(app, cli_name, "npm", &["install", "-g", package]).await
        }
        "homebrew" => {
            let package = get_brew_package(cli_name);
            execute_upgrade_command(app, cli_name, "brew", &["upgrade", package]).await
        }
        _ => {
            emit_log_event(app, cli_name, "📝 Re-running native installer...");
            let option = install_info
                .install_options
                .iter()
                .find(|o| o.method == method)
                .ok_or("Install option not found")?;
            execute_install_command(app, cli_name, method, &option.command)
        }
    }
}

/// 升级 CLI（自动检测安装来源，失败后依次尝试其他可用方式）
#[tauri::command]
pub async fn upgrade_cli(cli_name: String, app: AppHandle) -> Result<(), String> {
    begin_cli_operation(&cli_name)?;

    emit_log_event(
        &app,
        &cli_name,
        &format!("🔄 Starting upgrade of {}...", cli_name),
    );

    let result = async {
        let install_info = get_cli_install_options(cli_name.clone()).await?;
        let methods = build_upgrade_method_order(&cli_name, &install_info);

        if methods.is_empty() {
            return Err("No available upgrade method".to_string());
        }

        let mut last_error = String::new();
        for method in &methods {
            emit_log_event(
                &app,
                &cli_name,
                &format!("📦 Trying upgrade method: {}", method),
            );

            match execute_upgrade_by_method(&app, &cli_name, method, &install_info).await {
                Ok(_) => {
                    let version_state = check_cli_update(cli_name.clone()).await?;
                    if !version_state.has_update {
                        return Ok(());
                    }

                    let current = version_state
                        .current
                        .unwrap_or_else(|| "unknown".to_string());
                    let latest = version_state
                        .latest
                        .unwrap_or_else(|| "unknown".to_string());
                    let stale_error = format!(
                        "upgrade command completed but current version is still v{} (latest v{})",
                        current, latest
                    );
                    emit_log_event(&app, &cli_name, &format!("⚠️ {}", stale_error));
                    last_error = stale_error;
                }
                Err(e) => {
                    emit_log_event(
                        &app,
                        &cli_name,
                        &format!("⚠️ Method '{}' failed: {}", method, e),
                    );
                    last_error = e;
                }
            }
        }

        Err(last_error)
    }
    .await;

    finish_cli_operation(&cli_name);

    match result {
        Ok(_) => {
            emit_log_event(
                &app,
                &cli_name,
                &format!("✅ {} upgraded successfully!", cli_name),
            );
            let _ = app.emit(
                "cli-install-complete",
                InstallCompleteEvent {
                    cli_name: cli_name.clone(),
                    success: true,
                    error: None,
                },
            );
            Ok(())
        }
        Err(e) => {
            emit_log_event(
                &app,
                &cli_name,
                &format!("❌ All upgrade methods failed: {}", e),
            );
            let _ = app.emit(
                "cli-install-complete",
                InstallCompleteEvent {
                    cli_name: cli_name.clone(),
                    success: false,
                    error: Some(e.clone()),
                },
            );
            Err(e)
        }
    }
}

async fn execute_upgrade_command(
    app: &AppHandle,
    cli_name: &str,
    program: &str,
    args: &[&str],
) -> Result<(), String> {
    emit_log_event(
        app,
        cli_name,
        &format!("📝 Executing: {} {}", program, args.join(" ")),
    );

    let mut child = create_command(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start command: {}", e))?;

    // �?stdout 线程准备克隆
    let app_clone = app.clone();
    let cli_name_clone = cli_name.to_string();

    // 实时读取 stdout
    if let Some(stdout) = child.stdout.take() {
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                emit_log_event(&app_clone, &cli_name_clone, &line);
            }
        });
    }

    // �?stderr 线程准备克隆
    let app_clone = app.clone();
    let cli_name_clone = cli_name.to_string();

    // 实时读取 stderr
    if let Some(stderr) = child.stderr.take() {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                emit_log_event(&app_clone, &cli_name_clone, &line);
            }
        });
    }

    // 等待命令完成
    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for command: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Command failed with exit code: {:?}",
            status.code()
        ))
    }
}

#[tauri::command]
pub fn cancel_install() -> Result<(), String> {
    if let Ok(mut operations) = ACTIVE_CLI_OPERATIONS.lock() {
        operations.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{build_upgrade_method_order_for_context, CliInstallerInfo, InstallOption};

    fn create_install_info(options: Vec<InstallOption>) -> CliInstallerInfo {
        CliInstallerInfo {
            cli_name: "claude".to_string(),
            installed: true,
            current_version: Some("1.0.0".to_string()),
            install_options: options,
        }
    }

    #[test]
    fn prefers_non_native_methods_before_native_when_requested() {
        let install_info = create_install_info(vec![
            InstallOption {
                method: "native".to_string(),
                command: "native".to_string(),
                recommended: true,
                available: true,
                display_name: "Native".to_string(),
            },
            InstallOption {
                method: "npm".to_string(),
                command: "npm".to_string(),
                recommended: false,
                available: true,
                display_name: "npm".to_string(),
            },
        ]);

        let methods = build_upgrade_method_order_for_context(
            "claude",
            &install_info,
            Some("native".to_string()),
            true,
        );

        assert_eq!(methods, vec!["npm".to_string(), "native".to_string()]);
    }

    #[test]
    fn keeps_detected_method_first_when_native_is_not_deprioritized() {
        let install_info = create_install_info(vec![
            InstallOption {
                method: "native".to_string(),
                command: "native".to_string(),
                recommended: true,
                available: true,
                display_name: "Native".to_string(),
            },
            InstallOption {
                method: "npm".to_string(),
                command: "npm".to_string(),
                recommended: false,
                available: true,
                display_name: "npm".to_string(),
            },
        ]);

        let methods = build_upgrade_method_order_for_context(
            "claude",
            &install_info,
            Some("native".to_string()),
            false,
        );

        assert_eq!(methods, vec!["native".to_string(), "npm".to_string()]);
    }
}
