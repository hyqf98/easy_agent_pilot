import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { useSessionStore } from './session'
import { useSessionExecutionStore } from './sessionExecution'
import { getErrorMessage } from '@/utils/api'
import type { CompressionStrategy } from './token'
import type { FileEditTrace } from '@/types/fileTrace'
import type { RuntimeNotice } from '@/utils/runtimeNotice'

export type MessageRole = 'user' | 'assistant' | 'system' | 'compression'
export type MessageStatus = 'pending' | 'streaming' | 'completed' | 'error' | 'interrupted'
export type ToolCallStatus = 'pending' | 'running' | 'success' | 'error'
export const MANUAL_STOP_ERROR_MARKER = '__manual_stop__'

export interface ToolCall {
  id: string
  name: string
  arguments: Record<string, unknown>
  status: ToolCallStatus
  result?: string
  errorMessage?: string
}

// 工具调用摘要
export interface ToolCallSummary {
  name: string
  count: number
  status: 'success' | 'error' | 'mixed'
}

export interface MessageAttachment {
  id: string
  name: string
  path: string
  mimeType: string
  size: number
  previewUrl?: string
}

// 压缩消息元数据
export interface CompressionMetadata {
  compressedAt: string
  originalMessageCount: number
  originalTokenCount: number
  strategy: CompressionStrategy
  toolCallsSummary?: ToolCallSummary[]
  panelExpanded?: boolean
}

export interface Message {
  id: string
  sessionId: string
  role: MessageRole
  content: string
  attachments?: MessageAttachment[]
  status: MessageStatus
  tokens?: number
  errorMessage?: string
  toolCalls?: ToolCall[]
  // 思考内容（扩展思维模型的思考过程）
  thinking?: string
  thinkingActive?: boolean
  editTraces?: FileEditTrace[]
  runtimeNotices?: RuntimeNotice[]
  createdAt: string
  // 压缩消息的元数据
  compressionMetadata?: CompressionMetadata
}

interface RustToolCall {
  id: string
  name: string
  arguments: string // JSON string
  status: string
  result?: string | null
  errorMessage?: string | null
  error_message?: string | null
}

interface RustMessage {
  id: string
  sessionId?: string
  session_id?: string
  role: string
  content: string
  attachments?: MessageAttachment[] | null
  status: string
  tokens: number | null
  errorMessage?: string | null
  error_message?: string | null
  toolCalls?: RustToolCall[] | null
  tool_calls?: RustToolCall[] | null
  thinking?: string | null
  editTraces?: string | null
  edit_traces?: string | null
  runtimeNotices?: string | null
  runtime_notices?: string | null
  compressionMetadata?: string | null
  compression_metadata?: string | null
  createdAt?: string
  created_at?: string
}

interface PaginatedRustMessages {
  messages: RustMessage[]
  total: number
  has_more: boolean
}

type SessionEditTrace = FileEditTrace & { messageId: string }

function resolveRawMessageCreatedAt(message?: RustMessage): string | null {
  if (!message) return null
  return message.created_at ?? message.createdAt ?? null
}

function compareMessageCreatedAt(left: Pick<Message, 'createdAt'>, right: Pick<Message, 'createdAt'>): number {
  return new Date(left.createdAt).getTime() - new Date(right.createdAt).getTime()
}

function dedupeMessagesById(items: Message[]): Message[] {
  const map = new Map<string, Message>()

  for (const message of items) {
    map.set(message.id, message)
  }

  return Array.from(map.values())
}

const EMPTY_MESSAGES: Message[] = []
const EMPTY_ASSISTANT_EDIT_TRACES: SessionEditTrace[] = []
const EMPTY_TRACE_MAP = new Map<string, { traceId: string, messageId: string, timestamp: string }>()
const EMPTY_VISIBLE_MESSAGE_TRACES: SessionEditTrace[] = []

interface CreateMessageInput {
  session_id: string
  role: string
  content: string
  attachments?: string
  status?: string
  tokens?: number
  error_message?: string
  tool_calls?: string // JSON string
  thinking?: string
  edit_traces?: string
  runtime_notices?: string
  compression_metadata?: string
}

interface UpdateMessageInput {
  content?: string
  attachments?: string
  status?: string
  tokens?: number
  error_message?: string
  tool_calls?: string // JSON string
  thinking?: string
  edit_traces?: string
  runtime_notices?: string
  compression_metadata?: string
}

