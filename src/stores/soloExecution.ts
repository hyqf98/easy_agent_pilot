import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AgentConfig } from '@/stores/agent'
import { inferAgentProvider, useAgentStore } from '@/stores/agent'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { useProjectStore } from '@/stores/project'
import { useSettingsStore } from '@/stores/settings'
import { useSoloRunStore } from './soloRun'
import { agentExecutor } from '@/services/conversation/AgentExecutor'
import type { ConversationContext, McpServerConfig, StreamEvent } from '@/services/conversation/strategies/types'
import { extractFirstFormRequest } from '@/utils/structuredContent'
import { parseExecutionResult } from '@/utils/taskExecutionText'
import { buildToolCallMapFromLogs } from '@/utils/toolCallLog'
import { mergeToolInputArguments } from '@/utils/toolInput'
import {
  buildContextStrategyNotice,
  buildUsageNotice,
  formatRuntimeNoticeAsSystemContent
} from '@/utils/runtimeNotice'
import {
  isCumulativeUsageRuntime,
  normalizeRuntimeUsage,
  type UsageBaseline
} from '@/utils/runtimeUsage'
import { recordAgentCliUsageInBackground, resolveRecordedModelId } from '@/services/usage/agentCliUsageRecorder'
import {
  buildExpertCatalogPrompt,
  resolveExpertById,
  resolveExpertRuntime
} from '@/services/agentTeams/runtime'
import { loadAgentMcpServers } from '@/utils/mcpServerConfig'
import {
  deleteSoloRuntimeBinding,
  getSoloRuntimeBinding,
  isInvalidCliResumeError,
  resolveRuntimeBindingKey,
  upsertSoloRuntimeBinding
} from '@/services/conversation/runtimeBindings'
import {
  buildSoloBuiltinCoordinatorPrompt,
  buildSoloControlJsonSchema,
  buildSoloControlPrompt,
  buildSoloCoordinatorSystemPrompt,
  buildSoloExecutionPrompt,
  buildSoloExecutionSystemPrompt,
  buildSoloInputRequest,
  buildSoloResumeContext,
  compactSoloSummary,
  parseSoloCoordinatorDecision
} from '@/services/solo/prompts'
import { loadMountedMemoryPrompt } from '@/services/memory/mountedMemoryPrompt'
import type {
  CreateSoloLogInput,
  CreateSoloStepInput,
  SoloExecutionState,
  SoloLogEntry,
  SoloRun,
  SoloStep,
  UpdateSoloStepInput
} from '@/types/solo'
import type { ExecutionLogMetadata } from '@/types/taskExecution'
import type { ToolCall } from '@/stores/message'
import type { AgentRuntimeKey } from '@/services/conversation/runtimeProfiles'
import {
  classifyCliFailureWithExplicitPriority,
  createCliFailureFragment,
  type CliFailureMatch
} from '@/utils/cliFailureMonitor'

const SOLO_STOPPED_ERROR = '__SOLO_STOPPED__'
const SOLO_CLI_FAILURE_RETRY_DELAY_MS = 10_000

interface RustSoloStep {
  id: string
  run_id: string
  step_ref: string
  parent_step_ref?: string | null
  depth: number
  title: string
  description?: string | null
  execution_prompt?: string | null
  selected_expert_id?: string | null
  status: SoloStep['status']
  summary?: string | null
  result_summary?: string | null
  result_files_json?: string | null
  fail_reason?: string | null
  created_at: string
  updated_at: string
  started_at?: string | null
  completed_at?: string | null
}

interface RustSoloLog {
  id: string
  run_id: string
  step_id?: string | null
  scope: SoloLogEntry['scope']
  type: SoloLogEntry['type']
  content: string
  metadata?: string | null
  created_at: string
}

