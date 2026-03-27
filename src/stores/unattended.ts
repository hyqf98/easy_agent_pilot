import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { unattendedService } from '@/services/unattended/UnattendedService'
import { buildUnattendedSystemPrompt } from '@/services/unattended/promptBuilder'
import {
  buildUnattendedWorkspaceContext,
  getUnattendedDeliveryCapabilities
} from '@/services/unattended/contextBuilder'
import {
  detectUnattendedIntent,
  extractTaskExecutionDraft,
  extractTaskCreateDraft,
  extractTaskUpdateDraft,
  parseStructuredFormResponse,
  type UnattendedIntent
} from '@/services/unattended/intentParser'
import type {
  RuntimeStatusEvent,
  RuntimeStatusSummary,
  UnattendedChannel,
  UnattendedChannelAccount,
  UnattendedEventRecord,
  UnattendedInboundMessage,
  UnattendedThread
} from '@/services/unattended/types'
import { inferAgentProvider, useAgentStore } from '@/stores/agent'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { usePlanStore } from '@/stores/plan'
import { useTaskStore } from '@/stores/task'
import { useTaskExecutionStore } from '@/stores/taskExecution'
import { useTaskSplitStore } from '@/stores/taskSplit'
import { useProjectStore, type Project } from '@/stores/project'
import { useSessionStore } from '@/stores/session'
import { useMessageStore } from '@/stores/message'
import { conversationService } from '@/services/conversation/ConversationService'
import type { AgentConfig } from '@/stores/agent'
import type { DynamicFormSchema, Plan, Task, TaskPriority, TaskStatus, UpdateTaskInput } from '@/types/plan'

interface WeixinLoginSession {
  qrcode: string
  qrcodeImg: string
  status: string
}

const INCOMING_EVENT = 'unattended:incoming-message'
const STATUS_EVENT = 'unattended:runtime-status'
const LOGIN_POLL_INTERVAL_MS = 2500
const PROCESSING_NOTICE_DELAY_MS = 1200

function compactText(value: string, fallback: string): string {
  const normalized = value.replace(/\s+/g, ' ').trim()
  if (!normalized) {
    return fallback
  }
  return normalized.length > 120 ? `${normalized.slice(0, 120)}...` : normalized
}

function normalizeUnattendedAssistantReply(value: string): string {
  const normalized = value.replace(/\r\n/g, '\n').trim()
  if (!normalized) {
    return ''
  }

  const lines = normalized
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)
    .filter(line => !/^请执行此操作[。.!！]?$/u.test(line))
    .filter(line => !/^内部(执行|调用|过程)/u.test(line))

  let merged = lines.join('\n')
    .replace(/^确认(?:一下)?[:：]\s*/u, '')
    .replace(/\n{3,}/g, '\n\n')
    .trim()

  if (/^将/u.test(merged) && /^确认(?:一下)?[:：]/u.test(normalized)) {
    merged = `已${merged}`
  }

  return merged
}

function buildProcessingNotice(intent: UnattendedIntent): string {
  switch (intent.type) {
    case 'list_projects':
      return '已收到，正在整理当前工作区项目列表，马上回复你。'
    case 'create_plan':
      return '已收到，正在为你创建计划并整理执行上下文，完成后马上回复你。'
    case 'create_task':
      return '已收到，正在为你创建任务，完成后马上回复你。'
    case 'start_task':
    case 'update_task':
    case 'stop_task':
      return '已收到，正在更新任务状态，完成后马上回复你。'
    case 'start_split':
    case 'continue_split':
    case 'form_response':
      return '已收到，正在继续处理计划拆分，完成后马上回复你。'
    case 'start_plan':
    case 'pause_plan':
    case 'resume_plan':
      return '已收到，正在更新计划执行状态，完成后马上回复你。'
    case 'query_plan_progress':
    case 'query_task_status':
    case 'query_execution':
      return '已收到，正在查询当前进度信息，稍后把结果发给你。'
    case 'chat':
    default:
      return '已收到，正在处理你的消息，完成后马上回复你。'
  }
}

function shouldDelayProcessingNotice(intent: UnattendedIntent): boolean {
  return !['list_projects', 'switch_project', 'switch_agent', 'switch_model'].includes(intent.type)
}

function schemaToReplyTemplate(schema: DynamicFormSchema): string {
  return [
    `当前需要补充信息：${schema.title}`,
    ...schema.fields.map(field => `- ${field.label}: `),
    '请按“字段: 内容”逐行回复。'
  ].join('\n')
}

function normalizeLoginStatus(status: string): string {
  switch (status) {
    case 'wait':
      return 'waiting'
    case 'scaned':
      return 'scanned'
    default:
      return status
  }
}

function mapStructuredValuesToSchema(
  schema: DynamicFormSchema,
  values: Record<string, string>
): Record<string, string> {
  const mappedEntries = Object.entries(values)
    .map(([key, value]) => {
      const field = schema.fields.find(item =>
        item.name === key
        || item.label === key
        || item.name.toLowerCase() === key.toLowerCase()
        || item.label.toLowerCase() === key.toLowerCase()
      )
      return field ? [field.name, value] as const : null
    })
    .filter((entry): entry is readonly [string, string] => entry !== null)

  return Object.fromEntries(mappedEntries)
}

function normalizeTaskTitle(value?: string): string | undefined {
  const normalized = value
    ?.replace(/^(任务|创建任务|新建任务|新增任务)/u, '')
    ?.replace(/[，。,；;！!].*$/u, '')
    ?.trim()

  return normalized || undefined
}

function translateTaskPriority(priority?: TaskPriority): string | null {
  switch (priority) {
    case 'high':
      return '高'
    case 'medium':
      return '中'
    case 'low':
      return '低'
    default:
      return null
  }
}

function translateTaskStatus(status?: TaskStatus): string | null {
  switch (status) {
    case 'pending':
      return '待办'
    case 'in_progress':
      return '进行中'
    case 'completed':
      return '已完成'
    case 'blocked':
      return '阻塞'
    case 'failed':
      return '失败'
    case 'cancelled':
      return '已取消'
    default:
      return null
  }
}

