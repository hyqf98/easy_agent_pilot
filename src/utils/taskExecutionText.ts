import type { Task } from '@/types/plan'
import type {
  ExecutionLogEntry,
  PlanExecutionProgress,
  TaskExecutionResultRecord,
  TaskExecutionState
} from '@/types/taskExecution'
import i18n from '@/i18n'
import { extractExecutionResult } from '@/utils/structuredContent'
import { buildPlanExecutionSnapshot } from '@/utils/planExecutionProgress'

const MAX_RESULT_SUMMARY_LENGTH = 180
const MAX_RESUME_LOG_LINES = 50
const MAX_RESUME_CONTEXT_LENGTH = 9000

function t(key: string, params?: Record<string, unknown>): string {
  return params ? i18n.global.t(key, params) as string : i18n.global.t(key) as string
}

function getCurrentLocale(): string {
  return i18n.mode === 'legacy'
    ? String((i18n.global as any).locale)
    : String((i18n.global.locale as any).value)
}

function buildFormRequestExample(): string {
  return getCurrentLocale() === 'en-US'
    ? '<form-request>\n{"type":"form_request","question":"Describe the missing information","formSchema":{"formId":"id","title":"Title","fields":[{"name":"field","label":"Label","type":"select","suggestion":"recommended value","allowOther":true,"otherLabel":"Other (custom)","options":[{"label":"Option A","value":"a"},{"label":"Option B","value":"b"}]}]}}\n</form-request>'
    : '<form-request>\n{"type":"form_request","question":"问题描述","formSchema":{"formId":"id","title":"标题","fields":[{"name":"字段","label":"标签","type":"select","suggestion":"推荐值","allowOther":true,"otherLabel":"其他（自定义）","options":[{"label":"选项A","value":"a"},{"label":"选项B","value":"b"}]}]}}\n</form-request>'
}

function buildResultExample(): string {
  return getCurrentLocale() === 'en-US'
    ? '{"result_summary":"Summarize the execution result in 1-3 sentences","generated_files":["src/new-file.ts"],"modified_files":["src/task.ts:42","src/view.vue#L88"],"deleted_files":[]}'
    : '{"result_summary":"1-3句总结本次执行结果","generated_files":["src/new-file.ts"],"modified_files":["src/task.ts:42","src/view.vue#L88"],"deleted_files":[]}'
}

export function parseExecutionResult(content: string): { summary: string; files: string[] } {
  const trimmed = content.trim()
  if (!trimmed) {
    return {
      summary: t('prompts.taskExecution.noDetailedOutput'),
      files: []
    }
  }

  const parsedResult = extractExecutionResult(trimmed)
  if (parsedResult) {
    return {
      summary: compactExecutionSummary(parsedResult.summary || fallbackSummary(trimmed)),
      files: uniqueStrings([
        ...parsedResult.generatedFiles.map(file => `added:${file}`),
        ...parsedResult.modifiedFiles.map(file => `modified:${file}`),
        ...parsedResult.changedFiles.map(file => `changed:${file}`),
        ...parsedResult.deletedFiles.map(file => `deleted:${file}`)
      ])
    }
  }

  return {
    summary: compactExecutionSummary(fallbackSummary(trimmed)),
    files: uniqueStrings(extractFileLinks(trimmed).map(file => `changed:${file}`))
  }
}

