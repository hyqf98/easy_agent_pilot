import { invoke } from '@tauri-apps/api/core'
import { useMessageStore, type Message, type MessageAttachment, type ToolCall } from '@/stores/message'
import { useSessionStore } from '@/stores/session'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { useProjectStore } from '@/stores/project'
import { useAgentStore, type AgentConfig } from '@/stores/agent'
import { buildAgentProfileSystemPrompt, useAgentProfileStore } from '@/stores/agentProfile'
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
import {
  buildCliEnvironmentNotice,
  buildRuntimeNoticeFromSystemContent,
  buildUsageNotice,
  upsertRuntimeNotice
} from '@/utils/runtimeNotice'

interface CliConfigModelProfile {
  main_model?: string | null
  codex_model?: string | null
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
    const agentProfileStore = useAgentProfileStore()
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

      const usageModelHint = await this.resolveUsageModelHint(agent)
      const executionAgent = (!agent.modelId?.trim() && usageModelHint)
        ? { ...agent, modelId: usageModelHint }
        : agent
      const executionProfile = await agentProfileStore.resolveExecutionProfile(executionAgent.id)

      const environmentNotice = await buildCliEnvironmentNotice(executionAgent)
      if (environmentNotice) {
        await messageStore.updateMessage(aiMessage.id, {
          runtimeNotices: [environmentNotice]
        })
      }

