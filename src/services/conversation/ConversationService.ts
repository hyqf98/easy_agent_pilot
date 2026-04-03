import { useMessageStore, type Message, type MessageAttachment, type ToolCall } from '@/stores/message'
import { useSessionStore, type Session } from '@/stores/session'
import { useSessionExecutionStore, type ComposerMemoryReference } from '@/stores/sessionExecution'
import { useProjectStore } from '@/stores/project'
import { useAgentStore, type AgentConfig, inferAgentProvider } from '@/stores/agent'
import { useNotificationStore } from '@/stores/notification'
import { useTokenStore } from '@/stores/token'
import { useMemoryStore } from '@/stores/memory'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import type { MemoryLibrary } from '@/types/memory'
import { agentExecutor } from './AgentExecutor'
import type { ConversationContext, StreamEvent } from './strategies/types'
import { compressionService } from '@/services/compression/CompressionService'
import { buildConversationMessages } from './buildConversationMessages'
import { buildProjectMemorySystemPrompt } from '@/services/memory'
import type { FileEditTrace } from '@/types/fileTrace'
import { FileTraceCollector } from './fileTraceCollector'
import { buildMainConversationFormRequestPrompt } from './prompts'
import { resolveUsageModelHint } from './usageModelHint'
import {
  buildCliEnvironmentNotice,
  buildContextStrategyNotice,
  buildProcessingTimeNotice,
  buildRuntimeNoticeFromSystemContent,
  buildUsageNotice,
  upsertRuntimeNotice
} from '@/utils/runtimeNotice'
import { loadAgentMcpServers } from '@/utils/mcpServerConfig'
import { mergeToolInputArguments } from '@/utils/toolInput'
import { getErrorMessage } from '@/utils/api'
import { recordAgentCliUsageInBackground, resolveRecordedModelId } from '@/services/usage/agentCliUsageRecorder'
import { buildExpertSystemPrompt, resolveExpertById } from '@/services/agentTeams/runtime'
import { writeFrontendRuntimeLog } from '@/services/runtimeLog/client'
import {
  deleteSessionRuntimeBinding,
  getSessionRuntimeBinding,
  isInvalidCliResumeError,
  resolveRuntimeBindingKey,
  upsertSessionRuntimeBinding
} from './runtimeBindings'

interface StreamTimingMetrics {
  startedAt: number
  firstEventAt?: number
  firstRenderAt?: number
  firstContentAt?: number
  firstThinkingAt?: number
  firstToolAt?: number
  doneAt?: number
  persistedAt?: number
}

const REFERENCED_MEMORY_BLOCK_HEADER = '[用户主动引用的历史记忆]'
const CURRENT_INPUT_BLOCK_HEADER = '[用户当前输入]'

function finalizePendingToolCalls(toolCalls: ToolCall[]): ToolCall[] {
  let changed = false
  const finalized = toolCalls.map(toolCall => {
    if (toolCall.status !== 'running') {
      return toolCall
    }

    changed = true
    return {
      ...toolCall,
      status: 'success' as const
    }
  })

  return changed ? finalized : toolCalls
}

function extractRawMemoryCaptureContent(content: string): string {
  const trimmed = content.trim()
  if (!trimmed.startsWith(REFERENCED_MEMORY_BLOCK_HEADER)) {
    return trimmed
  }

  const currentInputIndex = trimmed.indexOf(CURRENT_INPUT_BLOCK_HEADER)
  if (currentInputIndex === -1) {
    return trimmed
  }

  return trimmed.slice(currentInputIndex + CURRENT_INPUT_BLOCK_HEADER.length).trim()
}

/**
 * 对话服务
 * 封装消息发送逻辑，处理流式事件更新
 */
export class ConversationService {
  private static instance: ConversationService | null = null
  private readonly queueDrainLocks = new Set<string>()
  private readonly dedupedInjectedSystemPrompts = new Map<string, Set<string>>()

  private constructor() {}

  /**
   * 记录已在指定会话成功注入过的系统提示词。
   * 仅用于需要“同一会话只注入一次”的场景，避免重复堆积上下文。
   */
  private markInjectedSystemMessages(sessionId: string, messages: string[]): void {
    if (!sessionId || messages.length === 0) {
      return
    }

    const existing = this.dedupedInjectedSystemPrompts.get(sessionId) ?? new Set<string>()
    messages.forEach(message => existing.add(message))
    this.dedupedInjectedSystemPrompts.set(sessionId, existing)
  }

  /**
   * 过滤已在当前会话注入过的系统提示词，保证同一提示词只追加一次。
   */
  private filterSessionScopedInjectedMessages(sessionId: string, messages: string[]): string[] {
    if (!sessionId || messages.length === 0) {
      return messages
    }

    const existing = this.dedupedInjectedSystemPrompts.get(sessionId)
    if (!existing?.size) {
      return messages
    }

    return messages.filter(message => !existing.has(message))
  }

  /**
   * 获取单例实例
   */
  static getInstance(): ConversationService {
    if (!ConversationService.instance) {
      ConversationService.instance = new ConversationService()
    }
    return ConversationService.instance
  }

