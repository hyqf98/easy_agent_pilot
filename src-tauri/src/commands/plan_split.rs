use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use super::conversation::executor::get_registry;
use super::conversation::set_abort_flag;
use super::conversation::types::{ExecutionRequest, MessageInput};

fn get_db_path() -> Result<std::path::PathBuf> {
    let persistence_dir = super::get_persistence_dir_path()?;
    Ok(persistence_dir.join("data").join("easy-agent.db"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSplitMessage {
    pub id: String,
    pub role: String,
    pub content: String,
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
    pub execution_request: ExecutionRequest,
    pub llm_messages: Vec<MessageInput>,
    pub messages: Vec<PlanSplitMessage>,
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
    pub metadata: Option<String>,
    pub created_at: String,
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
}

#[derive(Debug, Clone)]
enum ParsedSplitOutput {
    FormRequest { question: String, forms: Vec<Value> },
    TaskSplit { tasks: Vec<Value> },
}

fn read_session(conn: &rusqlite::Connection, plan_id: &str) -> Result<Option<PlanSplitSession>, String> {
    let result = conn.query_row(
        "SELECT id, plan_id, status, execution_session_id, raw_content, parsed_output,
                parse_error, error_message, granularity, llm_messages_json, messages_json,
                execution_request_json, form_queue_json, current_form_index,
                created_at, updated_at, started_at, completed_at, stopped_at
         FROM task_split_sessions WHERE plan_id = ?1",
        [&plan_id],
        |row| {
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
                llm_messages_json: row.get(9)?,
                messages_json: row.get(10)?,
                execution_request_json: row.get(11)?,
                form_queue_json: row.get(12)?,
                current_form_index: row.get(13)?,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
                started_at: row.get(16)?,
                completed_at: row.get(17)?,
                stopped_at: row.get(18)?,
            })
        },
    );

    match result {
        Ok(session) => Ok(Some(session)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn insert_or_update_session(conn: &rusqlite::Connection, session: &PlanSplitSession) -> Result<(), String> {
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
                 llm_messages_json = ?8,
                 messages_json = ?9,
                 execution_request_json = ?10,
                 form_queue_json = ?11,
                 current_form_index = ?12,
                 updated_at = ?13,
                 started_at = ?14,
                 completed_at = ?15,
                 stopped_at = ?16
             WHERE id = ?17",
            rusqlite::params![
                &session.status,
                &session.execution_session_id,
                &session.raw_content,
                &session.result_json,
                &session.parse_error,
                &session.error_message,
                &session.granularity,
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
              error_message, granularity, llm_messages_json, messages_json, execution_request_json,
              form_queue_json, current_form_index, created_at, updated_at, started_at, completed_at, stopped_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
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

fn load_messages_json(raw: Option<&String>) -> Vec<PlanSplitMessage> {
    raw.and_then(|value| serde_json::from_str::<Vec<PlanSplitMessage>>(value).ok())
        .unwrap_or_default()
}

fn load_llm_messages_json(raw: Option<&String>) -> Vec<MessageInput> {
    raw.and_then(|value| serde_json::from_str::<Vec<MessageInput>>(value).ok())
        .unwrap_or_default()
}

fn load_form_queue(raw: Option<&String>) -> Vec<Value> {
    raw.and_then(|value| serde_json::from_str::<Vec<Value>>(value).ok())
        .unwrap_or_default()
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
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
}

fn build_form_response_prompt(form_id: &str, values: &Value) -> String {
    let serialized = match values {
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| {
                if value.is_object() || value.is_array() {
                    format!("{key}: {}", serde_json::to_string(value).unwrap_or_default())
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

fn as_non_empty_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|item| item.as_str()).map(str::trim).filter(|item| !item.is_empty()).map(ToOwned::to_owned)
}

fn as_string_array(value: Option<&Value>) -> Vec<String> {
    value.and_then(|item| item.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::trim).filter(|text| !text.is_empty()).map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn normalize_task(task: &Value) -> Result<Value, String> {
    let task_obj = task.as_object().ok_or_else(|| "tasks 中存在无效任务对象。".to_string())?;
    let title = as_non_empty_string(task_obj.get("title")).ok_or_else(|| "任务缺少 title。".to_string())?;
    let description = as_non_empty_string(task_obj.get("description")).ok_or_else(|| "任务缺少 description。".to_string())?;
    let priority = as_non_empty_string(task_obj.get("priority")).ok_or_else(|| "任务缺少 priority。".to_string())?;
    let implementation_steps = as_string_array(task_obj.get("implementationSteps").or_else(|| task_obj.get("implementation_steps")).or_else(|| task_obj.get("steps")));
    let test_steps = as_string_array(task_obj.get("testSteps").or_else(|| task_obj.get("test_steps")).or_else(|| task_obj.get("testingSteps")).or_else(|| task_obj.get("testing_steps")));
    let acceptance_criteria = as_string_array(task_obj.get("acceptanceCriteria").or_else(|| task_obj.get("acceptance_criteria")));
    if implementation_steps.is_empty() || test_steps.is_empty() || acceptance_criteria.is_empty() {
        return Err("任务步骤字段不能为空（implementationSteps/testSteps/acceptanceCriteria）。".to_string());
    }

    let mut normalized = serde_json::Map::new();
    normalized.insert("title".to_string(), Value::String(title));
    normalized.insert("description".to_string(), Value::String(description));
    normalized.insert("priority".to_string(), Value::String(priority));
    normalized.insert(
        "implementationSteps".to_string(),
        Value::Array(implementation_steps.into_iter().map(Value::String).collect()),
    );
    normalized.insert(
        "testSteps".to_string(),
        Value::Array(test_steps.into_iter().map(Value::String).collect()),
    );
    normalized.insert(
        "acceptanceCriteria".to_string(),
        Value::Array(acceptance_criteria.into_iter().map(Value::String).collect()),
    );
    if let Some(depends_on) = task_obj.get("dependsOn").or_else(|| task_obj.get("depends_on")).and_then(|value| value.as_array()) {
        normalized.insert("dependsOn".to_string(), Value::Array(depends_on.clone()));
    }
    Ok(Value::Object(normalized))
}

fn normalize_form_schema(schema: &Value) -> Option<Value> {
    let schema_obj = schema.as_object()?;
    let form_id = as_non_empty_string(schema_obj.get("formId").or_else(|| schema_obj.get("form_id")))?;
    let title = as_non_empty_string(schema_obj.get("title"))?;
    let fields = schema_obj.get("fields")?.as_array()?.clone();
    if fields.is_empty() {
        return None;
    }

    let mut normalized = serde_json::Map::new();
    normalized.insert("formId".to_string(), Value::String(form_id));
    normalized.insert("title".to_string(), Value::String(title));
    if let Some(description) = schema_obj.get("description").and_then(|value| value.as_str()) {
        normalized.insert("description".to_string(), Value::String(description.to_string()));
    }
    if let Some(submit_text) = schema_obj.get("submitText").or_else(|| schema_obj.get("submit_text")).and_then(|value| value.as_str()) {
        normalized.insert("submitText".to_string(), Value::String(submit_text.to_string()));
    }
    normalized.insert("fields".to_string(), Value::Array(fields));
    Some(Value::Object(normalized))
}

fn parse_split_output(raw_content: &str, min_task_count: i32) -> Result<ParsedSplitOutput, String> {
    for candidate in extract_json_candidates(raw_content) {
        let parsed: Value = match serde_json::from_str(&candidate) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Some(result_string) = parsed.get("result").and_then(|value| value.as_str()) {
            if let Ok(output) = parse_split_output(result_string, min_task_count) {
                return Ok(output);
            }
        }
        let record = match parsed.as_object() {
            Some(object) => object,
            None => continue,
        };
        let output_type = as_non_empty_string(record.get("type")).unwrap_or_default().to_lowercase();
        if output_type == "form_request" {
            let question = as_non_empty_string(record.get("question")).unwrap_or_else(|| "请先补充以下信息。".to_string());
            let mut forms = record
                .get("forms")
                .and_then(|value| value.as_array())
                .map(|items| items.iter().filter_map(normalize_form_schema).collect::<Vec<_>>())
                .unwrap_or_default();
            if forms.is_empty() {
                if let Some(schema) = record.get("formSchema").or_else(|| record.get("form_schema")).or_else(|| record.get("schema")).and_then(normalize_form_schema) {
                    forms.push(schema);
                }
            }
            if forms.is_empty() {
                return Err("form_request 缺少有效 forms。".to_string());
            }
            return Ok(ParsedSplitOutput::FormRequest { question, forms });
        }

        let is_done = record
            .get("done")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            || as_non_empty_string(record.get("status").or_else(|| record.get("state")).or_else(|| record.get("phase")))
                .map(|value| value.to_uppercase() == "DONE")
                .unwrap_or(false);

        if output_type == "task_split" || output_type == "done" || (is_done && record.get("tasks").is_some()) {
            if !is_done {
                return Err("task_split 必须包含 status: DONE。".to_string());
            }
            let tasks_raw = record
                .get("tasks")
                .and_then(|value| value.as_array())
                .ok_or_else(|| "task_split 缺少 tasks 数组。".to_string())?;
            if tasks_raw.len() < min_task_count.max(1) as usize {
                return Err(format!("拆分任务数量不足，至少需要 {} 个。", min_task_count.max(1)));
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
            "SELECT content, metadata FROM plan_split_logs
             WHERE session_id = ?1 AND log_type = 'tool_use'
             ORDER BY created_at ASC",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map([session_id], |row| {
            let content: String = row.get(0)?;
            let metadata: Option<String> = row.get(1)?;
            Ok((content, metadata))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let outputs = rows
        .into_iter()
        .filter_map(|(content, metadata)| {
            let metadata_value = metadata
                .as_deref()
                .and_then(|raw| serde_json::from_str::<Value>(raw).ok());
            let tool_name = metadata_value
                .as_ref()
                .and_then(|value| {
                    as_non_empty_string(value.get("toolName").or_else(|| value.get("tool_name")))
                })
                .unwrap_or_default();

            if tool_name.eq_ignore_ascii_case("StructuredOutput")
                || tool_name.eq_ignore_ascii_case("structured_output")
            {
                Some(content)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(outputs)
}

fn serialize_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}

fn refresh_session_after_turn(app: &AppHandle, plan_id: &str, session_id: &str) -> Result<(), String> {
    let db_path = get_db_path().map_err(|error| error.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|error| error.to_string())?;
    let mut session = read_session(&conn, plan_id)?.ok_or_else(|| "计划拆分会话不存在".to_string())?;
    if session.status == "stopped" {
        emit_session_updated(app, &session);
        return Ok(());
    }

    let raw_content = load_content_logs(&conn, session_id)?;
    let parsed = match parse_split_output(&raw_content, session.granularity) {
        Ok(output) => Ok((output, raw_content.clone())),
        Err(content_error) => {
            let structured_outputs = load_structured_output_logs(&conn, session_id)?;
            let parsed_from_tool = structured_outputs
                .iter()
                .rev()
                .find_map(|candidate| {
                    parse_split_output(candidate, session.granularity)
                        .ok()
                        .map(|output| (output, candidate.clone()))
                });

            parsed_from_tool.ok_or(content_error)
        }
    };
    let now = chrono::Utc::now().to_rfc3339();
    session.raw_content = Some(raw_content.clone());
    session.updated_at = now.clone();
    session.completed_at = Some(now);
    session.execution_session_id = Some(session_id.to_string());
    session.parse_error = None;
    session.error_message = None;

    let mut llm_messages = load_llm_messages_json(session.llm_messages_json.as_ref());
    let mut messages = load_messages_json(session.messages_json.as_ref());

    match parsed {
        Ok((output, parsed_raw_content)) => {
            session.raw_content = Some(parsed_raw_content);
            append_ui_message(
                &mut messages,
                "assistant",
                match &output {
                    ParsedSplitOutput::FormRequest { question, .. } => question.clone(),
                    ParsedSplitOutput::TaskSplit { tasks } => {
                        format!("DONE：任务拆分完成，共生成 {} 个任务，请确认。", tasks.len())
                    }
                },
            );
            llm_messages.push(MessageInput {
                role: "assistant".to_string(),
                content: extract_assistant_summary(&output),
            });
            session.llm_messages_json = Some(serialize_json(&llm_messages)?);
            session.messages_json = Some(serialize_json(&messages)?);
            match output {
                ParsedSplitOutput::FormRequest { forms, .. } => {
                    session.status = "waiting_input".to_string();
                    session.form_queue_json = Some(serialize_json(&forms)?);
                    session.current_form_index = Some(0);
                    session.result_json = None;
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
        }
        Err(error_message) => {
            append_ui_message(&mut messages, "assistant", format!("解析失败：{error_message}"));
            session.status = "failed".to_string();
            session.parse_error = Some(error_message.clone());
            session.error_message = Some(error_message);
            session.messages_json = Some(serialize_json(&messages)?);
            session.llm_messages_json = Some(serialize_json(&llm_messages)?);
            session.form_queue_json = None;
            session.current_form_index = None;
            session.result_json = None;
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
    let db_path = get_db_path().map_err(|error| error.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|error| error.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let metadata = serde_json::to_string(&serde_json::json!({
        "toolName": event.tool_name,
        "toolCallId": event.tool_call_id,
        "toolInput": event.tool_input,
        "toolResult": event.tool_result,
        "error": event.error,
    }))
    .ok();
    let content = event
        .content
        .clone()
        .or_else(|| event.error.clone())
        .or_else(|| event.tool_result.clone())
        .or_else(|| event.tool_input.clone())
        .unwrap_or_default();

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
        metadata,
        created_at: now,
    };
    let event_name = format!("plan-split-stream-{plan_id}");
    let _ = app.emit(&event_name, &payload);

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
    let db_path = get_db_path().map_err(|error| error.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|error| error.to_string())?;
    let mut session = read_session(&conn, plan_id)?.ok_or_else(|| "计划拆分会话不存在".to_string())?;
    if session.status == "stopped" {
        emit_session_updated(app, &session);
        return Ok(());
    }

    let now = chrono::Utc::now().to_rfc3339();
    session.status = "failed".to_string();
    session.execution_session_id = Some(session_id.to_string());
    session.error_message = Some(error_message.to_string());
    session.parse_error = Some(error_message.to_string());
    session.updated_at = now.clone();
    session.completed_at = Some(now);
    session.raw_content = Some(load_content_logs(&conn, session_id).unwrap_or_default());
    let mut messages = load_messages_json(session.messages_json.as_ref());
    append_ui_message(&mut messages, "assistant", format!("拆分失败：{error_message}"));
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
    let now = chrono::Utc::now().to_rfc3339();
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
    let db_path = get_db_path().map_err(|error| error.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|error| error.to_string())?;
    read_session(&conn, &plan_id)
}

#[tauri::command]
pub fn list_plan_split_logs(plan_id: String) -> Result<Vec<PlanSplitLog>, String> {
    let db_path = get_db_path().map_err(|error| error.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, plan_id, session_id, log_type, content, metadata, created_at
             FROM plan_split_logs
             WHERE plan_id = ?1
             ORDER BY created_at ASC",
        )
        .map_err(|error| error.to_string())?;

    let logs = stmt.query_map([&plan_id], |row| {
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

#[tauri::command]
pub fn start_plan_split(app: AppHandle, input: StartPlanSplitInput) -> Result<PlanSplitSession, String> {
    let db_path = get_db_path().map_err(|error| error.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM plan_split_logs WHERE plan_id = ?1", [&input.plan_id])
        .map_err(|error| error.to_string())?;

    let mut request = input.execution_request.clone();
    request.plan_id = Some(input.plan_id.clone());
    request.messages = input.llm_messages.clone();

    let session = build_running_session(&StartPlanSplitInput {
        execution_request: request.clone(),
        ..input.clone()
    })?;
    insert_or_update_session(&conn, &session)?;
    emit_session_updated(&app, &session);
    run_split_turn(app, request);
    Ok(session)
}

#[tauri::command]
pub fn resume_plan_split(plan_id: String) -> Result<Option<PlanSplitSession>, String> {
    get_plan_split_session(plan_id)
}

#[tauri::command]
pub fn submit_plan_split_form(
    app: AppHandle,
    input: SubmitPlanSplitFormInput,
) -> Result<PlanSplitSession, String> {
    let db_path = get_db_path().map_err(|error| error.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|error| error.to_string())?;
    let mut session = read_session(&conn, &input.plan_id)?.ok_or_else(|| "计划拆分会话不存在".to_string())?;

    let forms = load_form_queue(session.form_queue_json.as_ref());
    let current_index = session.current_form_index.unwrap_or(0).max(0) as usize;
    if current_index >= forms.len() {
        return Err("当前没有待提交的表单。".to_string());
    }

    let active_form = forms[current_index]
        .as_object()
        .ok_or_else(|| "活动表单结构无效。".to_string())?;
    let active_form_id = as_non_empty_string(active_form.get("formId").or_else(|| active_form.get("form_id")))
        .ok_or_else(|| "活动表单缺少 formId。".to_string())?;
    if active_form_id != input.form_id {
        return Err("提交的表单不是当前活动表单。".to_string());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let mut llm_messages = load_llm_messages_json(session.llm_messages_json.as_ref());
    let mut messages = load_messages_json(session.messages_json.as_ref());
    llm_messages.push(MessageInput {
        role: "user".to_string(),
        content: build_form_response_prompt(&input.form_id, &input.values),
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
pub async fn stop_plan_split(app: AppHandle, plan_id: String) -> Result<Option<PlanSplitSession>, String> {
    let db_path = get_db_path().map_err(|error| error.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|error| error.to_string())?;
    let Some(mut session) = read_session(&conn, &plan_id)? else {
        return Ok(None);
    };

    if let Some(execution_session_id) = session.execution_session_id.clone() {
        set_abort_flag(&execution_session_id, true).await;
    }

    let now = chrono::Utc::now().to_rfc3339();
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
            chrono::Utc::now().to_rfc3339()
        ],
    )
    .map_err(|error| error.to_string())?;

    emit_session_updated(&app, &session);
    Ok(Some(session))
}

#[tauri::command]
pub async fn clear_plan_split_session(plan_id: String) -> Result<(), String> {
    let db_path = get_db_path().map_err(|error| error.to_string())?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|error| error.to_string())?;
    if let Some(session) = read_session(&conn, &plan_id)? {
        if let Some(execution_session_id) = session.execution_session_id {
            set_abort_flag(&execution_session_id, true).await;
        }
    }
    conn.execute("DELETE FROM plan_split_logs WHERE plan_id = ?1", [&plan_id])
        .map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM task_split_sessions WHERE plan_id = ?1", [&plan_id])
        .map_err(|error| error.to_string())?;
    Ok(())
}
