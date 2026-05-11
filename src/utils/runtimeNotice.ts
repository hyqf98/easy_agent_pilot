import { invoke } from '@tauri-apps/api/core'
import { normalizeCliCommand, type AgentConfig } from '@/stores/agent'

export interface RuntimeNotice {
  id: string
  title: string
  content: string
  tone?: 'info' | 'success' | 'warning'
}

export interface UsageNoticeSummary {
  model: string | null
  input: string | null
  output: string | null
}

export interface ProcessingTimeNoticeSummary {
  label: string | null
}

export interface ContextStrategyNoticeOptions {
  strategy: string
  runtime?: string | null
  model?: string | null
  expert?: string | null
  systemMessageCount?: number | null
  userMessageCount?: number | null
  assistantMessageCount?: number | null
  historyMessageCount?: number | null
  resumeSessionId?: string | null
}

interface RuntimeNoticeLine {
  label: string
  value: string
}

interface CliConfigScanResult {
  skills: Array<{ name: string }>
  plugins: Array<{ name: string }>
}

interface CliConfig {
  mcp_servers?: Record<string, unknown>
  mcpServers?: Record<string, unknown>
}

interface UsageSnapshot {
  model?: string
  inputTokens?: number
  outputTokens?: number
  contextWindowOccupancy?: number
}

interface RuntimeNoticeDescriptor {
  id: RuntimeNotice['id']
  matches: (notice: Pick<RuntimeNotice, 'id' | 'title'>) => boolean
  summarize: (lines: RuntimeNoticeLine[]) => string[]
}

interface CachedRuntimeNoticeEntry {
  expiresAt: number
  value: Promise<RuntimeNotice | null>
}

const ENVIRONMENT_NOTICE_CACHE_TTL_MS = 10_000
const environmentNoticeCache = new Map<string, CachedRuntimeNoticeEntry>()

function formatNameList(label: string, names: string[], maxCount: number = 5): string | null {
  if (names.length === 0) return null

  const visibleNames = names.slice(0, maxCount)
  const suffix = names.length > maxCount
    ? ` 等 ${names.length} 个`
    : ` (${names.length})`

  return `- ${label}: ${visibleNames.join('、')}${suffix}`
}

function uniqueNames(names: string[]): string[] {
  return Array.from(new Set(names.map(name => name.trim()).filter(Boolean)))
}

function parseRuntimeNoticeLines(content: string): RuntimeNoticeLine[] {
  return content
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)
    .map((line) => line.replace(/^-\s*/, ''))
    .map((line) => {
      const separatorIndex = line.indexOf(':')
      if (separatorIndex < 0) {
        return null
      }

      return {
        label: line.slice(0, separatorIndex).trim(),
        value: line.slice(separatorIndex + 1).trim()
      }
    })
    .filter((line): line is RuntimeNoticeLine => Boolean(line?.label && line.value))
}

function isRuntimeNoticeLineLabel(label: string): boolean {
  return [
    '模型',
    'model',
    '策略',
    'strategy',
    '运行时',
    'runtime',
    '专家',
    'expert',
    'system messages',
    'user messages',
    'assistant messages',
    'history messages',
    'resume session',
    '输入 Token',
    'input token',
    '输出 Token',
    'output token',
    '输入',
    '输出',
    '用时',
    '耗时',
    'elapsed',
    'duration',
    'Skills',
    'Plugins',
    'MCP',
    '当前任务',
    '状态'
  ].some(keyword => label.includes(keyword))
}

function isModelLabel(label: string): boolean {
  const normalized = label.trim().toLowerCase()
  return normalized.includes('模型') || normalized === 'model'
}

function isInputLabel(label: string): boolean {
  const normalized = label.trim().toLowerCase()
  return normalized.includes('输入') || normalized.includes('input')
}

function isOutputLabel(label: string): boolean {
  const normalized = label.trim().toLowerCase()
  return normalized.includes('输出') || normalized.includes('output')
}

function isElapsedLabel(label: string): boolean {
  const normalized = label.trim().toLowerCase()
  return normalized.includes('用时')
    || normalized.includes('耗时')
    || normalized.includes('elapsed')
    || normalized.includes('duration')
}

function formatCompactNumber(value: number): string {
  if (value >= 1000000) return `${(value / 1000000).toFixed(1).replace(/\.0$/, '')}m`
  if (value >= 1000) return `${(value / 1000).toFixed(1).replace(/\.0$/, '')}k`
  return String(value)
}