  /**
   * 发送消息并执行对话
   * @param projectId 可选的项目 ID,用于指定工作目录
   */
  async sendMessage(
    sessionId: string,
    content: string,
    agentId: string,
    projectId?: string,
    attachments: MessageAttachment[] = [],
    options?: {
      workingDirectory?: string
      modelId?: string
      injectedSystemMessages?: string[]
      dedupeInjectedSystemMessagesBySession?: boolean
      previewContent?: string
      memoryReferencesToPersist?: ComposerMemoryReference[]
      existingUserMessageId?: string
    }
  ): Promise<void> {
    const messageStore = useMessageStore()
    const sessionStore = useSessionStore()
    const sessionExecutionStore = useSessionExecutionStore()
    const tokenStore = useTokenStore()
    const projectStore = useProjectStore()
    const agentStore = useAgentStore()
    const memoryStore = useMemoryStore()

    // 获取智能体配置
    const storedAgent = agentStore.agents.find(a => a.id === agentId)
    if (!storedAgent) {
      throw new Error('智能体不存在')
    }
    const modelIdOverride = options?.modelId?.trim() || undefined
    const agent = modelIdOverride
      ? { ...storedAgent, modelId: modelIdOverride }
      : storedAgent

    // 检查策略支持
    if (!agentExecutor.isSupported(agent)) {
      throw new Error(`不支持的智能体类型: ${agent.type}`)
    }

    // 开始发送状态
    sessionExecutionStore.startSending(sessionId)

    try {
      const existingUserMessageId = options?.existingUserMessageId?.trim()
      const existingSessionMessages = messageStore.messagesBySession(sessionId)
      const userMessage = existingUserMessageId
        ? existingSessionMessages.find(message => message.id === existingUserMessageId && message.role === 'user')
        : undefined
      const targetUserMessage = userMessage ?? await messageStore.addMessage({
        sessionId,
        role: 'user',
        content,
        attachments,
        status: 'completed'
      })

      if (!userMessage) {
        const rawMemoryContent = extractRawMemoryCaptureContent(content)
        if (rawMemoryContent) {
          await memoryStore.captureUserMessage({
            sessionId,
            messageId: targetUserMessage.id,
            content: rawMemoryContent
          })
        }
        await memoryStore.recordSessionMemoryReferences({
          sessionId,
          messageId: targetUserMessage.id,
          references: (options?.memoryReferencesToPersist ?? []).map(reference => ({
            sourceType: reference.sourceType,
            sourceId: reference.sourceId
          }))
        })

        const messagePreview = this.buildMessagePreview(options?.previewContent ?? content, attachments)
        sessionStore.updateLastMessage(sessionId, messagePreview)

        const session = sessionStore.sessions.find(s => s.id === sessionId)
        if (session && (session.name === '未命名会话' || session.name.startsWith('新会话'))) {
          const titleSource = this.buildMessagePreview(options?.previewContent ?? content, attachments)
          const newTitle = titleSource.replace(/\n/g, ' ').slice(0, 20).trim()
          const finalTitle = newTitle.length < titleSource.length ? newTitle + '...' : newTitle
          if (finalTitle) {
            await sessionStore.updateSession(sessionId, { name: finalTitle })
          }
        }
      }

      // 创建流式 AI 响应消息
      const aiMessage = await messageStore.addMessage({
        sessionId,
        role: 'assistant',
        content: '',
        status: 'streaming'
      })

      const usageModelHint = await resolveUsageModelHint(agent)
      const executionAgent = (!agent.modelId?.trim() && usageModelHint)
        ? { ...agent, modelId: usageModelHint }
        : agent

      if (usageModelHint) {
        tokenStore.updateRealtimeTokens(sessionId, undefined, undefined, usageModelHint)
      }

      const environmentNotice = await buildCliEnvironmentNotice(executionAgent)
      if (environmentNotice) {
        messageStore.updateMessageBuffered(aiMessage.id, {
          runtimeNotices: [environmentNotice]
        })
      }

      const initialUsageNotice = buildUsageNotice({ model: usageModelHint })
      if (initialUsageNotice) {
        const currentMessage = messageStore.messagesBySession(sessionId)
          .find(message => message.id === aiMessage.id)
        messageStore.updateMessageBuffered(aiMessage.id, {
          runtimeNotices: upsertRuntimeNotice(currentMessage?.runtimeNotices, initialUsageNotice)
        })
      }

      // 保存当前流式消息 ID
      sessionExecutionStore.setCurrentStreamingMessageId(sessionId, aiMessage.id)

      // 获取工作目录：优先使用传入的项目 ID，否则使用会话关联的项目
      let workingDirectory: string | undefined
      let targetProject = projectId
        ? projectStore.projects.find(p => p.id === projectId)
        : undefined

      if (options?.workingDirectory) {
        workingDirectory = options.workingDirectory
      } else if (projectId) {
        workingDirectory = targetProject?.path
      } else {
        const session = sessionStore.sessions.find(s => s.id === sessionId)
        if (session?.projectId) {
          targetProject = projectStore.projects.find(p => p.id === session.projectId)
          workingDirectory = targetProject?.path
        }
      }

      const [projectMemoryPrompt, mcpServers] = await Promise.all([
        this.buildProjectMemoryPrompt(targetProject?.memoryLibraryIds ?? []),
        loadAgentMcpServers(executionAgent).catch((error) => {
          console.warn('[ConversationService] Failed to load MCP servers:', error)
          return []
        })
      ])

      const rawInjectedSystemMessages = (options?.injectedSystemMessages ?? [])
        .map(message => message.trim())
        .filter(message => message.length > 0)

      const sessionScopedInjectedSystemMessages = options?.dedupeInjectedSystemMessagesBySession
        ? this.filterSessionScopedInjectedMessages(sessionId, rawInjectedSystemMessages)
        : rawInjectedSystemMessages

      const injectedSystemMessages = [
        ...sessionScopedInjectedSystemMessages,
        ...(projectMemoryPrompt ? [projectMemoryPrompt] : []),
        buildMainConversationFormRequestPrompt()
      ]

      const session = sessionStore.sessions.find(s => s.id === sessionId)
      const sessionMessages = messageStore.messagesBySession(sessionId)
      const executionMessages = existingUserMessageId
        ? this.sliceMessagesForRetry(sessionMessages, targetUserMessage.id)
        : sessionMessages
      const hasCompressionMessage = sessionMessages.some(message => message.role === 'compression')
      const reusableCliSessionId = await this.resolveReusableCliSessionId(
        session,
        executionAgent,
        hasCompressionMessage
      )
      const fullMessages = this.buildExecutionMessages(
        executionMessages,
        targetUserMessage,
        sessionId,
        injectedSystemMessages,
        content
      )

      // 构建对话上下文
      const messages = this.buildExecutionMessages(
        executionMessages,
        targetUserMessage,
        sessionId,
        injectedSystemMessages,
        content,
        reusableCliSessionId
      )
      const userMessages = messages.filter(message => message.role === 'user')
      const systemMessages = messages.filter(message => message.role === 'system')
      const assistantMessages = messages.filter(message => message.role === 'assistant')
      const selectedExpert = resolveExpertById(session?.expertId, useAgentTeamsStore().experts)
      const contextStrategyNotice = buildContextStrategyNotice({
        strategy: reusableCliSessionId ? 'CLI Resume + Delta Prompt' : 'Full Conversation Context',
        runtime: inferAgentProvider(executionAgent)?.toUpperCase() || executionAgent.type,
        model: executionAgent.modelId || usageModelHint || undefined,
        expert: selectedExpert?.name || session?.expertId || undefined,
        systemMessageCount: systemMessages.length,
        userMessageCount: userMessages.length,
        assistantMessageCount: assistantMessages.length,
        historyMessageCount: Math.max(0, fullMessages.length - 1),
        resumeSessionId: reusableCliSessionId ?? undefined
      })

      console.info('[ConversationService] assembled context messages', {
        sessionId,
        messageCount: messages.length,
        systemMessageCount: systemMessages.length,
        assistantMessageCount: assistantMessages.length,
        historyMessageCount: Math.max(0, fullMessages.length - 1),
        resumeSessionId: reusableCliSessionId ?? null,
        lastUserMessageLength: userMessages.length > 0
          ? userMessages[userMessages.length - 1].content.length
          : 0
      })

      if (contextStrategyNotice) {
        const currentMessage = messageStore.messagesBySession(sessionId)
          .find(message => message.id === aiMessage.id)
        messageStore.updateMessageBuffered(aiMessage.id, {
          runtimeNotices: upsertRuntimeNotice(currentMessage?.runtimeNotices, contextStrategyNotice)
        })
      }

      const context: ConversationContext = {
        sessionId,
        agent: executionAgent,
        messages,
        workingDirectory,
        mcpServers: mcpServers.length > 0 ? mcpServers : undefined,
        executionMode: 'chat',
        responseMode: 'stream_text',
        resumeSessionId: reusableCliSessionId
      }
      const fallbackContext = reusableCliSessionId
        ? {
            ...context,
            messages: fullMessages,
            resumeSessionId: undefined
          }
        : undefined

      await this.syncSessionExecutionBinding(sessionId, executionAgent)

      // 执行对话
      await this.executeConversation(context, aiMessage, sessionId, targetProject?.id, fallbackContext)
      if (options?.dedupeInjectedSystemMessagesBySession) {
        this.markInjectedSystemMessages(sessionId, sessionScopedInjectedSystemMessages)
      }

    } catch (error) {
      this.finalizeSend(sessionId)
      throw error
    }
  }

