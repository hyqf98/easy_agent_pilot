import type { Task } from '@/types/plan'
import type {
  ExecutionLogEntry,
  PlanExecutionProgress,
  TaskExecutionResultRecord,
  TaskExecutionState
} from '@/types/taskExecution'
import { extractExecutionResult } from '@/utils/structuredContent'
import { buildPlanExecutionSnapshot } from '@/utils/planExecutionProgress'

const MAX_RESULT_SUMMARY_LENGTH = 180
const MAX_RESUME_LOG_LINES = 50
const MAX_RESUME_CONTEXT_LENGTH = 9000

export function parseExecutionResult(content: string): { summary: string; files: string[] } {
  const trimmed = content.trim()
  if (!trimmed) {
    return {
      summary: '任务已执行完成（无详细输出）',
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
    parts.push('## 恢复上下文')
    parts.push(resumeContext)
    parts.push('')
  }

  parts.push('# 任务')
  parts.push(`标题: ${task.title}`)
  parts.push('')

  if (task.description) {
    parts.push('描述:')
    parts.push(task.description)
    parts.push('')
  }

  if (task.implementationSteps && task.implementationSteps.length > 0) {
    parts.push('实现步骤:')
    task.implementationSteps.forEach((step, index) => {
      parts.push(`${index + 1}. ${step}`)
    })
    parts.push('')
  }

  if (task.testSteps && task.testSteps.length > 0) {
    parts.push('测试步骤:')
    task.testSteps.forEach((step, index) => {
      parts.push(`${index + 1}. ${step}`)
    })
    parts.push('')
  }

  if (task.acceptanceCriteria && task.acceptanceCriteria.length > 0) {
    parts.push('验收标准:')
    task.acceptanceCriteria.forEach(criteria => {
      parts.push(`- [ ] ${criteria}`)
    })
    parts.push('')
  }

  if (task.inputResponse && Object.keys(task.inputResponse).length > 0) {
    parts.push('用户补充:')
    Object.entries(task.inputResponse).forEach(([key, value]) => {
      parts.push(`- ${key}: ${typeof value === 'object' ? JSON.stringify(value) : value}`)
    })
    parts.push('')
  }

  parts.push('要求:')
  parts.push('- 基于已有上下文继续，不重复已完成步骤。')
  parts.push('- 需要用户补充信息时，只输出 JSON：')
  parts.push('```json')
  parts.push('{"type":"form_request","question":"问题描述","formSchema":{"formId":"id","title":"标题","fields":[{"name":"字段","label":"标签","type":"text"}]}}')
  parts.push('```')
  parts.push('- 完成后，只输出 JSON：')
  parts.push('```json')
  parts.push('{"result_summary":"1-3句总结本次执行结果","generated_files":[],"modified_files":[],"deleted_files":[]}')
  parts.push('```')
  parts.push('- result_summary 只写结果、关键改动和遗留风险。')

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

  const lines: string[] = ['## 最近结果', '']

  results.slice(-4).forEach((result, index) => {
    const status = result.result_status === 'success' ? '成功' : '失败'
    lines.push(`${index + 1}. [${status}] ${result.task_title_snapshot}`)
    if (result.result_summary) {
      lines.push(`   摘要: ${fallbackSummary(result.result_summary)}`)
    }
    if (result.fail_reason) {
      lines.push(`   失败: ${result.fail_reason}`)
    }
  })

  return lines.join('\n')
}

function formatResumeLogLine(log: ExecutionLogEntry): string {
  const content = log.content.trim()
  if (!content) {
    if (log.type === 'thinking_start') {
      return '[thinking] 思考开始'
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
    return '任务已执行完成（无详细输出）'
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
  const lines: string[] = ['## 计划进度', '']
  const failedCount = snapshot.failedTasks.length

  lines.push(
    `- 总任务: ${snapshot.totalTasks}，已完成 ${snapshot.completedTasks.length}，进行中 ${planProgress.in_progress_count}，阻塞 ${planProgress.blocked_count}，失败 ${failedCount}，待执行 ${planProgress.pending_count}`
  )

  if (snapshot.currentTaskIndex) {
    lines.push(`- 当前任务: 第 ${snapshot.currentTaskIndex}/${snapshot.totalTasks} 个`)
  }

  if (snapshot.completedTasks.length > 0) {
    lines.push('', '已完成:')
    snapshot.completedTasks.slice(-3).forEach((item) => {
      lines.push(`- ${item.title}: ${compactExecutionSummary(item.last_result_summary || '暂无摘要')}`)
    })
  }

  if (snapshot.failedTasks.length > 0) {
    lines.push('', '失败:')
    snapshot.failedTasks.slice(-2).forEach((item) => {
      const reason = item.last_fail_reason || item.last_result_summary || '暂无失败原因'
      lines.push(`- ${item.title}: ${compactExecutionSummary(reason)}`)
    })
  }

  const fileLines = formatPlanFileChanges(snapshot.fileGroups)
  if (fileLines.length > 0) {
    lines.push('', '文件:', ...fileLines)
  }

  return lines.join('\n')
}

function formatPlanFileChanges(fileGroups: ReturnType<typeof buildPlanExecutionSnapshot>['fileGroups']): string[] {
  return [
    formatFileGroupLine('新增', fileGroups.generatedFiles),
    formatFileGroupLine('修改', fileGroups.modifiedFiles),
    formatFileGroupLine('变更', fileGroups.changedFiles),
    formatFileGroupLine('删除', fileGroups.deletedFiles)
  ].filter(Boolean) as string[]
}

function formatFileGroupLine(label: string, files: string[]): string | null {
  if (files.length === 0) {
    return null
  }

  const visible = files.slice(0, 5)
  const overflow = files.length - visible.length
  const suffix = overflow > 0 ? ` 等 ${files.length} 个` : ''
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
