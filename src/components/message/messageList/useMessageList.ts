import { computed, nextTick, onMounted, onUnmounted, ref, watch, type ComponentPublicInstance } from 'vue'
import { useMessageStore, type Message } from '@/stores/message'
import { useSessionStore } from '@/stores/session'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { useUIStore } from '@/stores/ui'

interface VirtualMessageItem {
  message: Message
  top: number
  height: number
}

interface SessionScrollSnapshot {
  scrollTop: number
  isAtBottom: boolean
}

export interface MessageListProps {
  sessionId?: string
  hideContextStrategyNotice?: boolean
  topSafeInset?: number
}

export interface MessageListEmits {
  (event: 'retry', message: Message): void
  (event: 'formSubmit', formId: string, values: Record<string, unknown>, assistantMessageId?: string): void
  (event: 'openEditTrace', messageId: string, traceId: string): void
}

const sessionScrollSnapshots = new Map<string, SessionScrollSnapshot>()
const DEFAULT_MESSAGE_HEIGHT = 220
const VIRTUALIZE_THRESHOLD = 40
const VIRTUAL_OVERSCAN_PX = 1200
const BOTTOM_SCROLL_BUFFER = 160
const SCROLL_THRESHOLD = 100
const LOAD_MORE_THRESHOLD = 100

function detectWindowsRuntime(): boolean {
  const platform = (navigator as Navigator & {
    userAgentData?: { platform?: string }
  }).userAgentData?.platform || navigator.platform || navigator.userAgent

  return /win/i.test(platform)
}

/**
 * 会话消息列表逻辑。
 * 负责虚拟滚动、滚动位置恢复、历史消息加载以及流式消息期间的自动滚动状态。
 */