function extractListCount(value: string): number | null {
  const match = value.match(/等\s+(\d+)\s+个|\((\d+)\)$/)
  if (!match) {
    return null
  }

  return Number(match[1] || match[2] || 0) || null
}

const runtimeNoticeDescriptors: RuntimeNoticeDescriptor[] = [
  {
    id: 'context',
    matches: (notice) => notice.id === 'context' || /上下文策略|context strategy/i.test(notice.title),
    summarize: (lines) => lines
      .map((line) => {
        const normalized = line.label.trim().toLowerCase()
        if (normalized.includes('策略') || normalized.includes('strategy')) {
          return line.value
        }
        if (normalized.includes('运行时') || normalized.includes('runtime')) {
          return line.value
        }
        if (normalized.includes('resume session')) {
          return `Resume ${line.value.slice(0, 8)}`
        }
        if (normalized.includes('system')) {
          return `Sys ${line.value}`
        }
        if (normalized.includes('user')) {
          return `User ${line.value}`
        }
        if (normalized.includes('history')) {
          return `Hist ${line.value}`
        }
        return `${line.label} ${line.value}`.trim()
      })
      .filter(Boolean)
      .slice(0, 4)
  },
  {
    id: 'environment',
    matches: (notice) => notice.id === 'environment' || notice.title.includes('运行扩展'),
    summarize: (lines) => lines
      .map((line) => {
        const count = extractListCount(line.value)
        if (count !== null) {
          return `${line.label} ${count}`
        }

        const preview = line.value.split('、')[0]?.trim()
        return preview ? `${line.label} ${preview}` : line.label
      })
      .slice(0, 5)
  },
  {
    id: 'usage',
    matches: (notice) => notice.id === 'usage' || notice.title.includes('用量'),
    summarize: (lines) => lines
      .map((line) => {
        if (isModelLabel(line.label)) {
          return line.value
        }

        const numeric = Number(line.value.replace(/[^\d]/g, ''))
        if (Number.isFinite(numeric) && numeric > 0) {
          const prefix = isInputLabel(line.label) ? 'In' : isOutputLabel(line.label) ? 'Out' : line.label
          return `${prefix} ${formatCompactNumber(numeric)}`
        }

        return `${line.label} ${line.value}`.trim()
      })
      .slice(0, 3)
  },
  {
    id: 'processing-time',
    matches: (notice) =>
      notice.id === 'processing-time'
      || /处理用时|processing time/i.test(notice.title),
    summarize: (lines) => lines
      .map((line) => line.value || `${line.label}`.trim())
      .filter(Boolean)
      .slice(0, 1)
  }
]

function resolveRuntimeNoticeDescriptor(
  notice: Pick<RuntimeNotice, 'id' | 'title'>
): RuntimeNoticeDescriptor | null {
  return runtimeNoticeDescriptors.find((descriptor) => descriptor.matches(notice)) ?? null
}

export function isContextRuntimeNotice(
  notice: Pick<RuntimeNotice, 'id' | 'title'>
): boolean {
  return resolveRuntimeNoticeDescriptor(notice)?.id === 'context'
}

export function isProcessingTimeRuntimeNotice(
  notice: Pick<RuntimeNotice, 'id' | 'title'>
): boolean {
  return resolveRuntimeNoticeDescriptor(notice)?.id === 'processing-time'
}

export function isEnvironmentRuntimeNotice(
  notice: Pick<RuntimeNotice, 'id' | 'title'>
): boolean {
  return resolveRuntimeNoticeDescriptor(notice)?.id === 'environment'
}

function inferRuntimeNoticeId(title: string): string {
  return resolveRuntimeNoticeDescriptor({ id: '', title })?.id ?? 'system'
}