  private finalizeSend(sessionId: string) {
    const sessionExecutionStore = useSessionExecutionStore()
    sessionExecutionStore.endSending(sessionId)
    void this.processQueuedMessages(sessionId)
  }

  async drainQueue(sessionId: string): Promise<void> {
    await this.processQueuedMessages(sessionId)
  }

  private async processQueuedMessages(sessionId: string): Promise<void> {
    if (this.queueDrainLocks.has(sessionId)) {
      return
    }

    const sessionExecutionStore = useSessionExecutionStore()
    if (sessionExecutionStore.getIsSending(sessionId)) {
      return
    }

    const nextDraft = sessionExecutionStore.popNextQueuedMessage(sessionId)
    if (!nextDraft) {
      return
    }

    this.queueDrainLocks.add(sessionId)

    try {
      const sessionStore = useSessionStore()
      const agentTeamsStore = useAgentTeamsStore()
      const projectId = sessionStore.sessions.find(session => session.id === sessionId)?.projectId
      const expert = resolveExpertById(nextDraft.expertId, agentTeamsStore.experts)
      await this.sendMessage(
        sessionId,
        nextDraft.content,
        nextDraft.agentId,
        projectId,
        nextDraft.attachments,
        {
          modelId: nextDraft.modelId,
          injectedSystemMessages: expert
            ? [buildExpertSystemPrompt(expert.prompt)]
            : undefined,
          previewContent: nextDraft.displayContent,
          memoryReferencesToPersist: nextDraft.memoryReferences ?? []
        }
      )
    } catch (error) {
      const notificationStore = useNotificationStore()
      const errorMessage = getErrorMessage(error, '发送待发送消息失败')
      void writeFrontendRuntimeLog(
        'ERROR',
        'conversation-queue',
        `drainQueue failed | sessionId=${sessionId} | draftId=${nextDraft.id} | agentId=${nextDraft.agentId} | expertId=${nextDraft.expertId} | error=${errorMessage}`,
        error
      )
      sessionExecutionStore.restoreQueuedMessage(sessionId, {
        ...nextDraft,
        status: 'failed',
        errorMessage
      })
      notificationStore.smartError('发送待发送消息', error instanceof Error ? error : new Error(errorMessage))
    } finally {
      this.queueDrainLocks.delete(sessionId)
    }
  }

  private buildMessagePreview(content: string, attachments: MessageAttachment[]): string {
    const trimmed = content.trim()
    if (trimmed) {
      return trimmed.slice(0, 50)
    }

    if (attachments.length === 1) {
      return `[图片] ${attachments[0].name}`
    }

    if (attachments.length > 1) {
      return `[${attachments.length} 张图片]`
    }

    return ''
  }