// 分页状态
export interface PaginationState {
  total: number
  hasMore: boolean
  isLoadingMore: boolean
  oldestMessageCreatedAt: string | null
}

interface BufferedMessageUpdateOptions {
  immediate?: boolean
}

interface FlushBufferedMessageOptions {
  notifyOnFailure?: boolean
}

const MESSAGE_FLUSH_INTERVAL_MS = 300

function serializeToolCalls(toolCalls: ToolCall[]): string {
  return JSON.stringify(toolCalls.map(toolCall => ({
    ...toolCall,
    arguments: JSON.stringify(toolCall.arguments ?? {})
  })))
}

function finalizeToolCallsForStatus(
  toolCalls: ToolCall[] | undefined,
  status: MessageStatus | undefined
): ToolCall[] | undefined {
  if (!toolCalls?.length) {
    return toolCalls
  }

  if (!status || (status !== 'completed' && status !== 'interrupted' && status !== 'error')) {
    return toolCalls
  }

  return toolCalls.map(toolCall => {
    if (toolCall.status !== 'running') {
      return toolCall
    }

    return {
      ...toolCall,
      status: status === 'error' ? 'error' : 'success',
      errorMessage: status === 'error'
        ? toolCall.errorMessage || '消息执行失败'
        : toolCall.errorMessage
    }
  })
}

function buildLatestAssistantTraceMap(
  messages: Message[]
): Map<string, { traceId: string, messageId: string, timestamp: string }> {
  const traceMap = new Map<string, { traceId: string, messageId: string, timestamp: string }>()

  for (const message of messages) {
    if (message.role !== 'assistant' || !message.editTraces?.length) {
      continue
    }

    for (const trace of message.editTraces) {
      const existing = traceMap.get(trace.filePath)
      if (!existing || existing.timestamp <= trace.timestamp) {
        traceMap.set(trace.filePath, {
          traceId: trace.id,
          messageId: message.id,
          timestamp: trace.timestamp
        })
      }
    }
  }

  return traceMap
}

function buildAssistantEditTraces(messages: Message[]): SessionEditTrace[] {
  const traces: SessionEditTrace[] = []

  for (const message of messages) {
    if (message.role !== 'assistant' || !message.editTraces?.length) {
      continue
    }

    for (const trace of message.editTraces) {
      traces.push({
        ...trace,
        messageId: message.id
      })
    }
  }

  return traces
}

function buildAssistantTraceDigest(traces: SessionEditTrace[]): string {
  const latestTrace = traces[traces.length - 1]
  return `${traces.length}:${latestTrace?.id ?? ''}:${latestTrace?.timestamp ?? ''}`
}

function buildVisibleAssistantEditTracesByMessage(
  messages: Message[],
  latestTraceByFile: Map<string, { traceId: string, messageId: string, timestamp: string }>
): Map<string, SessionEditTrace[]> {
  const tracesByMessage = new Map<string, SessionEditTrace[]>()

  for (const message of messages) {
    if (message.role !== 'assistant' || !message.editTraces?.length) {
      continue
    }

    const visibleTraces = message.editTraces
      .filter((trace) => {
        const latest = latestTraceByFile.get(trace.filePath)
        return latest?.traceId === trace.id && latest.messageId === message.id
      })
      .map(trace => ({
        ...trace,
        messageId: message.id
      }))
      .sort((left, right) => right.timestamp.localeCompare(left.timestamp))

    if (visibleTraces.length > 0) {
      tracesByMessage.set(message.id, visibleTraces)
    }
  }

  return tracesByMessage
}

function shouldReconcileStreamingMessage(
  message: Message,
  currentStreamingMessageId: string | null,
  isSessionBusy: boolean,
  hasLocalActivity: boolean
): boolean {
  if (message.role !== 'assistant' || message.status !== 'streaming') {
    return false
  }

  if (isSessionBusy) {
    return false
  }

  if (hasLocalActivity) {
    return false
  }

  if (!currentStreamingMessageId) {
    return true
  }

  return message.id !== currentStreamingMessageId
}