export function useMessageList(props: MessageListProps, emit: MessageListEmits) {
  const messageStore = useMessageStore()
  const sessionStore = useSessionStore()
  const sessionExecutionStore = useSessionExecutionStore()
  const uiStore = useUIStore()

  const listRef = ref<HTMLElement | null>(null)
  const messageHeights = ref<Record<string, number>>({})
  const viewportHeight = ref(0)
  const scrollTop = ref(0)
  const isUserAtBottom = ref(true)
  const previousMessageCount = ref(0)
  const savedScrollHeight = ref(0)
  const showScrollToBottom = ref(false)

  const resizeObservers = new Map<string, ResizeObserver>()
  const isWindowsRuntime = detectWindowsRuntime()
  let scrollFramePending = false

  const resolvedSessionId = computed(() => props.sessionId || sessionStore.currentSessionId)

  const currentMessages = computed(() => {
    if (!resolvedSessionId.value) {
      return []
    }

    return messageStore.messagesBySession(resolvedSessionId.value)
  })

  const currentIsSending = computed(() => {
    if (!resolvedSessionId.value) {
      return false
    }

    return sessionExecutionStore.getIsSending(resolvedSessionId.value)
  })

  const isListVisible = computed(() => {
    if (props.sessionId) {
      return true
    }

    return uiStore.appMode === 'chat'
  })

  const shouldVirtualize = computed(() => (
    !isWindowsRuntime && currentMessages.value.length > VIRTUALIZE_THRESHOLD
  ))

  const latestMessageActivity = computed(() => {
    const messages = currentMessages.value
    const latestMessage = messages[messages.length - 1]
    const latestToolCall = latestMessage?.toolCalls?.[latestMessage.toolCalls.length - 1]

    return [
      messages.length,
      latestMessage?.id ?? '',
      latestMessage?.status ?? '',
      latestMessage?.content.length ?? 0,
      latestMessage?.thinking?.length ?? 0,
      latestMessage?.thinkingActive ? 1 : 0,
      latestMessage?.editTraces?.length ?? 0,
      latestMessage?.attachments?.length ?? 0,
      latestMessage?.runtimeNotices?.length ?? 0,
      latestToolCall?.id ?? '',
      latestToolCall?.status ?? '',
      latestToolCall?.result?.length ?? 0,
      latestToolCall?.errorMessage?.length ?? 0
    ].join(':')
  })

  const currentPagination = computed(() => {
    if (!resolvedSessionId.value) {
      return null
    }

    return messageStore.getPagination(resolvedSessionId.value)
  })

  const hasMoreMessages = computed(() => currentPagination.value?.hasMore ?? false)
  const isLoadingMore = computed(() => currentPagination.value?.isLoadingMore ?? false)

  const messageLayout = computed<VirtualMessageItem[]>(() => {
    let offset = 0

    return currentMessages.value.map(message => {
      const height = messageHeights.value[message.id] ?? DEFAULT_MESSAGE_HEIGHT
      const item = { message, top: offset, height }
      offset += height
      return item
    })
  })

  const totalMessageHeight = computed(() => {
    const lastItem = messageLayout.value[messageLayout.value.length - 1]
    return lastItem ? lastItem.top + lastItem.height : 0
  })

  const virtualWindow = computed(() => {
    if (!shouldVirtualize.value) {
      return {
        start: 0,
        end: currentMessages.value.length,
        topSpacer: 0,
        bottomSpacer: 0
      }
    }

    const layouts = messageLayout.value
    const visibleTop = Math.max(0, scrollTop.value - VIRTUAL_OVERSCAN_PX)
    const visibleBottom = scrollTop.value + viewportHeight.value + VIRTUAL_OVERSCAN_PX

    let start = 0
    while (start < layouts.length && layouts[start].top + layouts[start].height < visibleTop) {
      start += 1
    }

    let end = start
    while (end < layouts.length && layouts[end].top < visibleBottom) {
      end += 1
    }

    const safeEnd = Math.max(end, start + 1)
    const lastVisible = layouts[safeEnd - 1]

    return {
      start,
      end: safeEnd,
      topSpacer: layouts[start]?.top ?? 0,
      bottomSpacer: Math.max(
        0,
        totalMessageHeight.value - ((lastVisible?.top ?? 0) + (lastVisible?.height ?? 0))
      )
    }
  })

  const visibleMessages = computed(() => {
    if (!shouldVirtualize.value) {
      return messageLayout.value
    }

    return messageLayout.value.slice(virtualWindow.value.start, virtualWindow.value.end)
  })

  const listStyle = computed(() => ({
    '--message-list-top-safe-space': `${Math.max(0, props.topSafeInset ?? 0)}px`
  }))

  function saveScrollSnapshot(sessionId?: string | null) {
    if (!sessionId || !listRef.value) {
      return
    }

    const { scrollTop: top, scrollHeight, clientHeight } = listRef.value
    sessionScrollSnapshots.set(sessionId, {
      scrollTop: top,
      isAtBottom: scrollHeight - top - clientHeight < SCROLL_THRESHOLD
    })
  }

  async function restoreScrollSnapshot(sessionId?: string | null) {
    if (!sessionId || !listRef.value) {
      return
    }

    const savedSnapshot = sessionScrollSnapshots.get(sessionId)
    if (!savedSnapshot) {
      showScrollToBottom.value = false
      scrollToBottom(false)
      return
    }

    await nextTick()
    if (!listRef.value) {
      return
    }

    if (savedSnapshot.isAtBottom) {
      isUserAtBottom.value = true
      showScrollToBottom.value = false
      scrollToBottom(false)
      updateViewportMetrics()
      return
    }

    const maxScrollTop = Math.max(0, listRef.value.scrollHeight - listRef.value.clientHeight)
    listRef.value.scrollTop = Math.min(savedSnapshot.scrollTop, maxScrollTop)
    updateViewportMetrics()
    isUserAtBottom.value = checkIsAtBottom()
    showScrollToBottom.value = !isUserAtBottom.value
  }

  function checkIsAtBottom() {
    if (!listRef.value) {
      return true
    }

    const { scrollTop: top, scrollHeight, clientHeight } = listRef.value
    return scrollHeight - top - clientHeight < SCROLL_THRESHOLD
  }

  function checkIsAtTop() {
    return Boolean(listRef.value && listRef.value.scrollTop < LOAD_MORE_THRESHOLD)
  }

  function scrollToBottom(smooth = true) {
    if (!listRef.value) {
      return
    }

    if (smooth && 'scrollBehavior' in document.documentElement.style) {
      listRef.value.scrollTo({
        top: listRef.value.scrollHeight + BOTTOM_SCROLL_BUFFER,
        behavior: 'smooth'
      })
      return
    }

    listRef.value.scrollTop = listRef.value.scrollHeight + BOTTOM_SCROLL_BUFFER
  }

  function updateViewportMetrics() {
    viewportHeight.value = listRef.value?.clientHeight ?? 0
    scrollTop.value = listRef.value?.scrollTop ?? 0
  }

  function handleScroll() {
    if (scrollFramePending) {
      return
    }

    scrollFramePending = true
    requestAnimationFrame(() => {
      scrollFramePending = false
      scrollTop.value = listRef.value?.scrollTop ?? 0
      isUserAtBottom.value = checkIsAtBottom()
      showScrollToBottom.value = !isUserAtBottom.value
      saveScrollSnapshot(resolvedSessionId.value)

      if (checkIsAtTop() && hasMoreMessages.value && !isLoadingMore.value && resolvedSessionId.value) {
        savedScrollHeight.value = listRef.value?.scrollHeight ?? 0
        void messageStore.loadMoreMessages(resolvedSessionId.value)
      }
    })
  }

  function handleScrollToBottom() {
    scrollToBottom()
    isUserAtBottom.value = true
    showScrollToBottom.value = false
  }

  function bindMessageElement(messageId: string, element: Element | ComponentPublicInstance | null) {
    const existingObserver = resizeObservers.get(messageId)

    if (!(element instanceof HTMLElement)) {
      existingObserver?.disconnect()
      resizeObservers.delete(messageId)
      return
    }

    existingObserver?.disconnect()

    const observer = new ResizeObserver((entries) => {
      const nextHeight = Math.ceil(entries[0]?.contentRect.height ?? 0)
      if (nextHeight > 0 && messageHeights.value[messageId] !== nextHeight) {
        messageHeights.value = {
          ...messageHeights.value,
          [messageId]: nextHeight
        }
      }
    })

    observer.observe(element)
    resizeObservers.set(messageId, observer)
  }

  function handleRetry(message: Message) {
    emit('retry', message)
  }

  function handleFormSubmit(
    formId: string,
    values: Record<string, unknown>,
    assistantMessageId?: string
  ) {
    emit('formSubmit', formId, values, assistantMessageId)
  }

  function handleOpenEditTrace(messageId: string, traceId: string) {
    emit('openEditTrace', messageId, traceId)
  }

  watch(resolvedSessionId, async (sessionId, previousSessionId) => {
    saveScrollSnapshot(previousSessionId)
    if (!sessionId) {
      return
    }

    const savedSnapshot = sessionScrollSnapshots.get(sessionId)
    isUserAtBottom.value = savedSnapshot?.isAtBottom ?? true
    previousMessageCount.value = 0
    messageHeights.value = {}
    await messageStore.loadMessages(sessionId)

    if (!isListVisible.value) {
      return
    }

    await restoreScrollSnapshot(sessionId)
  }, { immediate: true })

  watch(isListVisible, async (visible, wasVisible) => {
    if (!resolvedSessionId.value) {
      return
    }

    if (!visible) {
      saveScrollSnapshot(resolvedSessionId.value)
      return
    }

    if (wasVisible === false) {
      await restoreScrollSnapshot(resolvedSessionId.value)
    }
  })

  watch(latestMessageActivity, async () => {
    const messages = currentMessages.value
    const currentCount = messages.length
    const hasNewMessage = currentCount > previousMessageCount.value
    const latestMessage = messages[messages.length - 1]
    const hasStreamingMessage = latestMessage?.status === 'streaming'

    if (isLoadingMore.value && savedScrollHeight.value > 0 && listRef.value) {
      await nextTick()
      listRef.value.scrollTop = listRef.value.scrollHeight - savedScrollHeight.value
      savedScrollHeight.value = 0
      previousMessageCount.value = currentCount
      updateViewportMetrics()
      return
    }

    previousMessageCount.value = currentCount
    if (!isListVisible.value) {
      return
    }

    await nextTick()
    updateViewportMetrics()

    if (latestMessage?.role === 'user' && hasNewMessage) {
      isUserAtBottom.value = true
      showScrollToBottom.value = false
      scrollToBottom(false)
      return
    }

    if (isUserAtBottom.value && (hasNewMessage || hasStreamingMessage)) {
      scrollToBottom(hasNewMessage)
    }
  })

  watch(currentIsSending, async (sending, wasSending) => {
    if (!sending || wasSending || !isListVisible.value) {
      return
    }

    isUserAtBottom.value = true
    showScrollToBottom.value = false
    await nextTick()
    updateViewportMetrics()
    scrollToBottom(false)
  })

  watch(currentMessages, (messages) => {
    const activeIds = new Set(messages.map(message => message.id))
    const nextHeights: Record<string, number> = {}

    for (const [messageId, height] of Object.entries(messageHeights.value)) {
      if (activeIds.has(messageId)) {
        nextHeights[messageId] = height
        continue
      }

      resizeObservers.get(messageId)?.disconnect()
      resizeObservers.delete(messageId)
    }

    messageHeights.value = nextHeights
  })

  watch(totalMessageHeight, async (currentHeight, previousHeight) => {
    if (currentHeight === previousHeight || !isListVisible.value) {
      return
    }

    await nextTick()
    updateViewportMetrics()
    if (isUserAtBottom.value) {
      scrollToBottom(false)
    }
  })

  onMounted(() => {
    listRef.value?.addEventListener('scroll', handleScroll, { passive: true })
    window.addEventListener('resize', updateViewportMetrics, { passive: true })
    updateViewportMetrics()
  })

  onUnmounted(() => {
    saveScrollSnapshot(resolvedSessionId.value)
    listRef.value?.removeEventListener('scroll', handleScroll)
    window.removeEventListener('resize', updateViewportMetrics)
    resizeObservers.forEach(observer => observer.disconnect())
    resizeObservers.clear()
  })

  return {
    listRef,
    currentMessages,
    resolvedSessionId,
    hasMoreMessages,
    isLoadingMore,
    shouldVirtualize,
    totalMessageHeight,
    virtualWindow,
    visibleMessages,
    showScrollToBottom,
    listStyle,
    bindMessageElement,
    handleRetry,
    handleFormSubmit,
    handleOpenEditTrace,
    handleScrollToBottom
  }
}
