use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

use super::conversation::executor::{get_registry, is_execution_session_active_internal};
use super::conversation::set_abort_flag;
use super::conversation::strategies::abnormal_completion::{
    classify_cli_completion, CliCompletionFailureKind, CliTextFragment, CliTextSource,
};
use super::conversation::types::{ExecutionRequest, MessageInput};
use super::message::MessageAttachment;
use super::support::{now_rfc3339, open_db_connection};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSplitMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub attachments: Option<Vec<MessageAttachment>>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSplitSession {
    pub id: String,
    pub plan_id: String,
    pub status: String,
    pub execution_session_id: Option<String>,
    pub raw_content: Option<String>,
    pub result_json: Option<String>,
    pub parse_error: Option<String>,
    pub error_message: Option<String>,
    pub granularity: i32,
    pub task_count_mode: String,
    pub llm_messages_json: Option<String>,
    pub messages_json: Option<String>,
    pub execution_request_json: Option<String>,
    pub form_queue_json: Option<String>,
    pub current_form_index: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub stopped_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSplitLog {
    pub id: String,
    pub plan_id: String,
    pub session_id: String,
    #[serde(rename = "type")]
    pub log_type: String,
    pub content: String,
    pub metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPlanSplitInput {
    pub plan_id: String,
    pub granularity: i32,
    pub task_count_mode: Option<String>,
    pub execution_request: ExecutionRequest,
    pub llm_messages: Vec<MessageInput>,
    pub messages: Vec<PlanSplitMessage>,
    pub preserve_result: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitPlanSplitFormInput {
    pub plan_id: String,
    pub form_id: String,
    pub values: Value,
    pub display_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSplitStreamPayload {
    #[serde(rename = "type")]
    pub event_type: String,
    pub plan_id: String,
    pub session_id: String,
    pub content: Option<String>,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_input: Option<String>,
    pub tool_result: Option<String>,
    pub error: Option<String>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub model: Option<String>,
    pub external_session_id: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResetPlanSplitTurnOptions {
    pub preserve_result: Option<bool>,
    pub trim_latest_turn: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SplitStreamRecord {
    pub event_type: String,
    pub content: Option<String>,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_input: Option<String>,
    pub tool_result: Option<String>,
    pub error: Option<String>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub model: Option<String>,
    pub external_session_id: Option<String>,
}

#[derive(Debug, Clone)]
enum ParsedSplitOutput {
    FormRequest { question: String, forms: Vec<Value> },
    TaskSplit { tasks: Vec<Value> },
}

fn is_terminal_session_status(status: &str) -> bool {
    matches!(status, "waiting_input" | "completed" | "stopped" | "failed")
}

fn map_plan_split_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlanSplitSession> {
    Ok(PlanSplitSession {
        id: row.get(0)?,
        plan_id: row.get(1)?,
        status: row.get(2)?,
        execution_session_id: row.get(3)?,
        raw_content: row.get(4)?,
        result_json: row.get(5)?,
        parse_error: row.get(6)?,
        error_message: row.get(7)?,
        granularity: row.get(8)?,
        task_count_mode: row.get(9)?,
        llm_messages_json: row.get(10)?,
        messages_json: row.get(11)?,
        execution_request_json: row.get(12)?,
        form_queue_json: row.get(13)?,
        current_form_index: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        started_at: row.get(17)?,
        completed_at: row.get(18)?,
        stopped_at: row.get(19)?,
    })
}

fn parse_json_vec<T: DeserializeOwned>(raw: Option<&String>) -> Vec<T> {
    raw.and_then(|value| serde_json::from_str::<Vec<T>>(value).ok())
        .unwrap_or_default()
}

fn read_session(
    conn: &rusqlite::Connection,
    plan_id: &str,
) -> Result<Option<PlanSplitSession>, String> {
    let result = conn.query_row(
        "SELECT id, plan_id, status, execution_session_id, raw_content, parsed_output,
                parse_error, error_message, granularity, task_count_mode, llm_messages_json, messages_json,
                execution_request_json, form_queue_json, current_form_index,
                created_at, updated_at, started_at, completed_at, stopped_at
         FROM task_split_sessions WHERE plan_id = ?1",
        [&plan_id],
        map_plan_split_session,
    );

    match result {
        Ok(session) => Ok(Some(session)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn insert_or_update_session(
    conn: &rusqlite::Connection,
    session: &PlanSplitSession,
) -> Result<(), String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM task_split_sessions WHERE plan_id = ?1",
            [&session.plan_id],
            |row| row.get(0),
        )
        .ok();

    if let Some(existing_id) = existing {
        conn.execute(
            "UPDATE task_split_sessions
             SET status = ?1,
                 execution_session_id = ?2,
                 raw_content = ?3,
                 parsed_output = ?4,
                 parse_error = ?5,
                 error_message = ?6,
                 granularity = ?7,
                 task_count_mode = ?8,
                 llm_messages_json = ?9,
                 messages_json = ?10,
                 execution_request_json = ?11,
                 form_queue_json = ?12,
                 current_form_index = ?13,
                 updated_at = ?14,
                 started_at = ?15,
                 completed_at = ?16,
                 stopped_at = ?17
             WHERE id = ?18",
            rusqlite::params![
                &session.status,
                &session.execution_session_id,
                &session.raw_content,
                &session.result_json,
                &session.parse_error,
                &session.error_message,
                &session.granularity,
                &session.task_count_mode,
                &session.llm_messages_json,
                &session.messages_json,
                &session.execution_request_json,
                &session.form_queue_json,
                &session.current_form_index,
                &session.updated_at,
                &session.started_at,
                &session.completed_at,
                &session.stopped_at,
                &existing_id
            ],
        )
        .map_err(|error| error.to_string())?;
    } else {
        conn.execute(
            "INSERT INTO task_split_sessions
             (id, plan_id, status, execution_session_id, raw_content, parsed_output, parse_error,
              error_message, granularity, task_count_mode, llm_messages_json, messages_json, execution_request_json,
              form_queue_json, current_form_index, created_at, updated_at, started_at, completed_at, stopped_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            rusqlite::params![
                &session.id,
                &session.plan_id,
                &session.status,
                &session.execution_session_id,
                &session.raw_content,
                &session.result_json,
                &session.parse_error,
                &session.error_message,
                &session.granularity,
                &session.task_count_mode,
                &session.llm_messages_json,
                &session.messages_json,
                &session.execution_request_json,
                &session.form_queue_json,
                &session.current_form_index,
                &session.created_at,
                &session.updated_at,
                &session.started_at,
                &session.completed_at,
                &session.stopped_at
            ],
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn normalize_task_count_mode(value: Option<&str>) -> String {
    match value.map(str::trim).filter(|mode| !mode.is_empty()) {
        Some(mode) if mode.eq_ignore_ascii_case("exact") => "exact".to_string(),
        _ => "min".to_string(),
    }
}

fn task_count_mode_is_exact(value: &str) -> bool {
    value.eq_ignore_ascii_case("exact")
}

fn load_messages_json(raw: Option<&String>) -> Vec<PlanSplitMessage> {
    parse_json_vec(raw)
}

fn load_llm_messages_json(raw: Option<&String>) -> Vec<MessageInput> {
    parse_json_vec(raw)
}

fn load_form_queue(raw: Option<&String>) -> Vec<Value> {
    parse_json_vec(raw)
}

fn latest_user_message_timestamp(messages: &[PlanSplitMessage]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.timestamp.clone())
}

fn parse_rfc3339_millis(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|datetime| datetime.timestamp_millis())
}

fn delete_plan_split_logs_from_timestamp(
    conn: &rusqlite::Connection,
    plan_id: &str,
    timestamp: Option<&str>,
) -> Result<(), String> {
    let Some(boundary_ms) = timestamp.and_then(parse_rfc3339_millis) else {
        return Ok(());
    };

    let mut stmt = conn
        .prepare("SELECT id, created_at FROM plan_split_logs WHERE plan_id = ?1")
        .map_err(|error| error.to_string())?;
    let log_ids = stmt
        .query_map([plan_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?
        .filter_map(|row| row.ok())
        .filter_map(|(id, created_at)| {
            let created_ms = parse_rfc3339_millis(&created_at)?;
            (created_ms >= boundary_ms).then_some(id)
        })
        .collect::<Vec<_>>();

    for id in log_ids {
        conn.execute(
            "DELETE FROM plan_split_logs WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn trim_messages_after_last_user<T, F>(messages: &mut Vec<T>, get_role: F)
where
    F: Fn(&T) -> &str,
{
    let Some(last_user_index) = messages
        .iter()
        .rposition(|message| get_role(message) == "user")
    else {
        return;
    };

    while messages.len() > last_user_index + 1 {
        let should_remove = messages
            .last()
            .map(|message| get_role(message) == "assistant")
            .unwrap_or(false);
        if !should_remove {
            break;
        }
        messages.pop();
    }
}

fn emit_session_updated(app: &AppHandle, session: &PlanSplitSession) {
    let event_name = format!("plan-split-stream-{}", session.plan_id);
    let payload = serde_json::json!({
        "type": "session_updated",
        "planId": session.plan_id,
        "session": session
    });
    let _ = app.emit(&event_name, payload);
}

fn append_ui_message(messages: &mut Vec<PlanSplitMessage>, role: &str, content: String) {
    messages.push(PlanSplitMessage {
        id: uuid::Uuid::new_v4().to_string(),
        role: role.to_string(),
        content,
        attachments: None,
        timestamp: now_rfc3339(),
    });
}

fn build_form_response_prompt(form_id: &str, values: &Value) -> String {
    let serialized = match values {
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| {
                if value.is_object() || value.is_array() {
                    format!(
                        "{key}: {}",
                        serde_json::to_string(value).unwrap_or_default()
                    )
                } else if let Some(text) = value.as_str() {
                    format!("{key}: {text}")
                } else {
                    format!("{key}: {value}")
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => values.to_string(),
    };

    format!(
        "表单 {form_id} 用户回答: {serialized}\n\n继续：需要更多信息则输出 form_request；信息足够则输出 task_split（status=DONE）。"
    )
}

fn extract_assistant_summary(output: &ParsedSplitOutput) -> String {
    match output {
        ParsedSplitOutput::FormRequest { question, forms } => {
            format!("[AI提问] {}（共 {} 个表单）", question, forms.len())
        }
        ParsedSplitOutput::TaskSplit { tasks } => {
            format!("[AI完成任务拆分] 共 {} 个任务", tasks.len())
        }
    }
}

fn extract_json_candidates(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();
    results.push(trimmed.to_string());
    if let Some(fenced) = extract_fenced_json(trimmed) {
        results.push(fenced);
    }

    let mut start = None;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in trimmed.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == '{' {
            if depth == 0 {
                start = Some(index);
            }
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                if let Some(start_index) = start.take() {
                    results.push(trimmed[start_index..=index].to_string());
                }
            }
        }
    }

    results.reverse();
    results.dedup();
    results
}

fn extract_fenced_json(raw: &str) -> Option<String> {
    let fenced_start = raw.find("```")?;
    let after_start = &raw[fenced_start + 3..];
    let content_start = after_start.find('\n').map(|index| index + 1).unwrap_or(0);
    let after_language = &after_start[content_start..];
    let fenced_end = after_language.find("```")?;
    let content = after_language[..fenced_end].trim();
    (!content.is_empty()).then(|| content.to_string())
}

fn repair_json_candidate(raw: &str) -> String {
    let normalized = raw
        .replace(['“', '”'], "\"")
        .replace(['‘', '’'], "'")
        .replace('：', ":")
        .replace('，', ",")
        .replace('（', "(")
        .replace('）', ")")
        .replace('【', "[")
        .replace('】', "]")
        .replace('｛', "{")
        .replace('｝', "}")
        .replace('［', "[")
        .replace('］', "]");
    let mut repaired = String::with_capacity(normalized.len() + 8);
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in normalized.chars() {
        if in_string {
            repaired.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            repaired.push(ch);
            continue;
        }

        if ch == '{' || ch == '[' {
            stack.push(ch);
            repaired.push(ch);
            continue;
        }

        if ch == '}' || ch == ']' {
            let expected = if ch == '}' { '{' } else { '[' };
            if stack.last().copied() == Some(expected) {
                stack.pop();
                repaired.push(ch);
            }
            continue;
        }

        repaired.push(ch);
    }

    while let Some(open) = stack.pop() {
        repaired.push(if open == '{' { '}' } else { ']' });
    }

    repaired
}

fn as_non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_priority(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => {
            let normalized = text.trim().to_lowercase();
            match normalized.as_str() {
                "high" | "高" | "高优先级" => Some("high".to_string()),
                "medium" | "中" | "中优先级" => Some("medium".to_string()),
                "low" | "低" | "低优先级" => Some("low".to_string()),
                "1" => Some("high".to_string()),
                "2" => Some("medium".to_string()),
                "3" => Some("low".to_string()),
                _ => None,
            }
        }
        Value::Number(number) => match number.as_i64()? {
            1 => Some("high".to_string()),
            2 => Some("medium".to_string()),
            3 => Some("low".to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn as_step_text_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|item| item.as_array())
        .map(|items| {
            items
                .iter()
                .flat_map(|item| match item {
                    Value::String(text) => text
                        .trim()
                        .split('\n')
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>(),
                    Value::Object(map) => {
                        let mut lines = Vec::new();
                        if let Some(precondition) = as_non_empty_string(
                            map.get("precondition").or_else(|| map.get("condition")),
                        ) {
                            lines.push(format!("前置条件：{precondition}"));
                        }
                        if let Some(steps) = map.get("steps").and_then(|value| value.as_array()) {
                            lines.extend(steps.iter().filter_map(|step| {
                                step.as_str()
                                    .map(str::trim)
                                    .filter(|text| !text.is_empty())
                                    .map(ToOwned::to_owned)
                            }));
                        }
                        if let Some(expected_results) = map
                            .get("expectedResults")
                            .or_else(|| map.get("expected_results"))
                            .and_then(|value| value.as_array())
                        {
                            lines.extend(expected_results.iter().filter_map(|result| {
                                result
                                    .as_str()
                                    .map(str::trim)
                                    .filter(|text| !text.is_empty())
                                    .map(|text| format!("预期：{text}"))
                            }));
                        }
                        if lines.is_empty() {
                            if let Some(text) = as_non_empty_string(
                                map.get("content")
                                    .or_else(|| map.get("description"))
                                    .or_else(|| map.get("title"))
                                    .or_else(|| map.get("label")),
                            ) {
                                lines.push(text);
                            }
                        }
                        if lines.is_empty() {
                            if let Ok(serialized) = serde_json::to_string(item) {
                                lines.push(serialized);
                            }
                        }
                        lines
                    }
                    _ => {
                        let text = item.to_string();
                        if text.trim().is_empty() {
                            Vec::new()
                        } else {
                            vec![text]
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn normalize_task(task: &Value) -> Result<Value, String> {
    let task_obj = task
        .as_object()
        .ok_or_else(|| "tasks 中存在无效任务对象。".to_string())?;
    let title =
        as_non_empty_string(task_obj.get("title")).ok_or_else(|| "任务缺少 title。".to_string())?;
    let description = as_non_empty_string(task_obj.get("description"))
        .ok_or_else(|| "任务缺少 description。".to_string())?;
    let priority =
        normalize_priority(task_obj.get("priority")).unwrap_or_else(|| "medium".to_string());
    let expert_id = as_non_empty_string(
        task_obj
            .get("expertId")
            .or_else(|| task_obj.get("expert_id")),
    )
    .ok_or_else(|| "任务缺少 expertId。".to_string())?;
    let implementation_steps = as_step_text_array(
        task_obj
            .get("implementationSteps")
            .or_else(|| task_obj.get("implementation_steps"))
            .or_else(|| task_obj.get("steps")),
    );
    let test_steps = as_step_text_array(
        task_obj
            .get("testSteps")
            .or_else(|| task_obj.get("test_steps"))
            .or_else(|| task_obj.get("testingSteps"))
            .or_else(|| task_obj.get("testing_steps")),
    );
    let acceptance_criteria = as_step_text_array(
        task_obj
            .get("acceptanceCriteria")
            .or_else(|| task_obj.get("acceptance_criteria")),
    );
    if implementation_steps.is_empty() || test_steps.is_empty() || acceptance_criteria.is_empty() {
        return Err(
            "任务步骤字段不能为空（implementationSteps/testSteps/acceptanceCriteria）。"
                .to_string(),
        );
    }

    let mut normalized = serde_json::Map::new();
    normalized.insert("title".to_string(), Value::String(title));
    normalized.insert("description".to_string(), Value::String(description));
    normalized.insert("priority".to_string(), Value::String(priority));
    normalized.insert("expertId".to_string(), Value::String(expert_id));
    normalized.insert(
        "implementationSteps".to_string(),
        Value::Array(
            implementation_steps
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    normalized.insert(
        "testSteps".to_string(),
        Value::Array(test_steps.into_iter().map(Value::String).collect()),
    );
    normalized.insert(
        "acceptanceCriteria".to_string(),
        Value::Array(acceptance_criteria.into_iter().map(Value::String).collect()),
    );
    if let Some(agent_id) =
        as_non_empty_string(task_obj.get("agentId").or_else(|| task_obj.get("agent_id")))
    {
        normalized.insert("agentId".to_string(), Value::String(agent_id));
    }
    if let Some(model_id) =
        as_non_empty_string(task_obj.get("modelId").or_else(|| task_obj.get("model_id")))
    {
        normalized.insert("modelId".to_string(), Value::String(model_id));
    }
    if let Some(depends_on) = task_obj
        .get("dependsOn")
        .or_else(|| task_obj.get("depends_on"))
        .and_then(|value| value.as_array())
    {
        normalized.insert("dependsOn".to_string(), Value::Array(depends_on.clone()));
    }
    Ok(Value::Object(normalized))
}

fn normalize_form_schema(schema: &Value) -> Option<Value> {
    let schema_obj = schema.as_object()?;
    let form_id = as_non_empty_string(
        schema_obj
            .get("formId")
            .or_else(|| schema_obj.get("form_id")),
    )?;
    let title = as_non_empty_string(schema_obj.get("title"))?;
    let fields = schema_obj.get("fields")?.as_array()?.clone();
    if fields.is_empty() {
        return None;
    }

    let mut normalized = serde_json::Map::new();
    normalized.insert("formId".to_string(), Value::String(form_id));
    normalized.insert("title".to_string(), Value::String(title));
    if let Some(description) = schema_obj
        .get("description")
        .and_then(|value| value.as_str())
    {
        normalized.insert(
            "description".to_string(),
            Value::String(description.to_string()),
        );
    }
    if let Some(submit_text) = schema_obj
        .get("submitText")
        .or_else(|| schema_obj.get("submit_text"))
        .and_then(|value| value.as_str())
    {
        normalized.insert(
            "submitText".to_string(),
            Value::String(submit_text.to_string()),
        );
    }
    normalized.insert("fields".to_string(), Value::Array(fields));
    Some(Value::Object(normalized))
}

fn parse_split_output(
    raw_content: &str,
    min_task_count: i32,
    task_count_mode: &str,
) -> Result<ParsedSplitOutput, String> {
    for candidate in extract_json_candidates(raw_content) {
        let parsed: Value = match serde_json::from_str(&candidate) {
            Ok(value) => value,
            Err(_) => {
                let repaired = repair_json_candidate(&candidate);
                if repaired == candidate {
                    continue;
                }
                match serde_json::from_str(&repaired) {
                    Ok(value) => value,
                    Err(_) => continue,
                }
            }
        };
        if let Some(result_string) = parsed.get("result").and_then(|value| value.as_str()) {
            if let Ok(output) = parse_split_output(result_string, min_task_count, task_count_mode) {
                return Ok(output);
            }
        }
        let record = match parsed.as_object() {
            Some(object) => object,
            None => continue,
        };
        let output_type = as_non_empty_string(record.get("type"))
            .unwrap_or_default()
            .to_lowercase();
        let has_forms = record
            .get("forms")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(normalize_form_schema)
                    .collect::<Vec<_>>()
            })
            .filter(|forms| !forms.is_empty())
            .or_else(|| {
                record
                    .get("formSchema")
                    .or_else(|| record.get("form_schema"))
                    .or_else(|| record.get("schema"))
                    .and_then(normalize_form_schema)
                    .map(|schema| vec![schema])
            });
        let is_form_request = output_type == "form_request"
            || (output_type.is_empty()
                && has_forms.is_some()
                && record.get("question").and_then(|v| v.as_str()).is_some());
        if is_form_request {
            let forms = has_forms.unwrap_or_default();
            if forms.is_empty() {
                return Err("form_request 缺少有效 forms。".to_string());
            }
            let question = as_non_empty_string(record.get("question")).unwrap_or_else(|| {
                forms
                    .first()
                    .and_then(|form| form.get("title").and_then(|v| v.as_str()))
                    .unwrap_or("请填写以下信息")
                    .to_string()
            });
            return Ok(ParsedSplitOutput::FormRequest { question, forms });
        }

        let is_done = record
            .get("done")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            || as_non_empty_string(record.get("status"))
                .map(|value| value.to_uppercase() == "DONE")
                .unwrap_or(false);

        if output_type == "task_split" || (is_done && record.get("tasks").is_some()) {
            if !is_done {
                return Err("task_split 必须包含 status: DONE。".to_string());
            }
            let tasks_raw = record
                .get("tasks")
                .and_then(|value| value.as_array())
                .ok_or_else(|| "task_split 缺少 tasks 数组。".to_string())?;
            let expected_count = min_task_count.max(1) as usize;
            if task_count_mode_is_exact(task_count_mode) {
                if tasks_raw.len() != expected_count {
                    return Err(format!(
                        "拆分任务数量不匹配，必须严格返回 {} 个。",
                        expected_count
                    ));
                }
            } else if tasks_raw.len() < expected_count {
                return Err(format!(
                    "拆分任务数量不足，至少需要 {} 个。",
                    expected_count
                ));
            }
            let mut tasks = Vec::with_capacity(tasks_raw.len());
            for task in tasks_raw {
                tasks.push(normalize_task(task)?);
            }
            return Ok(ParsedSplitOutput::TaskSplit { tasks });
        }
    }

    Err("无法解析为有效的 JSON 输出。".to_string())
}

fn load_content_logs(conn: &rusqlite::Connection, session_id: &str) -> Result<String, String> {
    let mut stmt = conn
        .prepare(
            "SELECT content FROM plan_split_logs
             WHERE session_id = ?1 AND log_type = 'content'
             ORDER BY created_at ASC",
        )
        .map_err(|error| error.to_string())?;

    let chunks = stmt
        .query_map([session_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    Ok(chunks.join(""))
}

fn load_structured_output_logs(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT log_type, content, metadata FROM plan_split_logs
             WHERE session_id = ?1
               AND log_type IN ('tool_use', 'tool_input_delta', 'tool_result')
             ORDER BY created_at ASC",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([session_id], |row| {
            let log_type: String = row.get(0)?;
            let content: String = row.get(1)?;
            let metadata: Option<String> = row.get(2)?;
            Ok((log_type, content, metadata))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let mut outputs = Vec::new();
    let mut structured_tool_call_ids = HashSet::new();
    let mut structured_tool_call_order = Vec::new();
    let mut structured_tool_input_chunks = HashMap::<String, String>::new();

    for (log_type, content, metadata) in rows {
        let metadata_value = metadata
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok());
        let tool_name = metadata_value
            .as_ref()
            .and_then(|value| {
                as_non_empty_string(value.get("toolName").or_else(|| value.get("tool_name")))
            })
            .unwrap_or_default();
        let tool_call_id = metadata_value.as_ref().and_then(|value| {
            as_non_empty_string(
                value
                    .get("toolCallId")
                    .or_else(|| value.get("tool_call_id")),
            )
        });

        if log_type == "tool_use"
            && (tool_name.eq_ignore_ascii_case("StructuredOutput")
                || tool_name.eq_ignore_ascii_case("structured_output"))
        {
            if let Some(tool_call_id) = tool_call_id.clone() {
                if structured_tool_call_ids.insert(tool_call_id.clone()) {
                    structured_tool_call_order.push(tool_call_id);
                }
            }

            let candidate = metadata_value
                .as_ref()
                .and_then(|value| {
                    as_non_empty_string(value.get("toolInput").or_else(|| value.get("tool_input")))
                })
                .or_else(|| {
                    let trimmed = content.trim();
                    (!trimmed.is_empty()).then(|| trimmed.to_string())
                });
            if let Some(candidate) = candidate {
                outputs.push(candidate);
            }

            continue;
        }

        let Some(tool_call_id) = tool_call_id else {
            continue;
        };
        if !structured_tool_call_ids.contains(&tool_call_id) {
            continue;
        }

        match log_type.as_str() {
            "tool_input_delta" => {
                let chunk = if content.trim().is_empty() {
                    metadata_value
                        .as_ref()
                        .and_then(|value| {
                            as_non_empty_string(
                                value.get("toolInput").or_else(|| value.get("tool_input")),
                            )
                        })
                        .unwrap_or_default()
                } else {
                    content
                };

                if !chunk.trim().is_empty() {
                    structured_tool_input_chunks
                        .entry(tool_call_id)
                        .or_default()
                        .push_str(&chunk);
                }
            }
            "tool_result" => {
                let candidate = metadata_value
                    .as_ref()
                    .and_then(|value| {
                        as_non_empty_string(
                            value.get("toolResult").or_else(|| value.get("tool_result")),
                        )
                    })
                    .or_else(|| {
                        let trimmed = content.trim();
                        (!trimmed.is_empty()).then(|| trimmed.to_string())
                    });
                if let Some(candidate) = candidate {
                    outputs.push(candidate);
                }
            }
            _ => {}
        }
    }

    for tool_call_id in structured_tool_call_order {
        let Some(candidate) = structured_tool_input_chunks.remove(&tool_call_id) else {
            continue;
        };
        let trimmed = candidate.trim();
        if !trimmed.is_empty() {
            outputs.push(trimmed.to_string());
        }
    }

    Ok(outputs)
}

fn serialize_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}

fn apply_external_session_id_to_request(
    session: &mut PlanSplitSession,
    external_session_id: Option<&str>,
) -> Result<bool, String> {
    let Some(external_session_id) = external_session_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };

    let Some(raw_request) = session.execution_request_json.as_ref() else {
        return Ok(false);
    };
    let mut request = match serde_json::from_str::<ExecutionRequest>(raw_request) {
        Ok(request) => request,
        Err(_) => return Ok(false),
    };

    if request.agent_type != "cli" {
        return Ok(false);
    }
    if request.resume_session_id.as_deref() == Some(external_session_id) {
        return Ok(false);
    }

    request.resume_session_id = Some(external_session_id.to_string());
    session.execution_request_json = Some(serialize_json(&request)?);
    Ok(true)
}

fn apply_parsed_output_to_session(
    session: &mut PlanSplitSession,
    session_id: &str,
    output: ParsedSplitOutput,
    raw_content: String,
) -> Result<(), String> {
    let now = now_rfc3339();
    session.raw_content = Some(raw_content.clone());
    session.updated_at = now.clone();
    session.completed_at = Some(now);
    session.execution_session_id = Some(session_id.to_string());
    session.parse_error = None;
    session.error_message = None;
    session.stopped_at = None;

    let mut llm_messages = load_llm_messages_json(session.llm_messages_json.as_ref());
    let mut messages = load_messages_json(session.messages_json.as_ref());

    append_ui_message(
        &mut messages,
        "assistant",
        match &output {
            ParsedSplitOutput::FormRequest { question, .. } => question.clone(),
            ParsedSplitOutput::TaskSplit { .. } => raw_content.clone(),
        },
    );
    llm_messages.push(MessageInput {
        role: "assistant".to_string(),
        content: extract_assistant_summary(&output),
        attachments: None,
    });
    session.llm_messages_json = Some(serialize_json(&llm_messages)?);
    session.messages_json = Some(serialize_json(&messages)?);

    match output {
        ParsedSplitOutput::FormRequest { forms, .. } => {
            session.status = "waiting_input".to_string();
            session.form_queue_json = Some(serialize_json(&forms)?);
            session.current_form_index = Some(0);
        }
        ParsedSplitOutput::TaskSplit { tasks } => {
            session.status = "completed".to_string();
            session.form_queue_json = None;
            session.current_form_index = None;
            session.result_json = Some(serialize_json(&serde_json::json!({
                "type": "task_split",
                "status": "DONE",
                "tasks": tasks,
            }))?);
        }
    }

    Ok(())
}

fn extract_structured_output_payload(event: &SplitStreamRecord) -> Option<String> {
    if event.event_type != "tool_use" {
        return None;
    }

    let tool_name = event.tool_name.as_deref()?;
    if !tool_name.eq_ignore_ascii_case("StructuredOutput")
        && !tool_name.eq_ignore_ascii_case("structured_output")
    {
        return None;
    }

    event
        .tool_input
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            event
                .content
                .clone()
                .filter(|value| !value.trim().is_empty())
        })
}

fn finalize_session_from_structured_output(
    app: &AppHandle,
    plan_id: &str,
    session_id: &str,
    raw_output: &str,
) -> Result<bool, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let Some(mut session) = read_session(&conn, plan_id)? else {
        return Ok(false);
    };

    if session.execution_session_id.as_deref() != Some(session_id)
        || is_terminal_session_status(&session.status)
    {
        return Ok(false);
    }

    let output = match parse_split_output(raw_output, session.granularity, &session.task_count_mode)
    {
        Ok(output) => output,
        Err(_) => return Ok(false),
    };

    apply_parsed_output_to_session(&mut session, session_id, output, raw_output.to_string())?;
    insert_or_update_session(&conn, &session)?;
    emit_session_updated(app, &session);

    let session_id = session_id.to_string();
    tauri::async_runtime::spawn(async move {
        set_abort_flag(&session_id, true).await;
    });

    Ok(true)
}

fn refresh_session_after_turn(
    app: &AppHandle,
    plan_id: &str,
    session_id: &str,
) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut session =
        read_session(&conn, plan_id)?.ok_or_else(|| "计划拆分会话不存在".to_string())?;
    if session.status == "stopped" {
        emit_session_updated(app, &session);
        return Ok(());
    }

    let raw_content = load_content_logs(&conn, session_id)?;
    let parsed =
        match parse_split_output(&raw_content, session.granularity, &session.task_count_mode) {
            Ok(output) => Ok((output, raw_content.clone())),
            Err(content_error) => {
                let structured_outputs = load_structured_output_logs(&conn, session_id)?;
                let parsed_from_tool = structured_outputs.iter().rev().find_map(|candidate| {
                    parse_split_output(candidate, session.granularity, &session.task_count_mode)
                        .ok()
                        .map(|output| (output, candidate.clone()))
                });

                parsed_from_tool.ok_or(content_error)
            }
        };
    let now = now_rfc3339();
    session.raw_content = Some(raw_content.clone());
    session.updated_at = now.clone();
    session.completed_at = Some(now);
    session.execution_session_id = Some(session_id.to_string());
    session.parse_error = None;
    session.error_message = None;

    match parsed {
        Ok((output, parsed_raw_content)) => {
            apply_parsed_output_to_session(&mut session, session_id, output, parsed_raw_content)?;
        }
        Err(error_message) => {
            let llm_messages = load_llm_messages_json(session.llm_messages_json.as_ref());
            let failure = CliTextFragment::new(CliTextSource::Content, raw_content.clone())
                .into_iter()
                .collect::<Vec<_>>();
            let classified_failure = classify_cli_completion("Plan Split", &failure, false);

            let is_retryable_failure = matches!(
                classified_failure.as_ref().map(|item| item.kind),
                Some(CliCompletionFailureKind::Retryable)
            );
            session.status = if is_retryable_failure {
                "failed".to_string()
            } else {
                "stopped".to_string()
            };
            session.parse_error = Some(error_message.clone());
            session.error_message = classified_failure.as_ref().map(|item| item.message.clone());
            session.llm_messages_json = Some(serialize_json(&llm_messages)?);
            session.form_queue_json = None;
            session.current_form_index = None;
            session.stopped_at = if is_retryable_failure {
                None
            } else {
                session.completed_at.clone()
            };
        }
    }

    insert_or_update_session(&conn, &session)?;
    emit_session_updated(app, &session);
    Ok(())
}

pub fn record_plan_split_event(
    app: &AppHandle,
    plan_id: &str,
    session_id: &str,
    event: SplitStreamRecord,
) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let Some(mut session) = read_session(&conn, plan_id)? else {
        return Ok(());
    };

    if session.execution_session_id.as_deref() != Some(session_id)
        || is_terminal_session_status(&session.status)
    {
        return Ok(());
    }

    let now = now_rfc3339();
    let structured_output = extract_structured_output_payload(&event);
    let metadata = serde_json::to_string(&serde_json::json!({
        "toolName": event.tool_name.clone(),
        "toolCallId": event.tool_call_id.clone(),
        "toolInput": event.tool_input.clone(),
        "toolResult": event.tool_result.clone(),
        "error": event.error.clone(),
        "inputTokens": event.input_tokens,
        "outputTokens": event.output_tokens,
        "model": event.model.clone(),
        "externalSessionId": event.external_session_id.clone(),
    }))
    .ok();
    let content = event
        .content
        .clone()
        .or_else(|| event.error.clone())
        .or_else(|| event.tool_result.clone())
        .or_else(|| event.tool_input.clone())
        .unwrap_or_default();

    let payload = PlanSplitStreamPayload {
        event_type: event.event_type.clone(),
        plan_id: plan_id.to_string(),
        session_id: session_id.to_string(),
        content: event.content,
        tool_name: event.tool_name,
        tool_call_id: event.tool_call_id,
        tool_input: event.tool_input,
        tool_result: event.tool_result,
        error: event.error,
        input_tokens: event.input_tokens,
        output_tokens: event.output_tokens,
        model: event.model,
        external_session_id: event.external_session_id.clone(),
        metadata: metadata.clone(),
        created_at: now.clone(),
    };
    let event_name = format!("plan-split-stream-{plan_id}");
    let _ = app.emit(&event_name, &payload);

    if apply_external_session_id_to_request(&mut session, payload.external_session_id.as_deref())? {
        insert_or_update_session(&conn, &session)?;
    }

    if event.event_type != "done" {
        conn.execute(
            "INSERT INTO plan_split_logs (id, plan_id, session_id, log_type, content, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                plan_id,
                session_id,
                &event.event_type,
                &content,
                &metadata,
                &now
            ],
        )
        .map_err(|error| error.to_string())?;
    }

    if let Some(raw_output) = structured_output {
        if finalize_session_from_structured_output(app, plan_id, session_id, &raw_output)? {
            return Ok(());
        }
    }

    if payload.event_type == "done" {
        let _ = refresh_session_after_turn(app, plan_id, session_id);
    }

    Ok(())
}

pub fn mark_plan_split_failed(
    app: &AppHandle,
    plan_id: &str,
    session_id: &str,
    error_message: &str,
) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut session =
        read_session(&conn, plan_id)?.ok_or_else(|| "计划拆分会话不存在".to_string())?;

    if session.execution_session_id.as_deref() != Some(session_id)
        || is_terminal_session_status(&session.status)
    {
        emit_session_updated(app, &session);
        return Ok(());
    }

    let now = now_rfc3339();
    session.status = "failed".to_string();
    session.execution_session_id = Some(session_id.to_string());
    session.error_message = Some(error_message.to_string());
    session.parse_error = Some(error_message.to_string());
    session.updated_at = now.clone();
    session.completed_at = Some(now);
    session.raw_content = Some(load_content_logs(&conn, session_id).unwrap_or_default());
    let mut messages = load_messages_json(session.messages_json.as_ref());
    append_ui_message(
        &mut messages,
        "assistant",
        format!("拆分失败：{error_message}"),
    );
    session.messages_json = Some(serialize_json(&messages)?);
    insert_or_update_session(&conn, &session)?;
    emit_session_updated(app, &session);
    Ok(())
}

fn run_split_turn(app: AppHandle, request: ExecutionRequest) {
    tauri::async_runtime::spawn(async move {
        let plan_id = match request.plan_id.clone() {
            Some(value) => value,
            None => return,
        };

        let registry = get_registry().await;
        let registry = registry.read().await;
        let session_id = request.session_id.clone();
        if let Err(error) = registry.execute(app.clone(), request.clone()).await {
            let _ = mark_plan_split_failed(&app, &plan_id, &session_id, &error.to_string());
        }
    });
}

fn build_running_session(input: &StartPlanSplitInput) -> Result<PlanSplitSession, String> {
    let now = now_rfc3339();
    Ok(PlanSplitSession {
        id: uuid::Uuid::new_v4().to_string(),
        plan_id: input.plan_id.clone(),
        status: "running".to_string(),
        execution_session_id: Some(input.execution_request.session_id.clone()),
        raw_content: None,
        result_json: None,
        parse_error: None,
        error_message: None,
        granularity: input.granularity,
        task_count_mode: normalize_task_count_mode(input.task_count_mode.as_deref()),
        llm_messages_json: Some(serialize_json(&input.llm_messages)?),
        messages_json: Some(serialize_json(&input.messages)?),
        execution_request_json: Some(serialize_json(&input.execution_request)?),
        form_queue_json: None,
        current_form_index: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        started_at: Some(now),
        completed_at: None,
        stopped_at: None,
    })
}

#[tauri::command]
pub fn get_plan_split_session(plan_id: String) -> Result<Option<PlanSplitSession>, String> {
    let started_at = Instant::now();
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let result = read_session(&conn, &plan_id);
    match &result {
        Ok(Some(session)) => println!(
            "[PlanSplitPerf] backend:get_plan_split_session plan_id={} duration_ms={} status={} messages_bytes={} raw_bytes={} result_bytes={}",
            plan_id,
            started_at.elapsed().as_millis(),
            session.status,
            session.messages_json.as_ref().map(|value| value.len()).unwrap_or(0),
            session.raw_content.as_ref().map(|value| value.len()).unwrap_or(0),
            session.result_json.as_ref().map(|value| value.len()).unwrap_or(0)
        ),
        Ok(None) => println!(
            "[PlanSplitPerf] backend:get_plan_split_session plan_id={} duration_ms={} status=none",
            plan_id,
            started_at.elapsed().as_millis()
        ),
        Err(error) => println!(
            "[PlanSplitPerf] backend:get_plan_split_session plan_id={} duration_ms={} error={}",
            plan_id,
            started_at.elapsed().as_millis(),
            error
        ),
    }
    result
}

#[tauri::command]
pub fn list_plan_split_logs(plan_id: String) -> Result<Vec<PlanSplitLog>, String> {
    let started_at = Instant::now();
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, plan_id, session_id, log_type, content, metadata, created_at
             FROM plan_split_logs
             WHERE plan_id = ?1
             ORDER BY created_at ASC",
        )
        .map_err(|error| error.to_string())?;

    let logs = stmt
        .query_map([&plan_id], |row| {
            Ok(PlanSplitLog {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                session_id: row.get(2)?,
                log_type: row.get(3)?,
                content: row.get(4)?,
                metadata: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let content_bytes: usize = logs.iter().map(|log| log.content.len()).sum();
    let metadata_bytes: usize = logs
        .iter()
        .map(|log| log.metadata.as_ref().map(|value| value.len()).unwrap_or(0))
        .sum();
    println!(
        "[PlanSplitPerf] backend:list_plan_split_logs plan_id={} duration_ms={} count={} content_bytes={} metadata_bytes={}",
        plan_id,
        started_at.elapsed().as_millis(),
        logs.len(),
        content_bytes,
        metadata_bytes
    );

    Ok(logs)
}

#[tauri::command]
pub fn list_recent_plan_split_logs(
    plan_id: String,
    limit: Option<i64>,
) -> Result<Vec<PlanSplitLog>, String> {
    let limit = limit.unwrap_or(80).clamp(1, 500);
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, plan_id, session_id, log_type, content, metadata, created_at
             FROM (
                SELECT id, plan_id, session_id, log_type, content, metadata, created_at
                FROM plan_split_logs
                WHERE plan_id = ?1
                ORDER BY created_at DESC
                LIMIT ?2
             )
             ORDER BY created_at ASC",
        )
        .map_err(|error| error.to_string())?;

    let logs = stmt
        .query_map(rusqlite::params![&plan_id, limit], |row| {
            Ok(PlanSplitLog {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                session_id: row.get(2)?,
                log_type: row.get(3)?,
                content: row.get(4)?,
                metadata: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    Ok(logs)
}

/// 创建计划拆分日志。
///
/// 用途：
/// - 记录前端补充的系统态日志，例如自动重试进度；
/// - 返回落库后的日志主键，便于前端后续原地更新同一条记录。
///
/// 关键副作用：
/// - 写入 `plan_split_logs` 表；
/// - 不会触发会话状态流转。
#[tauri::command]
pub fn create_plan_split_log(
    plan_id: String,
    session_id: String,
    log_type: String,
    content: String,
    metadata: Option<String>,
) -> Result<PlanSplitLog, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();

    conn.execute(
        "INSERT INTO plan_split_logs (id, plan_id, session_id, log_type, content, metadata, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![&id, &plan_id, &session_id, &log_type, &content, &metadata, &now],
    )
    .map_err(|error| error.to_string())?;

    Ok(PlanSplitLog {
        id,
        plan_id,
        session_id,
        log_type,
        content,
        metadata,
        created_at: now,
    })
}

/// 更新计划拆分日志内容。
///
/// 用途：
/// - 原地刷新自动重试等累积型系统日志，避免重复插入多条近似记录。
///
/// 关键副作用：
/// - 更新 `plan_split_logs` 表中指定主键对应的内容与元数据；
/// - 不改动日志创建时间，也不会触发会话状态变更。
#[tauri::command]
pub fn update_plan_split_log(
    id: String,
    content: String,
    metadata: Option<String>,
) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;

    conn.execute(
        "UPDATE plan_split_logs SET content = ?1, metadata = ?2 WHERE id = ?3",
        rusqlite::params![&content, &metadata, &id],
    )
    .map_err(|error| error.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn start_plan_split(
    app: AppHandle,
    input: StartPlanSplitInput,
) -> Result<PlanSplitSession, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;

    let existing_session = read_session(&conn, &input.plan_id)?;
    if let Some(existing_session) = existing_session.as_ref() {
        if existing_session.status == "running" {
            let execution_session_id = existing_session
                .execution_session_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);

            if let Some(execution_session_id) = execution_session_id {
                if is_execution_session_active_internal(&execution_session_id).await {
                    emit_session_updated(&app, &existing_session);
                    return Ok(existing_session.clone());
                }
            }
        }
    }

    let mut request = input.execution_request.clone();
    request.plan_id = Some(input.plan_id.clone());
    request.messages = input.llm_messages.clone();

    let mut session = build_running_session(&StartPlanSplitInput {
        execution_request: request.clone(),
        ..input.clone()
    })?;
    if input.preserve_result.unwrap_or(false) {
        session.result_json = existing_session.and_then(|session| session.result_json);
    }
    insert_or_update_session(&conn, &session)?;
    emit_session_updated(&app, &session);
    run_split_turn(app, request);
    Ok(session)
}

#[tauri::command]
pub fn resume_plan_split(plan_id: String) -> Result<Option<PlanSplitSession>, String> {
    get_plan_split_session(plan_id)
}

/// 覆盖当前计划拆分会话的任务预览结果。
///
/// 用途：在用户手动编辑预览列表、应用整体优化或应用单任务继续拆分后，
/// 将当前前端确认中的任务列表同步回会话快照，避免后续会话刷新把结果回退为旧的拆分输出。
///
/// 参数：
/// - `plan_id`: 计划 ID
/// - `result`: 当前确认中的任务列表数组
/// - `messages`: 可选的前端 UI 消息数组，用于持久化指令操作记录
///
/// 返回：
/// - 更新后的计划拆分会话快照
#[tauri::command]
pub fn update_plan_split_result(
    app: AppHandle,
    plan_id: String,
    result: Value,
    messages: Option<Value>,
) -> Result<PlanSplitSession, String> {
    if !result.is_array() {
        return Err("任务拆分结果必须是任务数组。".to_string());
    }

    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut session =
        read_session(&conn, &plan_id)?.ok_or_else(|| "计划拆分会话不存在".to_string())?;

    session.result_json = Some(serialize_json(&serde_json::json!({
        "tasks": result
    }))?);
    if let Some(msgs) = messages {
        if msgs.is_array() {
            session.messages_json = Some(serialize_json(&msgs)?);
        }
    }
    session.parse_error = None;
    session.error_message = None;
    session.updated_at = now_rfc3339();

    insert_or_update_session(&conn, &session)?;
    emit_session_updated(&app, &session);
    Ok(session)
}

#[tauri::command]
pub fn submit_plan_split_form(
    app: AppHandle,
    input: SubmitPlanSplitFormInput,
) -> Result<PlanSplitSession, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let mut session =
        read_session(&conn, &input.plan_id)?.ok_or_else(|| "计划拆分会话不存在".to_string())?;

    let forms = load_form_queue(session.form_queue_json.as_ref());
    let current_index = session.current_form_index.unwrap_or(0).max(0) as usize;
    if current_index >= forms.len() {
        return Err("当前没有待提交的表单。".to_string());
    }

    let active_form = forms[current_index]
        .as_object()
        .ok_or_else(|| "活动表单结构无效。".to_string())?;
    let active_form_id = as_non_empty_string(
        active_form
            .get("formId")
            .or_else(|| active_form.get("form_id")),
    )
    .ok_or_else(|| "活动表单缺少 formId。".to_string())?;
    if active_form_id != input.form_id {
        return Err("提交的表单不是当前活动表单。".to_string());
    }

    let now = now_rfc3339();
    let mut llm_messages = load_llm_messages_json(session.llm_messages_json.as_ref());
    let mut messages = load_messages_json(session.messages_json.as_ref());
    llm_messages.push(MessageInput {
        role: "user".to_string(),
        content: build_form_response_prompt(&input.form_id, &input.values),
        attachments: None,
    });
    append_ui_message(&mut messages, "user", input.display_content);

    session.llm_messages_json = Some(serialize_json(&llm_messages)?);
    session.messages_json = Some(serialize_json(&messages)?);
    session.updated_at = now;
    session.error_message = None;
    session.parse_error = None;

    if current_index + 1 < forms.len() {
        session.current_form_index = Some((current_index + 1) as i32);
        session.status = "waiting_input".to_string();
        insert_or_update_session(&conn, &session)?;
        emit_session_updated(&app, &session);
        return Ok(session);
    }

    let mut request = session
        .execution_request_json
        .as_ref()
        .and_then(|value| serde_json::from_str::<ExecutionRequest>(value).ok())
        .ok_or_else(|| "会话缺少执行请求配置。".to_string())?;
    request.session_id = uuid::Uuid::new_v4().to_string();
    request.messages = llm_messages;
    request.plan_id = Some(input.plan_id.clone());

    session.execution_request_json = Some(serialize_json(&request)?);
    session.execution_session_id = Some(request.session_id.clone());
    session.status = "running".to_string();
    session.form_queue_json = None;
    session.current_form_index = None;
    session.completed_at = None;
    session.stopped_at = None;
    insert_or_update_session(&conn, &session)?;
    emit_session_updated(&app, &session);
    run_split_turn(app, request);
    Ok(session)
}

#[tauri::command]
pub async fn stop_plan_split(
    app: AppHandle,
    plan_id: String,
) -> Result<Option<PlanSplitSession>, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let Some(mut session) = read_session(&conn, &plan_id)? else {
        return Ok(None);
    };

    if let Some(execution_session_id) = session.execution_session_id.clone() {
        set_abort_flag(&execution_session_id, true).await;
    }

    let now = now_rfc3339();
    session.status = "stopped".to_string();
    session.stopped_at = Some(now.clone());
    session.completed_at = Some(now.clone());
    session.updated_at = now;
    session.error_message = None;
    insert_or_update_session(&conn, &session)?;

    conn.execute(
        "INSERT INTO plan_split_logs (id, plan_id, session_id, log_type, content, metadata, created_at)
         VALUES (?1, ?2, ?3, 'system', ?4, NULL, ?5)",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            &plan_id,
            session.execution_session_id.clone().unwrap_or_default(),
            "用户已停止后台拆分任务",
            now_rfc3339()
        ],
    )
    .map_err(|error| error.to_string())?;

    emit_session_updated(&app, &session);
    Ok(Some(session))
}

/// 重置当前拆分会话的执行状态，用于失败/停止后继续发起新一轮拆分。
///
/// 主要行为：
/// - 中止当前 execution_session_id 对应的后台执行；
/// - 清空本轮的错误、结果和表单状态；
/// - 当 `trim_latest_turn=true` 时，删除当前 assistant 轮次的消息和日志，用于“重试当前轮”；
/// - 当 `trim_latest_turn=false` 时，保留历史 `messages_json` 与 `plan_split_logs`，用于“追加新用户消息继续对话”。
#[tauri::command]
pub async fn reset_plan_split_turn_for_restart(
    app: AppHandle,
    plan_id: String,
    options: Option<ResetPlanSplitTurnOptions>,
) -> Result<PlanSplitSession, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let Some(mut session) = read_session(&conn, &plan_id)? else {
        return Err("计划拆分会话不存在".to_string());
    };
    let trim_latest_turn = options
        .as_ref()
        .and_then(|value| value.trim_latest_turn)
        .unwrap_or(false);
    let preserve_result = options
        .as_ref()
        .and_then(|value| value.preserve_result)
        .unwrap_or(false);
    let latest_user_timestamp = if trim_latest_turn {
        let messages = load_messages_json(session.messages_json.as_ref());
        latest_user_message_timestamp(&messages)
    } else {
        None
    };

    if let Some(execution_session_id) = session.execution_session_id.clone() {
        set_abort_flag(&execution_session_id, true).await;
        if trim_latest_turn {
            conn.execute(
                "DELETE FROM plan_split_logs
                 WHERE plan_id = ?1
                   AND session_id = ?2",
                rusqlite::params![&plan_id, &execution_session_id],
            )
            .map_err(|error| error.to_string())?;
        }
    }

    if trim_latest_turn {
        delete_plan_split_logs_from_timestamp(&conn, &plan_id, latest_user_timestamp.as_deref())?;

        let mut messages = load_messages_json(session.messages_json.as_ref());
        trim_messages_after_last_user(&mut messages, |message| message.role.as_str());
        session.messages_json = Some(serialize_json(&messages)?);

        let mut llm_messages = load_llm_messages_json(session.llm_messages_json.as_ref());
        trim_messages_after_last_user(&mut llm_messages, |message| message.role.as_str());
        session.llm_messages_json = Some(serialize_json(&llm_messages)?);
    }

    let now = now_rfc3339();
    session.execution_session_id = None;
    session.raw_content = None;
    if !preserve_result {
        session.result_json = None;
    }
    session.parse_error = None;
    session.error_message = None;
    session.form_queue_json = None;
    session.current_form_index = None;
    session.completed_at = None;
    session.stopped_at = None;
    session.status = "running".to_string();
    session.started_at = Some(now.clone());
    session.updated_at = now;

    insert_or_update_session(&conn, &session)?;
    emit_session_updated(&app, &session);
    Ok(session)
}

#[tauri::command]
pub async fn clear_plan_split_session(plan_id: String) -> Result<(), String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    if let Some(session) = read_session(&conn, &plan_id)? {
        if let Some(execution_session_id) = session.execution_session_id {
            set_abort_flag(&execution_session_id, true).await;
        }
    }
    conn.execute("DELETE FROM plan_split_logs WHERE plan_id = ?1", [&plan_id])
        .map_err(|error| error.to_string())?;
    conn.execute(
        "DELETE FROM task_split_sessions WHERE plan_id = ?1",
        [&plan_id],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
