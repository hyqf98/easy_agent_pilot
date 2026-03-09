import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'
import type { CompressionStrategy } from './token'

export type MessageRole = 'user' | 'assistant' | 'system' | 'compression'
export type MessageStatus = 'pending' | 'streaming' | 'completed' | 'error' | 'interrupted'
export type ToolCallStatus = 'pending' | 'running' | 'success' | 'error'

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

// 压缩消息元数据
export interface CompressionMetadata {
  compressedAt: string
  originalMessageCount: number
  originalTokenCount: number
  strategy: CompressionStrategy
  toolCallsSummary?: ToolCallSummary[]
}

export interface Message {
  id: string
  sessionId: string
  role: MessageRole
  content: string
  status: MessageStatus
  tokens?: number
  errorMessage?: string
  toolCalls?: ToolCall[]
  // 思考内容（扩展思维模型的思考过程）
  thinking?: string
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
  status: string
  tokens: number | null
  errorMessage?: string | null
  error_message?: string | null
  toolCalls?: RustToolCall[] | null
  tool_calls?: RustToolCall[] | null
  thinking?: string | null
  createdAt?: string
  created_at?: string
}

interface PaginatedRustMessages {
  messages: RustMessage[]
  total: number
  has_more: boolean
}

interface CreateMessageInput {
  session_id: string
  role: string
  content: string
  status?: string
  tokens?: number
  error_message?: string
  tool_calls?: string // JSON string
  thinking?: string
}

interface UpdateMessageInput {
  content?: string
  status?: string
  tokens?: number
  error_message?: string
  tool_calls?: string // JSON string
  thinking?: string
}

// 分页状态
export interface PaginationState {
  total: number
  hasMore: boolean
  isLoadingMore: boolean
  oldestMessageCreatedAt: string | null
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

  return {
    id: rustMsg.id,
    sessionId: sessionId || '',
    role: rustMsg.role as MessageRole,
    content: rustMsg.content,
    status: rustMsg.status as MessageStatus,
    tokens: rustMsg.tokens ?? undefined,
    errorMessage: errorMessage ?? undefined,
    toolCalls: toolCalls && toolCalls.length > 0 ? toolCalls : undefined,
    thinking: rustMsg.thinking ?? undefined,
    createdAt: createdAt || new Date().toISOString()
  }
}

export const useMessageStore = defineStore('message', () => {
  // State
  const messages = ref<Message[]>([])
  const isLoading = ref(false)
  const pagination = ref<Map<string, PaginationState>>(new Map())

  // 默认分页大小
  const PAGE_SIZE = 20

  // Getters
  const messagesBySession = computed(() => {
    return (sessionId: string) =>
      messages.value
        .filter(m => m.sessionId === sessionId)
        .sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime())
  })

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
  async function loadMessages(sessionId: string) {
    const notificationStore = useNotificationStore()
    isLoading.value = true
    try {
      const result = await invoke<PaginatedRustMessages>('list_messages', {
        sessionId,
        limit: PAGE_SIZE
      })

      messages.value = result.messages.map(transformMessage)

      // 更新分页状态
      const oldestMessage = result.messages[result.messages.length - 1]
      pagination.value.set(sessionId, {
        total: result.total,
        hasMore: result.has_more,
        isLoadingMore: false,
        oldestMessageCreatedAt: oldestMessage?.created_at || null
      })
    } catch (error) {
      console.error('Failed to load messages:', error)
      messages.value = []
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

      // 将新加载的消息添加到列表开头（因为它们是更早的消息）
      const newMessages = result.messages.map(transformMessage)
      messages.value = [...newMessages, ...messages.value.filter(m => m.sessionId === sessionId)]

      // 更新分页状态
      const oldestMessage = result.messages[result.messages.length - 1]
      pagination.value.set(sessionId, {
        total: result.total,
        hasMore: result.has_more,
        isLoadingMore: false,
        oldestMessageCreatedAt: oldestMessage?.created_at || currentPagination.oldestMessageCreatedAt
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
      status: message.status,
      tokens: message.tokens,
      error_message: message.errorMessage,
      tool_calls: message.toolCalls ? JSON.stringify(message.toolCalls) : undefined,
      thinking: message.thinking
    }

    try {
      const rustMsg = await invoke<RustMessage>('create_message', { input })
      const newMessage = transformMessage(rustMsg)
      messages.value.push(newMessage)
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
    const input: UpdateMessageInput = {}
    if (updates.content !== undefined) input.content = updates.content
    if (updates.status !== undefined) input.status = updates.status
    if (updates.tokens !== undefined) input.tokens = updates.tokens
    if (updates.errorMessage !== undefined) input.error_message = updates.errorMessage
    if (updates.toolCalls !== undefined) {
      input.tool_calls = JSON.stringify(updates.toolCalls)
      console.log('[MessageStore] 更新工具调用:', {
        messageId: id,
        toolCallsCount: updates.toolCalls.length,
        toolCalls: updates.toolCalls
      })
    }
    if (updates.thinking !== undefined) input.thinking = updates.thinking

    try {
      const rustMsg = await invoke<RustMessage>('update_message', { id, input })
      const updatedMessage = transformMessage(rustMsg)

      const index = messages.value.findIndex(m => m.id === id)
      if (index !== -1) {
        messages.value[index] = updatedMessage
        console.log('[MessageStore] 消息更新成功, toolCalls:', updatedMessage.toolCalls)
      } else {
        console.warn('[MessageStore] 未找到消息, id:', id)
      }
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
        messages.value.splice(index, 1)
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
      messages.value = messages.value.filter(m => m.sessionId !== sessionId)
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
    // Actions
    loadMessages,
    loadMoreMessages,
    addMessage,
    updateMessage,
    deleteMessage,
    clearSessionMessages
  }
})
