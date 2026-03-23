import { useMessageStore, type Message, type MessageAttachment, type ToolCall } from '@/stores/message'
import { useSessionStore } from '@/stores/session'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { useProjectStore } from '@/stores/project'
import { useAgentStore, type AgentConfig } from '@/stores/agent'
import { useNotificationStore } from '@/stores/notification'
import { useTokenStore } from '@/stores/token'
import { useMemoryStore } from '@/stores/memory'
import type { MemoryLibrary } from '@/types/memory'
import { agentExecutor } from './AgentExecutor'
import type { ConversationContext, StreamEvent } from './strategies/types'
import { compressionService } from '@/services/compression/CompressionService'
import { buildConversationMessages } from './buildConversationMessages'
import { buildProjectMemorySystemPrompt } from '@/services/memory'
import type { FileEditTrace } from '@/types/fileTrace'
import { FileTraceCollector } from './fileTraceCollector'
import { resolveUsageModelHint } from './usageModelHint'
import {
  buildCliEnvironmentNotice,
  buildRuntimeNoticeFromSystemContent,
  buildUsageNotice,
  upsertRuntimeNotice
} from '@/utils/runtimeNotice'
import { loadAgentMcpServers } from '@/utils/mcpServerConfig'
import { mergeToolInputArguments } from '@/utils/toolInput'

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

/**
 * 对话服务
 * 封装消息发送逻辑，处理流式事件更新
 */
export class ConversationService {
  private static instance: ConversationService | null = null
  private readonly queueDrainLocks = new Set<string>()

  private constructor() {}

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
    tokenStore.clearRealtimeTokens(sessionId)

    try {
      // 添加用户消息
      const userMessage = await messageStore.addMessage({
        sessionId,
        role: 'user',
        content,
        attachments,
        status: 'completed'
      })

      await memoryStore.captureUserMessage({
        sessionId,
        messageId: userMessage.id,
        content
      })

      // 更新会话最后消息
      const messagePreview = this.buildMessagePreview(content, attachments)
      sessionStore.updateLastMessage(sessionId, messagePreview)

      // 如果会话名称是默认名称（未命名会话），则用第一条消息的前几个字更新
      const session = sessionStore.sessions.find(s => s.id === sessionId)
      if (session && (session.name === '未命名会话' || session.name.startsWith('新会话'))) {
        // 提取前20个字符作为会话名称，去掉换行符
        const titleSource = this.buildMessagePreview(content, attachments)
        const newTitle = titleSource.replace(/\n/g, ' ').slice(0, 20).trim()
        const finalTitle = newTitle.length < titleSource.length ? newTitle + '...' : newTitle
        if (finalTitle) {
          await sessionStore.updateSession(sessionId, { name: finalTitle })
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

      // 构建对话上下文
      const messages = buildConversationMessages(
        messageStore.messagesBySession(sessionId),
        {
          fallbackUserContent: content,
          sessionId,
          injectedSystemMessages: projectMemoryPrompt ? [projectMemoryPrompt] : []
        }
      )
      const userMessages = messages.filter(message => message.role === 'user')
      const systemMessages = messages.filter(message => message.role === 'system')

      console.info('[ConversationService] assembled context messages', {
        sessionId,
        messageCount: messages.length,
        systemMessageCount: systemMessages.length,
        lastUserMessageLength: userMessages.length > 0
          ? userMessages[userMessages.length - 1].content.length
          : 0
      })

      const context: ConversationContext = {
        sessionId,
        agent: executionAgent,
        messages,
        workingDirectory,
        mcpServers: mcpServers.length > 0 ? mcpServers : undefined,
        executionMode: 'chat',
        responseMode: 'stream_text'
      }

      // 执行对话
      await this.executeConversation(context, aiMessage, sessionId)

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
      const projectId = sessionStore.sessions.find(session => session.id === sessionId)?.projectId
      await this.sendMessage(
        sessionId,
        nextDraft.content,
        nextDraft.agentId,
        projectId,
        nextDraft.attachments,
        { modelId: nextDraft.modelId }
      )
    } catch (error) {
      const notificationStore = useNotificationStore()
      const errorMessage = error instanceof Error ? error.message : String(error)
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

  /**
   * 执行对话
   */
  private async executeConversation(
    context: ConversationContext,
    aiMessage: Message,
    sessionId: string
  ): Promise<void> {
    const messageStore = useMessageStore()
    const sessionStore = useSessionStore()
    const sessionExecutionStore = useSessionExecutionStore()

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
    const streamMetrics: StreamTimingMetrics = {
      startedAt: globalThis.performance?.now() ?? Date.now()
    }
    let hasError = false
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

    const flushPendingUiUpdate = (options?: { immediate?: boolean }) => {
      clearScheduledUiFlush()
      if (!pendingUiUpdate) {
        return
      }

      markMetric('firstRenderAt')
      const updates = pendingUiUpdate
      pendingUiUpdate = null
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
      pendingUiUpdate = pendingUiUpdate
        ? { ...pendingUiUpdate, ...updates }
        : { ...updates }

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

    try {
      await agentExecutor.execute(context, (event: StreamEvent) => {
        markMetric('firstEventAt')
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
            // 更新消息状态
            if (!hasError) {
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
        const finalizedToolCalls = finalizePendingToolCalls(toolCalls)
        if (finalizedToolCalls !== toolCalls) {
          toolCalls.splice(0, toolCalls.length, ...finalizedToolCalls)
        }
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
        this.finalizeSend(sessionId)
      }
    } catch (error) {
      hasError = true
      const errorMessage = error instanceof Error ? error.message : String(error)
      bufferMessageUpdate({
        status: 'error',
        errorMessage,
        thinkingActive: false
      }, { immediate: true })
      await messageStore.flushBufferedMessageUpdate(aiMessage.id, { notifyOnFailure: true })
      markMetric('persistedAt')
      recordTimingSummary()
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
