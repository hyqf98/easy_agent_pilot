import type { AgentConfig } from '@/stores/agent'
import type { Project } from '@/stores/project'
import type { Plan, Task } from '@/types/plan'
import type { UnattendedChannel, UnattendedThread } from './types'

export interface UnattendedDeliveryCapabilities {
  canSendText: boolean
  canSendImage: boolean
  canSendFile: boolean
  note: string
}

export interface BuildUnattendedContextOptions {
  channel: UnattendedChannel
  thread: UnattendedThread
  agent?: AgentConfig | null
  project?: Project
  projects: Project[]
  plans: Plan[]
  activeTasks?: Task[]
  capabilities: UnattendedDeliveryCapabilities
}

export function getUnattendedDeliveryCapabilities(): UnattendedDeliveryCapabilities {
  return {
    canSendText: true,
    canSendImage: false,
    canSendFile: false,
    note: '当前已接入的微信无人值守 API 仅实现文本发送，未接入可确认可用的图片或文件发送接口。'
  }
}

function formatValue(value?: string | null, fallback: string = '未设置'): string {
  const normalized = value?.trim()
  return normalized || fallback
}

function formatTaskStatus(task: Task): string {
  const pieces: string[] = [task.status]
  if (task.priority) {
    pieces.push(`优先级 ${task.priority}`)
  }
  if (task.errorMessage?.trim()) {
    pieces.push(`错误 ${task.errorMessage.trim().slice(0, 48)}`)
  }
  return pieces.join(' / ')
}

function buildProjectSection(projects: Project[], currentProjectId?: string): string[] {
  if (projects.length === 0) {
    return ['- 当前工作区还没有导入项目']
  }

  return projects.slice(0, 8).map((project) => {
    const marker = project.id === currentProjectId ? ' [当前线程项目]' : ''
    return `- ${project.name}${marker} | 路径: ${project.path}`
  })
}

function buildPlanSection(plans: Plan[]): string[] {
  if (plans.length === 0) {
    return ['- 当前项目下还没有计划']
  }

  return plans.slice(0, 6).map((plan) => {
    const execution = plan.executionStatus ? ` / 执行 ${plan.executionStatus}` : ''
    const currentTask = plan.currentTaskId ? ` / 当前任务 ${plan.currentTaskId}` : ''
    return `- ${plan.name} | 状态 ${plan.status}${execution}${currentTask}`
  })
}

function buildTaskSection(tasks: Task[]): string[] {
  if (tasks.length === 0) {
    return ['- 当前没有需要额外展开的执行中或异常任务']
  }

  return tasks.slice(0, 8).map((task) => `- ${task.title} | ${formatTaskStatus(task)}`)
}

/**
 * 构造发给无人值守 AI 的压缩工作区上下文。
 */
export function buildUnattendedWorkspaceContext(
  options: BuildUnattendedContextOptions
): string {
  const {
    channel,
    thread,
    agent,
    project,
    projects,
    plans,
    activeTasks = [],
    capabilities
  } = options

  return [
    '[无人值守线程上下文]',
    `- 渠道: ${channel.name} (${channel.channelType})`,
    `- 远端用户: ${thread.peerNameSnapshot || thread.peerId}`,
    `- 当前项目: ${project?.name || formatValue(thread.activeProjectId, '未绑定项目')}`,
    `- 当前 Agent: ${agent?.name || formatValue(thread.activeAgentId)}`,
    `- 当前模型: ${formatValue(thread.activeModelId)}`,
    `- 最近计划: ${formatValue(thread.lastPlanId)}`,
    `- 最近任务: ${formatValue(thread.lastTaskId)}`,
    '',
    '[工作区已导入项目]',
    ...buildProjectSection(projects, project?.id || thread.activeProjectId),
    '',
    `[当前项目计划摘要 | 共 ${plans.length} 个]`,
    ...buildPlanSection(plans),
    '',
    '[当前执行中 / 异常任务摘录]',
    ...buildTaskSection(activeTasks),
    '',
    '[渠道发送能力]',
    `- 文本发送: ${capabilities.canSendText ? '支持' : '不支持'}`,
    `- 图片发送: ${capabilities.canSendImage ? '支持' : '不支持'}`,
    `- 文件发送: ${capabilities.canSendFile ? '支持' : '不支持'}`,
    `- 备注: ${capabilities.note}`
  ].join('\n')
}
