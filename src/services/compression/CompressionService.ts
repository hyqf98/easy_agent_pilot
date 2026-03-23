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
import i18n from '@/i18n'
import { extractExecutionResult } from '@/utils/structuredContent'
import type { FileEditTrace } from '@/types/fileTrace'

interface CompressionFileGroups {
  generatedFiles: string[]
  modifiedFiles: string[]
  changedFiles: string[]
  deletedFiles: string[]
}

const MAX_RECENT_SUMMARY_MESSAGES = 6
const MAX_SUMMARY_MESSAGE_LENGTH = 220
const MAX_VISIBLE_FILE_COUNT = 8
const MAX_VISIBLE_TOOL_COUNT = 6

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

  private t(key: string, params?: Record<string, unknown>): string {
    return params ? i18n.global.t(key, params) as string : i18n.global.t(key) as string
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
        error: this.t('compression.noMessages'),
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
        error: this.t('compression.sendingInProgress'),
        originalMessageCount: messageCount,
        originalTokenCount
      }
    }

    try {
      // 开始压缩状态
      sessionExecutionStore.startSending(sessionId)

      // 提取工具调用摘要
      const toolCallsSummary = this.extractToolCallsSummary(messages)
      const fileGroups = this.extractFileGroups(messages)

      let summaryContent = ''

      if (options.strategy === 'summary') {
        // 使用 AI 生成摘要
        summaryContent = await this.generateSummary(
          sessionId,
          agentId,
          messages,
          toolCallsSummary,
          fileGroups
        )
      } else {
        // 简单压缩：只保留基本信息
        summaryContent = this.generateSimpleSummary(messages, toolCallsSummary, fileGroups)
      }

      if (!summaryContent.trim()) {
        summaryContent = this.generateSimpleSummary(messages, toolCallsSummary, fileGroups)
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
        toolCallsSummary: toolCallsSummary.length > 0 ? toolCallsSummary : undefined,
        panelExpanded: false
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
  private generateSimpleSummary(
    messages: Message[],
    toolCallsSummary: ToolCallSummary[],
    fileGroups: CompressionFileGroups
  ): string {
    const lastUserMessage = [...messages].reverse().find(message => message.role === 'user')
    const lastAssistantMessage = [...messages].reverse().find(message => message.role === 'assistant')
    const pendingWork = this.resolvePendingWork(messages)
    const lines = [
      `## ${this.t('compression.summaryTitle')}`,
      '',
      `- **${this.t('compression.lastUserRequest')}**: ${this.compactText(lastUserMessage?.content)}`,
      `- **${this.t('compression.lastAssistantResponse')}**: ${this.resolveAssistantSummary(lastAssistantMessage)}`,
      `- **${this.t('compression.pendingWork')}**: ${pendingWork || this.t('compression.none')}`
    ]

    const fileLines = this.buildFileSummaryLines(fileGroups)
    if (fileLines.length > 0) {
      lines.push('', `### ${this.t('compression.fileChanges')}`, ...fileLines)
    }

    if (toolCallsSummary.length > 0) {
      lines.push('', `### ${this.t('compression.toolCallsSummary')}`)
      toolCallsSummary.slice(0, MAX_VISIBLE_TOOL_COUNT).forEach((tool) => {
        lines.push(`- ${tool.name}: ${this.t('compression.toolCount', { count: tool.count })}`)
      })
    }

    lines.push('', `> ${this.t('compression.releasedNotice')}`)
    return lines.join('\n')
  }

  /**
   * 使用 AI 生成对话摘要
   */
  private async generateSummary(
    sessionId: string,
    agentId: string,
    messages: Message[],
    toolCallsSummary: ToolCallSummary[],
    fileGroups: CompressionFileGroups
  ): Promise<string> {
    const agentStore = useAgentStore()
    const sessionStore = useSessionStore()
    const messageStore = useMessageStore()
    const sessionExecutionStore = useSessionExecutionStore()
    const projectStore = useProjectStore()

    // 获取智能体配置
    const agent = agentStore.agents.find(a => a.id === agentId)
    if (!agent) {
      throw new Error(this.t('compression.agentNotFound'))
    }

    // 构建摘要提示词
    const prompt = this.buildSummaryPrompt(messages, toolCallsSummary, fileGroups)

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
            reject(new Error(event.error || this.t('compression.summaryGenerationFailed')))
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
  private buildSummaryPrompt(
    messages: Message[],
    toolCallsSummary: ToolCallSummary[],
    fileGroups: CompressionFileGroups
  ): string {
    const recentMessages = messages
      .filter(message => message.role === 'user' || message.role === 'assistant')
      .slice(-MAX_RECENT_SUMMARY_MESSAGES)
      .map((message) => {
        const role = message.role === 'user' ? 'U' : 'A'
        return `${role}: ${this.compactText(message.content, MAX_SUMMARY_MESSAGE_LENGTH)}`
      })
      .join('\n')

    const toolSummary = toolCallsSummary.length > 0
      ? toolCallsSummary
        .slice(0, MAX_VISIBLE_TOOL_COUNT)
        .map(tool => `- ${tool.name}: ${tool.count}`)
        .join('\n')
      : '- 无'

    return [
      '请把这段会话压缩成后续继续工作所需的最小上下文。',
      '只输出中文 Markdown，只保留事实，不要解释和重复。',
      '',
      '固定结构：',
      '## 目标',
      '- ...',
      '## 已完成',
      '- ...',
      '## 文件变更',
      '- 新增: ...',
      '- 修改: ...',
      '- 删除: ...',
      '## 待继续',
      '- ...',
      '',
      '要求：总长度尽量 <= 220 字；文件只写相对路径；没信息写“无”。',
      '',
      '已知文件：',
      ...this.buildPromptFileLines(fileGroups),
      '',
      '工具：',
      toolSummary,
      '',
      '最近对话：',
      recentMessages || '无'
    ].join('\n')
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
        notificationStore.success(this.t('compression.autoCompressed', {
          tokens: result.originalTokenCount
        }))
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

  private resolveAssistantSummary(message?: Message): string {
    if (!message?.content) {
      return this.t('compression.none')
    }

    const structuredResult = extractExecutionResult(message.content)
    if (structuredResult?.summary) {
      return this.compactText(structuredResult.summary)
    }

    return this.compactText(message.content)
  }

  private resolvePendingWork(messages: Message[]): string {
    const lastMeaningfulMessage = [...messages]
      .reverse()
      .find(message => (message.role === 'user' || message.role === 'assistant') && message.content.trim())

    if (!lastMeaningfulMessage) {
      return ''
    }

    if (lastMeaningfulMessage.role === 'user') {
      return this.compactText(lastMeaningfulMessage.content)
    }

    return ''
  }

  private extractFileGroups(messages: Message[]): CompressionFileGroups {
    const fileGroups: CompressionFileGroups = {
      generatedFiles: [],
      modifiedFiles: [],
      changedFiles: [],
      deletedFiles: []
    }

    for (const message of messages) {
      for (const trace of message.editTraces ?? []) {
        this.appendTraceFile(fileGroups, trace)
      }

      if (message.role === 'assistant') {
        const executionResult = extractExecutionResult(message.content)
        if (executionResult) {
          fileGroups.generatedFiles.push(...executionResult.generatedFiles)
          fileGroups.modifiedFiles.push(...executionResult.modifiedFiles)
          fileGroups.changedFiles.push(...executionResult.changedFiles)
          fileGroups.deletedFiles.push(...executionResult.deletedFiles)
        }
      }

      for (const toolCall of message.toolCalls ?? []) {
        fileGroups.changedFiles.push(
          ...this.extractPathReferencesFromUnknown(toolCall.arguments),
          ...this.extractPathReferencesFromText(toolCall.result),
          ...this.extractPathReferencesFromText(toolCall.errorMessage)
        )
      }
    }

    const generatedFiles = this.uniqueStrings(fileGroups.generatedFiles)
    const modifiedFiles = this.uniqueStrings(fileGroups.modifiedFiles)
    const deletedFiles = this.uniqueStrings(fileGroups.deletedFiles)
    const changedFiles = this.uniqueStrings(fileGroups.changedFiles).filter(file =>
      !generatedFiles.includes(file) && !modifiedFiles.includes(file) && !deletedFiles.includes(file)
    )

    return {
      generatedFiles,
      modifiedFiles,
      changedFiles,
      deletedFiles
    }
  }

  private appendTraceFile(fileGroups: CompressionFileGroups, trace: FileEditTrace): void {
    const file = trace.relativePath?.trim() || trace.filePath?.trim()
    if (!file) {
      return
    }

    if (trace.changeType === 'create') {
      fileGroups.generatedFiles.push(file)
      return
    }

    if (trace.changeType === 'delete') {
      fileGroups.deletedFiles.push(file)
      return
    }

    fileGroups.modifiedFiles.push(file)
  }

  private buildFileSummaryLines(fileGroups: CompressionFileGroups): string[] {
    return [
      this.formatFileGroupLine(this.t('message.structured.generatedFiles'), fileGroups.generatedFiles),
      this.formatFileGroupLine(this.t('message.structured.modifiedFiles'), fileGroups.modifiedFiles),
      this.formatFileGroupLine(this.t('message.structured.changedFiles'), fileGroups.changedFiles),
      this.formatFileGroupLine(this.t('message.structured.deletedFiles'), fileGroups.deletedFiles)
    ].filter(Boolean) as string[]
  }

  private buildPromptFileLines(fileGroups: CompressionFileGroups): string[] {
    return [
      this.formatFileGroupLine('新增', fileGroups.generatedFiles),
      this.formatFileGroupLine('修改', fileGroups.modifiedFiles),
      this.formatFileGroupLine('变更', fileGroups.changedFiles),
      this.formatFileGroupLine('删除', fileGroups.deletedFiles)
    ]
  }

  private formatFileGroupLine(label: string, files: string[]): string {
    const visibleFiles = files.slice(0, MAX_VISIBLE_FILE_COUNT)
    const suffix = files.length > visibleFiles.length ? ` 等 ${files.length} 个` : ''
    const content = visibleFiles.length > 0 ? `${visibleFiles.join(', ')}${suffix}` : this.t('compression.none')
    return `- ${label}: ${content}`
  }

  private extractPathReferencesFromUnknown(value: unknown): string[] {
    const results: string[] = []
    this.walkPathReferences(value, results)
    return this.uniqueStrings(results)
  }

  private walkPathReferences(value: unknown, results: string[]): void {
    if (!value) {
      return
    }

    if (typeof value === 'string') {
      results.push(...this.extractPathReferencesFromText(value))
      return
    }

    if (Array.isArray(value)) {
      value.forEach(item => this.walkPathReferences(item, results))
      return
    }

    if (typeof value === 'object') {
      Object.values(value).forEach(item => this.walkPathReferences(item, results))
    }
  }

  private extractPathReferencesFromText(content?: string): string[] {
    if (!content) {
      return []
    }

    const results = new Set<string>()

    for (const match of content.matchAll(/\[[^\]]+\]\(([^)]+)\)/g)) {
      if (match[1]) {
        results.add(match[1].trim())
      }
    }

    for (const match of content.matchAll(/`([^`\n]+(?:\/|\\)[^`\n]+)`/g)) {
      if (match[1]) {
        results.add(match[1].trim())
      }
    }

    for (const match of content.matchAll(/\b(?:\.{0,2}\/)?[\w.-]+(?:\/[\w.-]+)+\b/g)) {
      if (match[0]) {
        results.add(match[0].trim())
      }
    }

    return this.uniqueStrings(Array.from(results))
  }

  private compactText(content?: string, limit: number = 140): string {
    const normalized = content?.replace(/\s+/g, ' ').trim() ?? ''
    if (!normalized) {
      return this.t('compression.none')
    }

    if (normalized.length <= limit) {
      return normalized
    }

    return `${normalized.slice(0, limit)}...`
  }

  private uniqueStrings(values: string[]): string[] {
    return Array.from(new Set(values.map(value => value.trim()).filter(Boolean)))
  }
}

// 导出单例
export const compressionService = CompressionService.getInstance()