  private async buildProjectMemoryPrompt(memoryLibraryIds: string[]): Promise<string | null> {
    if (memoryLibraryIds.length === 0) {
      return null
    }

    const memoryStore = useMemoryStore()
    const missingLibraryIds = memoryLibraryIds.filter(
      (libraryId) => !memoryStore.libraries.some((library) => library.id === libraryId)
    )

    if (missingLibraryIds.length > 0) {
      await memoryStore.loadLibraries()
    }

    const mountedLibraries = memoryLibraryIds
      .map((libraryId) => memoryStore.libraries.find((library) => library.id === libraryId))
      .filter((library): library is MemoryLibrary => Boolean(library))

    return buildProjectMemorySystemPrompt(mountedLibraries)
  }

  private resolveCliSessionProvider(agent: AgentConfig): string | undefined {
    const provider = inferAgentProvider(agent)
    return provider?.trim() || undefined
  }

  private async resolveReusableCliSessionId(
    session: Session | undefined,
    agent: AgentConfig,
    hasCompressionMessage: boolean
  ): Promise<string | undefined> {
    if (!session || agent.type !== 'cli' || hasCompressionMessage) {
      return undefined
    }

    const runtimeKey = resolveRuntimeBindingKey(agent)
    if (runtimeKey) {
      try {
        const binding = await getSessionRuntimeBinding(session.id, runtimeKey)
        const externalSessionId = binding?.externalSessionId?.trim()
        if (externalSessionId) {
          return externalSessionId
        }
      } catch (error) {
        console.warn('[ConversationService] Failed to read session runtime binding:', error)
      }
    }

    const cliSessionId = session.cliSessionId?.trim()
    if (!cliSessionId) {
      return undefined
    }

    const expectedProvider = this.resolveCliSessionProvider(agent)
    const boundProvider = session.cliSessionProvider?.trim()
    if (expectedProvider && boundProvider && expectedProvider !== boundProvider) {
      return undefined
    }

    return cliSessionId
  }

  private buildExecutionMessages(
    sessionMessages: Message[],
    currentUserMessage: Message,
    sessionId: string,
    injectedSystemMessages: string[],
    fallbackUserContent: string,
    resumeSessionId?: string
  ): Message[] {
    const sourceMessages = resumeSessionId
      ? [currentUserMessage]
      : sessionMessages

    return buildConversationMessages(sourceMessages, {
      fallbackUserContent,
      sessionId,
      injectedSystemMessages
    })
  }

  private sliceMessagesForRetry(sessionMessages: Message[], userMessageId: string): Message[] {
    const userMessageIndex = sessionMessages.findIndex(message => message.id === userMessageId)
    if (userMessageIndex < 0) {
      return sessionMessages
    }

    return sessionMessages.slice(0, userMessageIndex + 1)
  }

  private async syncSessionExecutionBinding(
    sessionId: string,
    agent: AgentConfig,
    overrides: Partial<Pick<Session, 'cliSessionId' | 'cliSessionProvider'>> = {}
  ): Promise<void> {
    const sessionStore = useSessionStore()
    const currentSession = sessionStore.sessions.find(session => session.id === sessionId)
    if (!currentSession) {
      return
    }

    const provider = this.resolveCliSessionProvider(agent)
    const nextAgentType = provider || agent.type
    const shouldRetainCliBinding = Boolean(provider)
    const currentCliSessionProvider = currentSession.cliSessionProvider?.trim()
    const shouldReuseCurrentCliSessionId = shouldRetainCliBinding
      && provider
      && currentCliSessionProvider === provider
    const nextCliSessionId = shouldRetainCliBinding
      ? (overrides.cliSessionId ?? (shouldReuseCurrentCliSessionId ? currentSession.cliSessionId : ''))
      : ''
    const nextCliSessionProvider = shouldRetainCliBinding
      ? (overrides.cliSessionProvider ?? provider ?? currentSession.cliSessionProvider)
      : ''

    if (
      currentSession.agentId === agent.id
      && currentSession.agentType === nextAgentType
      && (currentSession.cliSessionId ?? '') === (nextCliSessionId ?? '')
      && (currentSession.cliSessionProvider ?? '') === (nextCliSessionProvider ?? '')
    ) {
      return
    }

    await sessionStore.updateSession(sessionId, {
      agentId: agent.id,
      agentType: nextAgentType,
      cliSessionId: nextCliSessionId,
      cliSessionProvider: nextCliSessionProvider
    })
  }

