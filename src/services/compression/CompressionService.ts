import { useMessageStore, type Message, type ToolCallSummary, type CompressionMetadata } from '@/stores/message'
import { useSessionStore } from '@/stores/session'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { useAgentStore } from '@/stores/agent'
import { useTokenStore, type CompressionStrategy } from '@/stores/token'
import type { AgentConfig } from '@/stores/agent'
import type { MemoryLibrary } from '@/types/memory'
import { useSettingsStore } from '@/stores/settings'
import { useNotificationStore } from '@/stores/notification'
import { useProjectStore } from '@/stores/project'
import { useAiEditTraceStore } from '@/stores/aiEditTrace'
import { useTracePreviewStore } from '@/stores/tracePreview'
import { buildConversationMessages } from '@/services/conversation/buildConversationMessages'
import { buildProjectMemorySystemPrompt } from '@/services/memory'

/**
 * 压缩选项
 */
export interface CompressionOptions {
  strategy: CompressionStrategy
}

/**
 * 压缩结果
 */
export interface CompressionResult {
  success: boolean
  summary?: string
  originalMessageCount: number
  originalTokenCount: number
  error?: string
}

/**
 * 会话压缩服务
 * 负责压缩会话消息，生成摘要，释放 token 空间
 */
export class CompressionService {
  private static instance: CompressionService | null = null

  private constructor() {}

  /**
   * 获取单例实例
   */
  static getInstance(): CompressionService {
    if (!CompressionService.instance) {
      CompressionService.instance = new CompressionService()
    }
    return CompressionService.instance
  }