export async function buildCliEnvironmentNotice(agent: AgentConfig): Promise<RuntimeNotice | null> {
  if (agent.type !== 'cli') {
    return null
  }

  const cliPath = normalizeCliCommand(agent.cliPath) || agent.provider
  const cliType = agent.provider

  if (!cliPath || !cliType) {
    return null
  }

  const cacheKey = `${cliType}:${cliPath}`
  const now = Date.now()
  const cachedEntry = environmentNoticeCache.get(cacheKey)
  if (cachedEntry && cachedEntry.expiresAt > now) {
    const notice = await cachedEntry.value
    return notice ? { ...notice } : null
  }

  const loader = (async (): Promise<RuntimeNotice | null> => {
    try {
      const [scanResult, cliConfig] = await Promise.all([
        invoke<CliConfigScanResult>('scan_cli_config', { cliPath, cliType }),
        invoke<CliConfig>('read_cli_config', { cliPath, cliType })
      ])

      const skillNames = uniqueNames(scanResult.skills.map(skill => skill.name))
      const pluginNames = uniqueNames(scanResult.plugins.map(plugin => plugin.name))
      const mcpNames = uniqueNames(Object.keys(cliConfig.mcp_servers || cliConfig.mcpServers || {}))

      const lines = [
        formatNameList('Skills', skillNames),
        formatNameList('Plugins', pluginNames),
        formatNameList('MCP', mcpNames)
      ].filter(Boolean) as string[]

      if (lines.length === 0) {
        return null
      }

      return {
        id: 'environment',
        title: '已加载运行扩展',
        content: lines.join('\n'),
        tone: 'info'
      }
    } catch (error) {
      console.warn('[runtimeNotice] Failed to build CLI environment notice:', error)
      return null
    }
  })()

  environmentNoticeCache.set(cacheKey, {
    expiresAt: now + ENVIRONMENT_NOTICE_CACHE_TTL_MS,
    value: loader
  })

  try {
    const notice = await loader
    return notice ? { ...notice } : null
  } catch (error) {
    environmentNoticeCache.delete(cacheKey)
    throw error
  }
}

