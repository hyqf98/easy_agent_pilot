use crate::logging::{
    clear_runtime_log_files, get_runtime_log_summary, list_runtime_log_files,
    read_runtime_log_file, RuntimeLogFileInfo, RuntimeLogReadResult, RuntimeLogSummary,
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