      const initialUsageNotice = buildUsageNotice({ model: usageModelHint })
      if (initialUsageNotice) {
        const currentMessage = messageStore.messagesBySession(sessionId)
          .find(message => message.id === aiMessage.id)
        await messageStore.updateMessage(aiMessage.id, {
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

      const projectMemoryPrompt = await this.buildProjectMemoryPrompt(targetProject?.memoryLibraryIds ?? [])
      const agentSystemPrompt = buildAgentProfileSystemPrompt(executionProfile)

      // 构建对话上下文
      const messages = buildConversationMessages(
        messageStore.messagesBySession(sessionId),
        {
          fallbackUserContent: content,
          sessionId,
          injectedSystemMessages: [
            agentSystemPrompt,
            projectMemoryPrompt
          ].filter((item): item is string => Boolean(item))
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
        mcpServers: executionProfile.mcpServers.length > 0 ? executionProfile.mcpServers : undefined,
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
    const fileTraceCollector = new FileTraceCollector({
      sessionId,
      messageId: aiMessage.id,
      projectPath: context.workingDirectory
    })
    const pendingTraceTasks = new Set<Promise<void>>()
    let hasError = false

    const registerTraceTask = (task: Promise<void>) => {
      pendingTraceTasks.add(task)
      task.finally(() => pendingTraceTasks.delete(task))
    }

    try {
      await agentExecutor.execute(context, (event: StreamEvent) => {
        this.handleStreamEvent(event, {
          aiMessage,
          sessionId,
          toolCalls,
          onContent: (content) => {
            accumulatedContent += content
            // 更新消息内容
            messageStore.updateMessage(aiMessage.id, {
              content: accumulatedContent
            })
          },
          onThinking: (thinking) => {
            accumulatedThinking += thinking
            // 更新思考内容
            messageStore.updateMessage(aiMessage.id, {
              thinking: accumulatedThinking
            })
          },
          onToolUse: (toolCall) => {
            console.log('[ConversationService] onToolUse 被调用:', toolCall)
            // 添加或更新工具调用
            const existingIndex = toolCalls.findIndex(tc => tc.id === toolCall.id)
            if (existingIndex >= 0) {
              toolCalls[existingIndex] = toolCall
              console.log('[ConversationService] 更新已存在的工具调用, index:', existingIndex)
            } else {
              toolCalls.push(toolCall)
              console.log('[ConversationService] 添加新的工具调用, 当前数量:', toolCalls.length)
            }
            // 更新消息的工具调用
            console.log('[ConversationService] 更新消息的工具调用, toolCalls:', toolCalls)
            messageStore.updateMessage(aiMessage.id, {
              toolCalls: [...toolCalls]
            })
            registerTraceTask((async () => {
              await fileTraceCollector.captureToolUse(toolCall)
            })())
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
              // 更新消息的工具调用
              messageStore.updateMessage(aiMessage.id, {
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
                await messageStore.updateMessage(aiMessage.id, {
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
            void messageStore.updateMessage(aiMessage.id, {
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

            const currentMessage = messageStore.messagesBySession(sessionId)
              .find(message => message.id === aiMessage.id)
            void messageStore.updateMessage(aiMessage.id, {
              runtimeNotices: upsertRuntimeNotice(currentMessage?.runtimeNotices, usageNotice)
            })
          },
          onSystem: (content) => {
            const runtimeNotice = buildRuntimeNoticeFromSystemContent(content)
            if (!runtimeNotice) {
              return
            }

            const currentMessage = messageStore.messagesBySession(sessionId)
              .find(message => message.id === aiMessage.id)
            void messageStore.updateMessage(aiMessage.id, {
              runtimeNotices: upsertRuntimeNotice(currentMessage?.runtimeNotices, runtimeNotice)
            })
          },
          onError: (error) => {
            hasError = true
            messageStore.updateMessage(aiMessage.id, {
              status: 'error',
              errorMessage: error
            })
          },
          onDone: () => {
            // 更新消息状态
            if (!hasError) {
              messageStore.updateMessage(aiMessage.id, {
                status: 'completed'
              })
              // 更新会话最后消息
              sessionStore.updateLastMessage(
                sessionId,
                accumulatedContent.slice(0, 50)
              )
            }
            this.finalizeSend(sessionId)

            // 自动压缩检查
            compressionService.checkAndAutoCompress(sessionId, context.agent.id)
          }
        })
      })

      await Promise.allSettled(Array.from(pendingTraceTasks))

      // 兜底：部分后端/CLI 场景可能不会显式发出 done 事件，避免状态长期卡在“生成中”
      if (sessionExecutionStore.getIsSending(sessionId)) {
        if (!hasError) {
          await messageStore.updateMessage(aiMessage.id, {
            status: 'completed'
          })
          sessionStore.updateLastMessage(
            sessionId,
            accumulatedContent.slice(0, 50)
          )
        }
        this.finalizeSend(sessionId)
      }
    } catch (error) {
      hasError = true
      const errorMessage = error instanceof Error ? error.message : String(error)
      await messageStore.updateMessage(aiMessage.id, {
        status: 'error',
        errorMessage
      })
      this.finalizeSend(sessionId)
    }
  }

  private async resolveUsageModelHint(agent: AgentConfig): Promise<string | undefined> {
    const explicitModel = agent.modelId?.trim()
    if (explicitModel && explicitModel !== 'default') {
      return explicitModel
    }

    if (agent.type !== 'cli') {
      return undefined
    }

    const cliType = agent.provider === 'claude' || agent.provider === 'codex'
      ? agent.provider
      : null
    if (!cliType) {
      return undefined
    }

    try {
      const profile = await invoke<CliConfigModelProfile>('read_current_cli_config', { cliType })
      if (cliType === 'codex') {
        return profile.codex_model?.trim() || undefined
      }
      return profile.main_model?.trim() || undefined
    } catch (error) {
      console.warn('[ConversationService] Failed to resolve usage model hint:', error)
      return undefined
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
      onToolUse: (toolCall: ToolCall) => void
      onToolResult: (toolCallId: string, result: string, isError: boolean) => void
      onFileEdit: (trace: FileEditTrace) => void
      onUsage: (usage: { model?: string, inputTokens?: number, outputTokens?: number }) => void
      onSystem: (content: string) => void
      onError: (error: string) => void
      onDone: () => void
    }
  ): void {
    const { onContent, onThinking, onToolUse, onToolResult, onFileEdit, onUsage, onSystem, onError, onDone } = handlers
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

      case 'tool_use':
        // 处理工具调用 - 添加详细日志
        console.log('[ConversationService] 收到 tool_use 事件:', {
          toolName: event.toolName,
          toolCallId: event.toolCallId,
          toolInput: event.toolInput
        })
        if (event.toolName) {
          // 如果 toolCallId 为空，生成一个唯一的备用 ID
          const toolCallId = event.toolCallId || `tool-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
          const toolCall: ToolCall = {
            id: toolCallId,
            name: event.toolName,
            arguments: event.toolInput || {},
            status: 'running'
          }
          console.log('[ConversationService] 创建工具调用:', toolCall)
          onToolUse(toolCall)
        } else {
          console.warn('[ConversationService] tool_use 事件缺少 toolName，跳过处理')
        }
        break

      case 'tool_result':
        // 处理工具结果 - 添加详细日志
        console.log('[ConversationService] 收到 tool_result 事件:', {
          toolCallId: event.toolCallId,
          toolResultType: typeof event.toolResult,
          toolResultLength: typeof event.toolResult === 'string' ? event.toolResult.length : 'N/A'
        })
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
