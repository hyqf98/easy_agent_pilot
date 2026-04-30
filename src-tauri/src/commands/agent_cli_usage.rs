use anyhow::Result;
use rusqlite::{params, params_from_iter};
use serde::{Deserialize, Serialize};

use super::support::{now_rfc3339, open_db_connection};

const PRICING_VERSION: &str = "2026-03-25";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 记录 CLI 用量所需的输入参数。
///
/// 包含一次执行的上下文快照、所属 Provider/模型，以及输入输出 token 统计。
pub struct RecordAgentCliUsageInput {
    pub execution_id: String,
    pub execution_mode: String,
    pub provider: String,
    pub agent_id: Option<String>,
    pub agent_name_snapshot: Option<String>,
    pub model_id: Option<String>,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub message_id: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 单条 CLI 用量事实记录。
///
/// 用于落库存储和导出，保留定价快照与执行维度信息。
pub struct AgentCliUsageRecord {
    pub execution_id: String,
    pub execution_mode: String,
    pub provider: String,
    pub agent_id: Option<String>,
    pub agent_name_snapshot: Option<String>,
    pub model_id: Option<String>,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub message_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub call_count: i64,
    pub estimated_input_cost_usd: Option<f64>,
    pub estimated_output_cost_usd: Option<f64>,
    pub estimated_total_cost_usd: Option<f64>,
    pub pricing_status: String,
    pub pricing_version: String,
    pub occurred_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 查询 CLI 用量统计面板的筛选参数。
///
/// 支持时间范围、时间粒度、统计维度与 Provider 过滤。
pub struct QueryAgentCliUsageStatsInput {
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub granularity: String,
    pub dimension: String,
    pub provider_filter: Option<String>,
    pub model_keyword: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 查询结果的总览汇总。
pub struct AgentCliUsageSummary {
    pub total_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_total_cost_usd: f64,
    pub unpriced_calls: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 时间趋势图中的单个聚合点。
pub struct AgentCliUsageTimelinePoint {
    pub bucket: String,
    pub label: String,
    pub call_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 按 Agent 或模型聚合后的明细行。
pub struct AgentCliUsageBreakdownRow {
    pub dimension_id: String,
    pub label: String,
    pub provider: String,
    pub call_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_total_cost_usd: f64,
    pub unpriced_calls: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 堆叠图使用的时间桶分布点。
pub struct AgentCliUsageStackedPoint {
    pub bucket: String,
    pub label: String,
    pub dimension_id: String,
    pub dimension_label: String,
    pub provider: String,
    pub call_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub estimated_total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 查询返回的元信息。
///
/// 主要用于前端展示当前筛选条件和价格快照版本。
pub struct AgentCliUsageMeta {
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub granularity: String,
    pub dimension: String,
    pub provider_filter: String,
    pub model_keyword: Option<String>,
    pub pricing_version: String,
    pub cost_partial: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// CLI 用量统计查询的完整响应。
pub struct AgentCliUsageStatsResponse {
    pub summary: AgentCliUsageSummary,
    pub timeline: Vec<AgentCliUsageTimelinePoint>,
    pub breakdown: Vec<AgentCliUsageBreakdownRow>,
    pub stacked_timeline: Vec<AgentCliUsageStackedPoint>,
    pub meta: AgentCliUsageMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// CLI 用量历史修复结果。
///
/// 用于反馈本次是否命中自动纠偏条件，以及实际修复了多少条历史记录。
pub struct RepairAgentCliUsageHistoryResult {
    pub provider: String,
    pub target_model_id: Option<String>,
    pub updated_count: i64,
    pub skipped_reason: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct ModelPricing {
    input_per_million_usd: f64,
    output_per_million_usd: f64,
}

struct PricingEstimate {
    estimated_input_cost_usd: Option<f64>,
    estimated_output_cost_usd: Option<f64>,
    estimated_total_cost_usd: Option<f64>,
    pricing_status: String,
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let normalized = item.trim().to_string();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

fn normalize_provider(value: &str) -> String {
    let normalized = value.trim().to_lowercase();
    match normalized.as_str() {
        "codex" => "codex".to_string(),
        "opencode" => "opencode".to_string(),
        _ => "claude".to_string(),
    }
}

fn normalize_granularity(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "year" => "year".to_string(),
        "month" => "month".to_string(),
        _ => "day".to_string(),
    }
}

fn normalize_dimension(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "model" => "model".to_string(),
        _ => "agent".to_string(),
    }
}

fn normalize_provider_filter(value: Option<String>) -> String {
    match value
        .unwrap_or_else(|| "all".to_string())
        .trim()
        .to_lowercase()
        .as_str()
    {
        "claude" => "claude".to_string(),
        "codex" => "codex".to_string(),
        "opencode" => "opencode".to_string(),
        _ => "all".to_string(),
    }
}

fn normalize_model_id(model_id: Option<&str>) -> Option<String> {
    model_id.and_then(|value| {
        let normalized = value.trim().to_lowercase();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

fn is_builtin_claude_model(model_id: &str) -> bool {
    let normalized = model_id.trim().to_lowercase();
    normalized.starts_with("claude")
        || normalized.contains("opus")
        || normalized.contains("sonnet")
        || normalized.contains("haiku")
}

fn suspicious_claude_history_models() -> &'static [&'static str] {
    &[
        "claude-haiku-4-5",
        "claude-haiku4-5",
        "claude-haiku-4.5",
        "haiku-4.5",
        "haiku-4-5",
        "haiku",
    ]
}

fn resolve_model_pricing(provider: &str, model_id: Option<&str>) -> Option<ModelPricing> {
    let normalized_provider = normalize_provider(provider);
    let normalized_model = normalize_model_id(model_id);
    let model = normalized_model.as_deref()?;

    if normalized_provider == "codex" {
        if model.starts_with("gpt-5.4") {
            return Some(ModelPricing {
                input_per_million_usd: 2.5,
                output_per_million_usd: 15.0,
            });
        }

        return match model {
            "gpt-5-codex" => Some(ModelPricing {
                input_per_million_usd: 1.25,
                output_per_million_usd: 10.0,
            }),
            "gpt-5.3-codex" => Some(ModelPricing {
                input_per_million_usd: 4.0,
                output_per_million_usd: 16.0,
            }),
            "gpt-5.2-codex" | "gpt-5.2" => Some(ModelPricing {
                input_per_million_usd: 1.75,
                output_per_million_usd: 14.0,
            }),
            "gpt-5.1-codex" | "gpt-5.1" | "gpt-5" => Some(ModelPricing {
                input_per_million_usd: 1.25,
                output_per_million_usd: 10.0,
            }),
            _ => None,
        };
    }

    if model.contains("opus") {
        return Some(ModelPricing {
            input_per_million_usd: 15.0,
            output_per_million_usd: 75.0,
        });
    }

    if model.contains("sonnet") {
        return Some(ModelPricing {
            input_per_million_usd: 3.0,
            output_per_million_usd: 15.0,
        });
    }

    if model.contains("haiku") {
        return Some(ModelPricing {
            input_per_million_usd: 0.8,
            output_per_million_usd: 4.0,
        });
    }

    None
}

fn estimate_pricing(
    provider: &str,
    model_id: Option<&str>,
    input_tokens: i64,
    output_tokens: i64,
) -> PricingEstimate {
    if input_tokens == 0 && output_tokens == 0 {
        return PricingEstimate {
            estimated_input_cost_usd: None,
            estimated_output_cost_usd: None,
            estimated_total_cost_usd: None,
            pricing_status: "missing_usage".to_string(),
        };
    }

    let Some(pricing) = resolve_model_pricing(provider, model_id) else {
        return PricingEstimate {
            estimated_input_cost_usd: None,
            estimated_output_cost_usd: None,
            estimated_total_cost_usd: None,
            pricing_status: "unmapped".to_string(),
        };
    };

    let input_cost = (input_tokens as f64 / 1_000_000_f64) * pricing.input_per_million_usd;
    let output_cost = (output_tokens as f64 / 1_000_000_f64) * pricing.output_per_million_usd;

    PricingEstimate {
        estimated_input_cost_usd: Some(input_cost),
        estimated_output_cost_usd: Some(output_cost),
        estimated_total_cost_usd: Some(input_cost + output_cost),
        pricing_status: "estimated".to_string(),
    }
}

fn reference_exists(conn: &rusqlite::Connection, table: &str, id: &str) -> Result<bool, String> {
    let sql = format!("SELECT 1 FROM {} WHERE id = ?1 LIMIT 1", table);
    conn.query_row(&sql, [id], |_| Ok(()))
        .map(|_| true)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(false),
            other => Err(other.to_string()),
        })
}

fn normalize_reference_id(
    conn: &rusqlite::Connection,
    table: &str,
    value: Option<String>,
) -> Result<Option<String>, String> {
    let normalized = normalize_optional_text(value);
    let Some(id) = normalized else {
        return Ok(None);
    };

    if reference_exists(conn, table, &id)? {
        Ok(Some(id))
    } else {
        Ok(None)
    }
}

fn build_where_clause(input: &QueryAgentCliUsageStatsInput) -> (String, Vec<String>) {
    let mut clauses = vec!["1 = 1".to_string()];
    let mut params = Vec::new();

    if let Some(start_at) = normalize_optional_text(input.start_at.clone()) {
        clauses.push("occurred_at >= ?".to_string());
        params.push(start_at);
    }

    if let Some(end_at) = normalize_optional_text(input.end_at.clone()) {
        clauses.push("occurred_at <= ?".to_string());
        params.push(end_at);
    }

    let provider_filter = normalize_provider_filter(input.provider_filter.clone());
    if provider_filter != "all" {
        clauses.push("provider = ?".to_string());
        params.push(provider_filter);
    }

    if let Some(model_keyword) = normalize_optional_text(input.model_keyword.clone()) {
        clauses.push("LOWER(COALESCE(model_id, '')) LIKE ?".to_string());
        params.push(format!("%{}%", model_keyword.to_lowercase()));
    }

    (clauses.join(" AND "), params)
}

fn bucket_sql(granularity: &str) -> &'static str {
    match granularity {
        "year" => "strftime('%Y', datetime(occurred_at, 'localtime'))",
        "month" => "strftime('%Y-%m', datetime(occurred_at, 'localtime'))",
        _ => "strftime('%Y-%m-%d', datetime(occurred_at, 'localtime'))",
    }
}

fn dimension_sql(dimension: &str, include_provider_prefix: bool) -> (&'static str, &'static str) {
    if dimension == "model" {
        if include_provider_prefix {
            (
                "COALESCE(NULLIF(model_id, ''), '__default_model__')",
                "CASE
                    WHEN model_id IS NULL OR trim(model_id) = '' THEN provider || ' / Default model'
                    ELSE provider || ' / ' || model_id
                 END",
            )
        } else {
            (
                "COALESCE(NULLIF(model_id, ''), '__default_model__')",
                "COALESCE(NULLIF(model_id, ''), 'Default model')",
            )
        }
    } else {
        (
            "COALESCE(NULLIF(agent_id, ''), '__unknown_agent__')",
            "COALESCE(NULLIF(agent_name_snapshot, ''), NULLIF(agent_id, ''), 'Unknown agent')",
        )
    }
}

fn breakdown_order_sql(dimension: &str) -> &'static str {
    if dimension == "model" {
        "SUM(estimated_total_cost_usd) DESC, SUM(total_tokens) DESC, dimension_label ASC"
    } else {
        "SUM(total_tokens) DESC, SUM(estimated_total_cost_usd) DESC, dimension_label ASC"
    }
}

fn repair_claude_usage_history(
    conn: &rusqlite::Connection,
) -> Result<RepairAgentCliUsageHistoryResult, String> {
    let current_profile = super::provider_profile::read_current_cli_config("claude".to_string())?;
    let Some(target_model_id) = normalize_optional_text(current_profile.main_model.clone()) else {
        return Ok(RepairAgentCliUsageHistoryResult {
            provider: "claude".to_string(),
            target_model_id: None,
            updated_count: 0,
            skipped_reason: Some("missing_current_model".to_string()),
        });
    };

    if is_builtin_claude_model(&target_model_id) {
        return Ok(RepairAgentCliUsageHistoryResult {
            provider: "claude".to_string(),
            target_model_id: Some(target_model_id),
            updated_count: 0,
            skipped_reason: Some("current_model_is_builtin".to_string()),
        });
    }

    let suspicious_models = suspicious_claude_history_models();
    let suspicious_count: i64 = conn
        .query_row(
            r#"
            SELECT COUNT(*)
            FROM agent_cli_usage_records
            WHERE provider = 'claude'
              AND LOWER(COALESCE(model_id, '')) IN (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                suspicious_models[0],
                suspicious_models[1],
                suspicious_models[2],
                suspicious_models[3],
                suspicious_models[4],
                suspicious_models[5]
            ],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    if suspicious_count == 0 {
        return Ok(RepairAgentCliUsageHistoryResult {
            provider: "claude".to_string(),
            target_model_id: Some(target_model_id),
            updated_count: 0,
            skipped_reason: Some("no_suspicious_history".to_string()),
        });
    }

    let mut select_stmt = conn
        .prepare(
            r#"
            SELECT execution_id, input_tokens, output_tokens
            FROM agent_cli_usage_records
            WHERE provider = 'claude'
              AND LOWER(COALESCE(model_id, '')) IN (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .map_err(|error| error.to_string())?;

    let repair_rows = select_stmt
        .query_map(
            params![
                suspicious_models[0],
                suspicious_models[1],
                suspicious_models[2],
                suspicious_models[3],
                suspicious_models[4],
                suspicious_models[5]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let mut updated_count = 0_i64;
    for (execution_id, input_tokens, output_tokens) in repair_rows {
        let pricing = estimate_pricing(
            "claude",
            Some(target_model_id.as_str()),
            input_tokens,
            output_tokens,
        );

        updated_count += conn
            .execute(
                r#"
                UPDATE agent_cli_usage_records
                SET model_id = ?1,
                    estimated_input_cost_usd = ?2,
                    estimated_output_cost_usd = ?3,
                    estimated_total_cost_usd = ?4,
                    pricing_status = ?5,
                    pricing_version = ?6
                WHERE execution_id = ?7
                "#,
                params![
                    target_model_id,
                    pricing.estimated_input_cost_usd,
                    pricing.estimated_output_cost_usd,
                    pricing.estimated_total_cost_usd,
                    pricing.pricing_status,
                    PRICING_VERSION,
                    execution_id
                ],
            )
            .map_err(|error| error.to_string())? as i64;
    }

    Ok(RepairAgentCliUsageHistoryResult {
        provider: "claude".to_string(),
        target_model_id: Some(target_model_id),
        updated_count,
        skipped_reason: None,
    })
}

/// 记录一次 CLI 用量统计。
///
/// 用途：在 CLI 主执行链路完成后异步落库，用于设置页图表和汇总统计。
/// 主要参数：包含执行上下文、Agent/模型快照以及输入输出 token。
/// 返回值：返回已写入或已更新的统计记录。
/// 关键副作用：写入本地 SQLite 统计表；同一 execution_id 会执行幂等更新。
#[tauri::command]
pub fn record_agent_cli_usage(
    input: RecordAgentCliUsageInput,
) -> Result<AgentCliUsageRecord, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let now = now_rfc3339();
    let RecordAgentCliUsageInput {
        execution_id,
        execution_mode,
        provider,
        agent_id,
        agent_name_snapshot,
        model_id,
        project_id,
        session_id,
        task_id,
        message_id,
        input_tokens,
        output_tokens,
        occurred_at,
    } = input;
    let occurred_at = normalize_optional_text(occurred_at).unwrap_or_else(|| now.clone());
    let provider = normalize_provider(&provider);
    let input_tokens = input_tokens.unwrap_or(0).max(0);
    let output_tokens = output_tokens.unwrap_or(0).max(0);
    let total_tokens = input_tokens + output_tokens;
    let model_id = normalize_optional_text(model_id);
    let agent_id = normalize_reference_id(&conn, "agents", agent_id)?;
    let project_id = normalize_reference_id(&conn, "projects", project_id)?;
    let session_id = normalize_reference_id(&conn, "sessions", session_id)?;
    let task_id = normalize_reference_id(&conn, "tasks", task_id)?;
    let message_id = normalize_reference_id(&conn, "messages", message_id)?;
    let agent_name_snapshot = normalize_optional_text(agent_name_snapshot);
    let pricing = estimate_pricing(&provider, model_id.as_deref(), input_tokens, output_tokens);

    conn.execute(
        r#"
        INSERT INTO agent_cli_usage_records (
            execution_id, execution_mode, provider, agent_id, agent_name_snapshot, model_id,
            project_id, session_id, task_id, message_id, input_tokens, output_tokens, total_tokens,
            call_count, estimated_input_cost_usd, estimated_output_cost_usd, estimated_total_cost_usd,
            pricing_status, pricing_version, occurred_at, created_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            1, ?14, ?15, ?16,
            ?17, ?18, ?19, ?20
        )
        ON CONFLICT(execution_id) DO UPDATE SET
            execution_mode = excluded.execution_mode,
            provider = excluded.provider,
            agent_id = excluded.agent_id,
            agent_name_snapshot = excluded.agent_name_snapshot,
            model_id = excluded.model_id,
            project_id = excluded.project_id,
            session_id = excluded.session_id,
            task_id = excluded.task_id,
            message_id = excluded.message_id,
            input_tokens = excluded.input_tokens,
            output_tokens = excluded.output_tokens,
            total_tokens = excluded.total_tokens,
            call_count = excluded.call_count,
            estimated_input_cost_usd = excluded.estimated_input_cost_usd,
            estimated_output_cost_usd = excluded.estimated_output_cost_usd,
            estimated_total_cost_usd = excluded.estimated_total_cost_usd,
            pricing_status = excluded.pricing_status,
            pricing_version = excluded.pricing_version,
            occurred_at = excluded.occurred_at
        "#,
        params![
            execution_id,
            execution_mode,
            provider,
            agent_id,
            agent_name_snapshot,
            model_id,
            project_id,
            session_id,
            task_id,
            message_id,
            input_tokens,
            output_tokens,
            total_tokens,
            pricing.estimated_input_cost_usd,
            pricing.estimated_output_cost_usd,
            pricing.estimated_total_cost_usd,
            pricing.pricing_status,
            PRICING_VERSION,
            occurred_at,
            now,
        ],
    )
    .map_err(|error| error.to_string())?;

    conn.query_row(
        r#"
        SELECT execution_id, execution_mode, provider, agent_id, agent_name_snapshot, model_id,
               project_id, session_id, task_id, message_id, input_tokens, output_tokens, total_tokens,
               call_count, estimated_input_cost_usd, estimated_output_cost_usd, estimated_total_cost_usd,
               pricing_status, pricing_version, occurred_at, created_at
        FROM agent_cli_usage_records
        WHERE execution_id = ?1
        "#,
        [execution_id],
        |row| {
            Ok(AgentCliUsageRecord {
                execution_id: row.get(0)?,
                execution_mode: row.get(1)?,
                provider: row.get(2)?,
                agent_id: row.get(3)?,
                agent_name_snapshot: row.get(4)?,
                model_id: row.get(5)?,
                project_id: row.get(6)?,
                session_id: row.get(7)?,
                task_id: row.get(8)?,
                message_id: row.get(9)?,
                input_tokens: row.get(10)?,
                output_tokens: row.get(11)?,
                total_tokens: row.get(12)?,
                call_count: row.get(13)?,
                estimated_input_cost_usd: row.get(14)?,
                estimated_output_cost_usd: row.get(15)?,
                estimated_total_cost_usd: row.get(16)?,
                pricing_status: row.get(17)?,
                pricing_version: row.get(18)?,
                occurred_at: row.get(19)?,
                created_at: row.get(20)?,
            })
        },
    )
    .map_err(|error| error.to_string())
}

/// 查询 CLI 用量统计聚合结果。
///
/// 用途：为设置页统计面板提供时间范围、粒度和维度聚合后的图表数据。
/// 主要参数：开始/结束时间、年/月/日粒度、Agent/模型维度、Provider 过滤器。
/// 返回值：返回汇总、趋势、分组明细和图表堆叠序列。
/// 关键副作用：无，仅执行只读查询。
#[tauri::command]
pub fn query_agent_cli_usage_stats(
    input: QueryAgentCliUsageStatsInput,
) -> Result<AgentCliUsageStatsResponse, String> {
    let conn = open_db_connection().map_err(|error| error.to_string())?;
    let granularity = normalize_granularity(&input.granularity);
    let dimension = normalize_dimension(&input.dimension);
    let provider_filter = normalize_provider_filter(input.provider_filter.clone());
    let (where_clause, params) = build_where_clause(&input);
    let bucket_expr = bucket_sql(&granularity);
    let (dimension_id_expr, dimension_label_expr) =
        dimension_sql(&dimension, provider_filter == "all" && dimension == "model");
    let breakdown_order_expr = breakdown_order_sql(&dimension);

    let summary_sql = format!(
        r#"
        SELECT
            COALESCE(SUM(call_count), 0),
            COALESCE(SUM(input_tokens), 0),
            COALESCE(SUM(output_tokens), 0),
            COALESCE(SUM(total_tokens), 0),
            COALESCE(SUM(estimated_total_cost_usd), 0),
            COALESCE(SUM(CASE WHEN pricing_status != 'estimated' THEN call_count ELSE 0 END), 0)
        FROM agent_cli_usage_records
        WHERE {}
        "#,
        where_clause
    );

    let summary = conn
        .query_row(&summary_sql, params_from_iter(params.iter()), |row| {
            Ok(AgentCliUsageSummary {
                total_calls: row.get(0)?,
                input_tokens: row.get(1)?,
                output_tokens: row.get(2)?,
                total_tokens: row.get(3)?,
                estimated_total_cost_usd: row.get(4)?,
                unpriced_calls: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?;

    let timeline_sql = format!(
        r#"
        SELECT
            {bucket_expr} AS bucket,
            COALESCE(SUM(call_count), 0),
            COALESCE(SUM(input_tokens), 0),
            COALESCE(SUM(output_tokens), 0),
            COALESCE(SUM(total_tokens), 0),
            COALESCE(SUM(estimated_total_cost_usd), 0)
        FROM agent_cli_usage_records
        WHERE {where_clause}
        GROUP BY bucket
        ORDER BY bucket ASC
        "#
    );

    let mut timeline_stmt = conn
        .prepare(&timeline_sql)
        .map_err(|error| error.to_string())?;
    let timeline = timeline_stmt
        .query_map(params_from_iter(params.iter()), |row| {
            let bucket: String = row.get(0)?;
            Ok(AgentCliUsageTimelinePoint {
                label: bucket.clone(),
                bucket,
                call_count: row.get(1)?,
                input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                total_tokens: row.get(4)?,
                estimated_total_cost_usd: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let breakdown_sql = format!(
        r#"
        SELECT
            {dimension_id_expr} AS dimension_id,
            {dimension_label_expr} AS dimension_label,
            MIN(provider) AS provider,
            COALESCE(SUM(call_count), 0),
            COALESCE(SUM(input_tokens), 0),
            COALESCE(SUM(output_tokens), 0),
            COALESCE(SUM(total_tokens), 0),
            COALESCE(SUM(estimated_total_cost_usd), 0),
            COALESCE(SUM(CASE WHEN pricing_status != 'estimated' THEN call_count ELSE 0 END), 0)
        FROM agent_cli_usage_records
        WHERE {where_clause}
        GROUP BY dimension_id, dimension_label
        ORDER BY {breakdown_order_expr}
        "#
    );

    let mut breakdown_stmt = conn
        .prepare(&breakdown_sql)
        .map_err(|error| error.to_string())?;
    let breakdown = breakdown_stmt
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(AgentCliUsageBreakdownRow {
                dimension_id: row.get(0)?,
                label: row.get(1)?,
                provider: row.get(2)?,
                call_count: row.get(3)?,
                input_tokens: row.get(4)?,
                output_tokens: row.get(5)?,
                total_tokens: row.get(6)?,
                estimated_total_cost_usd: row.get(7)?,
                unpriced_calls: row.get(8)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let stacked_sql = format!(
        r#"
        SELECT
            {bucket_expr} AS bucket,
            {dimension_id_expr} AS dimension_id,
            {dimension_label_expr} AS dimension_label,
            MIN(provider) AS provider,
            COALESCE(SUM(call_count), 0),
            COALESCE(SUM(input_tokens), 0),
            COALESCE(SUM(output_tokens), 0),
            COALESCE(SUM(total_tokens), 0),
            COALESCE(SUM(estimated_total_cost_usd), 0)
        FROM agent_cli_usage_records
        WHERE {where_clause}
        GROUP BY bucket, dimension_id, dimension_label
        ORDER BY bucket ASC, dimension_label ASC
        "#
    );

    let mut stacked_stmt = conn
        .prepare(&stacked_sql)
        .map_err(|error| error.to_string())?;
    let stacked_timeline = stacked_stmt
        .query_map(params_from_iter(params.iter()), |row| {
            let bucket: String = row.get(0)?;
            Ok(AgentCliUsageStackedPoint {
                label: bucket.clone(),
                bucket,
                dimension_id: row.get(1)?,
                dimension_label: row.get(2)?,
                provider: row.get(3)?,
                call_count: row.get(4)?,
                input_tokens: row.get(5)?,
                output_tokens: row.get(6)?,
                total_tokens: row.get(7)?,
                estimated_total_cost_usd: row.get(8)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let cost_partial = summary.unpriced_calls > 0;

    Ok(AgentCliUsageStatsResponse {
        summary,
        timeline,
        breakdown,
        stacked_timeline,
        meta: AgentCliUsageMeta {
            start_at: normalize_optional_text(input.start_at),
            end_at: normalize_optional_text(input.end_at),
            granularity,
            dimension,
            provider_filter,
            model_keyword: normalize_optional_text(input.model_keyword),
            pricing_version: PRICING_VERSION.to_string(),
            cost_partial,
        },
    })
}

/// 自动修复 CLI 用量历史中的错误模型归属。
///
/// 用途：在统计页加载前，纠偏旧版本把 Claude 兼容源请求错误写成 Haiku 的历史记录。
/// 主要参数：可选 Provider；当前仅对 `claude` 生效。
/// 返回值：返回目标模型、修复数量和跳过原因。
/// 关键副作用：可能更新本地 SQLite 中的 `agent_cli_usage_records` 历史数据。
#[tauri::command]
pub fn repair_agent_cli_usage_history(
    provider: Option<String>,
) -> Result<RepairAgentCliUsageHistoryResult, String> {
    let normalized_provider = normalize_provider_filter(provider);
    let conn = open_db_connection().map_err(|error| error.to_string())?;

    if normalized_provider != "all" && normalized_provider != "claude" {
        return Ok(RepairAgentCliUsageHistoryResult {
            provider: normalized_provider,
            target_model_id: None,
            updated_count: 0,
            skipped_reason: Some("provider_not_supported".to_string()),
        });
    }

    repair_claude_usage_history(&conn)
}

#[cfg(test)]
mod tests {
    use super::{estimate_pricing, resolve_model_pricing};

    #[test]
    fn resolves_gpt_5_4_pricing() {
        let pricing = resolve_model_pricing("codex", Some("gpt-5.4")).unwrap();
        assert_eq!(pricing.input_per_million_usd, 2.5);
        assert_eq!(pricing.output_per_million_usd, 15.0);
    }

    #[test]
    fn resolves_gpt_5_codex_pricing() {
        let pricing = resolve_model_pricing("codex", Some("gpt-5-codex")).unwrap();
        assert_eq!(pricing.input_per_million_usd, 1.25);
        assert_eq!(pricing.output_per_million_usd, 10.0);
    }

    #[test]
    fn resolves_known_codex_pricing() {
        let pricing = resolve_model_pricing("codex", Some("gpt-5.3-codex")).unwrap();
        assert_eq!(pricing.input_per_million_usd, 4.0);
        assert_eq!(pricing.output_per_million_usd, 16.0);
    }

    #[test]
    fn marks_unknown_models_as_unmapped() {
        let pricing = estimate_pricing("claude", Some("claude-unknown"), 1000, 1000);
        assert_eq!(pricing.pricing_status, "unmapped");
        assert!(pricing.estimated_total_cost_usd.is_none());
    }
}
