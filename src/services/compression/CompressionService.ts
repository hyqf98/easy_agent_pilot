import { useMessageStore, type Message, type ToolCallSummary, type CompressionMetadata } from '@/stores/message'
import { useSessionStore, type Session } from '@/stores/session'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { useAgentStore } from '@/stores/agent'
import { useTokenStore, type CompressionStrategy } from '@/stores/token'
import type { AgentConfig } from '@/stores/agent'
import { useSettingsStore } from '@/stores/settings'
import { useProjectStore } from '@/stores/project'
import { useTracePreviewStore } from '@/stores/tracePreview'
import { agentExecutor } from '@/services/conversation/AgentExecutor'
import { buildConversationMessages } from '@/services/conversation/buildConversationMessages'
import { loadMountedMemoryPrompt } from '@/services/memory/mountedMemoryPrompt'
import { shouldAutoCompressByThreshold } from './autoCompression'
import {
  deleteSessionRuntimeBinding,
  getSessionRuntimeBinding,
  resolveRuntimeBindingKey
} from '@/services/conversation/runtimeBindings'
import type { ConversationContext } from '@/services/conversation/strategies/types'
import i18n from '@/i18n'
import { extractExecutionResult } from '@/utils/structuredContent'
import { resolveSessionAgent } from '@/utils/sessionAgent'
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
  triggerSource?: 'manual' | 'auto' | 'silent'
}

/**
 * 压缩结果
 */
export interface CompressionResult {
  success: boolean
  summary?: string
  originalMessageCount: number
  originalTokenCount: number
  newSessionId?: string
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

  private syncTemporarySessionPreview(
    session: Session | undefined,
    triggerMessage: string,
    messageCount: number
  ) {
    if (!session) {
      return
    }

    session.lastMessage = triggerMessage
    session.messageCount = messageCount
    session.updatedAt = new Date().toISOString()
  }

  private async renderCompressionExecutionFlow(
    sessionId: string,
    sourceSession: Session | undefined,
    triggerSource: 'manual' | 'auto'
  ): Promise<{ assistantMessage: Message }> {
    const messageStore = useMessageStore()
    const tracePreviewStore = useTracePreviewStore()
    const triggerMessage = this.buildCompressionTriggerMessage(triggerSource)

    messageStore.clearSessionMessagesCache(sessionId)
    tracePreviewStore.clear()
    this.syncTemporarySessionPreview(sourceSession, triggerMessage, 2)

    await messageStore.addMessage({
      sessionId,
      role: 'user',
      content: triggerMessage,
      status: 'completed'
    })

    const assistantMessage = await messageStore.addMessage({
      sessionId,
      role: 'assistant',
      content: this.t('compression.processing'),
      status: 'streaming'
    })

    return { assistantMessage }
  }