  /**
   * 执行对话
   */
  private async executeConversation(
    context: ConversationContext,
    aiMessage: Message,
    sessionId: string,
    projectId?: string,
    fallbackContext?: ConversationContext
  ): Promise<void> {
    const messageStore = useMessageStore()
    const sessionStore = useSessionStore()
    const sessionExecutionStore = useSessionExecutionStore()
    const tokenStore = useTokenStore()
    const resolvedProjectId = projectId
      ?? sessionStore.sessions.find(session => session.id === sessionId)?.projectId
      ?? null

    let accumulatedContent = ''
    let accumulatedThinking = ''
    const toolCalls: ToolCall[] = []
    const editTraces: FileEditTrace[] = []
    const usageState: { model?: string, inputTokens?: number, outputTokens?: number } = {
      model: context.agent.modelId?.trim() || undefined
    }
    let runtimeNoticesState = messageStore.messagesBySession(sessionId)
      .find(message => message.id === aiMessage.id)
      ?.runtimeNotices
    const fileTraceCollector = new FileTraceCollector({
      sessionId,
      messageId: aiMessage.id,
      projectPath: context.workingDirectory
    })
    const pendingTraceTasks = new Set<Promise<void>>()
    const pendingPersistenceTasks = new Set<Promise<void>>()
    const cliSessionProvider = this.resolveCliSessionProvider(context.agent)
    const runtimeKey = resolveRuntimeBindingKey(context.agent)
    const streamMetrics: StreamTimingMetrics = {
      startedAt: globalThis.performance?.now() ?? Date.now()
    }
    let hasError = false
    let usageRecorded = false
    let pendingUiUpdate: Partial<Message> | null = null
    let scheduledUiFlushHandle: number | ReturnType<typeof setTimeout> | null = null

    const registerTraceTask = (task: Promise<void>) => {
      pendingTraceTasks.add(task)
      task.finally(() => pendingTraceTasks.delete(task))
    }

    const registerPersistenceTask = (task: Promise<void>) => {
      pendingPersistenceTasks.add(task)
      task.finally(() => pendingPersistenceTasks.delete(task))
    }

    const registerCliSessionBinding = (externalSessionId?: string) => {
      const normalizedExternalSessionId = externalSessionId?.trim()
      if (!normalizedExternalSessionId || !cliSessionProvider) {
        return
      }

      registerPersistenceTask(
        (async () => {
          if (runtimeKey) {
            await upsertSessionRuntimeBinding(
              sessionId,
              runtimeKey,
              normalizedExternalSessionId
            )
          }

          await this.syncSessionExecutionBinding(sessionId, context.agent, {
            cliSessionId: normalizedExternalSessionId,
            cliSessionProvider
          })
        })()
      )
    }

    const now = () => globalThis.performance?.now() ?? Date.now()

    const markMetric = (key: keyof Omit<StreamTimingMetrics, 'startedAt'>) => {
      if (!streamMetrics[key]) {
        streamMetrics[key] = now()
      }
    }

    const clearScheduledUiFlush = () => {
      if (scheduledUiFlushHandle === null) {
        return
      }

      if (typeof scheduledUiFlushHandle === 'number' && typeof globalThis.cancelAnimationFrame === 'function') {
        globalThis.cancelAnimationFrame(scheduledUiFlushHandle)
      } else {
        clearTimeout(scheduledUiFlushHandle as ReturnType<typeof setTimeout>)
      }

      scheduledUiFlushHandle = null
    }

    const getCurrentAiMessage = () => {
      return messageStore.messagesBySession(sessionId)
        .find(message => message.id === aiMessage.id)
    }

    const isAiMessageInterrupted = () => getCurrentAiMessage()?.status === 'interrupted'

    const normalizeBufferedMessageUpdate = (
      updates: Partial<Message>
    ): Partial<Message> | null => {
      if (!isAiMessageInterrupted()) {
        return updates
      }

      const nextUpdates: Partial<Message> = { ...updates }

      if (nextUpdates.status === 'completed' || nextUpdates.status === 'error') {
        delete nextUpdates.status
      }

      delete nextUpdates.errorMessage

      if (nextUpdates.thinkingActive !== undefined) {
        nextUpdates.thinkingActive = false
      }

      return Object.keys(nextUpdates).length > 0 ? nextUpdates : null
    }

    const flushPendingUiUpdate = (options?: { immediate?: boolean }) => {
      clearScheduledUiFlush()
      if (!pendingUiUpdate) {
        return
      }

      markMetric('firstRenderAt')
      const updates = normalizeBufferedMessageUpdate(pendingUiUpdate)
      pendingUiUpdate = null
      if (!updates) {
        return
      }
      messageStore.updateMessageBuffered(aiMessage.id, updates, options)
    }

    const scheduleUiFlush = () => {
      if (scheduledUiFlushHandle !== null) {
        return
      }

      if (typeof globalThis.requestAnimationFrame === 'function') {
        scheduledUiFlushHandle = globalThis.requestAnimationFrame(() => {
          scheduledUiFlushHandle = null
          flushPendingUiUpdate()
        })
        return
      }

      scheduledUiFlushHandle = setTimeout(() => {
        scheduledUiFlushHandle = null
        flushPendingUiUpdate()
      }, 16)
    }

    const bufferMessageUpdate = (
      updates: Partial<Message>,
      options?: { immediate?: boolean }
    ) => {
      const normalizedUpdates = normalizeBufferedMessageUpdate(updates)
      if (!normalizedUpdates) {
        return
      }

      pendingUiUpdate = pendingUiUpdate
        ? { ...pendingUiUpdate, ...normalizedUpdates }
        : { ...normalizedUpdates }

      if (options?.immediate) {
        flushPendingUiUpdate(options)
        return
      }

      scheduleUiFlush()
    }

    const recordTimingSummary = () => {
      const summary = {
        sessionId,
        messageId: aiMessage.id,
        firstEventMs: streamMetrics.firstEventAt
          ? Number((streamMetrics.firstEventAt - streamMetrics.startedAt).toFixed(1))
          : null,
        firstRenderMs: streamMetrics.firstRenderAt
          ? Number((streamMetrics.firstRenderAt - streamMetrics.startedAt).toFixed(1))
          : null,
        firstContentMs: streamMetrics.firstContentAt
          ? Number((streamMetrics.firstContentAt - streamMetrics.startedAt).toFixed(1))
          : null,
        firstThinkingMs: streamMetrics.firstThinkingAt
          ? Number((streamMetrics.firstThinkingAt - streamMetrics.startedAt).toFixed(1))
          : null,
        firstToolMs: streamMetrics.firstToolAt
          ? Number((streamMetrics.firstToolAt - streamMetrics.startedAt).toFixed(1))
          : null,
        doneMs: streamMetrics.doneAt
          ? Number((streamMetrics.doneAt - streamMetrics.startedAt).toFixed(1))
          : null,
        persistedMs: streamMetrics.persistedAt
          ? Number((streamMetrics.persistedAt - streamMetrics.startedAt).toFixed(1))
          : null
      }

      console.info('[ConversationService] stream timing metrics', summary)
      ;(globalThis as { __EASY_AGENT_LAST_STREAM_METRICS?: typeof summary }).__EASY_AGENT_LAST_STREAM_METRICS = summary
    }

    const recordUsageOnce = (occurredAt?: string) => {
      if (usageRecorded || context.agent.type !== 'cli') {
        return
      }

      usageRecorded = true
      recordAgentCliUsageInBackground(context.agent, {
        executionId: `chat-${aiMessage.id}`,
        executionMode: 'chat',
        modelId: resolveRecordedModelId({
          reportedModelId: usageState.model,
          requestedModelId: context.agent.modelId
        }),
        projectId: resolvedProjectId,
        sessionId,
        messageId: aiMessage.id,
        inputTokens: usageState.inputTokens,
        outputTokens: usageState.outputTokens,
        occurredAt: occurredAt || new Date().toISOString()
      })
    }

    const syncFinalUsageNotice = () => {
      const realtimeUsage = tokenStore.realtimeTokens.get(sessionId)
      if (realtimeUsage) {
        if ((!usageState.model || usageState.model.trim().length === 0) && realtimeUsage.model) {
          usageState.model = realtimeUsage.model
        }
        if ((usageState.inputTokens ?? 0) <= 0 && realtimeUsage.inputTokens > 0) {
          usageState.inputTokens = realtimeUsage.inputTokens
        }
        if ((usageState.outputTokens ?? 0) <= 0 && realtimeUsage.outputTokens > 0) {
          usageState.outputTokens = realtimeUsage.outputTokens
        }
      }

      const usageNotice = buildUsageNotice(usageState)
      if (!usageNotice) {
        return
      }

      runtimeNoticesState = upsertRuntimeNotice(runtimeNoticesState, usageNotice)
      bufferMessageUpdate({
        runtimeNotices: runtimeNoticesState
      })
    }

    const syncProcessingTimeNotice = () => {
      const finishedAt = streamMetrics.doneAt ?? now()
      const durationMs = Math.max(0, finishedAt - streamMetrics.startedAt)
      const processingTimeNotice = buildProcessingTimeNotice(durationMs)
      if (!processingTimeNotice) {
        return
      }

      runtimeNoticesState = upsertRuntimeNotice(runtimeNoticesState, processingTimeNotice)
      bufferMessageUpdate({
        runtimeNotices: runtimeNoticesState
      })
    }

    try {
      await agentExecutor.execute(context, (event: StreamEvent) => {
        markMetric('firstEventAt')
        registerCliSessionBinding(event.externalSessionId)
        this.handleStreamEvent(event, {
          aiMessage,
          sessionId,
          toolCalls,
          onContent: (content) => {
            markMetric('firstContentAt')
            accumulatedContent += content
            bufferMessageUpdate({
              content: accumulatedContent
            })
          },
          onThinking: (thinking) => {
            markMetric('firstThinkingAt')
            accumulatedThinking += thinking
            bufferMessageUpdate({
              thinking: accumulatedThinking,
              thinkingActive: true
            })
          },
          onThinkingStart: () => {
            bufferMessageUpdate({
              thinkingActive: true
            })
          },
          onToolUse: (toolCall) => {
            markMetric('firstToolAt')
            // 添加或更新工具调用
            const existingIndex = toolCalls.findIndex(tc => tc.id === toolCall.id)
            if (existingIndex >= 0) {
              toolCalls[existingIndex] = toolCall
            } else {
              toolCalls.push(toolCall)
            }
            bufferMessageUpdate({
              toolCalls: [...toolCalls]
            })
            registerTraceTask((async () => {
              await fileTraceCollector.captureToolUse(toolCall)
            })())
          },
          onToolInputDelta: (toolCallId, toolInput) => {
            const targetToolCall = (toolCallId
              ? toolCalls.find(tool => tool.id === toolCallId)
              : null) || [...toolCalls].reverse().find(tool => tool.status === 'running')

            if (!targetToolCall) {
              return
            }

            targetToolCall.arguments = mergeToolInputArguments(targetToolCall.arguments, toolInput)
            bufferMessageUpdate({
              toolCalls: [...toolCalls]
            })
          },
          onToolResult: (toolCallId, result, isError) => {
            // 更新工具调用的结果
            const tc = toolCalls.find(t => t.id === toolCallId)
            if (tc) {
              tc.result = result
              tc.status = isError ? 'error' : 'success'
              if (isError) {
                tc.errorMessage = result
              }
              bufferMessageUpdate({
                toolCalls: [...toolCalls]
              })
            }

            if (!isError) {
              registerTraceTask((async () => {
                const trace = await fileTraceCollector.resolveToolResult(toolCallId, result)
                if (!trace) {
                  return
                }

                editTraces.push(trace)
                bufferMessageUpdate({
                  editTraces: [...editTraces]
                })
              })())
            }
          },
          onFileEdit: (trace) => {
            const exists = editTraces.some(existingTrace => existingTrace.id === trace.id)
            if (exists) {
              return
            }

            editTraces.push(trace)
            bufferMessageUpdate({
              editTraces: [...editTraces]
            })
          },
          onUsage: (usage) => {
            if (usage.model) {
              usageState.model = usage.model
            }
            if (usage.inputTokens !== undefined) {
              usageState.inputTokens = usage.inputTokens
            }
            if (usage.outputTokens !== undefined) {
              usageState.outputTokens = usage.outputTokens
            }

            const usageNotice = buildUsageNotice(usageState)
            if (!usageNotice) {
              return
            }

            runtimeNoticesState = upsertRuntimeNotice(runtimeNoticesState, usageNotice)
            bufferMessageUpdate({
              runtimeNotices: runtimeNoticesState
            })
          },
          onSystem: (content) => {
            const runtimeNotice = buildRuntimeNoticeFromSystemContent(content)
            if (!runtimeNotice) {
              return
            }

            runtimeNoticesState = upsertRuntimeNotice(runtimeNoticesState, runtimeNotice)
            bufferMessageUpdate({
              runtimeNotices: runtimeNoticesState
            })
          },
          onError: (error) => {
            if (isAiMessageInterrupted()) {
              bufferMessageUpdate({
                thinkingActive: false
              }, { immediate: true })
              return
            }

            hasError = true
            bufferMessageUpdate({
              status: 'error',
              errorMessage: error,
              thinkingActive: false
            }, { immediate: true })
          },
          onDone: () => {
            markMetric('doneAt')
            const finalizedToolCalls = finalizePendingToolCalls(toolCalls)
            if (finalizedToolCalls !== toolCalls) {
              toolCalls.splice(0, toolCalls.length, ...finalizedToolCalls)
            }
            syncFinalUsageNotice()
            syncProcessingTimeNotice()
            // 更新消息状态
            if (!hasError && !isAiMessageInterrupted()) {
              bufferMessageUpdate({
                status: 'completed',
                thinkingActive: false,
                toolCalls: [...toolCalls]
              }, { immediate: true })
              // 更新会话最后消息
              sessionStore.updateLastMessage(
                sessionId,
                accumulatedContent.slice(0, 50)
              )
            }
            this.finalizeSend(sessionId)

            // 自动压缩检查
            compressionService.checkAndAutoCompress(sessionId, context.agent.id)

            registerPersistenceTask(
              messageStore.flushBufferedMessageUpdate(aiMessage.id, { notifyOnFailure: true })
            )
            recordUsageOnce(new Date().toISOString())
          }
        })
      })

      flushPendingUiUpdate()
      await Promise.allSettled(Array.from(pendingTraceTasks))
      await Promise.allSettled(Array.from(pendingPersistenceTasks))
      await messageStore.flushBufferedMessageUpdate(aiMessage.id, { notifyOnFailure: true })
      markMetric('persistedAt')
      recordTimingSummary()

      // 兜底：部分后端/CLI 场景可能不会显式发出 done 事件，避免状态长期卡在“生成中”
      if (sessionExecutionStore.getIsSending(sessionId)) {
        markMetric('doneAt')
        const finalizedToolCalls = finalizePendingToolCalls(toolCalls)
        if (finalizedToolCalls !== toolCalls) {
          toolCalls.splice(0, toolCalls.length, ...finalizedToolCalls)
        }
        syncFinalUsageNotice()
        syncProcessingTimeNotice()
        if (!hasError) {
          bufferMessageUpdate({
            status: 'completed',
            thinkingActive: false,
            toolCalls: [...toolCalls]
          }, { immediate: true })
          sessionStore.updateLastMessage(
            sessionId,
            accumulatedContent.slice(0, 50)
          )
        }
        await messageStore.flushBufferedMessageUpdate(aiMessage.id, { notifyOnFailure: true })
        markMetric('persistedAt')
        recordTimingSummary()
        recordUsageOnce(new Date().toISOString())
        this.finalizeSend(sessionId)
      }
    } catch (error) {
      hasError = true
      const errorMessage = getErrorMessage(error, '对话执行失败')
      void writeFrontendRuntimeLog(
        'ERROR',
        'conversation-service',
        `executeConversation failed | sessionId=${sessionId} | agentId=${context.agent.id} | provider=${context.agent.provider || context.agent.type} | error=${errorMessage}`,
        error
      )
      const shouldRetryWithoutResume = Boolean(
        fallbackContext
        && context.resumeSessionId
        && isInvalidCliResumeError(errorMessage, runtimeKey)
      )

      if (shouldRetryWithoutResume) {
        clearScheduledUiFlush()
        pendingUiUpdate = null
        await Promise.allSettled(Array.from(pendingTraceTasks))
        await Promise.allSettled(Array.from(pendingPersistenceTasks))

        if (runtimeKey) {
          try {
            await deleteSessionRuntimeBinding(sessionId, runtimeKey)
          } catch (bindingError) {
            console.warn('[ConversationService] Failed to clear invalid runtime binding:', bindingError)
          }
        }

        await this.syncSessionExecutionBinding(sessionId, context.agent, {
          cliSessionId: '',
          cliSessionProvider: ''
        })
        await messageStore.updateMessage(aiMessage.id, {
          content: '',
          thinking: '',
          thinkingActive: false,
          toolCalls: [],
          editTraces: [],
          errorMessage: '',
          status: 'pending'
        })

        return this.executeConversation(fallbackContext!, aiMessage, sessionId, projectId)
      }

      if (isAiMessageInterrupted()) {
        markMetric('doneAt')
        syncFinalUsageNotice()
        syncProcessingTimeNotice()
        bufferMessageUpdate({
          thinkingActive: false
        }, { immediate: true })
        await messageStore.flushBufferedMessageUpdate(aiMessage.id, { notifyOnFailure: true })
        markMetric('persistedAt')
        recordTimingSummary()
        recordUsageOnce(new Date().toISOString())
        this.finalizeSend(sessionId)
        return
      }

      syncFinalUsageNotice()
      markMetric('doneAt')
      syncProcessingTimeNotice()
      bufferMessageUpdate({
        status: 'error',
        errorMessage,
        thinkingActive: false
      }, { immediate: true })
      await messageStore.flushBufferedMessageUpdate(aiMessage.id, { notifyOnFailure: true })
      markMetric('persistedAt')
      recordTimingSummary()
      recordUsageOnce(new Date().toISOString())
      this.finalizeSend(sessionId)
    } finally {
      clearScheduledUiFlush()
    }
  }

