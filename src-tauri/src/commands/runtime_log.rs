use crate::logging::{
    clear_crash_log, clear_runtime_log_files, get_runtime_log_summary, list_runtime_log_files,
    read_crash_log, read_runtime_log_file, write_crash_log, write_log, CrashLogStatus,
    RuntimeLogFileInfo, RuntimeLogReadResult, RuntimeLogSummary,
};

#[tauri::command]
pub fn get_runtime_log_summary_command() -> Result<RuntimeLogSummary, String> {
    get_runtime_log_summary().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_runtime_log_files_command() -> Result<Vec<RuntimeLogFileInfo>, String> {
    list_runtime_log_files().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn read_runtime_log_file_command(
    file_name: Option<String>,
    tail_lines: Option<usize>,
) -> Result<RuntimeLogReadResult, String> {
    read_runtime_log_file(file_name.as_deref(), tail_lines).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_runtime_log_files_command() -> Result<usize, String> {
    clear_runtime_log_files().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn write_runtime_log_command(
    level: String,
    target: String,
    message: String,
) -> Result<(), String> {
    write_log(level.trim(), target.trim(), message.trim());
    Ok(())
}

#[tauri::command]
pub fn read_crash_log_command() -> Result<CrashLogStatus, String> {
    read_crash_log().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn write_crash_log_command(
    source: String,
    message: String,
    stack_trace: Option<String>,
) -> Result<(), String> {
    write_crash_log(
        source.trim(),
        message.trim(),
        stack_trace.as_deref(),
    );
    Ok(())
}

#[tauri::command]
pub fn clear_crash_log_command() -> Result<bool, String> {
    clear_crash_log().map_err(|error| error.to_string())
}
