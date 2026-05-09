use anyhow::Result;
use chrono::{DateTime, Local};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static LOG_WRITE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static PANIC_HOOK_INSTALLED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

const LOG_FILE_PREFIX: &str = "app-";
const LOG_FILE_SUFFIX: &str = ".log";
const CRASH_LOG_FILE: &str = "crash.log";
const DEFAULT_TAIL_LINES: usize = 500;
const MAX_TAIL_LINES: usize = 5_000;
const TAIL_READ_CHUNK_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashLogEntry {
    pub timestamp: String,
    pub source: String,
    pub message: String,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashLogStatus {
    pub has_crash_log: bool,
    pub crash_log_path: Option<String>,
    pub entries: Vec<CrashLogEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogFileInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogSummary {
    pub log_dir: String,
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub latest_file: Option<RuntimeLogFileInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLogReadResult {
    pub file: RuntimeLogFileInfo,
    pub content: String,
    pub truncated: bool,
    pub line_count: usize,
}

fn normalize_tail_lines(tail_lines: Option<usize>) -> usize {
    tail_lines
        .unwrap_or(DEFAULT_TAIL_LINES)
        .clamp(1, MAX_TAIL_LINES)
}

fn read_log_tail(path: &Path, limit: usize) -> Result<(String, bool, usize)> {
    let mut file = fs::File::open(path)?;
    let file_size = file.metadata()?.len();
    if file_size == 0 {
        return Ok((String::new(), false, 0));
    }

    let mut offset = file_size;
    let mut newline_count = 0usize;
    let mut chunks: Vec<Vec<u8>> = Vec::new();

    while offset > 0 && newline_count <= limit {
        let read_size = usize::min(TAIL_READ_CHUNK_BYTES, offset as usize);
        offset -= read_size as u64;
        file.seek(SeekFrom::Start(offset))?;

        let mut chunk = vec![0u8; read_size];
        file.read_exact(&mut chunk)?;
        newline_count += chunk.iter().filter(|byte| **byte == b'\n').count();
        chunks.push(chunk);
    }

    let total_bytes = chunks.iter().map(|chunk| chunk.len()).sum();
    let mut combined = Vec::with_capacity(total_bytes);
    for chunk in chunks.iter().rev() {
        combined.extend_from_slice(chunk);
    }

    let decoded = String::from_utf8_lossy(&combined);
    let mut lines: Vec<&str> = decoded.lines().collect();
    let truncated = offset > 0 || lines.len() > limit;

    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }

    Ok((lines.join("\n"), truncated, lines.len()))
}

fn get_log_dir() -> Result<PathBuf> {
    let path = crate::commands::get_persistence_dir_path()?.join("logs");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn build_today_log_path() -> Result<PathBuf> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    Ok(get_log_dir()?.join(format!("{LOG_FILE_PREFIX}{date}{LOG_FILE_SUFFIX}")))
}

fn normalize_modified_at(metadata: &fs::Metadata) -> Option<String> {
    metadata
        .modified()
        .ok()
        .map(DateTime::<Local>::from)
        .map(|time| time.to_rfc3339())
}

fn build_file_info(path: &Path, metadata: &fs::Metadata) -> RuntimeLogFileInfo {
    RuntimeLogFileInfo {
        name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        path: path.to_string_lossy().to_string(),
        size_bytes: metadata.len(),
        modified_at: normalize_modified_at(metadata),
    }
}

fn sanitize_log_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') {
        return None;
    }
    if !trimmed.starts_with(LOG_FILE_PREFIX) || !trimmed.ends_with(LOG_FILE_SUFFIX) {
        return None;
    }
    Some(trimmed.to_string())
}

pub fn init_runtime_logging() -> Result<()> {
    let _ = get_log_dir()?;

    let mut installed = PANIC_HOOK_INSTALLED.lock().expect("panic hook poisoned");
    if !*installed {
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let payload = panic_info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| {
                    panic_info
                        .payload()
                        .downcast_ref::<String>()
                        .cloned()
                })
                .unwrap_or_else(|| "unknown panic payload".to_string());

            let location = panic_info
                .location()
                .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
                .unwrap_or_else(|| "unknown location".to_string());

            let full_message = format!("PANIC: {}\nLocation: {}", payload, location);

            write_log("ERROR", "panic", &full_message);
            write_crash_log("rust-panic", &full_message, None);
            previous_hook(panic_info);
        }));
        *installed = true;
    }

    write_log("INFO", "app", "runtime logging initialized");
    Ok(())
}