  /**
   * 执行会话压缩
   */
  async compressSession(
    sessionId: string,
    agentId: string,
    options: CompressionOptions
  ): Promise<CompressionResult> {
    const messageStore = useMessageStore()
    const sessionStore = useSessionStore()
    const sessionExecutionStore = useSessionExecutionStore()
    const tokenStore = useTokenStore()
    const aiEditTraceStore = useAiEditTraceStore()
    const tracePreviewStore = useTracePreviewStore()

    // 获取当前会话的所有消息
    const messages = messageStore.messagesBySession(sessionId)
    const messageCount = messages.length

    if (messageCount === 0) {
      return {
        success: false,
        error: '没有消息可以压缩',
        originalMessageCount: 0,
        originalTokenCount: 0
      }
    }

    // 计算当前 token 使用量
    const tokenUsage = tokenStore.getTokenUsage(sessionId)
    const originalTokenCount = tokenUsage.used

    // 检查是否正在发送消息
    if (sessionExecutionStore.getIsSending(sessionId)) {
      return {
        success: false,
        error: '正在发送消息，请稍后再试',
        originalMessageCount: messageCount,
        originalTokenCount
      }
    }

    try {
      // 开始压缩状态
      sessionExecutionStore.startSending(sessionId)

      // 提取工具调用摘要
      const toolCallsSummary = this.extractToolCallsSummary(messages)

      let summaryContent = ''

      if (options.strategy === 'summary') {
        // 使用 AI 生成摘要
        summaryContent = await this.generateSummary(sessionId, agentId, messages, toolCallsSummary)
      } else {
        // 简单压缩：只保留基本信息
        summaryContent = this.generateSimpleSummary(messages, toolCallsSummary)
      }

      // 清空当前会话消息
      await messageStore.clearSessionMessages(sessionId)
      aiEditTraceStore.resetSession(sessionId)
      tracePreviewStore.clear()

      // 创建压缩摘要消息
      const compressionMetadata: CompressionMetadata = {
        compressedAt: new Date().toISOString(),
        originalMessageCount: messageCount,
        originalTokenCount,
        strategy: options.strategy,
        toolCallsSummary: toolCallsSummary.length > 0 ? toolCallsSummary : undefined
      }

      // 添加压缩消息作为第一条消息
      await messageStore.addMessage({
        sessionId,
        role: 'compression',
        content: summaryContent,
        status: 'completed',
        compressionMetadata
      })

      // 更新会话最后消息
      sessionStore.updateLastMessage(sessionId, summaryContent.slice(0, 50))

      // 清除 token 缓存并重置实时 token 数据
      tokenStore.clearSessionTokenCache(sessionId)
      // 重置实时 token 为 0，确保进度条正确更新
      tokenStore.updateRealtimeTokens(sessionId, 0, 0)

      return {
        success: true,
        summary: summaryContent,
        originalMessageCount: messageCount,
        originalTokenCount
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return {
        success: false,
        error: errorMessage,
        originalMessageCount: messageCount,
        originalTokenCount
      }
    } finally {
      sessionExecutionStore.endSending(sessionId)
    }
  }

  /**
   * 提取工具调用摘要
   */
  private extractToolCallsSummary(messages: Message[]): ToolCallSummary[] {
    const toolCallMap = new Map<string, { count: number; successCount: number; errorCount: number }>()

    for (const message of messages) {
      if (message.toolCalls) {
        for (const toolCall of message.toolCalls) {
          const existing = toolCallMap.get(toolCall.name) || { count: 0, successCount: 0, errorCount: 0 }
          existing.count++
          if (toolCall.status === 'success') {
            existing.successCount++
          } else if (toolCall.status === 'error') {
            existing.errorCount++
          }
          toolCallMap.set(toolCall.name, existing)
        }
      }
    }

    return Array.from(toolCallMap.entries()).map(([name, data]) => ({
      name,
      count: data.count,
      status: data.errorCount === 0
        ? 'success'
        : data.successCount === 0
          ? 'error'
          : 'mixed'
    }))
  }

  /**
   * 生成简单摘要
   */
  private generateSimpleSummary(messages: Message[], toolCallsSummary: ToolCallSummary[]): string {
    const userMessages = messages.filter(m => m.role === 'user')
    const assistantMessages = messages.filter(m => m.role === 'assistant')

    let summary = `## 会话摘要\n\n`
    summary += `- **用户消息**: ${userMessages.length} 条\n`
    summary += `- **AI 回复**: ${assistantMessages.length} 条\n`

    if (toolCallsSummary.length > 0) {
      summary += `\n### 工具调用记录\n`
      for (const tool of toolCallsSummary) {
        const statusEmoji = tool.status === 'success' ? '✅' : tool.status === 'error' ? '❌' : '⚠️'
        summary += `- ${statusEmoji} **${tool.name}**: ${tool.count} 次\n`
      }
    }

    summary += `\n> 此会话已压缩以释放 token 空间。\n`

    return summary
  }

  /**
   * 使用 AI 生成对话摘要
   */
  private async generateSummary(
    sessionId: string,
    agentId: string,
    messages: Message[],
    toolCallsSummary: ToolCallSummary[]
  ): Promise<string> {
    const agentStore = useAgentStore()
    const sessionStore = useSessionStore()
    const messageStore = useMessageStore()
    const sessionExecutionStore = useSessionExecutionStore()
    const projectStore = useProjectStore()

    // 获取智能体配置
    const agent = agentStore.agents.find(a => a.id === agentId)
    if (!agent) {
      throw new Error('智能体不存在')
    }

    // 构建摘要提示词
    const prompt = this.buildSummaryPrompt(messages, toolCallsSummary)

    // 获取工作目录
    const session = sessionStore.sessions.find(s => s.id === sessionId)
    let workingDirectory: string | undefined
    let projectMemoryPrompt: string | null = null
    if (session?.projectId) {
      const project = projectStore.projects.find(p => p.id === session.projectId)
      workingDirectory = project?.path
      projectMemoryPrompt = await this.buildProjectMemoryPrompt(project?.memoryLibraryIds ?? [])
    }

    // 保存当前流式消息
    let summaryContent = ''

    // 创建一个临时的摘要请求消息
    const tempUserMessage = await messageStore.addMessage({
      sessionId,
      role: 'user',
      content: prompt,
      status: 'completed'
    })

    const aiMessage = await messageStore.addMessage({
      sessionId,
      role: 'assistant',
      content: '',
      status: 'streaming'
    })

    sessionExecutionStore.setCurrentStreamingMessageId(sessionId, aiMessage.id)

    try {
      const executionMessages = buildConversationMessages(
        [
          ...messages,
          {
            id: tempUserMessage.id,
            sessionId,
            role: 'user',
            content: prompt,
            status: 'completed',
            createdAt: tempUserMessage.createdAt
          }
        ],
        {
          sessionId,
          injectedSystemMessages: projectMemoryPrompt ? [projectMemoryPrompt] : []
        }
      )

      // 使用对话服务生成摘要
      await this.executeSummaryGeneration(agent, sessionId, workingDirectory, executionMessages, (content) => {
        summaryContent += content
        messageStore.updateMessage(aiMessage.id, {
          content: summaryContent
        })
      })

      // 更新消息状态
      await messageStore.updateMessage(aiMessage.id, {
        status: 'completed'
      })

      // 删除临时消息
      await messageStore.deleteMessage(tempUserMessage.id)
      await messageStore.deleteMessage(aiMessage.id)

      return summaryContent
    } catch (error) {
      // 清理临时消息
      try {
        await messageStore.deleteMessage(tempUserMessage.id)
        await messageStore.deleteMessage(aiMessage.id)
      } catch {
        // 忽略删除错误
      }
      throw error
    }
  }

  /**
   * 执行摘要生成
   */
  private async executeSummaryGeneration(
    agent: AgentConfig,
    sessionId: string,
    workingDirectory: string | undefined,
    messages: Message[],
    onContent: (content: string) => void
  ): Promise<void> {
    const { agentExecutor } = await import('@/services/conversation/AgentExecutor')

    const context = {
      sessionId,
      agent,
      messages,
      workingDirectory
    }

    let accumulatedContent = ''

    return new Promise((resolve, reject) => {
      agentExecutor.execute(context, (event) => {
        switch (event.type) {
          case 'content':
            if (event.content) {
              accumulatedContent += event.content
              onContent(accumulatedContent)
            }
            break
          case 'error':
            reject(new Error(event.error || '生成摘要失败'))
            break
          case 'done':
            resolve()
            break
        }
      }).catch(reject)
    })
  }

  private async buildProjectMemoryPrompt(memoryLibraryIds: string[]): Promise<string | null> {
    if (memoryLibraryIds.length === 0) {
      return null
    }

    const memoryStore = await import('@/stores/memory').then(module => module.useMemoryStore())
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
   * 构建摘要提示词
   */
  private buildSummaryPrompt(messages: Message[], toolCallsSummary: ToolCallSummary[]): string {
    const userMessages = messages.filter(m => m.role === 'user')
    const assistantMessages = messages.filter(m => m.role === 'assistant')

    let prompt = `生成对话摘要，保留关键信息。

对话: ${userMessages.length} 条用户消息，${assistantMessages.length} 条 AI 回复

`

    if (toolCallsSummary.length > 0) {
      prompt += `工具调用:\n`
      for (const tool of toolCallsSummary) {
        const statusEmoji = tool.status === 'success' ? '✅' : tool.status === 'error' ? '❌' : '⚠️'
        prompt += `- ${statusEmoji} ${tool.name}: ${tool.count} 次\n`
      }
      prompt += '\n'
    }

    prompt += `最近对话:\n`

    const recentMessages = messages.slice(-8)
    for (const msg of recentMessages) {
      const role = msg.role === 'user' ? '用户' : 'AI'
      const content = msg.content.slice(0, 300) + (msg.content.length > 300 ? '...' : '')
      prompt += `**${role}**: ${content}\n\n`
    }

    prompt += `输出结构化摘要（中文）：
1. 主要话题
2. 关键信息
3. 未完成事项
4. 后续建议`

    return prompt
  }

  /**
   * 检查并执行自动压缩
   * @returns 是否执行了压缩
   */
  async checkAndAutoCompress(sessionId: string, agentId: string): Promise<boolean> {
    const settingsStore = useSettingsStore()
    const tokenStore = useTokenStore()
    const notificationStore = useNotificationStore()
    const messageStore = useMessageStore()

    const meaningfulMessages = messageStore
      .messagesBySession(sessionId)
      .filter(message => message.role !== 'compression')

    // 短会话自动压缩会引入额外的 CLI 调用，先用消息数做硬门槛避免误触发。
    if (meaningfulMessages.length < 8) {
      console.log(
        `[CompressionService] 跳过自动压缩: 消息数不足 (${meaningfulMessages.length}/8)`
      )
      return false
    }

    // 检查是否启用自动压缩
    if (!settingsStore.settings.autoCompressionEnabled) {
      return false
    }

    // 检查是否需要压缩
    if (!tokenStore.needsCompression(sessionId)) {
      return false
    }

    // 获取当前 token 使用情况
    const usage = tokenStore.getTokenUsage(sessionId)
    const threshold = settingsStore.settings.compressionThreshold

    if (usage.used < 8000) {
      console.log(
        `[CompressionService] 跳过自动压缩: token 使用量不足 (${usage.used}/8000)`
      )
      return false
    }

    console.log(`[CompressionService] 检查自动压缩: ${usage.percentage.toFixed(1)}% >= ${threshold}%`)

    // 执行压缩
    try {
      const result = await this.compressSession(sessionId, agentId, {
        strategy: settingsStore.settings.compressionStrategy as CompressionStrategy
      })

      if (result.success) {
        notificationStore.success(`会话已自动压缩 (从 ${result.originalTokenCount} tokens 释放空间)`)
        console.log(`[CompressionService] 自动压缩成功: ${result.summary?.slice(0, 100)}...`)
        return true
      } else {
        console.warn(`[CompressionService] 自动压缩失败: ${result.error}`)
        return false
      }
    } catch (error) {
      console.error('[CompressionService] 自动压缩出错:', error)
      return false
    }
  }
}

// 导出单例
export const compressionService = CompressionService.getInstance()