export const useUnattendedStore = defineStore('unattended', () => {
  const channels = ref<UnattendedChannel[]>([])
  const accounts = ref<UnattendedChannelAccount[]>([])
  const threads = ref<UnattendedThread[]>([])
  const events = ref<UnattendedEventRecord[]>([])
  const runtimeStatuses = ref<RuntimeStatusSummary[]>([])
  const loginSessions = ref<Record<string, WeixinLoginSession>>({})
  const isLoading = ref(false)
  const initialized = ref(false)
  const handlingThreadIds = ref<Set<string>>(new Set())
  const unlisteners = ref<UnlistenFn[]>([])
  const loginPollTimers = ref<Record<string, number>>({})
  const pendingChannelPatches = ref<Record<string, Partial<UnattendedChannel>>>({})

  const channelsById = computed(() => new Map(channels.value.map(item => [item.id, item])))
  const accountsById = computed(() => new Map(accounts.value.map(item => [item.id, item])))
  const threadsById = computed(() => new Map(threads.value.map(item => [item.id, item])))

  function mergeChannelPatch(channel: UnattendedChannel): UnattendedChannel {
    const patch = pendingChannelPatches.value[channel.id]
    return patch ? { ...channel, ...patch } : channel
  }

  function applyPendingChannelPatches(nextChannels: UnattendedChannel[]): UnattendedChannel[] {
    return nextChannels.map(mergeChannelPatch)
  }

  function patchChannelLocally(channelId: string, patch: Partial<UnattendedChannel>): void {
    pendingChannelPatches.value = {
      ...pendingChannelPatches.value,
      [channelId]: {
        ...(pendingChannelPatches.value[channelId] || {}),
        ...patch
      }
    }

    channels.value = channels.value.map(channel =>
      channel.id === channelId
        ? { ...channel, ...patch }
        : channel
    )
  }

  function clearChannelPatch(channelId: string): void {
    if (!pendingChannelPatches.value[channelId]) {
      return
    }
    const nextPatches = { ...pendingChannelPatches.value }
    delete nextPatches[channelId]
    pendingChannelPatches.value = nextPatches
  }

  async function loadAll(): Promise<void> {
    isLoading.value = true
    try {
      const [nextChannels, nextAccounts, nextThreads, nextEvents, nextRuntimeStatuses] = await Promise.all([
        unattendedService.listChannels(),
        unattendedService.listAccounts(),
        unattendedService.listThreads(),
        unattendedService.listEvents({ limit: 200 }),
        unattendedService.listRuntimeStatus()
      ])
      channels.value = applyPendingChannelPatches(nextChannels)
      accounts.value = nextAccounts
      threads.value = nextThreads
      events.value = nextEvents
      runtimeStatuses.value = nextRuntimeStatuses
    } finally {
      isLoading.value = false
    }
  }

  async function initialize(): Promise<void> {
    if (initialized.value) {
      return
    }
    initialized.value = true
    await loadAll()

    const incomingUnlisten = await listen<UnattendedInboundMessage>(INCOMING_EVENT, (event) => {
      void handleInboundMessage(event.payload)
    })
    const statusUnlisten = await listen<RuntimeStatusEvent>(STATUS_EVENT, (event) => {
      applyRuntimeStatus(event.payload)
    })
    unlisteners.value.push(incomingUnlisten, statusUnlisten)
  }

  function dispose(): void {
    unlisteners.value.forEach(unlisten => unlisten())
    unlisteners.value = []
    Object.values(loginPollTimers.value).forEach(timer => window.clearTimeout(timer))
    loginPollTimers.value = {}
    initialized.value = false
  }

  function clearLoginPolling(channelId: string): void {
    const timer = loginPollTimers.value[channelId]
    if (timer) {
      window.clearTimeout(timer)
    }
    const nextTimers = { ...loginPollTimers.value }
    delete nextTimers[channelId]
    loginPollTimers.value = nextTimers
  }

  function scheduleLoginPolling(channelId: string): void {
    clearLoginPolling(channelId)
    loginPollTimers.value = {
      ...loginPollTimers.value,
      [channelId]: window.setTimeout(() => {
        void pollWeixinLogin(channelId)
      }, LOGIN_POLL_INTERVAL_MS)
    }
  }

  function applyRuntimeStatus(payload: RuntimeStatusEvent): void {
    const index = runtimeStatuses.value.findIndex(item => item.channelAccountId === payload.channelAccountId)
    if (index >= 0) {
      runtimeStatuses.value[index] = {
        ...runtimeStatuses.value[index],
        runtimeStatus: payload.runtimeStatus,
        lastError: payload.lastError
      }
    } else {
      runtimeStatuses.value.push({
        accountId: accountsById.value.get(payload.channelAccountId)?.accountId || payload.channelAccountId,
        channelAccountId: payload.channelAccountId,
        runtimeStatus: payload.runtimeStatus,
        lastError: payload.lastError
      })
    }
    const account = accounts.value.find(item => item.id === payload.channelAccountId)
    if (account) {
      account.runtimeStatus = payload.runtimeStatus
      account.lastError = payload.lastError
    }
  }

  async function createWeixinChannel(): Promise<void> {
    const agentStore = useAgentStore()
    const agentConfigStore = useAgentConfigStore()
    if (agentStore.agents.length === 0) {
      await agentStore.loadAgents()
    }

    const defaultAgent = agentStore.agents[0]
    let defaultModelId: string | undefined
    if (defaultAgent) {
      const models = await agentConfigStore.ensureModelsConfigs(
        defaultAgent.id,
        inferAgentProvider(defaultAgent)
      )
      const enabledModels = models.filter(model => model.enabled)
      const preferredModel = enabledModels.find(model => model.isDefault) || enabledModels[0]
      defaultModelId = preferredModel?.modelId
    }

    await unattendedService.createChannel({
      channelType: 'weixin',
      name: `微信监听 ${channels.value.filter(item => item.channelType === 'weixin').length + 1}`,
      enabled: true,
      defaultAgentId: defaultAgent?.id,
      defaultModelId,
      replyStyle: 'final_only',
      allowAllSenders: true
    })
    await loadAll()
  }

  async function updateChannel(id: string, input: Partial<UnattendedChannel>): Promise<void> {
    const patch: Partial<UnattendedChannel> = {}
    if (Object.prototype.hasOwnProperty.call(input, 'name')) {
      patch.name = input.name
    }
    if (Object.prototype.hasOwnProperty.call(input, 'enabled')) {
      patch.enabled = input.enabled
    }
    if (Object.prototype.hasOwnProperty.call(input, 'defaultProjectId')) {
      patch.defaultProjectId = input.defaultProjectId
    }
    if (Object.prototype.hasOwnProperty.call(input, 'defaultAgentId')) {
      patch.defaultAgentId = input.defaultAgentId
    }
    if (Object.prototype.hasOwnProperty.call(input, 'defaultModelId')) {
      patch.defaultModelId = input.defaultModelId
    }
    if (Object.prototype.hasOwnProperty.call(input, 'replyStyle')) {
      patch.replyStyle = input.replyStyle
    }
    if (Object.prototype.hasOwnProperty.call(input, 'allowAllSenders')) {
      patch.allowAllSenders = input.allowAllSenders
    }

    patchChannelLocally(id, patch)

    try {
      const nextChannel = await unattendedService.updateChannel(id, {
        name: input.name,
        enabled: input.enabled,
        defaultProjectId: input.defaultProjectId,
        defaultAgentId: input.defaultAgentId,
        defaultModelId: input.defaultModelId,
        replyStyle: input.replyStyle,
        allowAllSenders: input.allowAllSenders
      })
      clearChannelPatch(id)
      channels.value = channels.value.map(channel =>
        channel.id === id ? nextChannel : channel
      )
      await loadAll()
    } catch (error) {
      clearChannelPatch(id)
      await loadAll()
      throw error
    }
  }

  async function deleteChannel(id: string): Promise<void> {
    await unattendedService.stopRuntime(id).catch(() => undefined)
    clearLoginPolling(id)
    await unattendedService.deleteChannel(id)
    await loadAll()
  }

  async function startWeixinLogin(channelId: string): Promise<void> {
    clearLoginPolling(channelId)
    const login = await unattendedService.startWeixinLogin(channelId)
    loginSessions.value[channelId] = {
      qrcode: login.qrcode,
      qrcodeImg: login.qrcodeImg,
      status: 'waiting'
    }
    scheduleLoginPolling(channelId)
  }

  async function pollWeixinLogin(channelId: string): Promise<void> {
    const loginSession = loginSessions.value[channelId]
    if (!loginSession) {
      return
    }
    const status = await unattendedService.getWeixinLoginStatus(channelId, loginSession.qrcode)
    const normalizedStatus = normalizeLoginStatus(status.status)
    loginSessions.value[channelId] = {
      ...loginSession,
      status: normalizedStatus
    }
    if (normalizedStatus === 'confirmed') {
      clearLoginPolling(channelId)
      const nextLoginSessions = { ...loginSessions.value }
      delete nextLoginSessions[channelId]
      loginSessions.value = nextLoginSessions
      await unattendedService.startRuntime(channelId).catch(() => undefined)
      await loadAll()
      return
    }
    if (normalizedStatus === 'waiting' || normalizedStatus === 'scanned') {
      scheduleLoginPolling(channelId)
      return
    }
    clearLoginPolling(channelId)
  }

  async function startRuntime(channelId: string): Promise<void> {
    await unattendedService.startRuntime(channelId)
    await loadAll()
  }

  async function stopRuntime(channelId: string): Promise<void> {
    clearLoginPolling(channelId)
    if (loginSessions.value[channelId]) {
      const nextLoginSessions = { ...loginSessions.value }
      delete nextLoginSessions[channelId]
      loginSessions.value = nextLoginSessions
    }

    accounts.value = accounts.value.map(account =>
      account.channelId === channelId
        ? { ...account, runtimeStatus: 'stopped', lastError: undefined }
        : account
    )
    runtimeStatuses.value = runtimeStatuses.value.map(status =>
      accountsById.value.get(status.channelAccountId)?.channelId === channelId
        ? { ...status, runtimeStatus: 'stopped', lastError: undefined }
        : status
    )

    await unattendedService.stopRuntime(channelId)
    await loadAll()
  }

  async function logoutAccount(accountRowId: string): Promise<void> {
    const account = accounts.value.find(item => item.id === accountRowId)
    if (account) {
      await unattendedService.stopRuntime(account.channelId).catch(() => undefined)
    }
    await unattendedService.logoutAccount(accountRowId)
    await loadAll()
  }

  async function handleInboundMessage(payload: UnattendedInboundMessage): Promise<void> {
    if (handlingThreadIds.value.has(payload.threadId)) {
      return
    }

    handlingThreadIds.value = new Set(handlingThreadIds.value).add(payload.threadId)
    const intent = detectUnattendedIntent(payload.text)
    let processingNoticeTimer: number | null = null
    let processingNoticeTask: Promise<void> | null = null

    if (shouldDelayProcessingNotice(intent)) {
      const processingNotice = buildProcessingNotice(intent)
      processingNoticeTimer = window.setTimeout(() => {
        processingNoticeTask = unattendedService.sendText(
          payload.channelAccountId,
          payload.peerId,
          processingNotice,
          payload.contextToken,
          payload.messageId
        ).catch(console.error)
      }, PROCESSING_NOTICE_DELAY_MS)
    }

    try {
      await loadAll()
      const thread = threadsById.value.get(payload.threadId)
      const account = accountsById.value.get(payload.channelAccountId)
      const channel = account ? channelsById.value.get(account.channelId) : undefined

      if (!thread || !account || !channel) {
        if (processingNoticeTimer) {
          window.clearTimeout(processingNoticeTimer)
          processingNoticeTimer = null
        }
        return
      }

      const reply = await routeInboundMessage(channel, thread, payload)
      if (processingNoticeTimer) {
        window.clearTimeout(processingNoticeTimer)
        processingNoticeTimer = null
      }
      if (processingNoticeTask) {
        await processingNoticeTask
      }
      if (!reply) {
        return
      }

      await unattendedService.sendText(
        payload.channelAccountId,
        payload.peerId,
        reply,
        payload.contextToken,
        payload.messageId
      )
      await loadAll()
    } catch (error) {
      if (processingNoticeTimer) {
        window.clearTimeout(processingNoticeTimer)
        processingNoticeTimer = null
      }
      if (processingNoticeTask) {
        try {
          await processingNoticeTask
        } catch {
          // ignore delayed processing notice failures and continue with final error reply
        }
      }
      const message = error instanceof Error ? error.message : String(error)
      await unattendedService.sendText(
        payload.channelAccountId,
        payload.peerId,
        `处理失败：${message}`,
        payload.contextToken,
        payload.messageId
      ).catch(console.error)
    } finally {
      if (processingNoticeTimer) {
        window.clearTimeout(processingNoticeTimer)
      }
      const next = new Set(handlingThreadIds.value)
      next.delete(payload.threadId)
      handlingThreadIds.value = next
    }
  }

  async function routeInboundMessage(
    channel: UnattendedChannel,
    thread: UnattendedThread,
    payload: UnattendedInboundMessage
  ): Promise<string | null> {
    const agentStore = useAgentStore()
    const agentConfigStore = useAgentConfigStore()
    const projectStore = useProjectStore()
    const planStore = usePlanStore()
    const taskStore = useTaskStore()
    const taskExecutionStore = useTaskExecutionStore()
    const taskSplitStore = useTaskSplitStore()
    const messageStore = useMessageStore()

    if (projectStore.projects.length === 0) {
      await projectStore.loadProjects()
    }
    if (agentStore.agents.length === 0) {
      await agentStore.loadAgents()
    }

    const replyWithTranscript = async (
      replyText: string,
      options?: {
        projectId?: string
      }
    ): Promise<string> => {
      await appendThreadTranscript(channel, thread, payload.text, replyText, options?.projectId)
      return replyText
    }

    const syncVisiblePlanTaskState = async (targetProjectId: string, targetPlanId?: string): Promise<void> => {
      if (projectStore.currentProjectId !== targetProjectId) {
        return
      }

      await planStore.loadPlans(targetProjectId)

      if (!targetPlanId) {
        return
      }

      const shouldRefreshTasks = planStore.currentPlanId === targetPlanId
        || taskStore.tasks.some(task => task.planId === targetPlanId)

      if (!shouldRefreshTasks) {
        return
      }

      await taskStore.loadTasks(targetPlanId)
    }

    const intent = detectUnattendedIntent(payload.text)

    if (intent.type === 'list_projects') {
      return replyWithTranscript(buildProjectSummary(channel, thread, projectStore))
    }

    if (intent.type === 'switch_project') {
      const nextProject = resolveProjectFromIntent(intent, projectStore.projects)
      if (!nextProject) {
        return '没有匹配到要切换的项目，请在话里带上项目名称。'
      }
      projectStore.setCurrentProject(nextProject.id)
      await unattendedService.updateThreadContext(thread.id, {
        activeProjectId: nextProject.id
      })
      return replyWithTranscript(`后续将默认使用项目“${nextProject.name}”处理这个微信线程。`, {
        projectId: nextProject.id
      })
    }

    if (intent.type === 'switch_agent') {
      const agent = resolveAgentFromIntent(intent, agentStore.agents)
      if (!agent) {
        return '没有匹配到要切换的 Agent，请在话里带上 Agent 名称。'
      }
      const modelId = await resolveModelId(agentConfigStore, agent, channel.defaultModelId)
      await unattendedService.updateThreadContext(thread.id, {
        activeAgentId: agent.id,
        activeModelId: modelId
      })
      return replyWithTranscript(`后续将默认使用 ${agent.name} 处理这个微信线程。`)
    }

    if (intent.type === 'switch_model') {
      const activeAgent = resolveActiveAgent(thread, channel, agentStore.agents)
      if (!activeAgent) {
        return '当前线程没有可用 Agent，请先切换或绑定 Agent。'
      }
      const modelId = await resolveModelFromIntent(agentConfigStore, activeAgent, intent.modelHint || intent.rawText)
      if (!modelId) {
        return '没有匹配到要切换的模型，请在对话里带上模型名称。'
      }
      await unattendedService.updateThreadContext(thread.id, {
        activeAgentId: activeAgent.id,
        activeModelId: modelId
      })
      return replyWithTranscript(`后续将默认使用模型“${modelId}”处理这个微信线程。`)
    }

    const projectContext = resolveProjectContext(thread, channel, projectStore)
    if (!projectContext.projectId || !projectContext.project) {
      return '当前还没有可用项目，请先在软件里创建项目，或在微信里发送“切换到项目 xxx”。'
    }

    const projectId = projectContext.projectId
    const project = projectContext.project

    if (thread.activeProjectId !== projectId) {
      await unattendedService.updateThreadContext(thread.id, {
        activeProjectId: projectId
      })
    }

    await planStore.loadPlans(projectId)

    if (intent.type === 'create_plan') {
      const activeAgent = resolveActiveAgent(thread, channel, agentStore.agents)
      if (!activeAgent) {
        return '当前线程没有可用 Agent，请先在设置页绑定默认 Agent。'
      }
      const modelId = await resolveModelId(
        agentConfigStore,
        activeAgent,
        thread.activeModelId || (activeAgent.id === channel.defaultAgentId ? channel.defaultModelId : undefined)
      )
      const planName = compactText(intent.planName || intent.rawText, '新建计划')
      const newPlan = await planStore.createPlan({
        projectId,
        name: planName,
        description: payload.text,
        splitMode: 'ai',
        splitAgentId: activeAgent.id,
        splitModelId: modelId
      })
      await unattendedService.updateThreadContext(thread.id, {
        activeProjectId: projectId,
        activeAgentId: activeAgent.id,
        activeModelId: modelId,
        lastPlanId: newPlan.id
      })

      if (intent.executeAfterCreate) {
        await taskSplitStore.initSession({
          planId: newPlan.id,
          planName: newPlan.name,
          planDescription: newPlan.description,
          granularity: newPlan.granularity,
          agentId: activeAgent.id,
          modelId: modelId || '',
          workingDirectory: project.path
        })
        return replyWithTranscript(`计划“${newPlan.name}”已创建，并开始进入拆分流程。`, {
          projectId
        })
      }

      return replyWithTranscript(`计划“${newPlan.name}”已创建完成。`, {
        projectId
      })
    }

    if (intent.type === 'query_plan_progress') {
      return replyWithTranscript(await buildPlanSummary(projectId), { projectId })
    }

    if (intent.type === 'query_execution') {
      return replyWithTranscript(await buildExecutionSummary(projectId), { projectId })
    }

    if (intent.type === 'query_task_status') {
      return replyWithTranscript(await buildTaskSummary(projectId, intent), { projectId })
    }

    if (intent.type === 'create_task') {
      const projectPlans = planStore.plans.filter(item => item.projectId === projectId)
      const taskDraft = extractTaskCreateDraft(payload.text)
      const targetPlan = resolvePlanByText(taskDraft.planHint || intent.planName || intent.rawText, projectPlans)
        || (thread.lastPlanId ? projectPlans.find(item => item.id === thread.lastPlanId) : undefined)
        || projectPlans.find(item => item.status === 'executing' || item.executionStatus === 'running')
        || projectPlans[0]

      if (!targetPlan) {
        return '当前项目下还没有可承载任务的计划，请先创建计划。'
      }

      const title = normalizeTaskTitle(taskDraft.title)
      if (!title || title.length < 2) {
        return `要创建任务的话，请至少告诉我任务标题。当前可用计划是“${targetPlan.name}”。`
      }

      const activeAgent = resolveActiveAgent(thread, channel, agentStore.agents)
      const modelId = activeAgent
        ? await resolveModelId(
          agentConfigStore,
          activeAgent,
          thread.activeModelId || (activeAgent.id === channel.defaultAgentId ? channel.defaultModelId : undefined)
        )
        : undefined

      const createdTask = await taskStore.createTask({
        planId: targetPlan.id,
        title,
        description: payload.text,
        priority: taskDraft.priority,
        agentId: activeAgent?.id,
        modelId
      })

      await unattendedService.updateThreadContext(thread.id, {
        lastPlanId: targetPlan.id,
        lastTaskId: createdTask.id
      })
      await syncVisiblePlanTaskState(projectId, targetPlan.id)

      const priorityLabel = translateTaskPriority(createdTask.priority)
      const prioritySummary = priorityLabel ? `，优先级 ${priorityLabel}` : ''
      return replyWithTranscript(
        `已在计划“${targetPlan.name}”下创建任务“${createdTask.title}”${prioritySummary}。`,
        { projectId }
      )
    }

    if (intent.type === 'start_task' || intent.type === 'update_task' || intent.type === 'stop_task') {
      const projectPlans = planStore.plans.filter(item => item.projectId === projectId)
      const loadedTaskGroups = await loadTasksForPlanCandidates(projectPlans, thread)
      const updateDraft = extractTaskUpdateDraft(payload.text)
      const executionDraft = extractTaskExecutionDraft(payload.text)
      const targetHint = intent.type === 'start_task'
        ? (executionDraft.targetHint || intent.taskHint || intent.rawText)
        : (updateDraft.targetHint || intent.taskHint || intent.rawText)
      const taskCandidate = loadedTaskGroups
        .map(group => ({
          plan: group.plan,
          task: resolveTaskFromText(targetHint, group.tasks, thread)
        }))
        .find(group => group.task)

      if (!taskCandidate?.task) {
        return '没有匹配到对应任务，请在消息里带上任务标题。'
      }

      const targetTask = taskCandidate.task
      const targetPlan = taskCandidate.plan

      if (intent.type === 'start_task') {
        if (targetTask.status === 'completed') {
          return `任务“${targetTask.title}”已经完成，不需要再次启动。`
        }

        if (targetTask.status === 'cancelled') {
          return `任务“${targetTask.title}”当前已取消，如需重新执行，请先将状态改回待办。`
        }

        await planStore.startPlanExecution(targetPlan.id)

        if (taskExecutionStore.isTaskExecuting(targetTask.id)) {
          await syncVisiblePlanTaskState(projectId, targetPlan.id)
          await unattendedService.updateThreadContext(thread.id, {
            lastPlanId: targetPlan.id,
            lastTaskId: targetTask.id
          })
          return replyWithTranscript(
            `任务“${targetTask.title}”已经在执行队列中，当前状态为 ${translateTaskStatus(targetTask.status) || targetTask.status}。`,
            { projectId }
          )
        }

        if (targetTask.status === 'in_progress') {
          await taskExecutionStore.resumeTaskExecution(targetTask.id)
        } else {
          await taskExecutionStore.enqueueTask(targetPlan.id, targetTask.id)
        }

        const refreshedTask = taskStore.tasks.find(task => task.id === targetTask.id) || targetTask
        await unattendedService.updateThreadContext(thread.id, {
          lastPlanId: targetPlan.id,
          lastTaskId: refreshedTask.id
        })
        await syncVisiblePlanTaskState(projectId, targetPlan.id)
        return replyWithTranscript(
          `任务“${refreshedTask.title}”已开始执行，当前状态为 ${translateTaskStatus(refreshedTask.status) || refreshedTask.status}。`,
          { projectId }
        )
      }

      if (intent.type === 'stop_task') {
        const stoppedTask = await taskStore.stopTask(targetTask.id)
        await unattendedService.updateThreadContext(thread.id, {
          lastPlanId: targetPlan.id,
          lastTaskId: stoppedTask.id
        })
        await syncVisiblePlanTaskState(projectId, targetPlan.id)
        return replyWithTranscript(
          `任务“${stoppedTask.title}”已停止，当前状态为 ${translateTaskStatus(stoppedTask.status) || stoppedTask.status}。`,
          { projectId }
        )
      }

      const updates: UpdateTaskInput = {}
      const nextTitle = normalizeTaskTitle(updateDraft.title)
      if (nextTitle && nextTitle !== targetTask.title) {
        updates.title = nextTitle
      }
      if (updateDraft.description) {
        updates.description = updateDraft.description
      }
      if (updateDraft.priority && updateDraft.priority !== targetTask.priority) {
        updates.priority = updateDraft.priority
      }
      if (updateDraft.status && updateDraft.status !== targetTask.status) {
        updates.status = updateDraft.status
      }

      if (Object.keys(updates).length === 0) {
        return `我已经定位到任务“${targetTask.title}”，但还缺少明确的修改内容，请补充要改的标题、状态、优先级或说明。`
      }

      const updatedTask = await taskStore.updateTask(targetTask.id, updates)
      await unattendedService.updateThreadContext(thread.id, {
        lastPlanId: targetPlan.id,
        lastTaskId: updatedTask.id
      })
      await syncVisiblePlanTaskState(projectId, targetPlan.id)

      const updatedFields = [
        updates.title ? `标题改为“${updatedTask.title}”` : null,
        updates.status ? `状态改为 ${translateTaskStatus(updatedTask.status) || updatedTask.status}` : null,
        updates.priority ? `优先级改为 ${translateTaskPriority(updatedTask.priority) || updatedTask.priority}` : null,
        updates.description ? '说明已更新' : null
      ].filter((item): item is string => Boolean(item))

      return replyWithTranscript(
        `任务“${targetTask.title}”已更新：${updatedFields.join('，')}。`,
        { projectId }
      )
    }

    if (intent.type === 'start_plan' || intent.type === 'pause_plan' || intent.type === 'resume_plan') {
      const plan = resolvePlanFromIntent(intent, planStore.plans.filter(item => item.projectId === projectId))
      if (!plan) {
        return '没有找到对应的计划，请在对话里带上计划名称。'
      }
      await taskStore.loadTasks(plan.id)

      if (intent.type === 'start_plan') {
        const readyTasks = taskStore.getReadyTasks(plan.id)
        await planStore.startPlanExecution(plan.id)
        for (const task of readyTasks) {
          await taskExecutionStore.enqueueTask(plan.id, task.id)
        }
        await unattendedService.updateThreadContext(thread.id, { lastPlanId: plan.id })
        await syncVisiblePlanTaskState(projectId, plan.id)
        return replyWithTranscript(`计划“${plan.name}”已开始执行。当前待执行任务 ${readyTasks.length} 个。`, {
          projectId
        })
      }

      if (intent.type === 'pause_plan') {
        await taskExecutionStore.pausePlanExecutionFlow(plan.id)
        await planStore.pausePlanExecution(plan.id)
        await unattendedService.updateThreadContext(thread.id, { lastPlanId: plan.id })
        await syncVisiblePlanTaskState(projectId, plan.id)
        return replyWithTranscript(`计划“${plan.name}”已暂停。`, {
          projectId
        })
      }

      await taskExecutionStore.resumePlanExecutionFlow(plan.id)
      await planStore.resumePlanExecution(plan.id)
      await unattendedService.updateThreadContext(thread.id, { lastPlanId: plan.id })
      await syncVisiblePlanTaskState(projectId, plan.id)
      return replyWithTranscript(`计划“${plan.name}”已恢复执行。`, {
        projectId
      })
    }

    if (intent.type === 'start_split' || intent.type === 'continue_split' || intent.type === 'form_response') {
      const plan = resolvePlanFromIntent(intent, planStore.plans.filter(item => item.projectId === projectId))
        || (thread.lastPlanId ? planStore.plans.find(item => item.id === thread.lastPlanId) : undefined)

      if (!plan) {
        return '没有找到要拆分的计划，请先在对话里说明计划名称。'
      }

      const splitAgentId = thread.activeAgentId || channel.defaultAgentId || plan.splitAgentId
      if (!splitAgentId) {
        return '当前计划没有可用的拆分 Agent，请先在设置页或计划里配置。'
      }

      const splitAgent = agentStore.agents.find(item => item.id === splitAgentId)
      if (!splitAgent) {
        return '拆分 Agent 不存在，请重新配置。'
      }

      await taskSplitStore.initSession({
        planId: plan.id,
        planName: plan.name,
        planDescription: plan.description,
        granularity: plan.granularity,
        agentId: splitAgent.id,
        modelId: await resolveModelId(
          agentConfigStore,
          splitAgent,
          thread.activeModelId
            || (splitAgent.id === channel.defaultAgentId ? channel.defaultModelId : undefined)
            || plan.splitModelId
        ) || '',
        workingDirectory: project.path
      })

      if (intent.type === 'form_response') {
        const schema = taskSplitStore.activeFormSchema
        if (!schema) {
          return '当前没有等待补充的拆分表单。'
        }
        const rawValues = parseStructuredFormResponse(payload.text)
        if (Object.keys(rawValues).length === 0) {
          return replyWithTranscript(schemaToReplyTemplate(schema), { projectId })
        }
        const structuredValues = mapStructuredValuesToSchema(schema, rawValues)
        if (Object.keys(structuredValues).length === 0) {
          return replyWithTranscript(schemaToReplyTemplate(schema), { projectId })
        }
        await taskSplitStore.submitFormResponse(schema.formId, structuredValues)
        await unattendedService.updateThreadContext(thread.id, { lastPlanId: plan.id })
        return replyWithTranscript('已收到补充信息，正在继续拆分。', { projectId })
      }

      await unattendedService.updateThreadContext(thread.id, { lastPlanId: plan.id })

      if (taskSplitStore.activeFormSchema) {
        return replyWithTranscript(schemaToReplyTemplate(taskSplitStore.activeFormSchema), { projectId })
      }

      if (taskSplitStore.isProcessing) {
        return replyWithTranscript(`计划“${plan.name}”正在拆分中，请稍后再问我进度。`, {
          projectId
        })
      }

      if (taskSplitStore.splitResult?.length) {
        return replyWithTranscript(
          `计划“${plan.name}”已经拆分完成，当前生成了 ${taskSplitStore.splitResult.length} 个任务。`,
          { projectId }
        )
      }

      return replyWithTranscript(`计划“${plan.name}”已开始进入拆分流程。`, {
        projectId
      })
    }

    const agent = resolveActiveAgent(thread, channel, agentStore.agents)
    if (!agent) {
      return '当前线程没有可用 Agent，请先在设置页绑定默认 Agent。'
    }
    const modelId = await resolveModelId(
      agentConfigStore,
      agent,
      thread.activeModelId || (agent.id === channel.defaultAgentId ? channel.defaultModelId : undefined)
    )

    const sessionId = await ensureThreadSession(thread, projectId, agent.id, agent.type)
    const capabilities = getUnattendedDeliveryCapabilities()
    const systemPrompt = buildUnattendedSystemPrompt(channel, thread, agent, capabilities)
    const contextSnapshot = await buildUnattendedContextSnapshot(
      channel,
      thread,
      agent,
      project,
      projectId
    )

    await conversationService.sendMessage(
      sessionId,
      payload.text,
      agent.id,
      projectId,
      [],
      {
        modelId,
        injectedSystemMessages: [systemPrompt, contextSnapshot],
        dedupeInjectedSystemMessagesBySession: true
      }
    )

    const sessionMessages = messageStore.messagesBySession(sessionId)
    const latestAssistant = [...sessionMessages]
      .reverse()
      .find(message => message.role === 'assistant' && message.status === 'completed')

    await unattendedService.updateThreadContext(thread.id, {
      sessionId,
      activeProjectId: projectId,
      activeAgentId: agent.id,
      activeModelId: modelId,
      lastContextToken: payload.contextToken
    })

    const assistantReply = normalizeUnattendedAssistantReply(
      latestAssistant?.content || '任务已执行完成，但没有生成可回传的文本结果。'
    )

    return assistantReply || '任务已执行完成。'
  }

  async function appendThreadTranscript(
    channel: UnattendedChannel,
    thread: UnattendedThread,
    userText: string,
    assistantText: string,
    preferredProjectId?: string
  ): Promise<void> {
    const sessionStore = useSessionStore()
    const messageStore = useMessageStore()
    const projectStore = useProjectStore()
    const agentStore = useAgentStore()

    const projectId = preferredProjectId
      || thread.activeProjectId
      || channel.defaultProjectId
      || projectStore.currentProjectId
      || projectStore.projects[0]?.id

    if (!projectId) {
      return
    }

    let sessionId = thread.sessionId
    if (sessionId) {
      let existingSession = sessionStore.sessions.find(item => item.id === sessionId)
      if (!existingSession) {
        await sessionStore.loadSessions(projectId).catch(() => undefined)
        existingSession = sessionStore.sessions.find(item => item.id === sessionId)
      }
      if (existingSession?.projectId !== projectId) {
        sessionId = undefined
      }
    }

    if (!sessionId) {
      const agent = resolveActiveAgent(thread, channel, agentStore.agents)
      if (!agent) {
        return
      }
      sessionId = await ensureThreadSession(thread, projectId, agent.id, agent.type)
    }

    await messageStore.addMessage({
      sessionId,
      role: 'user',
      content: userText,
      status: 'completed'
    })
    await messageStore.addMessage({
      sessionId,
      role: 'assistant',
      content: assistantText,
      status: 'completed'
    })
    sessionStore.updateLastMessage(sessionId, compactText(assistantText, '无人值守回复'))
  }

  function resolveActiveAgent(
    thread: UnattendedThread,
    channel: UnattendedChannel,
    agents: AgentConfig[]
  ): AgentConfig | undefined {
    return agents.find(item => item.id === thread.activeAgentId)
      || agents.find(item => item.id === channel.defaultAgentId)
      || agents[0]
  }

  function resolveAgentFromIntent(intent: UnattendedIntent, agents: AgentConfig[]): AgentConfig | undefined {
    const hint = intent.agentHint || intent.rawText
    return agents.find(agent => hint.includes(agent.name) || hint.includes(agent.id))
  }

  function resolveProjectContext(
    thread: UnattendedThread,
    channel: UnattendedChannel,
    projectStore: ReturnType<typeof useProjectStore>
  ) {
    const explicitProjectId = thread.activeProjectId || channel.defaultProjectId || undefined
    if (explicitProjectId) {
      const explicitProject = projectStore.projects.find(item => item.id === explicitProjectId)
      return {
        projectId: explicitProject?.id,
        project: explicitProject
      }
    }

    const fallbackProjectId = projectStore.currentProjectId || projectStore.projects[0]?.id
    const fallbackProject = fallbackProjectId
      ? projectStore.projects.find(item => item.id === fallbackProjectId)
      : undefined
    return {
      projectId: fallbackProject?.id,
      project: fallbackProject
    }
  }

  function resolveProjectFromIntent(
    intent: UnattendedIntent,
    projects: Project[]
  ) {
    const hint = (intent.projectHint || intent.rawText).toLowerCase()
    return projects.find(project => hint.includes(project.name.toLowerCase()))
      || projects.find(project => hint.includes(project.id.toLowerCase()))
      || projects.find(project => hint.includes(project.path.toLowerCase()))
  }

  function resolvePlanFromIntent(intent: UnattendedIntent, plans: Plan[]) {
    const target = intent.targetName || intent.rawText
    return plans.find(plan => target.includes(plan.name))
      || plans.find(plan => target.includes(plan.id))
      || plans.find(plan => plan.status === 'executing' || plan.executionStatus === 'running')
      || plans[0]
  }

  function sortPlansForThread(thread: UnattendedThread, plans: Plan[]): Plan[] {
    return [...plans].sort((left, right) => {
      const leftScore = (
        (left.id === thread.lastPlanId ? 100 : 0)
        + ((left.status === 'executing' || left.executionStatus === 'running') ? 50 : 0)
      )
      const rightScore = (
        (right.id === thread.lastPlanId ? 100 : 0)
        + ((right.status === 'executing' || right.executionStatus === 'running') ? 50 : 0)
      )

      if (leftScore !== rightScore) {
        return rightScore - leftScore
      }

      return new Date(right.updatedAt).getTime() - new Date(left.updatedAt).getTime()
    })
  }

  function resolvePlanByText(hint: string | undefined, plans: Plan[]): Plan | undefined {
    const normalizedHint = hint?.trim().toLowerCase()
    if (!normalizedHint) {
      return undefined
    }

    return plans.find(plan => normalizedHint.includes(plan.name.toLowerCase()))
      || plans.find(plan => normalizedHint.includes(plan.id.toLowerCase()))
  }

  function resolveTaskFromText(hint: string | undefined, tasks: Task[], thread: UnattendedThread): Task | undefined {
    const normalizedHint = hint?.trim().toLowerCase()
    if (!normalizedHint) {
      return tasks.find(task => task.id === thread.lastTaskId)
        || tasks.find(task => task.status === 'in_progress')
    }

    return tasks.find(task => normalizedHint.includes(task.title.toLowerCase()))
      || tasks.find(task => normalizedHint.includes(task.id.toLowerCase()))
      || tasks.find(task => task.id === thread.lastTaskId)
      || tasks.find(task => task.status === 'in_progress')
  }

  async function loadTasksForPlanCandidates(
    planCandidates: Plan[],
    thread: UnattendedThread
  ): Promise<{ plan: Plan, tasks: Task[] }[]> {
    const taskStore = useTaskStore()
    const orderedPlans = sortPlansForThread(thread, planCandidates)
    const loaded: { plan: Plan, tasks: Task[] }[] = []

    for (const plan of orderedPlans.slice(0, 6)) {
      await taskStore.loadTasks(plan.id)
      loaded.push({
        plan,
        tasks: taskStore.tasks.filter(task => task.planId === plan.id)
      })
    }

    return loaded
  }

  async function resolveModelId(
    agentConfigStore: ReturnType<typeof useAgentConfigStore>,
    agent: AgentConfig,
    preferredModelId?: string
  ): Promise<string | undefined> {
    const models = await agentConfigStore.ensureModelsConfigs(agent.id, agent.provider)
    const enabledModels = models.filter(model => model.enabled)

    if (preferredModelId && enabledModels.some(model => model.modelId === preferredModelId)) {
      return preferredModelId
    }

    const defaultModel = enabledModels.find(model => model.isDefault)
    if (defaultModel) {
      return defaultModel.modelId
    }

    return enabledModels[0]?.modelId || agent.modelId
  }

  async function resolveModelFromIntent(
    agentConfigStore: ReturnType<typeof useAgentConfigStore>,
    agent: AgentConfig,
    hintText: string
  ): Promise<string | undefined> {
    const models = await agentConfigStore.ensureModelsConfigs(agent.id, inferAgentProvider(agent))
    const enabledModels = models.filter(model => model.enabled)
    const hint = hintText.toLowerCase()

    return enabledModels.find(model =>
      hint.includes(model.modelId.toLowerCase())
      || hint.includes(model.displayName.toLowerCase())
    )?.modelId
  }

  async function ensureThreadSession(
    thread: UnattendedThread,
    projectId: string,
    agentId: string,
    agentType: string
  ): Promise<string> {
    const sessionStore = useSessionStore()

    if (thread.sessionId) {
      let existingSession = sessionStore.sessions.find(item => item.id === thread.sessionId)
      if (!existingSession) {
        await sessionStore.loadSessions(projectId).catch(() => undefined)
        existingSession = sessionStore.sessions.find(item => item.id === thread.sessionId)
      }
      if (existingSession?.projectId === projectId) {
        return thread.sessionId
      }
    }

    const agentStore = useAgentStore()
    const agent = agentStore.agents.find(item => item.id === agentId)
    const session = await sessionStore.createSession({
      projectId,
      name: `无人值守 ${thread.peerNameSnapshot || thread.peerId}`,
      agentId,
      agentType: agentType || agent?.type || 'cli',
      status: 'idle'
    })
    await unattendedService.updateThreadContext(thread.id, {
      sessionId: session.id,
      activeProjectId: projectId,
      activeAgentId: agentId
    })
    return session.id
  }

  async function buildPlanSummary(projectId: string): Promise<string> {
    const planStore = usePlanStore()
    const plans = planStore.plans.filter(item => item.projectId === projectId)
    if (plans.length === 0) {
      return '当前项目下还没有计划。'
    }
    return [
      `当前项目共有 ${plans.length} 个计划：`,
      ...plans.slice(0, 8).map(plan =>
        `- ${plan.name}：${plan.status}${plan.executionStatus ? ` / ${plan.executionStatus}` : ''}`
      )
    ].join('\n')
  }

  function buildProjectSummary(
    channel: UnattendedChannel,
    thread: UnattendedThread,
    projectStore: ReturnType<typeof useProjectStore>
  ): string {
    if (projectStore.projects.length === 0) {
      return '当前工作区还没有导入项目。'
    }

    const activeProjectId = thread.activeProjectId || channel.defaultProjectId
    return [
      `当前工作区共导入 ${projectStore.projects.length} 个项目：`,
      ...projectStore.projects.slice(0, 8).map(project =>
        `- ${project.name}${project.id === activeProjectId ? '（当前线程项目）' : ''}：${project.path}`
      )
    ].join('\n')
  }

  async function buildExecutionSummary(projectId: string): Promise<string> {
    const planStore = usePlanStore()
    const taskExecutionStore = useTaskExecutionStore()
    const executingPlans = planStore.plans.filter(plan =>
      plan.projectId === projectId && (plan.status === 'executing' || plan.executionStatus === 'running')
    )
    if (executingPlans.length === 0) {
      return '当前没有正在执行的计划。'
    }
    const summaries = await Promise.all(executingPlans.slice(0, 4).map(async plan => {
      const progress = await taskExecutionStore.getPlanExecutionProgress(plan.id)
      if (!progress) {
        return `- ${plan.name}：执行中，暂时还没有进度快照`
      }
      const activeTasks = progress.tasks
        .filter(task => task.status === 'in_progress' || task.status === 'blocked' || task.status === 'failed')
        .slice(0, 3)
        .map(task => `${task.title}(${task.status})`)
      const activeTaskSummary = activeTasks.length > 0
        ? `；重点任务：${activeTasks.join('、')}`
        : ''
      return `- ${plan.name}：总任务 ${progress.total_tasks}，待办 ${progress.pending_count}，进行中 ${progress.in_progress_count}，完成 ${progress.completed_count}，失败 ${progress.failed_count}`
        + activeTaskSummary
    }))
    return ['当前执行进度：', ...summaries].join('\n')
  }

  async function buildTaskSummary(projectId: string, intent?: UnattendedIntent): Promise<string> {
    const planStore = usePlanStore()
    const taskStore = useTaskStore()
    const projectPlans = planStore.plans.filter(plan => plan.projectId === projectId)
    const targetPlan = (intent ? resolvePlanFromIntent(intent, projectPlans) : undefined)
      || projectPlans.find(plan => plan.status === 'executing' || plan.executionStatus === 'running')
      || projectPlans[0]

    if (!targetPlan) {
      return '当前项目下还没有计划任务。'
    }

    await taskStore.loadTasks(targetPlan.id)
    const tasks = taskStore.tasks.filter(task => task.planId === targetPlan.id)
    if (tasks.length === 0) {
      return `计划“${targetPlan.name}”还没有任务。`
    }

    return [
      `计划“${targetPlan.name}”任务概览：`,
      ...tasks.slice(0, 8).map(task => `- ${task.title}：${task.status}${task.errorMessage ? ` / ${compactText(task.errorMessage, '')}` : ''}`)
    ].join('\n')
  }

  async function buildUnattendedContextSnapshot(
    channel: UnattendedChannel,
    thread: UnattendedThread,
    agent: AgentConfig | undefined,
    project: Project,
    projectId: string
  ): Promise<string> {
    const planStore = usePlanStore()
    const taskStore = useTaskStore()
    const projectPlans = sortPlansForThread(
      thread,
      planStore.plans.filter(item => item.projectId === projectId)
    )
    const highlightedTasks: Task[] = []

    for (const plan of projectPlans.slice(0, 3)) {
      await taskStore.loadTasks(plan.id)
      highlightedTasks.push(
        ...taskStore.tasks
          .filter(task => task.planId === plan.id)
          .filter(task => task.status === 'in_progress' || task.status === 'blocked' || task.status === 'failed')
          .slice(0, 3)
      )
    }

    return buildUnattendedWorkspaceContext({
      channel,
      thread,
      agent,
      project,
      projects: useProjectStore().projects,
      plans: projectPlans,
      activeTasks: highlightedTasks,
      capabilities: getUnattendedDeliveryCapabilities()
    })
  }

  return {
    channels,
    accounts,
    threads,
    events,
    runtimeStatuses,
    loginSessions,
    isLoading,
    initialize,
    dispose,
    loadAll,
    createWeixinChannel,
    updateChannel,
    deleteChannel,
    startWeixinLogin,
    pollWeixinLogin,
    startRuntime,
    stopRuntime,
    logoutAccount
  }
})