export function buildExecutionPrompt(
  task: Task,
  recentResults: TaskExecutionResultRecord[] = [],
  planProgress: PlanExecutionProgress | null = null,
  resumeContext?: string
): string {
  const parts: string[] = []

  const recentContext = buildRecentResultsContext(recentResults)
  if (recentContext) {
    parts.push(recentContext)
    parts.push('')
  }

  const planProgressContext = buildPlanProgressContext(planProgress, task.id)
  if (planProgressContext) {
    parts.push(planProgressContext)
    parts.push('')
  }

  if (resumeContext) {
    parts.push(t('prompts.taskExecution.resumeContext'))
    parts.push(resumeContext)
    parts.push('')
  }

  parts.push(t('prompts.taskExecution.taskHeading'))
  parts.push(`${t('prompts.taskExecution.title')}: ${task.title}`)
  parts.push('')

  if (task.description) {
    parts.push(`${t('prompts.taskExecution.description')}:`)
    parts.push(task.description)
    parts.push('')
  }

  if (task.implementationSteps && task.implementationSteps.length > 0) {
    parts.push(`${t('prompts.taskExecution.implementationSteps')}:`)
    task.implementationSteps.forEach((step, index) => {
      parts.push(`${index + 1}. ${step}`)
    })
    parts.push('')
  }

  if (task.testSteps && task.testSteps.length > 0) {
    parts.push(`${t('prompts.taskExecution.testSteps')}:`)
    task.testSteps.forEach((step, index) => {
      parts.push(`${index + 1}. ${step}`)
    })
    parts.push('')
  }

  if (task.acceptanceCriteria && task.acceptanceCriteria.length > 0) {
    parts.push(`${t('prompts.taskExecution.acceptanceCriteria')}:`)
    task.acceptanceCriteria.forEach(criteria => {
      parts.push(`- [ ] ${criteria}`)
    })
    parts.push('')
  }

  if (task.inputResponse && Object.keys(task.inputResponse).length > 0) {
    parts.push(`${t('prompts.taskExecution.userSupplement')}:`)
    Object.entries(task.inputResponse).forEach(([key, value]) => {
      parts.push(`- ${key}: ${typeof value === 'object' ? JSON.stringify(value) : value}`)
    })
    parts.push('')
  }

  parts.push('执行要求:')
  parts.push('1. 基于当前工作区和已有上下文继续推进，不要重复已经完成的工作。')
  parts.push('2. 如果缺少继续执行所必需的信息，只输出一个 <form-request> 包裹的 form_request JSON。')
  parts.push('```json')
  parts.push(buildFormRequestExample())
  parts.push('```')
  parts.push('3. 完成后只输出一个 `result` JSON，用于记录任务结果。')
  parts.push('```json')
  parts.push(buildResultExample())
  parts.push('```')
  parts.push('4. `result_summary` 用 1-3 句话交代结果、关键变更和失败原因。')
  parts.push('5. `generated_files` / `modified_files` / `changed_files` / `deleted_files` 尽量填写真实文件路径；如果已知修改位置，请把行号一起带上，例如 `src/task.ts:42` 或 `src/view.vue#L88`。')

  return parts.join('\n')
}

export function buildResumeExecutionContext(state: TaskExecutionState | undefined): string {
  if (!state || state.logs.length === 0) {
    return ''
  }

  const lines = state.logs
    .slice(-MAX_RESUME_LOG_LINES)
    .map(formatResumeLogLine)
    .filter(Boolean)

  const transcript = lines.join('\n').trim()
  if (!transcript) {
    return ''
  }

  if (transcript.length <= MAX_RESUME_CONTEXT_LENGTH) {
    return transcript
  }

  return transcript.slice(transcript.length - MAX_RESUME_CONTEXT_LENGTH)
}

function buildRecentResultsContext(results: TaskExecutionResultRecord[]): string {
  if (results.length === 0) return ''

  const lines: string[] = [t('prompts.taskExecution.recentResults'), '']

  results.slice(-4).forEach((result, index) => {
    const status = result.result_status === 'success'
      ? t('prompts.taskExecution.success')
      : t('prompts.taskExecution.failed')
    lines.push(`${index + 1}. [${status}] ${result.task_title_snapshot}`)
    if (result.result_summary) {
      lines.push(`   ${t('prompts.taskExecution.summary')}: ${fallbackSummary(result.result_summary)}`)
    }
    if (result.fail_reason) {
      lines.push(`   ${t('prompts.taskExecution.failure')}: ${result.fail_reason}`)
    }
  })

  return lines.join('\n')
}

function formatResumeLogLine(log: ExecutionLogEntry): string {
  const content = log.content.trim()
  if (!content) {
    if (log.type === 'thinking_start') {
      return `[thinking] ${t('prompts.taskExecution.thinkingStarted')}`
    }
    return ''
  }

  const normalizedContent = content.replace(/\s+/g, ' ').trim()

  switch (log.type) {
    case 'content':
      return `[assistant] ${normalizedContent}`
    case 'thinking':
    case 'thinking_start':
      return `[thinking] ${normalizedContent}`
    case 'tool_use':
      return `[tool:${log.metadata?.toolName || 'unknown'}] ${normalizedContent}`
    case 'tool_input_delta':
      return `[tool-input] ${normalizedContent}`
    case 'tool_result':
      return `[tool-result] ${normalizedContent}`
    case 'error':
      return `[error] ${normalizedContent}`
    case 'system':
    default:
      return `[system] ${normalizedContent}`
  }
}

function fallbackSummary(content: string): string {
  const normalized = content.replace(/\s+/g, ' ').trim()
  if (normalized.length <= MAX_RESULT_SUMMARY_LENGTH) {
    return normalized
  }
  return `${normalized.slice(0, MAX_RESULT_SUMMARY_LENGTH)}...`
}