  /**
   * 处理流式事件
   */
  private handleStreamEvent(
    event: StreamEvent,
    handlers: {
      aiMessage: Message
      sessionId: string
      toolCalls: ToolCall[]
      onContent: (content: string) => void
      onThinking: (thinking: string) => void
      onThinkingStart: () => void
      onToolUse: (toolCall: ToolCall) => void
      onToolInputDelta: (toolCallId: string | undefined, toolInput: Record<string, unknown> | undefined) => void
      onToolResult: (toolCallId: string, result: string, isError: boolean) => void
      onFileEdit: (trace: FileEditTrace) => void
      onUsage: (usage: { model?: string, inputTokens?: number, outputTokens?: number }) => void
      onSystem: (content: string) => void
      onError: (error: string) => void
      onDone: () => void
    }
  ): void {
    const { onContent, onThinking, onThinkingStart, onToolUse, onToolInputDelta, onToolResult, onFileEdit, onUsage, onSystem, onError, onDone } = handlers
    const tokenStore = useTokenStore()

    // 处理 token 事件 - 优先使用 CLI 返回的真实 token 数据
    if (event.inputTokens !== undefined || event.outputTokens !== undefined) {
      tokenStore.updateRealtimeTokens(handlers.sessionId, event.inputTokens, event.outputTokens, event.model)
    }

    switch (event.type) {
      case 'content':
        if (event.content) {
          onContent(event.content)
        }
        break

      case 'thinking':
        // 处理思考内容
        if (event.content) {
          onThinking(event.content)
        }
        break

      case 'thinking_start':
        onThinkingStart()
        break

      case 'tool_use':
        if (event.toolName) {
          // 如果 toolCallId 为空，生成一个唯一的备用 ID
          const toolCallId = event.toolCallId || `tool-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
          const toolCall: ToolCall = {
            id: toolCallId,
            name: event.toolName,
            arguments: event.toolInput || {},
            status: 'running'
          }
          onToolUse(toolCall)
        } else {
          console.warn('[ConversationService] tool_use 事件缺少 toolName，跳过处理')
        }
        break

      case 'tool_input_delta':
        onToolInputDelta(event.toolCallId, event.toolInput)
        break

      case 'tool_result':
        if (event.toolCallId) {
          const result = typeof event.toolResult === 'string'
            ? event.toolResult
            : JSON.stringify(event.toolResult, null, 2)
          onToolResult(event.toolCallId, result, false)
        } else {
          console.warn('[ConversationService] tool_result 事件缺少 toolCallId，尝试匹配最后一个工具调用')
          // 如果没有 toolCallId，尝试更新最后一个 running 状态的工具调用
          const lastRunningTool = handlers.toolCalls.find(tc => tc.status === 'running')
          if (lastRunningTool && event.toolResult) {
            const result = typeof event.toolResult === 'string'
              ? event.toolResult
              : JSON.stringify(event.toolResult, null, 2)
            onToolResult(lastRunningTool.id, result, false)
          }
        }
        break

      case 'error':
        if (event.error) {
          onError(event.error)
        }
        break

      case 'usage':
        onUsage({
          model: event.model,
          inputTokens: event.inputTokens,
          outputTokens: event.outputTokens
        })
        break

      case 'system':
        if (event.content) {
          onSystem(event.content)
        }
        break

      case 'file_edit':
        if (event.fileEdit) {
          onFileEdit(event.fileEdit)
        }
        break

      case 'done':
        onDone()
        break
    }
  }

  /**
   * 中断指定会话的执行
   * @param sessionId 会话 ID
   * @param messageId 可选的消息 ID，用于更新消息状态
   */
  abort(sessionId: string, messageId?: string): void

  /**
   * 中断当前执行（向后兼容）
   * @deprecated 使用 abort(sessionId) 替代
   */
  abort(): void

  /**
   * 中断执行的具体实现
   */
  abort(sessionId?: string, messageId?: string): void {
    const messageStore = useMessageStore()
    const sessionExecutionStore = useSessionExecutionStore()

    if (sessionId) {
      // 中断指定会话
      // 1. 调用 AgentExecutor 中断策略
      agentExecutor.abort(sessionId)

      // 2. 更新消息状态为 interrupted
      if (messageId) {
        messageStore.updateMessage(messageId, { status: 'interrupted' })
      } else {
        // 如果没有传入 messageId，从 sessionExecutionStore 获取当前流式消息 ID
        const streamingMessageId = sessionExecutionStore.getExecutionState(sessionId).currentStreamingMessageId
        if (streamingMessageId) {
          messageStore.updateMessage(streamingMessageId, { status: 'interrupted' })
        }
      }

      // 3. 更新会话执行状态
      this.finalizeSend(sessionId)
    } else {
      // 向后兼容：中断所有正在执行的会话
      const runningIds = sessionExecutionStore.runningSessionIds
      for (const id of runningIds) {
        this.abort(id)
      }
    }
  }

  /**
   * 检查智能体是否可用
   */
  isAgentAvailable(agent: AgentConfig): { available: boolean; reason?: string } {
    if (!agentExecutor.isSupported(agent)) {
      return {
        available: false,
        reason: `不支持的智能体类型: ${agent.type}`
      }
    }

    // CLI 类型检查路径
    if (agent.type === 'cli' && !agent.cliPath) {
      return {
        available: false,
        reason: 'CLI 路径未配置'
      }
    }

    // SDK 类型检查 API Key
    if (agent.type === 'sdk') {
      if (!agent.apiKey) {
        return {
          available: false,
          reason: 'API Key 未配置'
        }
      }
      if (!agent.modelId) {
        return {
          available: false,
          reason: '模型未选择'
        }
      }
    }

    return { available: true }
  }
}

// 导出单例
export const conversationService = ConversationService.getInstance()