function transformMessage(rustMsg: RustMessage): Message {
  const rawToolCalls = rustMsg.toolCalls ?? rustMsg.tool_calls
  // 转换 tool calls
  const toolCalls: ToolCall[] | undefined = rawToolCalls?.map(tc => ({
    id: tc.id,
    name: tc.name,
    arguments: (() => {
      try {
        return JSON.parse(tc.arguments || '{}') as Record<string, unknown>
      } catch {
        return {}
      }
    })(),
    status: tc.status as ToolCallStatus,
    result: tc.result ?? undefined,
    errorMessage: tc.errorMessage ?? tc.error_message ?? undefined
  })) ?? undefined

  const sessionId = rustMsg.sessionId ?? rustMsg.session_id
  const createdAt = rustMsg.createdAt ?? rustMsg.created_at
  const errorMessage = rustMsg.errorMessage ?? rustMsg.error_message
  const rawCompressionMetadata = rustMsg.compressionMetadata ?? rustMsg.compression_metadata
  const rawEditTraces = rustMsg.editTraces ?? rustMsg.edit_traces
  const rawRuntimeNotices = rustMsg.runtimeNotices ?? rustMsg.runtime_notices
  const editTraces = (() => {
    if (!rawEditTraces) return undefined
    try {
      return JSON.parse(rawEditTraces) as FileEditTrace[]
    } catch {
      return undefined
    }
  })()
  const runtimeNotices = (() => {
    if (!rawRuntimeNotices) return undefined
    try {
      return JSON.parse(rawRuntimeNotices) as RuntimeNotice[]
    } catch {
      return undefined
    }
  })()
  const compressionMetadata = (() => {
    if (!rawCompressionMetadata) return undefined
    try {
      return JSON.parse(rawCompressionMetadata) as CompressionMetadata
    } catch {
      return undefined
    }
  })()

  return {
    id: rustMsg.id,
    sessionId: sessionId || '',
    role: rustMsg.role as MessageRole,
    content: rustMsg.content,
    attachments: rustMsg.attachments?.length ? rustMsg.attachments : undefined,
    status: rustMsg.status as MessageStatus,
    tokens: rustMsg.tokens ?? undefined,
    errorMessage: errorMessage ?? undefined,
    toolCalls: toolCalls && toolCalls.length > 0 ? toolCalls : undefined,
    thinking: rustMsg.thinking ?? undefined,
    editTraces: editTraces && editTraces.length > 0 ? editTraces : undefined,
    runtimeNotices: runtimeNotices && runtimeNotices.length > 0 ? runtimeNotices : undefined,
    compressionMetadata,
    createdAt: createdAt || new Date().toISOString()
  }
}