function parseJson<T>(raw?: string | null): T | undefined {
  if (!raw) return undefined
  try {
    return JSON.parse(raw) as T
  } catch {
    return undefined
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

function transformStep(raw: RustSoloStep): SoloStep {
  return {
    id: raw.id,
    runId: raw.run_id,
    stepRef: raw.step_ref,
    parentStepRef: raw.parent_step_ref || undefined,
    depth: raw.depth,
    title: raw.title,
    description: raw.description || undefined,
    executionPrompt: raw.execution_prompt || undefined,
    selectedExpertId: raw.selected_expert_id || undefined,
    status: raw.status,
    summary: raw.summary || undefined,
    resultSummary: raw.result_summary || undefined,
    resultFiles: parseJson<string[]>(raw.result_files_json) ?? [],
    failReason: raw.fail_reason || undefined,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
    startedAt: raw.started_at || undefined,
    completedAt: raw.completed_at || undefined
  }
}

function transformLog(raw: RustSoloLog): SoloLogEntry {
  return {
    id: raw.id,
    runId: raw.run_id,
    stepId: raw.step_id || undefined,
    scope: raw.scope,
    type: raw.type,
    content: raw.content,
    metadata: parseJson<ExecutionLogMetadata>(raw.metadata),
    timestamp: raw.created_at
  }
}

function createExecutionState(runId: string): SoloExecutionState {
  return {
    runId,
    status: 'idle',
    sessionId: null,
    startedAt: null,
    completedAt: null,
    currentStepId: null,
    logs: [],
    accumulatedContent: '',
    accumulatedThinking: '',
    toolCalls: [],
    tokenUsage: {
      inputTokens: 0,
      outputTokens: 0,
      resetCount: 0,
      lastUpdatedAt: null
    }
  }
}

function createMessage(runId: string, role: 'system' | 'user' | 'assistant', content: string) {
  const now = new Date().toISOString()
  return {
    id: `solo-${role}-${runId}-${Math.random().toString(36).slice(2, 8)}`,
    sessionId: runId,
    role,
    content,
    status: 'completed' as const,
    createdAt: now
  }
}

type SoloBindingRole = 'coordinator' | 'expert'

interface SoloResolvedRuntime {
  agent: AgentConfig
  workingDirectory?: string
  mcpServers?: McpServerConfig[]
  expertPrompt?: string
  expertName?: string
  expertId?: string
  runtimeKey: AgentRuntimeKey | null
  bindingKey: string | null
}

function buildSoloBindingKey(
  agent: Pick<AgentConfig, 'type' | 'provider' | 'name' | 'cliPath'>,
  role: SoloBindingRole,
  expertId?: string | null
): string | null {
  const runtimeKey = resolveRuntimeBindingKey(agent)
  if (!runtimeKey) {
    return null
  }

  if (role === 'coordinator') {
    return `${runtimeKey}::solo::coordinator`
  }

  return `${runtimeKey}::solo::expert::${expertId?.trim() || 'default'}`
}

function collectMountedMemoryLibraryIds(
  projectIds: string[] | undefined,
  runIds: string[] | undefined
): string[] {
  return Array.from(
    new Set(
      [...(projectIds ?? []), ...(runIds ?? [])]
        .map((id) => id.trim())
        .filter(Boolean)
    )
  )
}

export const useSoloExecutionStore = defineStore('soloExecution', () => {
  const stepsByRun = ref<Map<string, SoloStep[]>>(new Map())
  const executionStates = ref<Map<string, SoloExecutionState>>(new Map())
  const stopRequestedRunIds = ref<Set<string>>(new Set())
  const pendingLogWrites = new Map<string, Set<Promise<unknown>>>()
  const usageBaselines = new Map<string, UsageBaseline>()

  const getSteps = computed(() => (runId: string) => stepsByRun.value.get(runId) ?? [])
  const getExecutionState = computed(() => (runId: string) => executionStates.value.get(runId))
  const getCurrentStep = computed(() => (runId: string) => {
    const runStore = useSoloRunStore()
    const run = runStore.runs.find((item) => item.id === runId)
    if (!run?.currentStepId) return null
    return getSteps.value(runId).find((step) => step.id === run.currentStepId) || null
  })
  const getCurrentStepLogs = computed(() => (runId: string) => {
    const state = executionStates.value.get(runId)
    const currentStepId = state?.currentStepId
    return currentStepId
      ? (state?.logs ?? []).filter((log) => log.stepId === currentStepId)
      : []
  })

  function initExecutionState(runId: string): SoloExecutionState {
    let state = executionStates.value.get(runId)
    if (!state) {
      state = createExecutionState(runId)
      executionStates.value.set(runId, state)
    }
    return state
  }

  function trackPendingLogWrite<T>(runId: string, task: Promise<T>): Promise<T> {
    let tasks = pendingLogWrites.get(runId)
    if (!tasks) {
      tasks = new Set()
      pendingLogWrites.set(runId, tasks)
    }

    const tracked = task.finally(() => {
      const currentTasks = pendingLogWrites.get(runId)
      currentTasks?.delete(tracked)
      if (currentTasks && currentTasks.size === 0) {
        pendingLogWrites.delete(runId)
      }
    })

    tasks.add(tracked)
    return tracked
  }

  async function waitForPendingLogWrites(runId: string): Promise<void> {
    const tasks = pendingLogWrites.get(runId)
    if (!tasks || tasks.size === 0) {
      return
    }

    await Promise.allSettled(Array.from(tasks))
  }

  function setSteps(runId: string, steps: SoloStep[]): void {
    stepsByRun.value.set(runId, steps.sort((left, right) => new Date(left.createdAt).getTime() - new Date(right.createdAt).getTime()))
  }

  function detectSoloCliFailure(
    runId: string,
    runtimeLabel: string,
    options: {
      errorMessage?: string | null
      logStartIndex?: number
    } = {}
  ): CliFailureMatch | null {
    const state = executionStates.value.get(runId)
    if (!state) {
      return null
    }

    const logStartIndex = options.logStartIndex ?? 0
    const explicitFragments = [
      createCliFailureFragment('error', options.errorMessage),
      ...state.logs.slice(logStartIndex).flatMap((log) => [
        createCliFailureFragment('error', log.type === 'error' ? log.content : undefined)
      ]),
      ...state.toolCalls.flatMap((toolCall) => [
        createCliFailureFragment('error', toolCall.errorMessage)
      ])
    ].filter((item): item is NonNullable<typeof item> => Boolean(item))
    const fragments = [
      ...explicitFragments,
      createCliFailureFragment('content', state.accumulatedContent),
      ...state.logs.slice(logStartIndex).flatMap((log) => [
        createCliFailureFragment(
          log.type === 'error' ? 'error' : log.type === 'tool_result' ? 'tool_result' : log.type === 'system' ? 'system' : 'content',
          log.content
        )
      ]),
      ...state.toolCalls.flatMap((toolCall) => [
        createCliFailureFragment('tool_result', typeof toolCall.result === 'string' ? toolCall.result : undefined),
        createCliFailureFragment('error', toolCall.errorMessage)
      ])
    ].filter((item): item is NonNullable<typeof item> => Boolean(item))

    return classifyCliFailureWithExplicitPriority(runtimeLabel, explicitFragments, fragments)
  }

  async function loadSteps(runId: string): Promise<SoloStep[]> {
    const rawSteps = await invoke<RustSoloStep[]>('list_solo_steps', { runId })
    const steps = rawSteps.map(transformStep)
    setSteps(runId, steps)
    return steps
  }

  async function loadLogs(runId: string): Promise<SoloLogEntry[]> {
    const rawLogs = await invoke<RustSoloLog[]>('list_solo_logs', { runId, stepId: null })
    const state = initExecutionState(runId)
    state.logs = rawLogs.map(transformLog)

    let totalInput = 0
    let totalOutput = 0
    let lastModel: string | undefined
    for (const log of state.logs) {
      if (typeof log.metadata?.inputTokens === 'number') {
        totalInput += log.metadata.inputTokens
      }
      if (typeof log.metadata?.outputTokens === 'number') {
        totalOutput += log.metadata.outputTokens
      }
      if (typeof log.metadata?.model === 'string' && log.metadata.model.trim()) {
        lastModel = log.metadata.model.trim()
      }
    }

    state.tokenUsage = {
      inputTokens: totalInput,
      outputTokens: totalOutput,
      model: lastModel,
      resetCount: 0,
      lastUpdatedAt: state.logs[state.logs.length - 1]?.timestamp ?? null
    }

    const toolCallMap = buildToolCallMapFromLogs(state.logs, { fallbackStatus: state.status === 'running' ? 'running' : 'success' })
    state.toolCalls = Array.from(toolCallMap.values())
    return state.logs
  }

  async function createStep(input: CreateSoloStepInput): Promise<SoloStep> {
    const rawStep = await invoke<RustSoloStep>('create_solo_step', {
      input: {
        run_id: input.runId,
        step_ref: input.stepRef,
        parent_step_ref: input.parentStepRef ?? null,
        depth: input.depth,
        title: input.title,
        description: input.description ?? null,
        execution_prompt: input.executionPrompt ?? null,
        selected_expert_id: input.selectedExpertId ?? null,
        status: input.status ?? null,
        summary: input.summary ?? null,
        started_at: input.startedAt ?? null
      }
    })
    const step = transformStep(rawStep)
    const current = getSteps.value(input.runId)
    setSteps(input.runId, [...current.filter((item) => item.id !== step.id), step])
    return step
  }

  async function updateStep(id: string, updates: UpdateSoloStepInput): Promise<SoloStep> {
    const rawStep = await invoke<RustSoloStep>('update_solo_step', {
      id,
      input: {
        parent_step_ref: 'parentStepRef' in updates ? updates.parentStepRef ?? null : undefined,
        depth: 'depth' in updates ? updates.depth ?? null : undefined,
        title: 'title' in updates ? updates.title ?? null : undefined,
        description: 'description' in updates ? updates.description ?? null : undefined,
        execution_prompt: 'executionPrompt' in updates ? updates.executionPrompt ?? null : undefined,
        selected_expert_id: 'selectedExpertId' in updates ? updates.selectedExpertId ?? null : undefined,
        status: 'status' in updates ? updates.status ?? null : undefined,
        summary: 'summary' in updates ? updates.summary ?? null : undefined,
        result_summary: 'resultSummary' in updates ? updates.resultSummary ?? null : undefined,
        result_files_json: 'resultFiles' in updates ? JSON.stringify(updates.resultFiles ?? []) : undefined,
        fail_reason: 'failReason' in updates ? updates.failReason ?? null : undefined,
        started_at: 'startedAt' in updates ? updates.startedAt ?? null : undefined,
        completed_at: 'completedAt' in updates ? updates.completedAt ?? null : undefined
      }
    })
    const step = transformStep(rawStep)
    const current = getSteps.value(step.runId)
    setSteps(step.runId, [...current.filter((item) => item.id !== step.id), step])
    return step
  }

  async function addLog(input: CreateSoloLogInput): Promise<SoloLogEntry> {
    const rawLog = await invoke<RustSoloLog>('create_solo_log', {
      input: {
        run_id: input.runId,
        step_id: input.stepId ?? null,
        scope: input.scope,
        log_type: input.type,
        content: input.content,
        metadata: input.metadata ? JSON.stringify(input.metadata) : null
      }
    })
    const log = transformLog(rawLog)
    initExecutionState(input.runId).logs.push(log)
    return log
  }

  async function updateLog(
    id: string,
    content: string,
    metadata?: CreateSoloLogInput['metadata']
  ): Promise<void> {
    await invoke('update_solo_log', {
      id,
      content,
      metadata: metadata ? JSON.stringify(metadata) : null
    }).catch((error) => {
      console.warn('[SoloExecution] Failed to update log:', error)
    })
  }

  function updateTokenUsage(
    runId: string,
    usage: Pick<StreamEvent, 'inputTokens' | 'outputTokens' | 'model'>,
    requestedModelId?: string | null,
    didResetUsageWindow: boolean = false
  ) {
    const state = initExecutionState(runId)
    const current = state.tokenUsage
    const nextInputTokens = current.inputTokens + (typeof usage.inputTokens === 'number'
      ? Math.max(0, usage.inputTokens)
      : 0)
    const nextOutputTokens = current.outputTokens + (typeof usage.outputTokens === 'number'
      ? Math.max(0, usage.outputTokens)
      : 0)
    const resolvedModel = resolveRecordedModelId({
      reportedModelId: usage.model || current.model,
      requestedModelId
    }) ?? usage.model ?? current.model
    state.tokenUsage = {
      inputTokens: nextInputTokens,
      outputTokens: nextOutputTokens,
      model: resolvedModel,
      resetCount: didResetUsageWindow ? current.resetCount + 1 : current.resetCount,
      lastUpdatedAt: new Date().toISOString()
    }
  }

  function finalizeRunningToolCalls(runId: string): void {
    const state = executionStates.value.get(runId)
    if (!state) return
    state.toolCalls.forEach((toolCall) => {
      if (toolCall.status === 'running') {
        toolCall.status = 'success'
      }
    })
  }

  function resolveParticipantExperts(run: SoloRun) {
    const agentTeamsStore = useAgentTeamsStore()
    const enabledBuiltins = agentTeamsStore.enabledExperts.filter((expert) =>
      expert.isBuiltin && expert.builtinCode !== 'builtin-solo-coordinator'
    )
    const fallbackExperts = enabledBuiltins.length > 0 ? enabledBuiltins : agentTeamsStore.enabledExperts
    const selectedIds = new Set((run.participantExpertIds ?? []).filter(Boolean))

    if (selectedIds.size === 0) {
      return fallbackExperts
    }

    const selectedExperts = fallbackExperts.filter((expert) => selectedIds.has(expert.id))
    return selectedExperts.length > 0 ? selectedExperts : fallbackExperts
  }

  async function appendUsageNotice(runId: string, stepId?: string): Promise<void> {
    const state = executionStates.value.get(runId)
    if (!state) return
    const notice = buildUsageNotice({
      model: state.tokenUsage.model,
      inputTokens: state.tokenUsage.inputTokens,
      outputTokens: state.tokenUsage.outputTokens
    })
    const content = formatRuntimeNoticeAsSystemContent(notice)
    if (!content) return
    const lastLog = state.logs[state.logs.length - 1]
    if (lastLog?.type === 'system' && lastLog.content === content) return
    await addLog({
      runId,
      stepId,
      scope: stepId ? 'step' : 'coordinator',
      type: 'system',
      content,
      metadata: {
        model: state.tokenUsage.model,
        inputTokens: state.tokenUsage.inputTokens,
        outputTokens: state.tokenUsage.outputTokens
      }
    })
  }

  async function resolveCoordinatorRuntime(run: SoloRun): Promise<SoloResolvedRuntime> {
    const agentStore = useAgentStore()
    const agentTeamsStore = useAgentTeamsStore()
    const projectStore = useProjectStore()

    await Promise.all([
      agentStore.loadAgents(),
      agentTeamsStore.loadExperts()
    ])

    const runtimeExpert = resolveExpertById(run.coordinatorExpertId, agentTeamsStore.experts)
      || agentTeamsStore.builtinSoloCoordinatorExpert
      || agentTeamsStore.builtinPlannerExpert
      || agentTeamsStore.builtinGeneralExpert
      || agentTeamsStore.enabledExperts.find((expert) => expert.isBuiltin)
      || agentTeamsStore.enabledExperts[0]
      || null

    const runtime = resolveExpertRuntime(runtimeExpert, agentStore.agents, run.coordinatorModelId)
    const agent = (run.coordinatorAgentId ? agentStore.agents.find((item) => item.id === run.coordinatorAgentId) : null)
      || runtime?.agent
      || agentStore.agents.find((item) => item.type === 'cli')
      || agentStore.agents[0]

    if (!agent) {
      throw new Error('未找到可用的 SOLO 协调运行时')
    }

    const resolvedAgent = {
      ...agent,
      modelId: runtime?.modelId || run.coordinatorModelId || agent.modelId
    }
    const project = projectStore.projects.find((item) => item.id === run.projectId)
    const mcpServers = await loadAgentMcpServers(resolvedAgent).catch(() => [] as McpServerConfig[])

    return {
      agent: resolvedAgent,
      expertPrompt: runtimeExpert?.prompt || buildSoloBuiltinCoordinatorPrompt(),
      expertName: runtimeExpert?.name || 'SOLO Coordinator',
      expertId: runtimeExpert?.id,
      runtimeKey: resolveRuntimeBindingKey(resolvedAgent),
      bindingKey: buildSoloBindingKey(resolvedAgent, 'coordinator', runtimeExpert?.id),
      workingDirectory: run.executionPath || project?.path,
      mcpServers: mcpServers.length > 0 ? mcpServers : undefined
    }
  }

  async function resolveStepRuntime(run: SoloRun, step: SoloStep): Promise<SoloResolvedRuntime> {
    const agentStore = useAgentStore()
    const agentTeamsStore = useAgentTeamsStore()
    const projectStore = useProjectStore()

    await Promise.all([
      agentStore.loadAgents(),
      agentTeamsStore.loadExperts()
    ])

    const selectedExpert = resolveExpertById(step.selectedExpertId, agentTeamsStore.experts)
      || resolveParticipantExperts(run)[0]
      || resolveExpertById(run.coordinatorExpertId, agentTeamsStore.experts)
      || agentTeamsStore.builtinGeneralExpert
      || agentTeamsStore.enabledExperts.find((expert) => expert.isBuiltin)
      || agentTeamsStore.enabledExperts[0]
      || null

    const runtime = resolveExpertRuntime(selectedExpert, agentStore.agents)
    const fallbackAgent = agentStore.agents.find((item) => item.type === 'cli') || agentStore.agents[0]
    const resolvedAgent = runtime?.agent || fallbackAgent

    if (!resolvedAgent) {
      throw new Error('未找到可用的 SOLO 执行运行时')
    }

    const agent = {
      ...resolvedAgent,
      modelId: runtime?.modelId || resolvedAgent.modelId
    }
    const project = projectStore.projects.find((item) => item.id === run.projectId)
    const mcpServers = await loadAgentMcpServers(agent).catch(() => [] as McpServerConfig[])

    return {
      agent,
      workingDirectory: run.executionPath || project?.path,
      mcpServers: mcpServers.length > 0 ? mcpServers : undefined,
      expertPrompt: selectedExpert?.prompt,
      expertName: selectedExpert?.name || 'SOLO Expert',
      expertId: selectedExpert?.id || step.selectedExpertId,
      runtimeKey: resolveRuntimeBindingKey(agent),
      bindingKey: buildSoloBindingKey(agent, 'expert', selectedExpert?.id || step.selectedExpertId)
    }
  }

  async function executeTurn(options: {
    run: SoloRun
    state: SoloExecutionState
    agent: AgentConfig
    workingDirectory?: string
    mcpServers?: McpServerConfig[]
    messages: ReturnType<typeof createMessage>[]
    responseMode: ConversationContext['responseMode']
    jsonSchema?: string
    stepId?: string
    scope: SoloLogEntry['scope']
    strategy: string
    runtimeLabel: string
    expertLabel?: string
    resumeSessionId?: string
    persistContentLogs?: boolean
    runtimeBindingKey?: string | null
  }): Promise<{ content: string, externalSessionId?: string, turnTokens: { inputTokens: number, outputTokens: number, model?: string } }> {
    const {
      run,
      state,
      agent,
      workingDirectory,
      mcpServers,
      messages,
      responseMode,
      jsonSchema,
      stepId,
      scope,
      strategy,
      runtimeLabel,
      expertLabel,
      resumeSessionId,
      persistContentLogs = true,
      runtimeBindingKey = null
    } = options

    const sessionId = `solo-${run.id}-${crypto.randomUUID()}`
    state.sessionId = sessionId
    const turnTokens = { inputTokens: 0, outputTokens: 0, model: undefined as string | undefined }
    let latestExternalSessionId = resumeSessionId
    const settingsStore = useSettingsStore()
    const maxCliRetries = settingsStore.settings.cliFailureMaxRetries ?? 5
    const runtimeFailureLabel = runtimeLabel

    const contextNotice = buildContextStrategyNotice({
      strategy,
      runtime: runtimeLabel,
      model: agent.modelId,
      expert: expertLabel,
      systemMessageCount: messages.filter((message) => message.role === 'system').length,
      userMessageCount: messages.filter((message) => message.role === 'user').length,
      assistantMessageCount: messages.filter((message) => message.role === 'assistant').length,
      historyMessageCount: resumeSessionId ? Math.min(state.logs.length, 40) : 0,
      resumeSessionId
    })

    if (contextNotice) {
      await addLog({
        runId: run.id,
        stepId,
        scope,
        type: 'system',
        content: formatRuntimeNoticeAsSystemContent(contextNotice),
        metadata: {
          strategy,
          runtime: runtimeLabel,
          model: agent.modelId,
          expert: expertLabel,
          externalSessionId: resumeSessionId
        }
      })
    }

    const context: ConversationContext = {
      sessionId,
      agent,
      messages,
      workingDirectory,
      mcpServers,
      executionMode: 'solo_execution',
      responseMode,
      jsonSchema,
      resumeSessionId
    }

    if (!resumeSessionId || !isCumulativeUsageRuntime(inferAgentProvider(agent))) {
      usageBaselines.delete(run.id)
    }

    let fullContent: string

    try {
      let cliRetryCount = 0
      let cliRetryLogId: string | null = null

      while (true) {
        const logStartIndex = state.logs.length
        state.accumulatedContent = ''
        state.accumulatedThinking = ''
        state.toolCalls = []
        fullContent = ''

        try {
          await agentExecutor.execute(context, (event) => {
            const normalizedUsage = normalizeRuntimeUsage({
              provider: inferAgentProvider(agent),
              inputTokens: event.inputTokens,
              outputTokens: event.outputTokens,
              rawInputTokens: event.rawInputTokens,
              rawOutputTokens: event.rawOutputTokens,
              cacheReadInputTokens: event.cacheReadInputTokens,
              cacheCreationInputTokens: event.cacheCreationInputTokens,
              baseline: usageBaselines.get(run.id) ?? null
            })
            const normalizedEvent: StreamEvent = {
              ...event,
              inputTokens: normalizedUsage.inputTokens,
              outputTokens: normalizedUsage.outputTokens
            }

            if (normalizedUsage.nextBaseline) {
              usageBaselines.set(run.id, normalizedUsage.nextBaseline)
            } else {
              usageBaselines.delete(run.id)
            }

            if (
              normalizedEvent.inputTokens !== undefined
              || normalizedEvent.outputTokens !== undefined
              || normalizedEvent.model
            ) {
              updateTokenUsage(run.id, normalizedEvent, agent.modelId, normalizedUsage.didReset)
              if (typeof normalizedEvent.inputTokens === 'number') {
                turnTokens.inputTokens += Math.max(0, normalizedEvent.inputTokens)
              }
              if (typeof normalizedEvent.outputTokens === 'number') {
                turnTokens.outputTokens += Math.max(0, normalizedEvent.outputTokens)
              }
              if (normalizedEvent.model) turnTokens.model = normalizedEvent.model
            }

            if (normalizedEvent.externalSessionId?.trim()) {
              latestExternalSessionId = normalizedEvent.externalSessionId.trim()
              if (runtimeBindingKey) {
                void upsertSoloRuntimeBinding(run.id, runtimeBindingKey, latestExternalSessionId)
              }
            }

            handleStreamEvent(run.id, normalizedEvent, { stepId, scope, persistContentLogs, fullContentRef: (value) => { fullContent = value } })
          })

          await waitForPendingLogWrites(run.id)

          if (stopRequestedRunIds.value.has(run.id)) {
            throw new Error(SOLO_STOPPED_ERROR)
          }

          const abnormalCompletion = detectSoloCliFailure(run.id, runtimeFailureLabel, { logStartIndex })
          if (!abnormalCompletion) {
            break
          }

          throw new Error(abnormalCompletion.message)
        } catch (error) {
          await waitForPendingLogWrites(run.id)

          const errorMessage = error instanceof Error ? error.message : String(error)
          const classifiedFailure = detectSoloCliFailure(run.id, runtimeFailureLabel, {
            errorMessage,
            logStartIndex
          })

          if (errorMessage === SOLO_STOPPED_ERROR || stopRequestedRunIds.value.has(run.id)) {
            throw error
          }

          if (!classifiedFailure || classifiedFailure.kind !== 'retryable' || cliRetryCount >= maxCliRetries) {
            throw error
          }

          cliRetryCount += 1
          const metadata: ExecutionLogMetadata = {
            retryGroup: 'cli_failure_retry',
            retryCount: cliRetryCount,
            maxRetries: maxCliRetries,
            retryDelaySeconds: SOLO_CLI_FAILURE_RETRY_DELAY_MS / 1000,
            runtime: runtimeFailureLabel
          }
          const content = `检测到可恢复的 ${runtimeFailureLabel} CLI 异常，10 秒后开始第 ${cliRetryCount}/${maxCliRetries} 次底层重试...`

          if (cliRetryLogId) {
            const existingLog = state.logs.find((log) => log.id === cliRetryLogId)
            if (existingLog) {
              existingLog.content = content
              existingLog.metadata = {
                ...existingLog.metadata,
                ...metadata
              }
              await updateLog(cliRetryLogId, content, existingLog.metadata)
            } else {
              const createdLog = await addLog({
                runId: run.id,
                stepId,
                scope,
                type: 'system',
                content,
                metadata
              })
              cliRetryLogId = createdLog.id
            }
          } else {
            const createdLog = await addLog({
              runId: run.id,
              stepId,
              scope,
              type: 'system',
              content,
              metadata
            })
            cliRetryLogId = createdLog.id
          }
          await sleep(SOLO_CLI_FAILURE_RETRY_DELAY_MS)
        }
      }

      if (runtimeBindingKey && latestExternalSessionId && latestExternalSessionId !== resumeSessionId) {
        await upsertSoloRuntimeBinding(run.id, runtimeBindingKey, latestExternalSessionId)
      }
    } finally {
      await waitForPendingLogWrites(run.id)
      state.sessionId = null
    }

    recordAgentCliUsageInBackground(agent, {
      executionId: sessionId,
      executionMode: 'solo_execution',
      modelId: resolveRecordedModelId({
        reportedModelId: turnTokens.model,
        requestedModelId: agent.modelId
      }),
      projectId: run.projectId,
      inputTokens: turnTokens.inputTokens,
      outputTokens: turnTokens.outputTokens,
      occurredAt: new Date().toISOString()
    })

    return {
      content: fullContent,
      externalSessionId: latestExternalSessionId,
      turnTokens
    }
  }

  function handleStreamEvent(
    runId: string,
    event: StreamEvent,
    options: { stepId?: string, scope: SoloLogEntry['scope'], persistContentLogs: boolean, fullContentRef: (value: string) => void }
  ): void {
    const state = initExecutionState(runId)

    switch (event.type) {
      case 'content':
        if (event.content) {
          state.accumulatedContent += event.content
          options.fullContentRef(state.accumulatedContent)
          if (options.persistContentLogs) {
            trackPendingLogWrite(runId, addLog({
              runId,
              stepId: options.stepId,
              scope: options.scope,
              type: 'content',
              content: event.content
            }))
          }
        }
        break
      case 'thinking_start':
        trackPendingLogWrite(runId, addLog({
          runId,
          stepId: options.stepId,
          scope: options.scope,
          type: 'thinking_start',
          content: ''
        }))
        break
      case 'thinking':
        if (event.content) {
          state.accumulatedThinking += event.content
          if (options.persistContentLogs) {
            trackPendingLogWrite(runId, addLog({
              runId,
              stepId: options.stepId,
              scope: options.scope,
              type: 'thinking',
              content: event.content
            }))
          }
        }
        break
      case 'tool_use':
        if (event.toolName && event.toolCallId) {
          const toolCall: ToolCall = {
            id: event.toolCallId,
            name: event.toolName,
            arguments: event.toolInput || {},
            status: 'running'
          }
          const existingIndex = state.toolCalls.findIndex((item) => item.id === toolCall.id)
          if (existingIndex >= 0) {
            state.toolCalls[existingIndex] = toolCall
          } else {
            state.toolCalls.push(toolCall)
          }
          trackPendingLogWrite(runId, addLog({
            runId,
            stepId: options.stepId,
            scope: options.scope,
            type: 'tool_use',
            content: JSON.stringify(event.toolInput ?? {}, null, 2),
            metadata: {
              toolName: event.toolName,
              toolCallId: event.toolCallId,
              toolInput: JSON.stringify(event.toolInput ?? {})
            }
          }))
        }
        break
      case 'tool_input_delta': {
        const targetToolCall = (event.toolCallId
          ? state.toolCalls.find((tool) => tool.id === event.toolCallId)
          : null) || [...state.toolCalls].reverse().find((tool) => tool.status === 'running')

        if (targetToolCall) {
          targetToolCall.arguments = mergeToolInputArguments(targetToolCall.arguments, event.toolInput)
        }

        if (event.toolInput) {
          trackPendingLogWrite(runId, addLog({
            runId,
            stepId: options.stepId,
            scope: options.scope,
            type: 'tool_input_delta',
            content: typeof event.toolInput.raw === 'string'
              ? event.toolInput.raw
              : JSON.stringify(event.toolInput),
            metadata: {
              toolCallId: event.toolCallId,
              toolInput: typeof event.toolInput.raw === 'string'
                ? event.toolInput.raw
                : JSON.stringify(event.toolInput)
            }
          }))
        }
        break
      }
      case 'tool_result': {
        const result = typeof event.toolResult === 'string'
          ? event.toolResult
          : JSON.stringify(event.toolResult, null, 2)
        const toolCall = event.toolCallId
          ? state.toolCalls.find((item) => item.id === event.toolCallId)
          : null
        if (toolCall) {
          toolCall.result = result
          toolCall.status = 'success'
        }
        trackPendingLogWrite(runId, addLog({
          runId,
          stepId: options.stepId,
          scope: options.scope,
          type: 'tool_result',
          content: result,
          metadata: {
            toolCallId: event.toolCallId,
            toolResult: result,
            isError: false
          }
        }))
        break
      }
      case 'error':
        if (event.error) {
          trackPendingLogWrite(runId, addLog({
            runId,
            stepId: options.stepId,
            scope: options.scope,
            type: 'error',
            content: event.error
          }))
        }
        break
      default:
        break
    }
  }

  async function executeControlTurn(run: SoloRun, runtime: Awaited<ReturnType<typeof resolveCoordinatorRuntime>>): Promise<ReturnType<typeof parseSoloCoordinatorDecision>> {
    const state = initExecutionState(run.id)
    const steps = getSteps.value(run.id)
    const projectStore = useProjectStore()
    if (!projectStore.projects.some((project) => project.id === run.projectId)) {
      await projectStore.loadProjects()
    }
    const project = projectStore.projects.find((project) => project.id === run.projectId)
    const mountedMemoryPrompt = await loadMountedMemoryPrompt(
      collectMountedMemoryLibraryIds(project?.memoryLibraryIds, run.memoryLibraryIds)
    )
    state.accumulatedContent = ''
    state.accumulatedThinking = ''
    state.currentStepId = null
    const participantExperts = resolveParticipantExperts(run)
    const provider = inferAgentProvider(runtime.agent)
    const useSchemaResponse = provider !== 'codex'
    const systemPrompt = buildSoloCoordinatorSystemPrompt(
      runtime.expertPrompt,
      buildExpertCatalogPrompt(participantExperts, useAgentStore().agents)
    )

    const response = await executeTurn({
      run,
      state,
      agent: runtime.agent,
      workingDirectory: runtime.workingDirectory,
      mcpServers: undefined,
      messages: [
        ...(mountedMemoryPrompt ? [createMessage(run.id, 'system', mountedMemoryPrompt)] : []),
        createMessage(run.id, 'system', systemPrompt),
        createMessage(run.id, 'user', buildSoloControlPrompt({
          run,
          steps,
          inputResponse: run.inputResponse
        }))
      ],
      responseMode: useSchemaResponse ? 'json_once' : 'stream_text',
      jsonSchema: useSchemaResponse ? buildSoloControlJsonSchema() : undefined,
      scope: 'coordinator',
      strategy: 'solo_control',
      runtimeLabel: inferAgentProvider(runtime.agent)?.toUpperCase() || runtime.agent.type,
      expertLabel: runtime.expertName,
      persistContentLogs: !useSchemaResponse,
      runtimeBindingKey: null
    })

    await appendUsageNotice(run.id)
    return parseSoloCoordinatorDecision(response.content)
  }

  async function executeStepTurn(
    run: SoloRun,
    step: SoloStep,
    doneWhen: string[],
    options: { resume?: boolean } = {}
  ): Promise<void> {
    const state = initExecutionState(run.id)
    const projectStore = useProjectStore()
    if (!projectStore.projects.some((project) => project.id === run.projectId)) {
      await projectStore.loadProjects()
    }
    const project = projectStore.projects.find((project) => project.id === run.projectId)
    const mountedMemoryPrompt = await loadMountedMemoryPrompt(
      collectMountedMemoryLibraryIds(project?.memoryLibraryIds, run.memoryLibraryIds)
    )
    const stepRuntime = await resolveStepRuntime(run, step)
    const binding = stepRuntime.bindingKey ? await getSoloRuntimeBinding(run.id, stepRuntime.bindingKey).catch(() => null) : null
    const selectedExpert = resolveExpertById(step.selectedExpertId, useAgentTeamsStore().experts)
    const resumeContext = options.resume ? buildSoloResumeContext(state.logs.filter((log) => log.stepId === step.id)) : ''
    const userPrompt = options.resume && binding?.externalSessionId
      ? '继续执行当前步骤，直到满足本步完成条件。'
      : buildSoloExecutionPrompt({
          run,
          step,
          doneWhen,
          inputResponse: run.inputResponse,
          resumeContext
        })

    state.accumulatedContent = ''
    state.accumulatedThinking = ''
    state.currentStepId = step.id

    await addLog({
      runId: run.id,
      stepId: step.id,
      scope: 'step',
      type: 'system',
      content: options.resume
        ? `继续执行步骤: ${step.title} · ${stepRuntime.expertName || '未指定专家'}`
        : `开始执行步骤: ${step.title} · ${stepRuntime.expertName || '未指定专家'}`
    })

    const invokeStepTurn = async (resumeSessionId?: string) => executeTurn({
      run,
      state,
      agent: stepRuntime.agent,
      workingDirectory: stepRuntime.workingDirectory,
      mcpServers: stepRuntime.mcpServers,
      messages: [
        ...(!resumeSessionId && mountedMemoryPrompt
          ? [createMessage(run.id, 'system', mountedMemoryPrompt)]
          : []),
        ...(!resumeSessionId
          ? [createMessage(run.id, 'system', buildSoloExecutionSystemPrompt(selectedExpert?.prompt || stepRuntime.expertPrompt))]
          : []),
        createMessage(run.id, 'user', userPrompt)
      ],
      responseMode: 'stream_text',
      stepId: step.id,
      scope: 'step',
      strategy: options.resume ? 'solo_step_resume' : 'solo_step_execution',
      runtimeLabel: inferAgentProvider(stepRuntime.agent)?.toUpperCase() || stepRuntime.agent.type,
      expertLabel: selectedExpert?.name || stepRuntime.expertName,
      resumeSessionId,
      persistContentLogs: true,
      runtimeBindingKey: stepRuntime.bindingKey
    })

    let response: Awaited<ReturnType<typeof executeTurn>>
    try {
      response = await invokeStepTurn(binding?.externalSessionId)
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      const shouldRetryWithoutResume = Boolean(
        binding?.externalSessionId
        && stepRuntime.bindingKey
        && isInvalidCliResumeError(errorMessage, stepRuntime.runtimeKey)
      )

      if (!shouldRetryWithoutResume) {
        throw error
      }

      await addLog({
        runId: run.id,
        stepId: step.id,
        scope: 'step',
        type: 'system',
        content: '检测到无效的专家续接会话，已切换为重新执行当前步骤提示。'
      })
      await deleteSoloRuntimeBinding(run.id, stepRuntime.bindingKey!).catch(() => {})
      response = await invokeStepTurn(undefined)
    }

    await appendUsageNotice(run.id, step.id)

    const formRequest = extractFirstFormRequest(response.content)
    if (formRequest) {
      const schema = formRequest.formSchema ?? formRequest.forms?.[0]
      if (!schema) {
        throw new Error('SOLO 步骤请求了表单但缺少表单结构')
      }

      await updateStep(step.id, {
        status: 'blocked',
        summary: compactSoloSummary(formRequest.question || '等待用户补充输入')
      })
      await useSoloRunStore().updateRun(run.id, {
        status: 'blocked',
        executionStatus: 'blocked',
        inputRequest: buildSoloInputRequest(schema, formRequest.question, 'execution', step.id),
        currentStepId: step.id,
        inputResponse: null,
        lastError: null
      })
      return
    }

    const parsedResult = parseExecutionResult(response.content)
    await updateStep(step.id, {
      status: 'completed',
      summary: compactSoloSummary(parsedResult.summary),
      resultSummary: compactSoloSummary(parsedResult.summary),
      resultFiles: parsedResult.files,
      failReason: null,
      completedAt: new Date().toISOString()
    })
    await addLog({
      runId: run.id,
      stepId: step.id,
      scope: 'step',
      type: 'system',
      content: '步骤执行完成'
    })
  }

  async function runLoop(runId: string, options: { resume?: boolean } = {}): Promise<void> {
    const runStore = useSoloRunStore()
    let run = await runStore.getRun(runId)
    const runtime = await resolveCoordinatorRuntime(run)
    const state = initExecutionState(runId)

    state.status = 'running'
    state.startedAt = state.startedAt || new Date().toISOString()
    state.completedAt = null

    if (!options.resume) {
      await runStore.updateRun(runId, {
        status: 'running',
        executionStatus: 'running',
        startedAt: run.startedAt ?? new Date().toISOString(),
        stoppedAt: null,
        completedAt: null,
        lastError: null
      })
    }

    while (!stopRequestedRunIds.value.has(runId)) {
      run = await runStore.getRun(runId)
      if (run.status === 'blocked' || run.status === 'completed' || run.status === 'failed' || run.status === 'stopped') {
        break
      }

      let decision: ReturnType<typeof parseSoloCoordinatorDecision>
      try {
        decision = await executeControlTurn(run, runtime)
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error)
        if (errorMessage === SOLO_STOPPED_ERROR || stopRequestedRunIds.value.has(runId)) {
          break
        }
        finalizeRunningToolCalls(runId)
        state.status = 'error'
        state.completedAt = new Date().toISOString()
        await addLog({
          runId,
          scope: 'coordinator',
          type: 'error',
          content: errorMessage
        })
        await runStore.updateRun(runId, {
          status: 'failed',
          executionStatus: 'error',
          lastError: errorMessage
        })
        await clearRunRuntimeBindings(run).catch(() => {})
        return
      }

      if (decision.type === 'complete_run') {
        finalizeRunningToolCalls(runId)
        state.status = 'completed'
        state.completedAt = new Date().toISOString()
        await clearRunRuntimeBindings(run)
        await addLog({
          runId,
          scope: 'coordinator',
          type: 'system',
          content: `SOLO 运行完成: ${decision.summary}`
        })
        await runStore.updateRun(runId, {
          status: 'completed',
          executionStatus: 'completed',
          completedAt: new Date().toISOString(),
          currentStepId: null,
          lastError: null,
          inputRequest: null
        })
        return
      }

      if (decision.type === 'block_run') {
        finalizeRunningToolCalls(runId)
        state.status = 'blocked'
        state.completedAt = new Date().toISOString()
        await addLog({
          runId,
          scope: 'coordinator',
          type: 'system',
          content: `SOLO 暂停等待输入: ${decision.reason}`
        })
        await runStore.updateRun(runId, {
          status: 'blocked',
          executionStatus: 'blocked',
          lastError: decision.reason,
          inputRequest: decision.formSchema
            ? buildSoloInputRequest(decision.formSchema, decision.question || decision.reason, 'control')
            : null
        })
        return
      }

      if (decision.step.depth > run.maxDispatchDepth) {
        const message = `协调 AI 返回的步骤层数 ${decision.step.depth} 超过最大调度层数 ${run.maxDispatchDepth}`
        await addLog({
          runId,
          scope: 'coordinator',
          type: 'error',
          content: message
        })
        state.status = 'blocked'
        await runStore.updateRun(runId, {
          status: 'blocked',
          executionStatus: 'blocked',
          lastError: message
        })
        return
      }

      await addLog({
        runId,
        scope: 'coordinator',
        type: 'system',
        content: `协调 AI 派发步骤: ${decision.step.title}`
      })

      const participantExpertIds = new Set(resolveParticipantExperts(run).map((expert) => expert.id))
      const selectedExpertId = decision.step.selectedExpertId && participantExpertIds.has(decision.step.selectedExpertId)
        ? decision.step.selectedExpertId
        : undefined

      const step = await createStep({
        runId,
        stepRef: decision.step.stepRef,
        parentStepRef: decision.step.parentStepRef,
        depth: decision.step.depth,
        title: decision.step.title,
        description: decision.step.description,
        executionPrompt: [
          decision.step.executionPrompt,
          '',
          '本步完成条件:',
          ...decision.step.doneWhen.map((item, index) => `${index + 1}. ${item}`)
        ].join('\n'),
        selectedExpertId,
        status: 'running',
        summary: compactSoloSummary(decision.step.description),
        startedAt: new Date().toISOString()
      })

      await runStore.updateRun(runId, {
        status: 'running',
        executionStatus: 'running',
        currentDepth: decision.step.depth,
        currentStepId: step.id,
        inputRequest: null,
        inputResponse: null,
        lastError: null
      })

      try {
        await executeStepTurn(run, step, decision.step.doneWhen, { resume: false })
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error)
        if (errorMessage === SOLO_STOPPED_ERROR || stopRequestedRunIds.value.has(runId)) {
          break
        }
        finalizeRunningToolCalls(runId)
        state.status = 'error'
        state.completedAt = new Date().toISOString()
        await updateStep(step.id, {
          status: 'failed',
          failReason: errorMessage,
          summary: compactSoloSummary(errorMessage),
          completedAt: new Date().toISOString()
        })
        const stepRuntime = await resolveStepRuntime(run, step).catch(() => null)
        if (stepRuntime?.bindingKey) {
          await deleteSoloRuntimeBinding(run.id, stepRuntime.bindingKey).catch(() => {})
        }
        await addLog({
          runId,
          stepId: step.id,
          scope: 'step',
          type: 'error',
          content: errorMessage
        })
        await runStore.updateRun(runId, {
          status: 'failed',
          executionStatus: 'error',
          lastError: errorMessage,
          currentStepId: step.id
        })
        await clearRunRuntimeBindings(run).catch(() => {})
        return
      }

      run = await runStore.getRun(runId)
      if (run.status === 'blocked') {
        state.status = 'blocked'
        return
      }
    }

    if (stopRequestedRunIds.value.has(runId)) {
      stopRequestedRunIds.value.delete(runId)
    }
  }

  async function startRun(runId: string): Promise<void> {
    await Promise.all([
      loadSteps(runId),
      loadLogs(runId)
    ])
    await runLoop(runId)
  }

  async function pauseRun(runId: string): Promise<void> {
    const runStore = useSoloRunStore()
    const state = initExecutionState(runId)
    const activeStep = state.currentStepId
      ? getSteps.value(runId).find((step) => step.id === state.currentStepId && step.status === 'running') || null
      : null
    stopRequestedRunIds.value.add(runId)
    if (state.sessionId) {
      agentExecutor.abort(state.sessionId)
    }
    state.status = 'paused'
    state.completedAt = new Date().toISOString()
    await addLog({
      runId,
      stepId: activeStep?.id,
      scope: activeStep ? 'step' : 'coordinator',
      type: 'system',
      content: 'SOLO 运行已暂停'
    })
    await runStore.updateRun(runId, {
      status: 'paused',
      executionStatus: 'paused',
      stoppedAt: new Date().toISOString(),
      lastError: null
    })
  }

  async function resumeRun(runId: string): Promise<void> {
    const runStore = useSoloRunStore()
    const run = await runStore.getRun(runId)
    const state = initExecutionState(runId)
    state.status = 'running'
    stopRequestedRunIds.value.delete(runId)
    await runStore.updateRun(runId, {
      status: 'running',
      executionStatus: 'running',
      lastError: null
    })

    const currentStep = run.currentStepId
      ? getSteps.value(runId).find((step) => step.id === run.currentStepId)
      : null
    if (currentStep && currentStep.status === 'running') {
      const doneWhen = (currentStep.executionPrompt || '')
        .split('\n')
        .filter((line) => /^\d+\./.test(line.trim()))
        .map((line) => line.replace(/^\d+\.\s*/, '').trim())
        .filter(Boolean)
      try {
        await executeStepTurn(run, currentStep, doneWhen.length > 0 ? doneWhen : ['完成当前步骤目标'], { resume: true })
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error)
        if (errorMessage === SOLO_STOPPED_ERROR || stopRequestedRunIds.value.has(runId)) {
          return
        }
        await addLog({
          runId,
          stepId: currentStep.id,
          scope: 'step',
          type: 'error',
          content: errorMessage
        })
        await runStore.updateRun(runId, {
          status: 'failed',
          executionStatus: 'error',
          lastError: errorMessage
        })
        return
      }
      const refreshedRun = await runStore.getRun(runId)
      if (refreshedRun.status === 'blocked') {
        state.status = 'blocked'
        return
      }
    }

    await runLoop(runId, { resume: true })
  }

  async function submitRunInput(runId: string, values: Record<string, unknown>): Promise<void> {
    const runStore = useSoloRunStore()
    const run = await runStore.getRun(runId)
    const blockedStepId = run.inputRequest?.source === 'execution'
      ? run.inputRequest.stepId
      : undefined
    await addLog({
      runId,
      stepId: run.inputRequest?.stepId,
      scope: run.inputRequest?.stepId ? 'step' : 'coordinator',
      type: 'system',
      content: `用户提交补充信息: ${JSON.stringify(values)}`
    })
    await runStore.updateRun(runId, {
      inputRequest: null,
      inputResponse: values,
      status: 'running',
      executionStatus: 'running',
      lastError: null
    })
    if (blockedStepId) {
      await updateStep(blockedStepId, {
        status: 'running',
        failReason: null
      })
    }
    await resumeRun(runId)
  }

  async function clearRunRuntimeBindings(run: SoloRun): Promise<void> {
    const agentStore = useAgentStore()
    const agentTeamsStore = useAgentTeamsStore()

    await Promise.all([
      agentStore.loadAgents(),
      agentTeamsStore.loadExperts()
    ])

    const bindingKeys = new Set<string>()
    const coordinatorRuntime = resolveExpertRuntime(
      resolveExpertById(run.coordinatorExpertId, agentTeamsStore.experts)
        || agentTeamsStore.builtinSoloCoordinatorExpert
        || agentTeamsStore.builtinPlannerExpert
        || agentTeamsStore.builtinGeneralExpert,
      agentStore.agents,
      run.coordinatorModelId
    )

    if (coordinatorRuntime?.agent) {
      const key = buildSoloBindingKey(coordinatorRuntime.agent, 'coordinator', run.coordinatorExpertId)
      if (key) {
        bindingKeys.add(key)
      }
    }

    resolveParticipantExperts(run).forEach((expert) => {
      const runtime = resolveExpertRuntime(expert, agentStore.agents)
      if (!runtime?.agent) {
        return
      }
      const key = buildSoloBindingKey(runtime.agent, 'expert', expert.id)
      if (key) {
        bindingKeys.add(key)
      }
    })

    const fallbackAgent = agentStore.agents.find((item) => item.type === 'cli') || agentStore.agents[0]
    if (fallbackAgent) {
      const fallbackKey = buildSoloBindingKey(fallbackAgent, 'expert', 'default')
      if (fallbackKey) {
        bindingKeys.add(fallbackKey)
      }
    }

    await Promise.allSettled(
      Array.from(bindingKeys).map((bindingKey) => deleteSoloRuntimeBinding(run.id, bindingKey))
    )
  }

  async function stopRun(runId: string): Promise<void> {
    const runStore = useSoloRunStore()
    const state = initExecutionState(runId)
    stopRequestedRunIds.value.add(runId)
    if (state.sessionId) {
      agentExecutor.abort(state.sessionId)
    }
    finalizeRunningToolCalls(runId)
    state.status = 'stopped'
    state.completedAt = new Date().toISOString()

    const run = await runStore.getRun(runId)
    await clearRunRuntimeBindings(run)

    await addLog({
      runId,
      stepId: state.currentStepId || undefined,
      scope: state.currentStepId ? 'step' : 'system',
      type: 'system',
      content: 'SOLO 运行已停止'
    })
    await runStore.updateRun(runId, {
      status: 'stopped',
      executionStatus: 'stopped',
      stoppedAt: new Date().toISOString(),
      lastError: null
    })
  }

  return {
    stepsByRun,
    executionStates,
    getSteps,
    getExecutionState,
    getCurrentStep,
    getCurrentStepLogs,
    loadSteps,
    loadLogs,
    startRun,
    pauseRun,
    resumeRun,
    stopRun,
    submitRunInput
  }
})