  /**
   * 执行会话压缩。
   * 1. 可选地清空当前页面展示消息，并渲染一轮临时的用户触发消息 + AI 响应
   * 2. 生成摘要（AI 或简单）
   * 3. 清除当前会话所有消息（前端+后端）
   * 4. 删除运行时绑定并清空 CLI 会话 ID（强制新 CLI 会话）
   * 5. 清除 token 缓存
   * 6. 插入压缩摘要消息作为当前会话新的第一条消息
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
    const tracePreviewStore = useTracePreviewStore()
    const sourceSession = sessionStore.sessions.find(s => s.id === sessionId)

    const messages = messageStore.messagesBySession(sessionId)
    const messageCount = messages.length

    if (messageCount === 0) {
      return { success: false, error: this.t('compression.noMessages'), originalMessageCount: 0, originalTokenCount: 0 }
    }

    const originalTokenCount = tokenStore.getTokenUsage(sessionId).used

    if (sessionExecutionStore.getIsSending(sessionId)) {
      return { success: false, error: this.t('compression.sendingInProgress'), originalMessageCount: messageCount, originalTokenCount }
    }

    try {
      sessionExecutionStore.startSending(sessionId)

      const sourceMessages = [...messages]
      const toolCallsSummary = this.extractToolCallsSummary(sourceMessages)
      const fileGroups = this.extractFileGroups(sourceMessages)
      const triggerSource = options.triggerSource ?? 'silent'

      const compressionExecutionFlow = triggerSource !== 'silent'
        ? await this.renderCompressionExecutionFlow(sessionId, sourceSession, triggerSource)
        : null

      let summaryContent = options.strategy === 'summary'
        ? await this.generateSummary(
            agentId,
            sourceMessages,
            toolCallsSummary,
            fileGroups,
            sourceSession,
            (content) => {
              const assistantMessageId = compressionExecutionFlow?.assistantMessage.id
              if (!assistantMessageId) {
                return
              }

              messageStore.updateMessageBuffered(assistantMessageId, {
                content: content || this.t('compression.processing'),
                status: 'streaming'
              })
            }
          )
        : this.generateSimpleSummary(sourceMessages, toolCallsSummary, fileGroups)

      if (!summaryContent.trim()) {
        summaryContent = this.generateSimpleSummary(sourceMessages, toolCallsSummary, fileGroups)
      }

      const compressionMetadata: CompressionMetadata = {
        compressedAt: new Date().toISOString(),
        originalMessageCount: messageCount,
        originalTokenCount,
        strategy: options.strategy,
        summaryContent,
        toolCallsSummary: toolCallsSummary.length > 0 ? toolCallsSummary : undefined,
        panelExpanded: false,
        triggerSource
      }

      await messageStore.clearSessionMessages(sessionId)
      await this.clearRuntimeBindings(sourceSession)
      tokenStore.hardClearSessionTokens(sessionId)
      tracePreviewStore.clear()

      await messageStore.addMessage({
        sessionId,
        role: 'user',
        content: summaryContent,
        status: 'completed',
        compressionMetadata
      })
      sessionStore.updateLastMessage(sessionId, summaryContent.slice(0, 50))

      return {
        success: true,
        summary: summaryContent,
        originalMessageCount: messageCount,
        originalTokenCount,
        newSessionId: sessionId
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
        originalMessageCount: messageCount,
        originalTokenCount
      }
    } finally {
      sessionExecutionStore.endSending(sessionId)
    }
  }

  private async clearRuntimeBindings(session?: Session): Promise<void> {
    if (!session) return
    const sessionStore = useSessionStore()
    const agentStore = useAgentStore()
    const agent = resolveSessionAgent(session, agentStore.agents)
    if (agent) {
      const runtimeKey = resolveRuntimeBindingKey(agent)
      if (runtimeKey) {
        try {
          await deleteSessionRuntimeBinding(session.id, runtimeKey)
        } catch {
          // 绑定可能不存在
        }
      }
    }

    await sessionStore.updateSession(session.id, {
      cliSessionId: '',
      cliSessionProvider: ''
    })
  }

  private buildCompressionTriggerMessage(triggerSource: 'manual' | 'auto'): string {
    if (triggerSource === 'auto') {
      return this.t('compression.autoTriggerMessage')
    }

    return this.t('compression.manualTriggerMessage')
  }

  private shouldReuseCliSession(agent: AgentConfig): boolean {
    if (agent.type !== 'cli') {
      return false
    }

    return resolveRuntimeBindingKey(agent) !== 'codex-cli'
  }

  private resolveCliSessionProvider(agent: AgentConfig): string | undefined {
    return agent.provider?.trim() || agent.type?.trim() || undefined
  }

  private async resolveCompressionResumeSessionId(
    session: Session | undefined,
    agent: AgentConfig
  ): Promise<string | undefined> {
    if (!session || !this.shouldReuseCliSession(agent)) {
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
        console.warn('[CompressionService] Failed to read session runtime binding:', error)
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

  private async generateSummary(
    agentId: string,
    messages: Message[],
    toolCallsSummary: ToolCallSummary[],
    fileGroups: CompressionFileGroups,
    session?: Session,
    onContent?: (content: string) => void
  ): Promise<string> {
    const agentStore = useAgentStore()
    const projectStore = useProjectStore()

    const agent = agentStore.agents.find(a => a.id === agentId)
    if (!agent) {
      throw new Error(this.t('compression.agentNotFound'))
    }

    const prompt = this.buildSummaryPrompt(messages, toolCallsSummary, fileGroups)

    let workingDirectory: string | undefined
    let projectMemoryPrompt: string | null = null
    if (session?.projectId) {
      const project = projectStore.projects.find(p => p.id === session.projectId)
      workingDirectory = project?.path
      projectMemoryPrompt = await this.buildProjectMemoryPrompt(project?.memoryLibraryIds ?? [])
    }

    const resumeSessionId = await this.resolveCompressionResumeSessionId(session, agent)
    const compressionPromptMessage: Message = {
      id: `compression-prompt-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      sessionId: session?.id ?? `compress-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      role: 'user',
      content: prompt,
      status: 'completed',
      createdAt: new Date().toISOString()
    }
    const executionMessages = buildConversationMessages(
      resumeSessionId
        ? [compressionPromptMessage]
        : [...messages, compressionPromptMessage],
      {
        sessionId: session?.id ?? compressionPromptMessage.sessionId,
        injectedSystemMessages: projectMemoryPrompt ? [projectMemoryPrompt] : []
      }
    )

    let summaryContent = ''
    await this.executeSummaryGeneration(
      agent,
      compressionPromptMessage.sessionId,
      workingDirectory,
      executionMessages,
      resumeSessionId,
      (content) => {
        summaryContent = content
        onContent?.(content)
      }
    )

    return summaryContent
  }

  private async executeSummaryGeneration(
    agent: AgentConfig,
    sessionId: string,
    workingDirectory: string | undefined,
    messages: Message[],
    resumeSessionId: string | undefined,
    onContent: (content: string) => void
  ): Promise<void> {
    const context: ConversationContext = {
      sessionId,
      agent,
      messages,
      workingDirectory,
      resumeSessionId
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
    return loadMountedMemoryPrompt(memoryLibraryIds)
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

  shouldAutoCompressSession(sessionId: string): boolean {
    const settingsStore = useSettingsStore()
    const tokenStore = useTokenStore()
    const messageStore = useMessageStore()

    const meaningfulMessages = messageStore
      .messagesBySession(sessionId)
      .filter(message => !message.compressionMetadata)
    const usage = tokenStore.getTokenUsage(sessionId)

    return shouldAutoCompressByThreshold({
      autoCompressionEnabled: settingsStore.settings.autoCompressionEnabled,
      meaningfulMessageCount: meaningfulMessages.length,
      usagePercentage: usage.percentage,
      threshold: settingsStore.settings.compressionThreshold
    })
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
