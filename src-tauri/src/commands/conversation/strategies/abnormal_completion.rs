#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliCompletionFailureKind {
    Retryable,
    NonRetryable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliTextSource {
    Content,
    Error,
    ToolResult,
    System,
    Stderr,
}

#[derive(Debug, Clone)]
pub struct CliTextFragment {
    pub source: CliTextSource,
    pub text: String,
}

impl CliTextFragment {
    pub fn new(source: CliTextSource, text: impl Into<String>) -> Option<Self> {
        let normalized = text.into().trim().to_string();
        if normalized.is_empty() {
            return None;
        }

        Some(Self {
            source,
            text: normalized,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CliCompletionFailure {
    pub kind: CliCompletionFailureKind,
    pub message: String,
}

const RETRYABLE_PATTERNS: &[&str] = &[
    "429",
    "rate limit",
    "too many requests",
    "throttl",
    "quota",
    "insufficient_quota",
    "overloaded",
    "capacity",
    "temporarily unavailable",
    "temporarily_unavailable",
    "service unavailable",
    "timeout",
    "timed out",
    "network error",
    "connection reset",
    "connection refused",
    "connection aborted",
    "connection closed",
    "econnreset",
    "econnrefused",
    "etimedout",
    "api error",
    "apl error",
    "达到速率限制",
    "账户已达到速率限制",
    "请求频率",
    "请求过于频繁",
    "限流",
    "配额",
    "服务暂时不可用",
    "网络错误",
    "网络超时",
    "连接被重置",
];

const ERROR_CONTEXT_PATTERNS: &[&str] = &[
    "api error",
    "apl error",
    "error",
    "failed",
    "fatal",
    "exception",
    "traceback",
    "\"error\"",
    "请求失败",
    "错误",
    "异常",
    "失败",
];

pub fn is_shared_benign_stderr_warning(line: &str) -> bool {
    let normalized = normalize_text(line);

    (normalized.contains("rmcp::transport::worker")
        && normalized.contains("unexpectedcontenttype")
        && normalized.contains("missing-content-type"))
        || (normalized.contains("rmcp::transport::async_rw")
            && normalized.contains("serde error expected")
            && normalized.contains("line 1 column"))
        || (normalized.contains("rmcp::transport::worker")
            && normalized.contains("transport channel closed"))
        || (normalized.contains("failed to terminate mcp process group")
            && (normalized.contains("operation not permitted")
                || normalized.contains("os error 1")))
}

pub fn classify_cli_completion(
    provider: &str,
    fragments: &[CliTextFragment],
    emitted_error: bool,
) -> Option<CliCompletionFailure> {
    for fragment in fragments {
        let normalized = normalize_text(&fragment.text);
        if normalized.is_empty() {
            continue;
        }

        if is_retryable_failure(&normalized)
            && source_allows_retryable_match(fragment.source, &normalized)
        {
            return Some(build_failure(
                provider,
                CliCompletionFailureKind::Retryable,
                &fragment.text,
            ));
        }

        if is_non_retryable_failure(fragment.source, &normalized) {
            return Some(build_failure(
                provider,
                CliCompletionFailureKind::NonRetryable,
                &fragment.text,
            ));
        }
    }

    if emitted_error {
        return Some(build_failure(
            provider,
            CliCompletionFailureKind::NonRetryable,
            "CLI emitted an error event without a structured failure summary.",
        ));
    }

    None
}

fn build_failure(
    provider: &str,
    kind: CliCompletionFailureKind,
    snippet: &str,
) -> CliCompletionFailure {
    let prefix = match kind {
        CliCompletionFailureKind::Retryable => "可重试",
        CliCompletionFailureKind::NonRetryable => "不可重试",
    };

    CliCompletionFailure {
        kind,
        message: format!(
            "{provider} CLI 异常完成（{prefix}）: {}",
            snippet.trim().replace('\n', " ")
        ),
    }
}

fn normalize_text(text: &str) -> String {
    text.to_lowercase()
}

fn is_retryable_failure(normalized: &str) -> bool {
    RETRYABLE_PATTERNS
        .iter()
        .any(|pattern| normalized.contains(pattern))
}

fn contains_error_context(normalized: &str) -> bool {
    ERROR_CONTEXT_PATTERNS
        .iter()
        .any(|pattern| normalized.contains(pattern))
}

fn has_structured_task_result(normalized: &str) -> bool {
    normalized.contains("<task_result>") && normalized.contains("</task_result>")
}

fn starts_with_error_context(normalized: &str) -> bool {
    ERROR_CONTEXT_PATTERNS.iter().any(|pattern| {
        normalized.starts_with(pattern)
            || normalized.starts_with(&format!("{pattern}:"))
            || normalized.starts_with(&format!("{pattern}："))
            || normalized.starts_with(&format!("[{pattern}]"))
    })
}

fn has_structured_error_payload(normalized: &str) -> bool {
    let trimmed = normalized.trim_start();
    trimmed.starts_with("{\"error\"")
        || trimmed.starts_with("{'error'")
        || trimmed.starts_with("[{\"error\"")
        || trimmed.starts_with("[{'error'")
}

fn source_allows_retryable_match(source: CliTextSource, normalized: &str) -> bool {
    match source {
        CliTextSource::Error | CliTextSource::Stderr => true,
        CliTextSource::Content | CliTextSource::ToolResult | CliTextSource::System => {
            if has_structured_task_result(normalized) {
                return false;
            }

            starts_with_error_context(normalized) || has_structured_error_payload(normalized)
        }
    }
}

fn is_non_retryable_failure(source: CliTextSource, normalized: &str) -> bool {
    match source {
        CliTextSource::Error | CliTextSource::Stderr => contains_error_context(normalized),
        CliTextSource::Content | CliTextSource::ToolResult | CliTextSource::System => {
            if has_structured_task_result(normalized) {
                return false;
            }

            starts_with_error_context(normalized) || has_structured_error_payload(normalized)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_cli_completion, is_shared_benign_stderr_warning, CliCompletionFailureKind,
        CliTextFragment, CliTextSource,
    };

    #[test]
    fn classifies_retryable_rate_limit_payload() {
        let fragments = vec![CliTextFragment::new(
            CliTextSource::Content,
            r#"API Error: 429 {"error":{"code":"1302","message":"您的账户已达到速率限制，请您控制请求频率"}}"#,
        )
        .expect("fragment")];

        let failure =
            classify_cli_completion("OpenCode", &fragments, false).expect("should classify");

        assert_eq!(failure.kind, CliCompletionFailureKind::Retryable);
    }

    #[test]
    fn ignores_known_rmcp_warning() {
        assert!(is_shared_benign_stderr_warning(
            "rmcp::transport::worker: worker quit with fatal: Transport channel closed, when UnexpectedContentType(Missing-Content-Type)"
        ));
    }

    #[test]
    fn ignores_rmcp_process_group_termination_eperm_warning() {
        assert!(is_shared_benign_stderr_warning(
            "codex_rmcp_client::rmcp_client: Failed to terminate MCP process group 37060: Operation not permitted (os error 1)"
        ));
    }

    #[test]
    fn does_not_treat_task_result_report_as_failure() {
        let fragments = vec![CliTextFragment::new(
            CliTextSource::Content,
            "task_id: ses_xxx <task_result> Now I have a thorough understanding of the homepage architecture. File path contains error.log but this is a result report. </task_result>",
        )
        .expect("fragment")];

        let failure = classify_cli_completion("OpenCode", &fragments, false);
        assert!(failure.is_none());
    }

    #[test]
    fn does_not_treat_tool_output_source_code_as_failure() {
        let fragments = vec![CliTextFragment::new(
            CliTextSource::ToolResult,
            "command: /bin/zsh -lc 'nl -ba src/services/conversation/ConversationService.ts'\nstatus: completed\nexit_code: 0\nif (normalizedEvent.error) {\n  onError(normalizedEvent.error)\n}\nconst timeoutConfig = detectTimeout(config)\n",
        )
        .expect("fragment")];

        let failure = classify_cli_completion("Codex", &fragments, false);
        assert!(failure.is_none());
    }

    #[test]
    fn still_classifies_api_error_prefix_with_json_payload() {
        let fragments = vec![CliTextFragment::new(
            CliTextSource::ToolResult,
            r#"API Error: 429 {"error":{"code":"1302","message":"您的账户已达到速率限制，请您控制请求频率"}}"#,
        )
        .expect("fragment")];

        let failure =
            classify_cli_completion("OpenCode", &fragments, false).expect("should classify");

        assert_eq!(failure.kind, CliCompletionFailureKind::Retryable);
    }
}
