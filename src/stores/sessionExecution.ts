import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { MessageAttachment } from './message'
import type { MemorySuggestion, MemorySuggestionSourceType, SearchMemorySuggestionsResult } from '@/types/memory'

export interface PendingImageAttachment extends MessageAttachment {
  previewUrl: string
}

export interface ComposerFileMention {
  id: string
  displayText: string
  fullPath: string
  titleText: string
  insertText?: string
}

export interface ComposerMemoryReference {
  sourceType: MemorySuggestionSourceType
  sourceId: string
  title: string
  fullContent: string
  snippet: string
  libraryId?: string
  libraryName?: string
  sessionId?: string
  sessionName?: string
  projectId?: string
  projectName?: string
  createdAt?: string
}

export interface QueuedMessageDraft {
  id: string
  content: string
  displayContent?: string
  attachments: MessageAttachment[]
  expertId?: string
  agentId: string
  modelId?: string
  memoryReferences?: ComposerMemoryReference[]
  createdAt: string
  status: 'queued' | 'failed'
  errorMessage?: string
}

export interface SessionRetryState {
  assistantMessageId: string
  userMessageId: string
  current: number
  max: number
}

/**
 * 单个会话的执行状态
 */
export interface SessionExecutionState {
  /** 输入框内容 */
  inputText: string
  /** 输入框中的文件引用映射 */
  fileMentions: ComposerFileMention[]
  /** 当前草稿中的记忆引用 */
  memoryReferences: ComposerMemoryReference[]
  /** 当前草稿已忽略的建议 */
  dismissedMemorySuggestionKeys: string[]
  /** 最近一次触发检索的草稿内容 */
  lastMemoryQuery: string
  /** 最近一次记忆建议结果 */
  memorySuggestions: SearchMemorySuggestionsResult
  /** 是否正在检索记忆 */
  isSearchingMemory: boolean
  /** 待发送图片 */
  pendingImages: PendingImageAttachment[]
  /** 是否正在上传图片 */
  isUploadingImages: boolean
  /** 是否正在发送消息 */
  isSending: boolean
  /** 是否正在从待发送队列接力发起下一条消息 */
  isQueueDraining: boolean
  /** 是否正在等待自动重试 */
  isAwaitingRetry: boolean
  /** 是否正在流式输出 */
  isStreaming: boolean
  /** 流式输出定时器 ID */
  streamTimerId: ReturnType<typeof setInterval> | null
  /** 当前流式消息 ID */
  currentStreamingMessageId: string | null
  /** 当前正在展示的重试状态 */
  currentRetryState: SessionRetryState | null
  /** 同一条用户消息累计重试次数 */
  retryCountsByUserMessageId: Record<string, number>
  /** 待发送消息队列 */
  queuedMessages: QueuedMessageDraft[]
}

interface ComposerStateSnapshot {
  inputText: string
  fileMentions: ComposerFileMention[]
  memoryReferences: ComposerMemoryReference[]
  dismissedMemorySuggestionKeys: string[]
  lastMemoryQuery: string
  memorySuggestions: SearchMemorySuggestionsResult
  pendingImages: PendingImageAttachment[]
}

/**
 * 会话执行状态管理 Store
 *
 * 用于管理每个会话独立的执行状态，确保：
 * - 每个会话有独立的输入框内容
 * - 每个会话有独立的发送/流式输出状态
 * - 会话切换时状态保持独立
 * - 关闭会话时清理对应状态
 */
