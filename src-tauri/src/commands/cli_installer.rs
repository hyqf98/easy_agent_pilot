//! CLI 安装器模�?
//!

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::commands::cli_support::{
    configure_windows_std_command, find_cli_executable, get_cli_version,
};

/// 安装进行中标�?
static INSTALLING: AtomicBool = AtomicBool::new(false);

fn create_command(program: &str) -> Command {
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    configure_windows_std_command(&mut command);
    command
}

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
pub fn detect_package_managers() -> Result<Vec<PackageManager>, String> {
    let mut managers = Vec::new();

    if let Ok(output) = create_command("npm").arg("--version").output() {
        let available = output.status.success();
        let version = if available {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        };
        managers.push(PackageManager {
            name: "npm".to_string(),
            available,
            version,
        });
    } else {
        managers.push(PackageManager {
            name: "npm".to_string(),
            available: false,
            version: None,
        });
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = create_command("brew").arg("--version").output() {
            let available = output.status.success();
            let version = if available {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().next().map(|s| s.to_string())
            } else {
                None
            };
            managers.push(PackageManager {
                name: "homebrew".to_string(),
                available,
                version,
            });
        } else {
            managers.push(PackageManager {
                name: "homebrew".to_string(),
                available: false,
                version: None,
            });
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(output) = create_command("curl").arg("--version").output() {
            let available = output.status.success();
            let version = if available {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().next().map(|s| s.to_string())
            } else {
                None
            };
            managers.push(PackageManager {
                name: "curl".to_string(),
                available,
                version,
            });
        } else {
            managers.push(PackageManager {
                name: "curl".to_string(),
                available: false,
                version: None,
            });
        }
    }

    Ok(managers)
}

fn detect_cli_installed(cli_name: &str) -> (bool, Option<String>) {
    if let Some(cli_path) = find_cli_executable(cli_name, &[]) {
        return (true, get_cli_version(&cli_path));
    }

    (false, None)
}

/// 获取 CLI 的安装��项
#[tauri::command]
pub fn get_cli_install_options(cli_name: String) -> Result<CliInstallerInfo, String> {
    let managers = detect_package_managers()?;
    let manager_map: HashMap<&str, bool> = managers
        .iter()
        .map(|m| (m.name.as_str(), m.available))
        .collect();

    let (installed, current_version) = detect_cli_installed(&cli_name);

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
                    command: "irm https://claude.ai/install.ps1 | iex".to_string(),
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
                    available: *manager_map.get("homebrew").unwrap_or(&false),
                    display_name: "Homebrew".to_string(),
                });
            }

            options.push(InstallOption {
                method: "npm".to_string(),
                command: "npm install -g @anthropic-ai/claude-code".to_string(),
                recommended: false,
                available: *manager_map.get("npm").unwrap_or(&false),
                display_name: "npm".to_string(),
            });
        }
        "codex" => {
            // npm - 推荐方式
            options.push(InstallOption {
                method: "npm".to_string(),
                command: "npm install -g @openai/codex".to_string(),
                recommended: true,
                available: *manager_map.get("npm").unwrap_or(&false),
                display_name: "npm".to_string(),
            });

            // Homebrew - macOS
            #[cfg(target_os = "macos")]
            {
                options.push(InstallOption {
                    method: "homebrew".to_string(),
                    command: "brew install codex".to_string(),
                    recommended: false,
                    available: *manager_map.get("homebrew").unwrap_or(&false),
                    display_name: "Homebrew".to_string(),
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
        _ => "",
    }
}

/// 获取 brew 包名
fn get_brew_package(cli_name: &str) -> &'static str {
    match cli_name {
        "claude" => "claude-code",
        "codex" => "codex",
        _ => "",
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

#[tauri::command]
pub async fn install_cli(cli_name: String, method: String, app: AppHandle) -> Result<(), String> {
    // 防止重复安装
    if INSTALLING.load(Ordering::SeqCst) {
        return Err("An installation is already in progress".to_string());
    }
    INSTALLING.store(true, Ordering::SeqCst);

    emit_log_event(
        &app,
        &cli_name,
        &format!("🚀 Starting installation of {}...", cli_name),
    );

    // 获取安装选项
    let options = get_cli_install_options(cli_name.clone())?;
    let option = options
        .install_options
        .iter()
        .find(|o| o.method == method)
        .ok_or("Installation method not found")?;

    if !option.available {
        INSTALLING.store(false, Ordering::SeqCst);
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

    INSTALLING.store(false, Ordering::SeqCst);

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
    _command: &str,
) -> Result<(), String> {
    let mut child = match method {
        "native" => {
            #[cfg(not(windows))]
            {
                create_command("bash")
                    .arg("-c")
                    .arg(_command)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
            }
            #[cfg(windows)]
            {
                create_command("powershell")
                    .arg("-Command")
                    .arg(_command)
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
            for line in reader.lines().filter_map(|l| l.ok()) {
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
            for line in reader.lines().filter_map(|l| l.ok()) {
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
    let (_, current) = detect_cli_installed(&cli_name);

    if current.is_none() {
        return Ok(VersionInfo {
            current: None,
            latest: None,
            has_update: false,
            release_notes: None,
        });
    }

    let latest = fetch_npm_version(cli_name.as_str()).await;

    let has_update = match (&current, &latest) {
        (Some(curr), Some(lat)) => curr != lat,
        _ => false,
    };

    Ok(VersionInfo {
        current,
        latest,
        has_update,
        release_notes: None,
    })
}

async fn fetch_npm_version(cli_name: &str) -> Option<String> {
    let url = match cli_name {
        "claude" => "https://registry.npmjs.org/@anthropic-ai/claude-code/latest",
        "codex" => "https://registry.npmjs.org/@openai/codex/latest",
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

/// 升级 CLI
#[tauri::command]
pub async fn upgrade_cli(cli_name: String, app: AppHandle) -> Result<(), String> {
    // 防止重复操作
    if INSTALLING.load(Ordering::SeqCst) {
        return Err("An installation is already in progress".to_string());
    }
    INSTALLING.store(true, Ordering::SeqCst);

    emit_log_event(
        &app,
        &cli_name,
        &format!("🔄 Starting upgrade of {}...", cli_name),
    );

    // 获取安装信息
    let install_info = get_cli_install_options(cli_name.clone())?;

    let method = install_info
        .install_options
        .iter()
        .filter(|o| o.available)
        .max_by_key(|o| o.recommended as i32)
        .map(|o| o.method.clone())
        .ok_or("No available upgrade method")?;

    let result = match method.as_str() {
        "npm" => {
            let package = get_npm_package(&cli_name);
            execute_upgrade_command(&app, &cli_name, "npm", &["update", "-g", package]).await
        }
        "homebrew" => {
            let package = get_brew_package(&cli_name);
            execute_upgrade_command(&app, &cli_name, "brew", &["upgrade", package]).await
        }
        _ => {
            emit_log_event(&app, &cli_name, "📝 Re-running native installer...");
            let option = install_info
                .install_options
                .iter()
                .find(|o| o.method == method)
                .unwrap();
            execute_install_command(&app, &cli_name, &method, &option.command)
        }
    };

    INSTALLING.store(false, Ordering::SeqCst);

    match result {
        Ok(_) => {
            emit_log_event(
                &app,
                &cli_name,
                &format!("�?{} upgraded successfully!", cli_name),
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
            emit_log_event(&app, &cli_name, &format!("�?Upgrade failed: {}", e));
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
            for line in reader.lines().filter_map(|l| l.ok()) {
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
            for line in reader.lines().filter_map(|l| l.ok()) {
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
    INSTALLING.store(false, Ordering::SeqCst);
    Ok(())
}