export function compactExecutionSummary(summary: string): string {
  const normalized = summary
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()

  if (!normalized) {
    return t('prompts.taskExecution.noDetailedOutput')
  }

  if (normalized.length <= MAX_RESULT_SUMMARY_LENGTH) {
    return normalized
  }

  return `${normalized.slice(0, MAX_RESULT_SUMMARY_LENGTH)}...`
}

function buildPlanProgressContext(
  planProgress: PlanExecutionProgress | null,
  taskId: string
): string {
  if (!planProgress || planProgress.tasks.length === 0) {
    return ''
  }

  const snapshot = buildPlanExecutionSnapshot(planProgress, taskId)
  const lines: string[] = [t('prompts.taskExecution.planProgress'), '']
  const failedCount = snapshot.failedTasks.length

  lines.push(
    `- ${t('prompts.taskExecution.totalTasksLine', {
      total: snapshot.totalTasks,
      completed: snapshot.completedTasks.length,
      inProgress: planProgress.in_progress_count,
      blocked: planProgress.blocked_count,
      failed: failedCount,
      pending: planProgress.pending_count
    })}`
  )

  if (snapshot.currentTaskIndex) {
    lines.push(`- ${t('prompts.taskExecution.currentTaskLine', {
      current: snapshot.currentTaskIndex,
      total: snapshot.totalTasks
    })}`)
  }

  if (planProgress.execution_overview?.trim()) {
    lines.push('', t('taskBoard.planOverview.title'))
    lines.push(planProgress.execution_overview.trim())
  }

  if (snapshot.completedTasks.length > 0) {
    lines.push('', t('prompts.taskExecution.completedSection'))
    snapshot.completedTasks.slice(-3).forEach((item) => {
      const expert = item.expert_id ? ` [${item.expert_id}]` : ''
      const summary = compactExecutionSummary(item.last_result_summary || t('prompts.taskExecution.noSummary'))
      const files = (item.last_result_files ?? []).length > 0
        ? ` | ${t('prompts.taskExecution.files')}: ${(item.last_result_files ?? []).slice(0, 5).join(', ')}`
        : ''
      lines.push(`- ${item.title}${expert}: ${summary}${files}`)
    })
  }

  if (snapshot.failedTasks.length > 0) {
    lines.push('', t('prompts.taskExecution.failedSection'))
    snapshot.failedTasks.slice(-2).forEach((item) => {
      const expert = item.expert_id ? ` [${item.expert_id}]` : ''
      const reason = item.last_fail_reason || item.last_result_summary || t('prompts.taskExecution.noFailureReason')
      lines.push(`- ${item.title}${expert}: ${compactExecutionSummary(reason)}`)
    })
  }

  const fileLines = formatPlanFileChanges(snapshot.fileGroups)
  if (fileLines.length > 0) {
    lines.push('', t('prompts.taskExecution.filesSection'), ...fileLines)
  }

  return lines.join('\n')
}

function formatPlanFileChanges(fileGroups: ReturnType<typeof buildPlanExecutionSnapshot>['fileGroups']): string[] {
  return [
    formatFileGroupLine(t('prompts.taskExecution.fileAdded'), fileGroups.generatedFiles),
    formatFileGroupLine(t('prompts.taskExecution.fileModified'), fileGroups.modifiedFiles),
    formatFileGroupLine(t('prompts.taskExecution.fileChanged'), fileGroups.changedFiles),
    formatFileGroupLine(t('prompts.taskExecution.fileDeleted'), fileGroups.deletedFiles)
  ].filter(Boolean) as string[]
}

function formatFileGroupLine(label: string, files: string[]): string | null {
  if (files.length === 0) {
    return null
  }

  const visible = files.slice(0, 5)
  const overflow = files.length - visible.length
  const suffix = overflow > 0 ? t('prompts.taskExecution.fileOverflowSuffix', { count: files.length }) : ''
  return `- ${label}: ${visible.join('、')}${suffix}`
}

function extractFileLinks(content: string): string[] {
  const files: string[] = []

  for (const match of content.matchAll(/\[[^\]]+\]\(([^)]+)\)/g)) {
    if (match[1]) {
      files.push(match[1].trim())
    }
  }

  for (const match of content.matchAll(/`([^`\n]+(?:\/|\\)[^`\n]+)`/g)) {
    if (match[1]) {
      files.push(match[1].trim())
    }
  }

  return files
}

function uniqueStrings(values: string[]): string[] {
  return Array.from(new Set(values.filter(Boolean)))
}