export const useSessionExecutionStore = defineStore('sessionExecution', () => {
  // State - 使用 Map 存储每个会话的执行状态
  const executionStates = ref<Map<string, SessionExecutionState>>(new Map())

  /**
   * 获取指定会话的执行状态，如果不存在则创建默认状态
   */
  const getExecutionState = (sessionId: string): SessionExecutionState => {
    let state = executionStates.value.get(sessionId)
    if (!state) {
      state = createDefaultState()
      executionStates.value.set(sessionId, state)
    }
    return state
  }

  /**
   * 获取当前输入框内容（计算属性）
   */
  const getInputText = computed(() => {
    return (sessionId: string) => {
      return getExecutionState(sessionId).inputText
    }
  })

  /**
   * 获取当前发送状态（计算属性）
   */
  const getIsSending = computed(() => {
    return (sessionId: string) => {
      return getExecutionState(sessionId).isSending
    }
  })

  const getIsQueueDraining = computed(() => {
    return (sessionId: string) => {
      return getExecutionState(sessionId).isQueueDraining
    }
  })

  const getIsAwaitingRetry = computed(() => {
    return (sessionId: string) => {
      return getExecutionState(sessionId).isAwaitingRetry
    }
  })

  const getIsBusy = computed(() => {
    return (sessionId: string) => {
      const state = getExecutionState(sessionId)
      return state.isSending || state.isQueueDraining || state.isAwaitingRetry
    }
  })

  const getPendingImages = computed(() => {
    return (sessionId: string) => {
      return getExecutionState(sessionId).pendingImages
    }
  })

  const getQueuedMessages = computed(() => {
    return (sessionId: string) => {
      return getExecutionState(sessionId).queuedMessages
    }
  })

  const getIsUploadingImages = computed(() => {
    return (sessionId: string) => {
      return getExecutionState(sessionId).isUploadingImages
    }
  })

  /**
   * 获取当前流式输出状态（计算属性）
   */
  const getIsStreaming = computed(() => {
    return (sessionId: string) => {
      return getExecutionState(sessionId).isStreaming
    }
  })

  const getCurrentRetryState = computed(() => {
    return (sessionId: string) => {
      return getExecutionState(sessionId).currentRetryState
    }
  })

  /**
   * 创建默认的执行状态
   */
  function createDefaultState(): SessionExecutionState {
    return {
      inputText: '',
      fileMentions: [],
      memoryReferences: [],
      dismissedMemorySuggestionKeys: [],
      lastMemoryQuery: '',
      memorySuggestions: {
        librarySuggestions: [],
        rawSuggestions: []
      },
      isSearchingMemory: false,
      pendingImages: [],
      isUploadingImages: false,
      isSending: false,
      isQueueDraining: false,
      isAwaitingRetry: false,
      isStreaming: false,
      streamTimerId: null,
      currentStreamingMessageId: null,
      currentRetryState: null,
      retryCountsByUserMessageId: {},
      queuedMessages: []
    }
  }

  /**
   * 更新输入框内容
   */
  function setInputText(sessionId: string, text: string) {
    const state = getExecutionState(sessionId)
    state.inputText = text
  }

  function getFileMentions(sessionId: string) {
    return getExecutionState(sessionId).fileMentions
  }

  function setFileMentions(sessionId: string, mentions: ComposerFileMention[]) {
    const state = getExecutionState(sessionId)
    state.fileMentions = mentions
  }

  function getMemoryReferences(sessionId: string) {
    return getExecutionState(sessionId).memoryReferences
  }

  function dedupeMemoryReferences(references: ComposerMemoryReference[]) {
    const seen = new Set<string>()
    return references.filter(reference => {
      const key = `${reference.sourceType}:${reference.sourceId}`
      if (seen.has(key)) {
        return false
      }
      seen.add(key)
      return true
    })
  }

  function setMemoryReferences(sessionId: string, references: ComposerMemoryReference[]) {
    const state = getExecutionState(sessionId)
    state.memoryReferences = dedupeMemoryReferences(references)
  }

  function appendMemoryReference(sessionId: string, reference: ComposerMemoryReference) {
    const state = getExecutionState(sessionId)
    if (state.memoryReferences.some(item => item.sourceType === reference.sourceType && item.sourceId === reference.sourceId)) {
      return
    }
    state.memoryReferences = [...state.memoryReferences, reference]
  }

  function removeMemoryReference(sessionId: string, sourceType: MemorySuggestionSourceType, sourceId: string) {
    const state = getExecutionState(sessionId)
    state.memoryReferences = state.memoryReferences.filter(reference =>
      !(reference.sourceType === sourceType && reference.sourceId === sourceId)
    )
  }

  function clearMemoryReferences(sessionId: string) {
    const state = getExecutionState(sessionId)
    state.memoryReferences = []
  }

  function getMemorySuggestions(sessionId: string) {
    return getExecutionState(sessionId).memorySuggestions
  }

  function getLastMemoryQuery(sessionId: string) {
    return getExecutionState(sessionId).lastMemoryQuery
  }

  function setMemorySuggestions(sessionId: string, suggestions: SearchMemorySuggestionsResult, query: string) {
    const state = getExecutionState(sessionId)
    state.memorySuggestions = suggestions
    state.lastMemoryQuery = query
  }

  function clearMemorySuggestions(sessionId: string) {
    const state = getExecutionState(sessionId)
    state.memorySuggestions = {
      librarySuggestions: [],
      rawSuggestions: []
    }
    state.lastMemoryQuery = ''
  }

  function setIsSearchingMemory(sessionId: string, searching: boolean) {
    const state = getExecutionState(sessionId)
    state.isSearchingMemory = searching
  }

  function getIsSearchingMemory(sessionId: string) {
    return getExecutionState(sessionId).isSearchingMemory
  }

  function dismissMemorySuggestion(sessionId: string, suggestion: Pick<MemorySuggestion, 'sourceType' | 'sourceId'>) {
    const state = getExecutionState(sessionId)
    const key = `${suggestion.sourceType}:${suggestion.sourceId}`
    if (state.dismissedMemorySuggestionKeys.includes(key)) {
      return
    }
    state.dismissedMemorySuggestionKeys = [...state.dismissedMemorySuggestionKeys, key]
  }

  function getDismissedMemorySuggestionKeys(sessionId: string) {
    return getExecutionState(sessionId).dismissedMemorySuggestionKeys
  }

  function clearDismissedMemorySuggestionKeys(sessionId: string) {
    const state = getExecutionState(sessionId)
    state.dismissedMemorySuggestionKeys = []
  }

  function setPendingImages(sessionId: string, images: PendingImageAttachment[]) {
    const state = getExecutionState(sessionId)
    state.pendingImages = images
  }

  function appendPendingImages(sessionId: string, images: PendingImageAttachment[]) {
    const state = getExecutionState(sessionId)
    state.pendingImages = [...state.pendingImages, ...images]
  }

  function removePendingImage(sessionId: string, imageId: string) {
    const state = getExecutionState(sessionId)
    state.pendingImages = state.pendingImages.filter(image => image.id !== imageId)
  }

  function clearPendingImages(sessionId: string) {
    const state = getExecutionState(sessionId)
    state.pendingImages = []
  }

  /**
   * 复制会话编辑器草稿态到另一个会话。
   * 用于压缩后切换新会话时，保留用户当前未发送的输入上下文。
   */
  function copyComposerState(sourceSessionId: string, targetSessionId: string) {
    if (!sourceSessionId || !targetSessionId || sourceSessionId === targetSessionId) {
      return
    }

    const sourceState = getExecutionState(sourceSessionId)
    const targetState = getExecutionState(targetSessionId)
    const snapshot: ComposerStateSnapshot = {
      inputText: sourceState.inputText,
      fileMentions: [...sourceState.fileMentions],
      memoryReferences: dedupeMemoryReferences(sourceState.memoryReferences),
      dismissedMemorySuggestionKeys: [...sourceState.dismissedMemorySuggestionKeys],
      lastMemoryQuery: sourceState.lastMemoryQuery,
      memorySuggestions: {
        librarySuggestions: [...sourceState.memorySuggestions.librarySuggestions],
        rawSuggestions: [...sourceState.memorySuggestions.rawSuggestions]
      },
      pendingImages: [...sourceState.pendingImages]
    }

    targetState.inputText = snapshot.inputText
    targetState.fileMentions = snapshot.fileMentions
    targetState.memoryReferences = snapshot.memoryReferences
    targetState.dismissedMemorySuggestionKeys = snapshot.dismissedMemorySuggestionKeys
    targetState.lastMemoryQuery = snapshot.lastMemoryQuery
    targetState.memorySuggestions = snapshot.memorySuggestions
    targetState.pendingImages = snapshot.pendingImages
  }

  function queueMessage(
    sessionId: string,
    draft: Omit<QueuedMessageDraft, 'id' | 'createdAt' | 'status'>
  ): QueuedMessageDraft {
    const state = getExecutionState(sessionId)
    const queuedDraft: QueuedMessageDraft = {
      ...draft,
      id: `queued-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      createdAt: new Date().toISOString(),
      status: 'queued'
    }
    state.queuedMessages = [...state.queuedMessages, queuedDraft]
    return queuedDraft
  }

  function removeQueuedMessage(sessionId: string, draftId: string) {
    const state = getExecutionState(sessionId)
    state.queuedMessages = state.queuedMessages.filter(draft => draft.id !== draftId)
  }

  function restoreQueuedMessage(sessionId: string, draft: QueuedMessageDraft) {
    const state = getExecutionState(sessionId)
    state.queuedMessages = [draft, ...state.queuedMessages]
  }

  function updateQueuedMessage(
    sessionId: string,
    draftId: string,
    updates: Partial<Pick<QueuedMessageDraft, 'content' | 'displayContent' | 'attachments' | 'expertId' | 'agentId' | 'modelId' | 'memoryReferences' | 'status' | 'errorMessage'>>
  ) {
    const state = getExecutionState(sessionId)
    state.queuedMessages = state.queuedMessages.map(draft => {
      if (draft.id !== draftId) {
        return draft
      }

      return {
        ...draft,
        ...updates
      }
    })
  }

  function popNextQueuedMessage(sessionId: string): QueuedMessageDraft | null {
    const state = getExecutionState(sessionId)
    const index = state.queuedMessages.findIndex(draft => draft.status === 'queued')
    if (index < 0) {
      return null
    }

    const [draft] = state.queuedMessages.splice(index, 1)
    return draft ?? null
  }

  function markQueuedMessageStatus(
    sessionId: string,
    draftId: string,
    status: QueuedMessageDraft['status'],
    errorMessage?: string
  ) {
    const state = getExecutionState(sessionId)
    state.queuedMessages = state.queuedMessages.map(draft => {
      if (draft.id !== draftId) {
        return draft
      }

      return {
        ...draft,
        status,
        errorMessage
      }
    })
  }

  function retryQueuedMessage(sessionId: string, draftId: string) {
    const state = getExecutionState(sessionId)
    state.queuedMessages = state.queuedMessages.map(draft => {
      if (draft.id !== draftId) {
        return draft
      }

      return {
        ...draft,
        status: 'queued',
        errorMessage: undefined
      }
    })
  }

  function setIsUploadingImages(sessionId: string, uploading: boolean) {
    const state = getExecutionState(sessionId)
    state.isUploadingImages = uploading
  }

  /**
   * 设置发送状态
   */
  function setIsSending(sessionId: string, sending: boolean) {
    const state = getExecutionState(sessionId)
    state.isSending = sending
  }

  function setIsQueueDraining(sessionId: string, draining: boolean) {
    const state = getExecutionState(sessionId)
    state.isQueueDraining = draining
  }

  function setIsAwaitingRetry(sessionId: string, awaiting: boolean) {
    const state = getExecutionState(sessionId)
    state.isAwaitingRetry = awaiting
  }

  /**
   * 设置流式输出状态
   */
  function setIsStreaming(sessionId: string, streaming: boolean) {
    const state = getExecutionState(sessionId)
    state.isStreaming = streaming
  }

  /**
   * 设置流式输出定时器 ID
   */
  function setStreamTimerId(sessionId: string, timerId: ReturnType<typeof setInterval> | null) {
    const state = getExecutionState(sessionId)
    state.streamTimerId = timerId
  }

  /**
   * 设置当前流式消息 ID
   */
  function setCurrentStreamingMessageId(sessionId: string, messageId: string | null) {
    const state = getExecutionState(sessionId)
    state.currentStreamingMessageId = messageId
  }

  function beginRetryAttempt(
    sessionId: string,
    payload: {
      assistantMessageId: string
      userMessageId: string
      max: number
    }
  ): SessionRetryState {
    const state = getExecutionState(sessionId)
    const nextCurrent = (state.retryCountsByUserMessageId[payload.userMessageId] ?? 0) + 1
    state.retryCountsByUserMessageId = {
      ...state.retryCountsByUserMessageId,
      [payload.userMessageId]: nextCurrent
    }
    state.currentRetryState = {
      assistantMessageId: payload.assistantMessageId,
      userMessageId: payload.userMessageId,
      current: nextCurrent,
      max: payload.max
    }
    return state.currentRetryState
  }

  function clearCurrentRetryState(sessionId: string) {
    const state = getExecutionState(sessionId)
    state.currentRetryState = null
  }

  /**
   * 开始发送消息 - 设置相关状态
   */
  function startSending(sessionId: string) {
    const state = getExecutionState(sessionId)
    state.isSending = true
    state.isQueueDraining = false
    state.isAwaitingRetry = false
    state.isStreaming = true
  }

  /**
   * 结束发送消息 - 清除相关状态
   */
  function endSending(sessionId: string) {
    const state = getExecutionState(sessionId)
    state.isSending = false
    state.isQueueDraining = false
    state.isUploadingImages = false
    state.isStreaming = false
    state.streamTimerId = null
    state.currentStreamingMessageId = null
  }

  /**
   * 停止流式输出
   */
  function stopStreaming(sessionId: string) {
    const state = getExecutionState(sessionId)

    // 清除定时器
    if (state.streamTimerId) {
      clearInterval(state.streamTimerId)
      state.streamTimerId = null
    }

    // 重置状态
    state.isSending = false
    state.isQueueDraining = false
    state.isStreaming = false
    state.currentStreamingMessageId = null
  }

  /**
   * 清除指定会话的执行状态
   * 在关闭会话时调用
   */
  function clearExecutionState(sessionId: string) {
    const state = executionStates.value.get(sessionId)
    if (state) {
      // 清除可能存在的定时器
      if (state.streamTimerId) {
        clearInterval(state.streamTimerId)
      }
      // 删除状态
      executionStates.value.delete(sessionId)
    }
  }

  /**
   * 清除所有会话的执行状态
   */
  function clearAllExecutionStates() {
    // 清除所有定时器
    executionStates.value.forEach((state) => {
      if (state.streamTimerId) {
        clearInterval(state.streamTimerId)
      }
    })
    executionStates.value.clear()
  }

  /**
   * 检查是否有会话正在执行
   */
  const hasAnyRunningSession = computed(() => {
    for (const state of executionStates.value.values()) {
      if (state.isSending || state.isStreaming || state.isAwaitingRetry) {
        return true
      }
    }
    return false
  })

  /**
   * 获取所有正在执行的会话 ID
   */
  const runningSessionIds = computed(() => {
    const ids: string[] = []
    executionStates.value.forEach((state, sessionId) => {
      if (state.isSending || state.isStreaming || state.isAwaitingRetry) {
        ids.push(sessionId)
      }
    })
    return ids
  })

  return {
    // State
    executionStates,

    // Getters
    getInputText,
    getFileMentions,
    getMemoryReferences,
    getMemorySuggestions,
    getLastMemoryQuery,
    getIsSearchingMemory,
    getDismissedMemorySuggestionKeys,
    getPendingImages,
    getQueuedMessages,
    getIsUploadingImages,
    getIsSending,
    getIsQueueDraining,
    getIsAwaitingRetry,
    getIsBusy,
    getIsStreaming,
    getCurrentRetryState,
    hasAnyRunningSession,
    runningSessionIds,

    // Actions
    getExecutionState,
    setInputText,
    setFileMentions,
    setMemoryReferences,
    appendMemoryReference,
    removeMemoryReference,
    clearMemoryReferences,
    setMemorySuggestions,
    clearMemorySuggestions,
    setIsSearchingMemory,
    dismissMemorySuggestion,
    clearDismissedMemorySuggestionKeys,
    setPendingImages,
    appendPendingImages,
    removePendingImage,
    clearPendingImages,
    copyComposerState,
    queueMessage,
    removeQueuedMessage,
    restoreQueuedMessage,
    updateQueuedMessage,
    popNextQueuedMessage,
    markQueuedMessageStatus,
    retryQueuedMessage,
    setIsUploadingImages,
    setIsSending,
    setIsQueueDraining,
    setIsAwaitingRetry,
    setIsStreaming,
    setStreamTimerId,
    setCurrentStreamingMessageId,
    beginRetryAttempt,
    clearCurrentRetryState,
    startSending,
    endSending,
    stopStreaming,
    clearExecutionState,
    clearAllExecutionStates
  }
})
