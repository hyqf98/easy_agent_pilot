import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useAgentStore, type AgentConfig } from './agent'
import { useAgentStudioStore } from './agentStudio'
import { buildAgentProfileSystemPrompt, useAgentProfileStore } from './agentProfile'
import { useAgentTeamStore } from './agentTeam'
import { logger } from '@/utils/logger'
import { normalizeFormSchemaForRendering, normalizeFormSchemasForRendering } from '@/utils/formSchema'
import {
  buildPlanSplitJsonSchema,
  buildPlanSplitKickoffPrompt,
  buildPlanSplitSystemPrompt,
  buildTaskResplitKickoffPrompt
} from '@/services/plan'
import type { ExecutionRequest, MessageInput } from '@/services/conversation/strategies/types'
import type { RuntimeNotice } from '@/utils/runtimeNotice'
import { buildCliEnvironmentNotice } from '@/utils/runtimeNotice'
import type {
  AITaskItem,
  DynamicFormSchema,
  PlanSplitLogRecord,
  PlanSplitSessionRecord,
  PlanSplitStreamPayload,
  SplitMessage,
  TaskResplitConfig
} from '@/types/plan'

interface TaskSplitContext {
  planId: string
  planName: string
  planDescription?: string
  granularity: number
  agentId: string
  modelId: string
  splitExecutionMode?: 'single' | 'coordinator_subagents'
  splitTeamId?: string
  workingDirectory?: string
}

interface SubmittedFormSnapshot {
  formId: string
  schema: DynamicFormSchema
  promptText?: string
  values: Record<string, unknown>
  submittedAt: string
}

function getAllowedTools(provider: string): string[] {
  if ((provider || '').toLowerCase() === 'codex') {
    return ['Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash']
  }
  return ['Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash', 'WebFetch', 'WebSearch']
}

function formatOptionValue(field: DynamicFormSchema['fields'][number], value: unknown): string {
  if (value === undefined || value === null || value === '') return '-'
  if (Array.isArray(value)) {
    if (value.length === 0) return '-'
    return value.map(item => formatOptionValue(field, item)).join('、')
  }
  const matched = field.options?.find(option => option.value === value)
  if (matched) return matched.label
  return String(value)
}

function summarizeFormValues(schema: DynamicFormSchema, values: Record<string, unknown>): string {
  return schema.fields
    .map(field => `${field.label}：${formatOptionValue(field, values[field.name])}`)
    .join('\n')
}

function parseJson<T>(raw?: string | null, fallback?: T): T {
  if (!raw) return fallback as T
  try {
    return JSON.parse(raw) as T
  } catch (error) {
    logger.warn('[TaskSplit] JSON parse failed:', error)
    return fallback as T
  }
}

function buildExecutionRequest(
  context: TaskSplitContext,
  agent: AgentConfig,
  llmMessages: MessageInput[],
  mcpServers?: ExecutionRequest['mcpServers']
): ExecutionRequest {
  const provider = agent.provider || 'claude'
  const systemPrompt = llmMessages
    .filter(message => message.role === 'system')
    .map(message => message.content.trim())
    .filter(Boolean)
    .join('\n\n')

  return {
    sessionId: crypto.randomUUID(),
    planId: context.planId,
    agentType: agent.type as 'cli' | 'sdk',
    provider,
    cliPath: agent.type === 'cli' ? agent.cliPath : undefined,
    apiKey: agent.type === 'sdk' ? agent.apiKey : undefined,
    baseUrl: agent.type === 'sdk' ? agent.baseUrl : undefined,
    modelId: context.modelId || undefined,
    messages: llmMessages,
    workingDirectory: context.workingDirectory,
    allowedTools: agent.type === 'cli' ? getAllowedTools(provider) : undefined,
    systemPrompt: systemPrompt || undefined,
    maxTokens: agent.type === 'sdk' ? 4096 : undefined,
    cliOutputFormat: agent.type === 'cli' ? 'stream-json' : undefined,
    jsonSchema: agent.type === 'cli'
      ? buildPlanSplitJsonSchema(context.granularity, provider.toLowerCase() === 'codex' ? 'codex' : 'claude')
      : undefined,
    mcpServers,
    executionMode: 'task_split',
    responseMode: 'stream_text'
  }
}

