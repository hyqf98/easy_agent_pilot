export type CliFailureKind = 'retryable' | 'non_retryable'

export type CliFailureFragmentSource =
  | 'content'
  | 'error'
  | 'tool_result'
  | 'system'
  | 'stderr'

export interface CliFailureFragment {
  source: CliFailureFragmentSource
  text: string
}

export interface CliFailureMatch {
  kind: CliFailureKind
  message: string
  matchedText: string
}

const RETRYABLE_PATTERNS = [
  '429',
  'rate limit',
  'too many requests',
  'throttl',
  'quota',
  'insufficient_quota',
  'overloaded',
  'capacity',
  'temporarily unavailable',
  'temporarily_unavailable',
  'service unavailable',
  'timeout',
  'timed out',
  'network error',
  'connection reset',
  'connection refused',
  'connection aborted',
  'connection closed',
  'econnreset',
  'econnrefused',
  'etimedout',
  'api error',
  'apl error',
  '达到速率限制',
  '账户已达到速率限制',
  '请求频率',
  '请求过于频繁',
  '限流',
  '配额',
  '服务暂时不可用',
  '网络错误',
  '网络超时',
  '连接被重置'
]

const ERROR_CONTEXT_PATTERNS = [
  'api error',
  'apl error',
  'error',
  'failed',
  'fatal',
  'exception',
  'traceback',
  '"error"',
  '请求失败',
  '错误',
  '异常',
  '失败'
]

function isSharedBenignWarning(normalized: string): boolean {
  return (normalized.includes('rmcp::transport::worker')
    && normalized.includes('unexpectedcontenttype')
    && normalized.includes('missing-content-type'))
    || (normalized.includes('rmcp::transport::async_rw')
      && normalized.includes('serde error expected')
      && normalized.includes('line 1 column'))
    || (normalized.includes('rmcp::transport::worker')
      && normalized.includes('transport channel closed'))
    || (normalized.includes('failed to terminate mcp process group')
      && (normalized.includes('operation not permitted')
        || normalized.includes('os error 1')))
    || (normalized.includes('fatal: your current branch')
      && normalized.includes('does not have any commits yet'))
    || normalized.includes("fatal: bad revision 'head'")
    || normalized.includes('fatal: bad revision "head"')
    || normalized.includes("fatal: ambiguous argument 'head'")
    || normalized.includes('fatal: ambiguous argument "head"')
}

function normalizeText(value: string | null | undefined): string {
  return value?.trim().toLowerCase() || ''
}

function hasRetryablePattern(normalized: string): boolean {
  return RETRYABLE_PATTERNS.some(pattern => normalized.includes(pattern))
}

function hasErrorContext(normalized: string): boolean {
  return ERROR_CONTEXT_PATTERNS.some(pattern => normalized.includes(pattern))
}

function hasStructuredTaskResult(normalized: string): boolean {
  return normalized.includes('<task_result>') && normalized.includes('</task_result>')
}

function startsWithErrorContext(normalized: string): boolean {
  return ERROR_CONTEXT_PATTERNS.some(pattern =>
    normalized.startsWith(pattern)
    || normalized.startsWith(`${pattern}:`)
    || normalized.startsWith(`${pattern}：`)
    || normalized.startsWith(`[${pattern}]`)
  )
}

function hasStructuredErrorPayload(normalized: string): boolean {
  const trimmed = normalized.trimStart()
  return trimmed.startsWith('{"error"')
    || trimmed.startsWith("{'error'")
    || trimmed.startsWith('[{"error"')
    || trimmed.startsWith("[{'error'")
}

function isPrimaryResponseContent(normalized: string): boolean {
  return Boolean(normalized)
    && !hasStructuredTaskResult(normalized)
    && !startsWithErrorContext(normalized)
    && !hasStructuredErrorPayload(normalized)
    && !hasRetryablePattern(normalized)
}

function sourceAllowsRetryableMatch(
  source: CliFailureFragmentSource,
  normalized: string
): boolean {
  if (source === 'error' || source === 'stderr') {
    return true
  }

  if (hasStructuredTaskResult(normalized)) {
    return false
  }

  return startsWithErrorContext(normalized) || hasStructuredErrorPayload(normalized)
}

function isNonRetryableFailure(
  source: CliFailureFragmentSource,
  normalized: string
): boolean {
  if (source === 'error' || source === 'stderr') {
    return hasErrorContext(normalized)
  }

  if (hasStructuredTaskResult(normalized)) {
    return false
  }

  return startsWithErrorContext(normalized) || hasStructuredErrorPayload(normalized)
}

function stripAnsiEscapes(text: string): string {
  // eslint-disable-next-line no-control-regex
  return text.replace(/\x1b\[[0-9;]*m/g, '')
}

function buildFailureMessage(
  runtimeLabel: string,
  kind: CliFailureKind,
  matchedText: string
): CliFailureMatch {
  const cleaned = stripAnsiEscapes(matchedText).replace(/\s+/g, ' ').trim()
  return {
    kind,
    matchedText: cleaned,
    message: `${runtimeLabel} 异常完成（${kind === 'retryable' ? '可重试' : '不可重试'}）: ${cleaned}`
  }
}

export function createCliFailureFragment(
  source: CliFailureFragmentSource,
  text?: string | null
): CliFailureFragment | null {
  const normalized = text?.trim()
  if (!normalized) {
    return null
  }

  return {
    source,
    text: normalized
  }
}

export function classifyCliFailureFragments(
  runtimeLabel: string,
  fragments: CliFailureFragment[]
): CliFailureMatch | null {
  const hasPrimaryResponse = fragments.some(fragment =>
    fragment.source === 'content' && isPrimaryResponseContent(normalizeText(fragment.text))
  )

  for (const fragment of fragments) {
    const normalized = normalizeText(fragment.text)
    if (!normalized) {
      continue
    }

    if (isSharedBenignWarning(normalized)) {
      continue
    }

    if (hasPrimaryResponse && fragment.source !== 'content') {
      continue
    }

    if (hasRetryablePattern(normalized) && sourceAllowsRetryableMatch(fragment.source, normalized)) {
      return buildFailureMessage(runtimeLabel, 'retryable', fragment.text)
    }

    if (isNonRetryableFailure(fragment.source, normalized)) {
      return buildFailureMessage(runtimeLabel, 'non_retryable', fragment.text)
    }
  }

  return null
}