export function buildRuntimeNoticeFromSystemContent(content?: string | null): RuntimeNotice | null {
  const trimmed = content?.trim()
  if (!trimmed) {
    return null
  }

  const lines = trimmed.split('\n')
  const headerLine = lines[0]?.trim()
  const hasMarkdownHeader = Boolean(headerLine?.startsWith('### '))
  const title = hasMarkdownHeader
    ? headerLine.replace(/^###\s+/, '').trim()
    : '运行状态'
  const body = hasMarkdownHeader ? lines.slice(1).join('\n').trim() : trimmed

  if (!title || !body) {
    return null
  }

  if (!hasMarkdownHeader) {
    const parsedLines = parseRuntimeNoticeLines(body)
    if (parsedLines.length === 0 || !parsedLines.some(line => isRuntimeNoticeLineLabel(line.label))) {
      return null
    }
  }

  return {
    id: inferRuntimeNoticeId(title),
    title,
    content: body,
    tone: 'info'
  }
}

export function buildContextStrategyNotice(
  options: ContextStrategyNoticeOptions
): RuntimeNotice | null {
  const lines = [
    options.strategy ? `- 策略: ${options.strategy}` : null,
    options.runtime?.trim() ? `- 运行时: ${options.runtime.trim()}` : null,
    options.model?.trim() ? `- 模型: ${options.model.trim()}` : null,
    options.expert?.trim() ? `- 专家: ${options.expert.trim()}` : null,
    typeof options.systemMessageCount === 'number' ? `- System Messages: ${options.systemMessageCount}` : null,
    typeof options.userMessageCount === 'number' ? `- User Messages: ${options.userMessageCount}` : null,
    typeof options.assistantMessageCount === 'number' ? `- Assistant Messages: ${options.assistantMessageCount}` : null,
    typeof options.historyMessageCount === 'number' ? `- History Messages: ${options.historyMessageCount}` : null,
    options.resumeSessionId?.trim() ? `- Resume Session: ${options.resumeSessionId.trim()}` : null
  ].filter(Boolean) as string[]

  if (lines.length === 0) {
    return null
  }

  return {
    id: 'context',
    title: '上下文策略',
    content: lines.join('\n'),
    tone: 'info'
  }
}

export function formatRuntimeNoticeAsSystemContent(notice: RuntimeNotice | null): string {
  if (!notice) {
    return ''
  }

  return `### ${notice.title}\n${notice.content}`.trim()
}

export function summarizeRuntimeNotice(notice: RuntimeNotice): string[] {
  const lines = parseRuntimeNoticeLines(notice.content)
  const descriptor = resolveRuntimeNoticeDescriptor(notice)
  if (descriptor) {
    return descriptor.summarize(lines)
  }

  return lines
    .map(line => `${line.label} ${line.value}`.trim())
    .filter(Boolean)
    .slice(0, 3)
}

export function getUsageNoticeSummary(notice: RuntimeNotice): UsageNoticeSummary | null {
  if (resolveRuntimeNoticeDescriptor(notice)?.id !== 'usage') {
    return null
  }

  const lines = parseRuntimeNoticeLines(notice.content)
  let model: string | null = null
  let input: string | null = null
  let output: string | null = null

  lines.forEach((line) => {
    if (isModelLabel(line.label)) {
      model = line.value || null
      return
    }

    const numeric = Number(line.value.replace(/[^\d]/g, ''))
    const formatted = Number.isFinite(numeric) && numeric > 0
      ? formatCompactNumber(numeric)
      : (line.value || null)

    if (isInputLabel(line.label)) {
      input = formatted
      return
    }

    if (isOutputLabel(line.label)) {
      output = formatted
    }
  })

  return { model, input, output }
}

function extractNoticeModelValue(notice: RuntimeNotice): string | null {
  const usageSummary = getUsageNoticeSummary(notice)
  if (usageSummary?.model?.trim()) {
    return usageSummary.model.trim()
  }

  const lines = parseRuntimeNoticeLines(notice.content)
  const matchedLine = lines.find(line => isModelLabel(line.label) && line.value.trim())
  return matchedLine?.value.trim() || null
}

export function resolveRuntimeNoticeModel(notices?: RuntimeNotice[] | null): string | null {
  if (!notices?.length) {
    return null
  }

  const usageNotice = notices.find(notice => resolveRuntimeNoticeDescriptor(notice)?.id === 'usage')
  const usageNoticeModel = usageNotice ? extractNoticeModelValue(usageNotice) : null

  if (usageNoticeModel) {
    return usageNoticeModel
  }

  for (let index = notices.length - 1; index >= 0; index -= 1) {
    const model = extractNoticeModelValue(notices[index])
    if (model) {
      return model
    }
  }

  return null
}

export function buildUsageNotice(usage: UsageSnapshot): RuntimeNotice | null {
  const inputTokens = typeof usage.inputTokens === 'number' ? usage.inputTokens : undefined
  const outputTokens = typeof usage.outputTokens === 'number' ? usage.outputTokens : undefined
  const model = usage.model?.trim()
  const occupancy = typeof usage.contextWindowOccupancy === 'number' && usage.contextWindowOccupancy > 0
    ? usage.contextWindowOccupancy
    : undefined

  if (!model && inputTokens === undefined && outputTokens === undefined && occupancy === undefined) {
    return null
  }

  const lines = [
    model ? `- 模型: ${model}` : null,
    inputTokens !== undefined ? `- 输入 Tokens: ${inputTokens}` : null,
    outputTokens !== undefined ? `- 输出 Tokens: ${outputTokens}` : null,
    occupancy !== undefined && occupancy !== inputTokens ? `- 上下文占用 Tokens: ${occupancy}` : null
  ].filter(Boolean) as string[]

  return {
    id: 'usage',
    title: '模型与用量',
    content: lines.join('\n'),
    tone: 'info'
  }
}

export function formatProcessingDuration(durationMs: number | null | undefined): string | null {
  if (durationMs === null || durationMs === undefined || !Number.isFinite(durationMs)) {
    return null
  }

  const normalizedDurationMs = Math.max(0, durationMs)
  if (normalizedDurationMs < 250) {
    return null
  }

  if (normalizedDurationMs < 1_000) {
    return `${Math.round(normalizedDurationMs)}ms`
  }

  if (normalizedDurationMs < 60_000) {
    return `${(normalizedDurationMs / 1_000).toFixed(normalizedDurationMs >= 10_000 ? 0 : 1)}s`
  }

  const minutes = Math.floor(normalizedDurationMs / 60_000)
  const seconds = Math.round((normalizedDurationMs % 60_000) / 1_000)
  return `${minutes}m ${seconds}s`
}

export function buildProcessingTimeNotice(durationMs: number | null | undefined): RuntimeNotice | null {
  const label = formatProcessingDuration(durationMs)
  if (!label) {
    return null
  }

  return {
    id: 'processing-time',
    title: '处理用时',
    content: `- 用时: ${label}`,
    tone: 'info'
  }
}

export function upsertRuntimeNotice(
  notices: RuntimeNotice[] | undefined,
  nextNotice: RuntimeNotice | null
): RuntimeNotice[] | undefined {
  const current = notices ? [...notices] : []

  if (!nextNotice) {
    return current.length > 0 ? current : undefined
  }

  const existingIndex = current.findIndex(notice => notice.id === nextNotice.id)
  if (existingIndex >= 0) {
    current[existingIndex] = nextNotice
  } else {
    current.push(nextNotice)
  }

  return current
}

export function getProcessingTimeNoticeSummary(
  notice: RuntimeNotice
): ProcessingTimeNoticeSummary | null {
  if (!isProcessingTimeRuntimeNotice(notice)) {
    return null
  }

  const lines = parseRuntimeNoticeLines(notice.content)
  for (const line of lines) {
    if (isElapsedLabel(line.label)) {
      return { label: line.value || null }
    }
  }

  return { label: notice.content.trim() || null }
}