pub fn write_log(level: &str, target: &str, message: &str) {
    let Ok(path) = build_today_log_path() else {
        return;
    };

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let normalized = message
        .replace('\r', "\\r")
        .trim_end_matches('\n')
        .to_string();
    let line = format!("[{timestamp}] [{level}] [{target}] {normalized}\n");

    let _guard = LOG_WRITE_LOCK.lock().expect("log writer poisoned");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

pub fn list_runtime_log_files() -> Result<Vec<RuntimeLogFileInfo>> {
    let log_dir = get_log_dir()?;
    let mut files = Vec::new();

    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if sanitize_log_name(name).is_none() {
            continue;
        }

        let metadata = entry.metadata()?;
        files.push(build_file_info(&path, &metadata));
    }

    files.sort_by(|left, right| right.name.cmp(&left.name));
    Ok(files)
}

pub fn get_runtime_log_summary() -> Result<RuntimeLogSummary> {
    let files = list_runtime_log_files()?;
    let total_size_bytes = files.iter().map(|file| file.size_bytes).sum();
    Ok(RuntimeLogSummary {
        log_dir: get_log_dir()?.to_string_lossy().to_string(),
        file_count: files.len(),
        total_size_bytes,
        latest_file: files.first().cloned(),
    })
}

pub fn read_runtime_log_file(
    file_name: Option<&str>,
    tail_lines: Option<usize>,
) -> Result<RuntimeLogReadResult> {
    let files = list_runtime_log_files()?;
    let target_name = match file_name.and_then(sanitize_log_name) {
        Some(name) => name,
        None => files
            .first()
            .map(|file| file.name.clone())
            .ok_or_else(|| anyhow::anyhow!("No runtime log files found"))?,
    };

    let path = get_log_dir()?.join(&target_name);
    if !path.exists() {
        return Err(anyhow::anyhow!(
            "Runtime log file not found: {}",
            target_name
        ));
    }

    let metadata = fs::metadata(&path)?;
    let limit = normalize_tail_lines(tail_lines);
    let (content, truncated, line_count) = read_log_tail(&path, limit)?;

    Ok(RuntimeLogReadResult {
        file: build_file_info(&path, &metadata),
        content,
        truncated,
        line_count,
    })
}

pub fn clear_runtime_log_files() -> Result<usize> {
    let files = list_runtime_log_files()?;
    let mut removed = 0usize;

    for file in files {
        if fs::remove_file(&file.path).is_ok() {
            removed += 1;
        }
    }

    Ok(removed)
}

fn get_crash_log_path() -> Result<PathBuf> {
    Ok(get_log_dir()?.join(CRASH_LOG_FILE))
}

pub fn write_crash_log(source: &str, message: &str, stack_trace: Option<&str>) {
    let Ok(path) = get_crash_log_path() else {
        return;
    };

    let timestamp = Local::now().to_rfc3339();
    let trace_section = stack_trace
        .map(|trace| format!("\n[stack]\n{}", trace))
        .unwrap_or_default();
    let entry = format!(
        "--- CRASH ---\n[time] {}\n[source] {}\n[message] {}{}\n--- END ---\n\n",
        timestamp, source, message, trace_section
    );

    let _guard = LOG_WRITE_LOCK.lock().expect("log writer poisoned");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(entry.as_bytes());
    }
}

pub fn read_crash_log() -> Result<CrashLogStatus> {
    let path = get_crash_log_path()?;
    if !path.exists() {
        return Ok(CrashLogStatus {
            has_crash_log: false,
            crash_log_path: None,
            entries: Vec::new(),
        });
    }

    let content = fs::read_to_string(&path)?;
    let mut entries = Vec::new();

    for block in content.split("--- CRASH ---") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }

        let mut timestamp = String::new();
        let mut source = String::new();
        let mut message = String::new();
        let mut stack_trace: Option<String> = None;

        let mut in_stack = false;
        let mut stack_lines: Vec<String> = Vec::new();

        for line in block.lines() {
            let line = line.trim();
            if line == "--- END ---" {
                break;
            }
            if line == "[stack]" {
                in_stack = true;
                continue;
            }
            if in_stack {
                stack_lines.push(line.to_string());
                continue;
            }
            if let Some(rest) = line.strip_prefix("[time] ") {
                timestamp = rest.to_string();
            } else if let Some(rest) = line.strip_prefix("[source] ") {
                source = rest.to_string();
            } else if let Some(rest) = line.strip_prefix("[message] ") {
                message = rest.to_string();
            }
        }

        if !stack_lines.is_empty() {
            stack_trace = Some(stack_lines.join("\n"));
        }

        if !message.is_empty() {
            entries.push(CrashLogEntry {
                timestamp,
                source,
                message,
                stack_trace,
            });
        }
    }

    Ok(CrashLogStatus {
        has_crash_log: true,
        crash_log_path: Some(path.to_string_lossy().to_string()),
        entries,
    })
}

pub fn clear_crash_log() -> Result<bool> {
    let path = get_crash_log_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