export const useMessageStore = defineStore('message', () => {
  // State
  const messages = ref<Message[]>([])
  const isLoading = ref(false)
  const pagination = ref<Map<string, PaginationState>>(new Map())
  const sessionMessages = ref<Map<string, Message[]>>(new Map())
  const assistantEditTracesBySession = ref<Map<string, SessionEditTrace[]>>(new Map())
  const latestAssistantTraceBySession = ref<Map<string, Map<string, { traceId: string, messageId: string, timestamp: string }>>>(new Map())
  const visibleAssistantEditTracesByMessageBySession = ref<Map<string, Map<string, SessionEditTrace[]>>>(new Map())
  const assistantTraceDigestBySession = ref<Map<string, string>>(new Map())
  const pendingMessageUpdates = new Map<string, Partial<Message>>()
  const pendingMessageTimers = new Map<string, ReturnType<typeof setTimeout>>()
  const inFlightMessageFlushes = new Map<string, Promise<void>>()

  // 默认分页大小
  const PAGE_SIZE = 20

  const messagesBySession = computed(() => {
    return (sessionId: string) => sessionMessages.value.get(sessionId) ?? EMPTY_MESSAGES
  })

  const getLatestAssistantTraceIdsByFile = (sessionId: string) => {
    return latestAssistantTraceBySession.value.get(sessionId) ?? EMPTY_TRACE_MAP
  }

  const getVisibleAssistantEditTracesForMessage = (sessionId: string, messageId: string) => {
    return visibleAssistantEditTracesByMessageBySession.value.get(sessionId)?.get(messageId) ?? EMPTY_VISIBLE_MESSAGE_TRACES
  }

  const getAssistantEditTraces = (sessionId: string) => {
    return assistantEditTracesBySession.value.get(sessionId) ?? EMPTY_ASSISTANT_EDIT_TRACES
  }

  const getAssistantTraceDigest = (sessionId: string) => {
    return assistantTraceDigestBySession.value.get(sessionId) ?? '0::'
  }

  const lastMessage = computed(() => {
    return (sessionId: string) => {
      const sessionMessages = messagesBySession.value(sessionId)
      return sessionMessages[sessionMessages.length - 1]
    }
  })

  // 获取分页状态
  const getPagination = (sessionId: string): PaginationState => {
    return pagination.value.get(sessionId) || {
      total: 0,
      hasMore: false,
      isLoadingMore: false,
      oldestMessageCreatedAt: null
    }
  }

  // Actions
  function buildUpdateMessageInput(updates: Partial<Message>): UpdateMessageInput {
    const input: UpdateMessageInput = {}
    if (updates.content !== undefined) input.content = updates.content
    if (updates.attachments !== undefined) input.attachments = JSON.stringify(updates.attachments)
    if (updates.status !== undefined) input.status = updates.status
    if (updates.tokens !== undefined) input.tokens = updates.tokens
    if (updates.errorMessage !== undefined) input.error_message = updates.errorMessage
    if (updates.toolCalls !== undefined) {
      input.tool_calls = serializeToolCalls(updates.toolCalls)
    }
    if (updates.thinking !== undefined) input.thinking = updates.thinking
    if (updates.editTraces !== undefined) {
      input.edit_traces = JSON.stringify(updates.editTraces)
    }
    if (updates.runtimeNotices !== undefined) {
      input.runtime_notices = JSON.stringify(updates.runtimeNotices)
    }
    if (updates.compressionMetadata !== undefined) {
      input.compression_metadata = JSON.stringify(updates.compressionMetadata)
    }
    return input
  }

  function setSessionMessages(sessionId: string, nextMessages: Message[]): void {
    const normalizedMessages = dedupeMessagesById(nextMessages).sort(compareMessageCreatedAt)
    const assistantEditTraces = buildAssistantEditTraces(normalizedMessages)
    const latestAssistantTraceMap = buildLatestAssistantTraceMap(normalizedMessages)
    sessionMessages.value.set(sessionId, normalizedMessages)
    assistantEditTracesBySession.value.set(sessionId, assistantEditTraces)
    latestAssistantTraceBySession.value.set(sessionId, latestAssistantTraceMap)
    visibleAssistantEditTracesByMessageBySession.value.set(
      sessionId,
      buildVisibleAssistantEditTracesByMessage(normalizedMessages, latestAssistantTraceMap)
    )
    assistantTraceDigestBySession.value.set(sessionId, buildAssistantTraceDigest(assistantEditTraces))
  }

  function clearSessionDerivedState(sessionId: string): void {
    sessionMessages.value.delete(sessionId)
    assistantEditTracesBySession.value.delete(sessionId)
    latestAssistantTraceBySession.value.delete(sessionId)
    visibleAssistantEditTracesByMessageBySession.value.delete(sessionId)
    assistantTraceDigestBySession.value.delete(sessionId)
  }

  /**
   * 清理单个会话的消息缓存、分页和缓冲写入状态。
   * 用于删除会话后同步移除前端残留数据，避免当前运行期继续引用已删除消息。
   */
  function clearSessionMessagesCache(sessionId: string): void {
    const cachedSessionMessages = sessionMessages.value.get(sessionId) ?? EMPTY_MESSAGES
    const messageIds = [...messages.value, ...cachedSessionMessages]
      .filter(message => message.sessionId === sessionId)
      .map(message => message.id)

    for (const messageId of messageIds) {
      const timer = pendingMessageTimers.get(messageId)
      if (timer) {
        clearTimeout(timer)
        pendingMessageTimers.delete(messageId)
      }

      pendingMessageUpdates.delete(messageId)
      inFlightMessageFlushes.delete(messageId)
    }

    updateGlobalMessagesForSession(sessionId, [])
    clearSessionDerivedState(sessionId)
    pagination.value.delete(sessionId)
  }

  function updateGlobalMessagesForSession(sessionId: string, nextSessionMessages: Message[]): void {
    const otherSessionMessages = messages.value.filter(message => message.sessionId !== sessionId)
    messages.value = [...otherSessionMessages, ...nextSessionMessages]
  }

  function applyMessageUpdatesLocally(id: string, updates: Partial<Message>): void {
    const index = messages.value.findIndex(message => message.id === id)
    if (index === -1) {
      return
    }

    const currentMessage = messages.value[index]
    const nextStatus = updates.status ?? currentMessage.status
    const nextToolCalls = finalizeToolCallsForStatus(
      updates.toolCalls ?? currentMessage.toolCalls,
      nextStatus
    )

    const nextMessage = {
      ...currentMessage,
      ...updates,
      toolCalls: nextToolCalls
    }
    messages.value[index] = nextMessage

    const currentSessionMessages = sessionMessages.value.get(currentMessage.sessionId)
    if (!currentSessionMessages) {
      return
    }

    const sessionIndex = currentSessionMessages.findIndex(message => message.id === id)
    if (sessionIndex < 0) {
      return
    }

    const nextSessionMessages = [...currentSessionMessages]
    nextSessionMessages[sessionIndex] = nextMessage
    setSessionMessages(currentMessage.sessionId, nextSessionMessages)
  }

  function mergeBufferedMessageUpdate(
    previous: Partial<Message> | undefined,
    next: Partial<Message>
  ): Partial<Message> {
    if (!previous) {
      return { ...next }
    }

    return {
      ...previous,
      ...next
    }
  }

  async function persistMessageUpdates(id: string, updates: Partial<Message>): Promise<void> {
    const input = buildUpdateMessageInput(updates)

    if (Object.keys(input).length === 0) {
      return
    }

    await invoke('update_message_fields', { id, input })
  }

  function scheduleBufferedMessageFlush(id: string): void {
    const existingTimer = pendingMessageTimers.get(id)
    if (existingTimer) {
      clearTimeout(existingTimer)
    }

    const timer = setTimeout(() => {
      pendingMessageTimers.delete(id)
      void flushBufferedMessageUpdate(id)
    }, MESSAGE_FLUSH_INTERVAL_MS)

    pendingMessageTimers.set(id, timer)
  }

  async function flushBufferedMessageUpdate(
    id: string,
    options: FlushBufferedMessageOptions = {}
  ): Promise<void> {
    const existingTimer = pendingMessageTimers.get(id)
    if (existingTimer) {
      clearTimeout(existingTimer)
      pendingMessageTimers.delete(id)
    }

    const inFlightFlush = inFlightMessageFlushes.get(id)
    if (inFlightFlush) {
      await inFlightFlush
      if (pendingMessageUpdates.has(id)) {
        await flushBufferedMessageUpdate(id, options)
      }
      return
    }

    const updates = pendingMessageUpdates.get(id)
    if (!updates) {
      return
    }

    pendingMessageUpdates.delete(id)

    const notificationStore = useNotificationStore()
    const flushTask = persistMessageUpdates(id, updates)
      .catch((error) => {
        pendingMessageUpdates.set(
          id,
          mergeBufferedMessageUpdate(pendingMessageUpdates.get(id), updates)
        )

        if (options.notifyOnFailure) {
          notificationStore.databaseError(
            '刷新消息状态失败',
            getErrorMessage(error),
            () => flushBufferedMessageUpdate(id, options)
          )
        } else {
          console.warn('[MessageStore] Failed to flush buffered message update:', error)
          scheduleBufferedMessageFlush(id)
        }
      })
      .finally(() => {
        inFlightMessageFlushes.delete(id)
      })

    inFlightMessageFlushes.set(id, flushTask)
    await flushTask

    if (pendingMessageUpdates.has(id)) {
      await flushBufferedMessageUpdate(id, options)
    }
  }

  function updateMessageBuffered(
    id: string,
    updates: Partial<Message>,
    options: BufferedMessageUpdateOptions = {}
  ): void {
    applyMessageUpdatesLocally(id, updates)
    pendingMessageUpdates.set(
      id,
      mergeBufferedMessageUpdate(pendingMessageUpdates.get(id), updates)
    )

    if (options.immediate) {
      void flushBufferedMessageUpdate(id)
      return
    }

    scheduleBufferedMessageFlush(id)
  }

  async function loadMessages(sessionId: string) {
    const notificationStore = useNotificationStore()
    const sessionExecutionStore = useSessionExecutionStore()
    isLoading.value = true
    try {
      const result = await invoke<PaginatedRustMessages>('list_messages', {
        sessionId,
        limit: PAGE_SIZE
      })

      const currentExecutionState = sessionExecutionStore.getExecutionState(sessionId)
      const nextSessionMessages = result.messages.map(transformMessage)
      const currentSessionMessageMap = new Map(
        (sessionMessages.value.get(sessionId) ?? EMPTY_MESSAGES).map(message => [message.id, message] as const)
      )
      const isSessionBusy = currentExecutionState.isSending
        || currentExecutionState.isStreaming
        || currentExecutionState.isAwaitingRetry
        || currentExecutionState.isQueueDraining
      const streamingMessagesToReconcile = nextSessionMessages.filter(message => (
        shouldReconcileStreamingMessage(
          message,
          currentExecutionState.currentStreamingMessageId,
          isSessionBusy,
          currentSessionMessageMap.get(message.id)?.status === 'streaming'
            || pendingMessageUpdates.has(message.id)
            || inFlightMessageFlushes.has(message.id)
        )
      ))

      const normalizedSessionMessages = streamingMessagesToReconcile.length > 0
        ? nextSessionMessages.map(message => (
          streamingMessagesToReconcile.some(streamingMessage => streamingMessage.id === message.id)
            ? { ...message, status: 'interrupted' as const }
            : message
        ))
        : nextSessionMessages

      updateGlobalMessagesForSession(sessionId, normalizedSessionMessages)
      setSessionMessages(sessionId, normalizedSessionMessages)

      if (streamingMessagesToReconcile.length > 0) {
        await Promise.allSettled(streamingMessagesToReconcile.map(async message => {
          try {
            await persistMessageUpdates(message.id, { status: 'interrupted' })
          } catch (error) {
            console.warn('[MessageStore] Failed to reconcile stale streaming message:', message.id, error)
          }
        }))
      }

      // 更新分页状态
      const oldestMessage = result.messages[0]
      pagination.value.set(sessionId, {
        total: result.total,
        hasMore: result.has_more,
        isLoadingMore: false,
        oldestMessageCreatedAt: resolveRawMessageCreatedAt(oldestMessage)
      })
    } catch (error) {
      console.error('Failed to load messages:', error)
      updateGlobalMessagesForSession(sessionId, [])
      clearSessionDerivedState(sessionId)
      notificationStore.databaseError(
        '加载消息列表失败',
        getErrorMessage(error),
        () => loadMessages(sessionId)
      )
    } finally {
      isLoading.value = false
    }
  }

  // 加载更多历史消息
  async function loadMoreMessages(sessionId: string) {
    const notificationStore = useNotificationStore()
    const currentPagination = getPagination(sessionId)

    // 如果没有更多消息或正在加载，直接返回
    if (!currentPagination.hasMore || currentPagination.isLoadingMore) {
      return
    }

    // 如果没有最早消息的时间戳，无法加载更多
    if (!currentPagination.oldestMessageCreatedAt) {
      return
    }

    // 更新加载状态
    pagination.value.set(sessionId, {
      ...currentPagination,
      isLoadingMore: true
    })

    try {
      const result = await invoke<PaginatedRustMessages>('list_messages', {
        sessionId,
        limit: PAGE_SIZE,
        before: currentPagination.oldestMessageCreatedAt
      })

      // 将新加载的消息添加到当前会话列表开头，同时保留其他会话消息
      const newMessages = result.messages.map(transformMessage)
      const currentSessionMessages = sessionMessages.value.get(sessionId) ?? EMPTY_MESSAGES
      const nextSessionMessages = dedupeMessagesById([
        ...newMessages,
        ...currentSessionMessages
      ]).sort(compareMessageCreatedAt)
      updateGlobalMessagesForSession(sessionId, nextSessionMessages)
      setSessionMessages(sessionId, nextSessionMessages)

      // 更新分页状态
      const oldestMessage = result.messages[0]
      const resolvedOldestMessageCreatedAt = resolveRawMessageCreatedAt(oldestMessage)
      const hasMore = result.messages.length > 0
        && resolvedOldestMessageCreatedAt !== currentPagination.oldestMessageCreatedAt
        && result.has_more
      pagination.value.set(sessionId, {
        total: result.total,
        hasMore,
        isLoadingMore: false,
        oldestMessageCreatedAt: resolvedOldestMessageCreatedAt || currentPagination.oldestMessageCreatedAt
      })
    } catch (error) {
      console.error('Failed to load more messages:', error)
      notificationStore.databaseError(
        '加载历史消息失败',
        getErrorMessage(error),
        () => loadMoreMessages(sessionId)
      )
      // 恢复加载状态
      pagination.value.set(sessionId, {
        ...currentPagination,
        isLoadingMore: false
      })
    }
  }

  async function addMessage(message: Omit<Message, 'id' | 'createdAt'>) {
    const notificationStore = useNotificationStore()
    const input: CreateMessageInput = {
      session_id: message.sessionId,
      role: message.role,
      content: message.content,
      attachments: message.attachments ? JSON.stringify(message.attachments) : undefined,
      status: message.status,
      tokens: message.tokens,
      error_message: message.errorMessage,
      tool_calls: message.toolCalls ? serializeToolCalls(message.toolCalls) : undefined,
      thinking: message.thinking,
      edit_traces: message.editTraces ? JSON.stringify(message.editTraces) : undefined,
      runtime_notices: message.runtimeNotices ? JSON.stringify(message.runtimeNotices) : undefined,
      compression_metadata: message.compressionMetadata
        ? JSON.stringify(message.compressionMetadata)
        : undefined
    }

    try {
      const rustMsg = await invoke<RustMessage>('create_message', { input })
      const newMessage = transformMessage(rustMsg)
      messages.value.push(newMessage)
      const currentSessionMessages = sessionMessages.value.get(newMessage.sessionId) ?? EMPTY_MESSAGES
      setSessionMessages(newMessage.sessionId, [...currentSessionMessages, newMessage])
      return newMessage
    } catch (error) {
      console.error('Failed to add message:', error)
      notificationStore.databaseError(
        '添加消息失败',
        getErrorMessage(error),
        async () => { await addMessage(message) }
      )
      throw error
    }
  }

  async function updateMessage(id: string, updates: Partial<Message>) {
    const notificationStore = useNotificationStore()
    applyMessageUpdatesLocally(id, updates)

    try {
      await persistMessageUpdates(id, updates)
    } catch (error) {
      console.error('Failed to update message:', error)
      notificationStore.databaseError(
        '更新消息失败',
        getErrorMessage(error),
        () => updateMessage(id, updates)
      )
      throw error
    }
  }

  async function deleteMessage(id: string) {
    const notificationStore = useNotificationStore()

    try {
      await invoke('delete_message', { id })
      const index = messages.value.findIndex(m => m.id === id)
      if (index !== -1) {
        const deletedMessage = messages.value[index]
        messages.value.splice(index, 1)
        const currentSessionMessages = sessionMessages.value.get(deletedMessage.sessionId) ?? EMPTY_MESSAGES
        const nextSessionMessages = currentSessionMessages.filter(message => message.id !== id)
        if (nextSessionMessages.length === 0) {
          clearSessionDerivedState(deletedMessage.sessionId)
        } else {
          setSessionMessages(deletedMessage.sessionId, nextSessionMessages)
        }
      }
    } catch (error) {
      console.error('Failed to delete message:', error)
      notificationStore.databaseError(
        '删除消息失败',
        getErrorMessage(error),
        () => deleteMessage(id)
      )
      throw error
    }
  }

  async function clearSessionMessages(sessionId: string) {
    const notificationStore = useNotificationStore()

    try {
      await invoke('clear_session_messages', { sessionId })
      clearSessionMessagesCache(sessionId)

      const sessionStore = useSessionStore()
      const session = sessionStore.sessions.find(item => item.id === sessionId)
      if (session) {
        session.lastMessage = undefined
        session.messageCount = 0
      }
    } catch (error) {
      console.error('Failed to clear session messages:', error)
      notificationStore.databaseError(
        '清空会话消息失败',
        getErrorMessage(error),
        () => clearSessionMessages(sessionId)
      )
      throw error
    }
  }

  function clearProjectMessages(sessionIds: string[]) {
    if (sessionIds.length === 0) {
      return
    }

    const sessionIdSet = new Set(sessionIds)
    messages.value = messages.value.filter(message => !sessionIdSet.has(message.sessionId))
    sessionIds.forEach(sessionId => {
      clearSessionMessagesCache(sessionId)
    })
  }

  return {
    // State
    messages,
    isLoading,
    pagination,
    // Constants
    PAGE_SIZE,
    // Getters
    messagesBySession,
    lastMessage,
    getPagination,
    getAssistantEditTraces,
    getAssistantTraceDigest,
    getLatestAssistantTraceIdsByFile,
    getVisibleAssistantEditTracesForMessage,
    // Actions
    loadMessages,
    loadMoreMessages,
    addMessage,
    updateMessage,
    updateMessageBuffered,
    flushBufferedMessageUpdate,
    deleteMessage,
    clearSessionMessages,
    clearSessionMessagesCache,
    clearProjectMessages
  }
})
