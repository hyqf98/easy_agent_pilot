import type { Task } from '@/types/plan'
import type { PlanExecutionProgress, TaskExecutionResultRecord } from '@/types/taskExecution'
import { extractExecutionResult } from '@/utils/structuredContent'
import { buildPlanExecutionSnapshot } from '@/utils/planExecutionProgress'

const MAX_RESULT_SUMMARY_LENGTH = 180

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
  planProgress: PlanExecutionProgress | null = null
): string {
  const parts: string[] = []

  parts.push('# 任务执行')
  parts.push('')

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

  parts.push('## 任务标题')
  parts.push(task.title)
  parts.push('')

  if (task.description) {
    parts.push('## 任务描述')
    parts.push(task.description)
    parts.push('')
  }

  if (task.implementationSteps && task.implementationSteps.length > 0) {
    parts.push('## 实现步骤')
    task.implementationSteps.forEach((step, index) => {
      parts.push(`${index + 1}. ${step}`)
    })
    parts.push('')
  }

  if (task.testSteps && task.testSteps.length > 0) {
    parts.push('## 测试步骤')
    task.testSteps.forEach((step, index) => {
      parts.push(`${index + 1}. ${step}`)
    })
    parts.push('')
  }

  if (task.acceptanceCriteria && task.acceptanceCriteria.length > 0) {
    parts.push('## 验收标准')
    task.acceptanceCriteria.forEach(criteria => {
      parts.push(`- [ ] ${criteria}`)
    })
    parts.push('')
  }

  if (task.inputResponse && Object.keys(task.inputResponse).length > 0) {
    parts.push('## 用户输入')
    parts.push('用户已提供以下信息：')
    Object.entries(task.inputResponse).forEach(([key, value]) => {
      parts.push(`- ${key}: ${typeof value === 'object' ? JSON.stringify(value) : value}`)
    })
    parts.push('')
  }

  parts.push('---')
  parts.push('')
  parts.push('请按照以上要求执行任务。')
  parts.push('')
  parts.push('**如需用户输入**，输出 JSON：')
  parts.push('```json')
  parts.push('{"type":"form_request","question":"问题描述","formSchema":{"formId":"id","title":"标题","fields":[{"name":"字段","label":"标签","type":"text"}]}}')
  parts.push('```')
  parts.push('')
  parts.push('**任务完成时**，输出 JSON：')
  parts.push('```json')
  parts.push('{"result_summary":"1到3句简洁摘要，不重复日志","generated_files":[],"modified_files":[],"deleted_files":[]}')
  parts.push('```')
  parts.push('')
  parts.push('result_summary 会被写入计划整体进度，请只保留对后续任务有帮助的关键信息。')

  return parts.join('\n')
}

function buildRecentResultsContext(results: TaskExecutionResultRecord[]): string {
  if (results.length === 0) return ''

  const lines: string[] = ['## 历史任务（参考）', '']

  results.forEach((result, index) => {
    const status = result.result_status === 'success' ? '✓' : '✗'
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
  const lines: string[] = ['## 当前计划整体进度', '']
  const failedCount = snapshot.failedTasks.length

  lines.push(
    `- 任务总数: ${snapshot.totalTasks}，已完成 ${snapshot.completedTasks.length}，进行中 ${planProgress.in_progress_count}，阻塞 ${planProgress.blocked_count}，失败 ${failedCount}，待办 ${planProgress.pending_count}`
  )

  if (snapshot.currentTaskIndex) {
    lines.push(`- 当前任务位置: 第 ${snapshot.currentTaskIndex}/${snapshot.totalTasks} 个`)
  }

  if (snapshot.completedTasks.length > 0) {
    lines.push('', '### 已完成任务摘要')
    snapshot.completedTasks.slice(-5).forEach((item) => {
      lines.push(`- ${item.title}: ${compactExecutionSummary(item.last_result_summary || '已完成')}`)
    })
  }

  if (snapshot.failedTasks.length > 0) {
    lines.push('', '### 已失败任务')
    snapshot.failedTasks.slice(-3).forEach((item) => {
      const reason = item.last_fail_reason || item.last_result_summary || '失败原因未记录'
      lines.push(`- ${item.title}: ${compactExecutionSummary(reason)}`)
    })
  }

  const fileLines = formatPlanFileChanges(snapshot.fileGroups)
  if (fileLines.length > 0) {
    lines.push('', '### 已知文件变更', ...fileLines)
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
  const suffix = overflow > 0 ? ` 等 ${files.length} 个文件` : ''
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