function toSplitMessages(raw?: string | null): SplitMessage[] {
  const parsed = parseJson<Array<{ id: string; role: 'user' | 'assistant'; content: string; timestamp: string }>>(raw, [])
  return parsed.map(message => ({
    id: message.id,
    role: message.role,
    content: message.content,
    timestamp: message.timestamp
  }))
}

function toFormQueue(raw?: string | null): DynamicFormSchema[] {
  const forms = parseJson<DynamicFormSchema[]>(raw, [])
  return normalizeFormSchemasForRendering(forms)
}

function toSplitResult(raw?: string | null): AITaskItem[] | null {
  if (!raw) return null
  const parsed = parseJson<{ tasks?: AITaskItem[] }>(raw, {})
  return parsed.tasks ?? null
}

function toPlanSplitLogs(logs: PlanSplitLogRecord[]): PlanSplitLogRecord[] {
  return logs.map(log => ({
    ...log,
    type: log.type
  }))
}

export const useTaskSplitStore = defineStore('taskSplit', () => {
  const messages = ref<SplitMessage[]>([])
  const logs = ref<PlanSplitLogRecord[]>([])
  const isProcessing = ref(false)
  const splitResult = ref<AITaskItem[] | null>(null)
  const submittedForms = ref<SubmittedFormSnapshot[]>([])
  const currentFormId = ref<string | null>(null)
  const formQueue = ref<DynamicFormSchema[]>([])
  const currentFormIndex = ref<number>(0)
  const session = ref<PlanSplitSessionRecord | null>(null)
  const context = ref<TaskSplitContext | null>(null)
  const streamUnlisten = ref<UnlistenFn | null>(null)
  const runtimeNotices = ref<RuntimeNotice[]>([])

  const subSplitMode = ref(false)
  const subSplitTargetIndex = ref<number | null>(null)
  const subSplitOriginalTasks = ref<AITaskItem[]>([])
  const subSplitConfig = ref<TaskResplitConfig | null>(null)

  const activeFormSchema = computed(() => {
    if (!formQueue.value.length) return null
    return formQueue.value[currentFormIndex.value] ?? null
  })

  function detachStream() {
    if (streamUnlisten.value) {
      streamUnlisten.value()
      streamUnlisten.value = null
    }
  }

  function resetState() {
    messages.value = []
    logs.value = []
    isProcessing.value = false
    splitResult.value = null
    submittedForms.value = []
    currentFormId.value = null
    formQueue.value = []
    currentFormIndex.value = 0
    session.value = null
    context.value = null
    runtimeNotices.value = []
    subSplitMode.value = false
    subSplitTargetIndex.value = null
    subSplitOriginalTasks.value = []
    subSplitConfig.value = null
  }

  function applySessionSnapshot(snapshot: PlanSplitSessionRecord | null, preserveLogs: boolean = true) {
    session.value = snapshot
    if (!snapshot) {
      messages.value = []
      splitResult.value = null
      formQueue.value = []
      currentFormIndex.value = 0
      currentFormId.value = null
      isProcessing.value = false
      if (!preserveLogs) {
        logs.value = []
      }
      return
    }

    messages.value = toSplitMessages(snapshot.messagesJson)
    splitResult.value = toSplitResult(snapshot.resultJson)
    formQueue.value = toFormQueue(snapshot.formQueueJson)
    currentFormIndex.value = snapshot.currentFormIndex ?? 0
    currentFormId.value = formQueue.value[currentFormIndex.value]?.formId ?? null
    isProcessing.value = snapshot.status === 'running'
  }

  async function loadSession(planId: string) {
    const [snapshot, splitLogs] = await Promise.all([
      invoke<PlanSplitSessionRecord | null>('get_plan_split_session', { planId }),
      invoke<PlanSplitLogRecord[]>('list_plan_split_logs', { planId }).catch(() => [])
    ])

    logs.value = toPlanSplitLogs(splitLogs)
    applySessionSnapshot(snapshot, true)
  }

  async function subscribeToPlan(planId: string) {
    detachStream()
    streamUnlisten.value = await listen<PlanSplitStreamPayload>(`plan-split-stream-${planId}`, async (event) => {
      const payload = event.payload
      if (payload.type === 'session_updated' && payload.session) {
        applySessionSnapshot(payload.session, true)
        return
      }

      if (payload.type === 'done') {
        return
      }

      const content = payload.content ?? payload.error ?? payload.toolResult ?? payload.toolInput ?? ''
      logs.value.push({
        id: `${payload.sessionId || 'plan'}-${payload.type}-${payload.createdAt || Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        planId: payload.planId,
        sessionId: payload.sessionId || '',
        type: payload.type as PlanSplitLogRecord['type'],
        content,
        metadata: payload.metadata ?? null,
        createdAt: payload.createdAt || new Date().toISOString()
      })
    })
  }

  async function startBackgroundSession(
    nextContext: TaskSplitContext,
    llmMessages: MessageInput[],
    uiMessages: SplitMessage[],
    mcpServers?: ExecutionRequest['mcpServers']
  ) {
    const agentStore = useAgentStore()
    const agent = agentStore.agents.find(item => item.id === nextContext.agentId)
    if (!agent) {
      throw new Error('未找到选中的智能体，请重新选择后重试。')
    }
    if (agent.type === 'cli' && !agent.cliPath) {
      throw new Error('CLI 路径未配置')
    }
    if (agent.type === 'sdk' && !agent.apiKey) {
      throw new Error('SDK API Key 未配置')
    }

    logs.value = []
    const executionRequest = buildExecutionRequest(nextContext, agent, llmMessages, mcpServers)
    const snapshot = await invoke<PlanSplitSessionRecord>('start_plan_split', {
      input: {
        planId: nextContext.planId,
        granularity: nextContext.granularity,
        executionRequest,
        llmMessages,
        messages: uiMessages
      }
    })
    applySessionSnapshot(snapshot, true)
  }

  function rememberSubmittedForm(
    schema: DynamicFormSchema,
    values: Record<string, unknown>,
    promptText?: string
  ) {
    const submittedAt = new Date().toISOString()
    const snapshot: SubmittedFormSnapshot = {
      formId: schema.formId,
      schema: normalizeFormSchemaForRendering(
        JSON.parse(JSON.stringify(schema)) as DynamicFormSchema
      ),
      promptText,
      values: JSON.parse(JSON.stringify(values)) as Record<string, unknown>,
      submittedAt
    }

    submittedForms.value = [
      ...submittedForms.value.filter(item => item.formId !== schema.formId),
      snapshot
    ]
  }

  async function initSession(nextContext: TaskSplitContext) {
    const agentProfileStore = useAgentProfileStore()
    if (context.value?.planId !== nextContext.planId) {
      submittedForms.value = []
    }

    context.value = nextContext
    runtimeNotices.value = []
    const selectedAgent = useAgentStore().agents.find(agent => agent.id === nextContext.agentId)
    if (selectedAgent) {
      const environmentNotice = await buildCliEnvironmentNotice(selectedAgent).catch(() => null)
      if (environmentNotice) {
        runtimeNotices.value = [environmentNotice]
      }
    }
    const selectedExecutionProfile = selectedAgent
      ? await agentProfileStore.resolveExecutionProfile(selectedAgent.id)
      : null
    const selectedAgentSystemPrompt = buildAgentProfileSystemPrompt(selectedExecutionProfile)
    await subscribeToPlan(nextContext.planId)
    await loadSession(nextContext.planId)

    if (session.value) {
      return
    }

    await agentProfileStore.loadProfiles().catch(() => [])
    await useAgentStudioStore().loadAgents().catch(() => [])

    const llmMessages: MessageInput[] = [
      {
        role: 'system',
        content: buildPlanSplitSystemPrompt()
      },
      ...(selectedAgentSystemPrompt
        ? [{
            role: 'system' as const,
            content: selectedAgentSystemPrompt
          }]
        : []),
      {
        role: 'user',
        content: buildPlanSplitKickoffPrompt({
          planName: nextContext.planName,
          planDescription: nextContext.planDescription,
          minTaskCount: nextContext.granularity,
          executionMode: nextContext.splitExecutionMode,
          teamName: nextContext.splitTeamId
            ? useAgentTeamStore().getTeam(nextContext.splitTeamId)?.name
            : undefined,
          coordinatorLabel: selectedAgent?.name,
          teamSummary: nextContext.splitTeamId
            ? useAgentTeamStore().getTeam(nextContext.splitTeamId)?.members
              .map(member => {
                const studioAgent = useAgentStudioStore().getAgent(member.agentId)
                return `- ${member.displayName || studioAgent?.name || member.agentId} / ${member.role} / ${member.capabilityPreset}`
              })
              .join('\n')
            : undefined,
          agentCatalog: useAgentStudioStore().enabledAgents
            .map(studioAgent => {
              const runtimeAgent = useAgentStore().agents.find(agent => agent.id === studioAgent.runtimeAgentId)
              const capabilities = [
                studioAgent.planningCapability ? '规划' : '',
                studioAgent.readonlyResearcher ? '只读研究' : '',
                studioAgent.executionCapability ? '执行' : ''
              ].filter(Boolean)
              const specialization = studioAgent.specialization ? ` / 专业 ${studioAgent.specialization}` : ''
              const model = studioAgent.runtimeModelId || runtimeAgent?.modelId
              return `- ${studioAgent.runtimeAgentId}: ${studioAgent.name}${model ? ` / 默认模型 ${model}` : ''}${capabilities.length > 0 ? ` / 能力 ${capabilities.join('、')}` : ''}${specialization}`
            })
            .join('\n')
        })
      }
    ]

    const uiMessages: SplitMessage[] = [{
      id: crypto.randomUUID(),
      role: 'user',
      content: [
        `计划标题：${nextContext.planName}`,
        `计划描述：${nextContext.planDescription?.trim() || '（无）'}`,
        `拆分任务数量：至少拆分 ${nextContext.granularity} 个任务`
      ].join('\n'),
      timestamp: new Date().toISOString()
    }]

    await startBackgroundSession(
      nextContext,
      llmMessages,
      uiMessages,
      selectedExecutionProfile?.mcpServers
    )
  }

  async function submitFormResponse(formId: string, values: Record<string, unknown>) {
    if (!context.value) return
    const schema = activeFormSchema.value
    if (!schema || schema.formId !== formId) return
    const promptText = [...messages.value]
      .slice()
      .reverse()
      .find(message => message.role === 'assistant' && message.content.trim())
      ?.content
      ?.trim()

    const snapshot = await invoke<PlanSplitSessionRecord>('submit_plan_split_form', {
      input: {
        planId: context.value.planId,
        formId,
        values,
        displayContent: summarizeFormValues(schema, values)
      }
    })
    applySessionSnapshot(snapshot, true)
    rememberSubmittedForm(schema, values, promptText)
  }

  async function retry() {
    if (!context.value || !session.value) {
      return
    }

    const llmMessages = parseJson<MessageInput[]>(session.value.llmMessagesJson, [])
    const uiMessages = toSplitMessages(session.value.messagesJson)
    const previousRequest = parseJson<ExecutionRequest | null>(session.value.executionRequestJson, null)

    if (llmMessages.length === 0) {
      throw new Error('当前会话缺少可重试的上下文。')
    }

    await startBackgroundSession(context.value, llmMessages, uiMessages, previousRequest?.mcpServers)
  }

  async function stop() {
    if (!context.value) return
    const snapshot = await invoke<PlanSplitSessionRecord | null>('stop_plan_split', {
      planId: context.value.planId
    })
    applySessionSnapshot(snapshot, true)
  }

  async function clearAllSplitData(planId: string) {
    await invoke('clear_plan_split_session', { planId })
    if (context.value?.planId === planId) {
      resetState()
    }
  }

  function clearProjectSplitState(planIds: string[]) {
    if (planIds.length === 0) {
      return
    }

    if (context.value && planIds.includes(context.value.planId)) {
      detachStream()
      resetState()
    }
  }

  function updateSplitTask(index: number, updates: Partial<AITaskItem>) {
    if (!splitResult.value || !splitResult.value[index]) return
    splitResult.value[index] = { ...splitResult.value[index], ...updates }
  }

  function removeSplitTask(index: number) {
    if (!splitResult.value) return
    splitResult.value.splice(index, 1)
  }

  function addSplitTask() {
    const nextTask: AITaskItem = {
      title: '',
      description: '',
      priority: 'medium',
      implementationSteps: [''],
      testSteps: [''],
      acceptanceCriteria: ['']
    }
    if (!splitResult.value) {
      splitResult.value = [nextTask]
      return
    }
    splitResult.value.push(nextTask)
  }

  async function startSubSplit(index: number, config: TaskResplitConfig) {
    if (!context.value || !splitResult.value || !splitResult.value[index]) return
    const agentProfileStore = useAgentProfileStore()

    const targetTask = splitResult.value[index]
    subSplitMode.value = true
    subSplitTargetIndex.value = index
    subSplitOriginalTasks.value = JSON.parse(JSON.stringify(splitResult.value))
    subSplitConfig.value = config

    const nextContext: TaskSplitContext = {
      ...context.value,
      granularity: config.granularity,
      agentId: config.agentId || context.value.agentId,
      modelId: config.modelId || context.value.modelId,
      splitExecutionMode: context.value.splitExecutionMode,
      splitTeamId: context.value.splitTeamId
    }
    context.value = nextContext
    const executionProfile = await agentProfileStore.resolveExecutionProfile(nextContext.agentId)
    const agentSystemPrompt = buildAgentProfileSystemPrompt(executionProfile)

    const llmMessages: MessageInput[] = [
      {
        role: 'system',
        content: buildPlanSplitSystemPrompt()
      },
      ...(agentSystemPrompt
        ? [{
            role: 'system' as const,
            content: agentSystemPrompt
          }]
        : []),
      {
        role: 'user',
        content: buildTaskResplitKickoffPrompt({
          planName: nextContext.planName,
          planDescription: nextContext.planDescription,
          taskTitle: targetTask.title,
          taskDescription: targetTask.description,
          implementationSteps: targetTask.implementationSteps || [],
          testSteps: targetTask.testSteps || [],
          acceptanceCriteria: targetTask.acceptanceCriteria,
          userPrompt: config.customPrompt,
          minTaskCount: config.granularity
        })
      }
    ]

    const uiMessages: SplitMessage[] = [{
      id: crypto.randomUUID(),
      role: 'user',
      content: [
        `继续拆分任务：${targetTask.title}`,
        `原任务描述：${targetTask.description || '（无）'}`,
        `拆分颗粒度：至少 ${config.granularity} 个子任务`,
        config.customPrompt ? `额外要求：${config.customPrompt}` : ''
      ].filter(Boolean).join('\n'),
      timestamp: new Date().toISOString()
    }]

    await startBackgroundSession(nextContext, llmMessages, uiMessages, executionProfile.mcpServers)
  }

  function completeSubSplit(newTasks: AITaskItem[]) {
    if (!subSplitMode.value || subSplitTargetIndex.value === null) {
      return
    }

    const originalTasks = [...subSplitOriginalTasks.value]
    originalTasks.splice(subSplitTargetIndex.value, 1)
    splitResult.value = [...originalTasks, ...newTasks]
    subSplitMode.value = false
    subSplitTargetIndex.value = null
    subSplitOriginalTasks.value = []
    subSplitConfig.value = null
  }

  function cancelSubSplit() {
    if (!subSplitMode.value) return
    splitResult.value = [...subSplitOriginalTasks.value]
    subSplitMode.value = false
    subSplitTargetIndex.value = null
    subSplitOriginalTasks.value = []
    subSplitConfig.value = null
  }

  function reset() {
    detachStream()
    resetState()
  }

  function detach() {
    detachStream()
    isProcessing.value = false
  }

  return {
    messages,
    logs,
    isProcessing,
    splitResult,
    submittedForms,
    currentFormId,
    currentFormIndex,
    formQueue,
    activeFormSchema,
    session,
    context,
    runtimeNotices,
    subSplitMode,
    subSplitTargetIndex,
    subSplitOriginalTasks,
    subSplitConfig,
    initSession,
    submitFormResponse,
    retry,
    stop,
    updateSplitTask,
    removeSplitTask,
    addSplitTask,
    startSubSplit,
    completeSubSplit,
    cancelSubSplit,
    clearAllSplitData,
    clearPlanSplitSessions: clearAllSplitData,
    clearProjectSplitState,
    reset,
    detach,
    loadSession
  }
})
