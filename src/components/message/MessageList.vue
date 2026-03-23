<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessageStore } from '@/stores/message'
import { useSessionStore } from '@/stores/session'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { EaIcon } from '@/components/common'
import MessageBubble from './MessageBubble.vue'
import type { Message } from '@/stores/message'

interface VirtualMessageItem {
  message: Message
  top: number
  height: number
}

const { t } = useI18n()
const messageStore = useMessageStore()
const sessionStore = useSessionStore()
const sessionExecutionStore = useSessionExecutionStore()

const props = defineProps<{
  sessionId?: string
}>()

const emit = defineEmits<{
  retry: [message: Message]
  formSubmit: [formId: string, values: Record<string, unknown>]
  openEditTrace: [messageId: string, traceId: string]
}>()

const listRef = ref<HTMLElement | null>(null)
let scrollFramePending = false
const messageHeights = ref<Record<string, number>>({})
const viewportHeight = ref(0)
const scrollTop = ref(0)
const resizeObservers = new Map<string, ResizeObserver>()

const DEFAULT_MESSAGE_HEIGHT = 220
const VIRTUALIZE_THRESHOLD = 40
const VIRTUAL_OVERSCAN_PX = 1200
const BOTTOM_SCROLL_BUFFER = 160

const isUserAtBottom = ref(true)
// 距离底部的阈值（像素），小于此值视为在底部
const SCROLL_THRESHOLD = 100
const LOAD_MORE_THRESHOLD = 100
const previousMessageCount = ref(0)
// 加载更多时保存滚动位置
const savedScrollHeight = ref(0)
// 是否显示回到底部按钮
const showScrollToBottom = ref(false)

const currentMessages = computed(() => {
  const targetSessionId = props.sessionId || sessionStore.currentSessionId
  if (!targetSessionId) return []
  return messageStore.messagesBySession(targetSessionId)
})

const currentIsSending = computed(() => {
  const targetSessionId = props.sessionId || sessionStore.currentSessionId
  if (!targetSessionId) return false
  return sessionExecutionStore.getIsSending(targetSessionId)
})

const shouldVirtualize = computed(() => currentMessages.value.length > VIRTUALIZE_THRESHOLD)

const latestMessageActivity = computed(() => {
  const messages = currentMessages.value
  const latestMessage = messages[messages.length - 1]
  const latestToolCalls = latestMessage?.toolCalls
  const latestToolCall = latestToolCalls ? latestToolCalls[latestToolCalls.length - 1] : undefined

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
  const targetSessionId = props.sessionId || sessionStore.currentSessionId
  if (!targetSessionId) return null
  return messageStore.getPagination(targetSessionId)
})

const hasMoreMessages = computed(() => currentPagination.value?.hasMore ?? false)

// 是否正在加载更多
const isLoadingMore = computed(() => currentPagination.value?.isLoadingMore ?? false)

const messageLayout = computed<VirtualMessageItem[]>(() => {
  let offset = 0

  return currentMessages.value.map(message => {
    const height = messageHeights.value[message.id] ?? DEFAULT_MESSAGE_HEIGHT
    const item = {
      message,
      top: offset,
      height
    }
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
    bottomSpacer: Math.max(0, totalMessageHeight.value - ((lastVisible?.top ?? 0) + (lastVisible?.height ?? 0)))
  }
})

const visibleMessages = computed(() => {
  if (!shouldVirtualize.value) {
    return messageLayout.value
  }

  return messageLayout.value.slice(virtualWindow.value.start, virtualWindow.value.end)
})

const checkIsAtBottom = () => {
  if (!listRef.value) return true
  const { scrollTop, scrollHeight, clientHeight } = listRef.value
  // 距离底部小于阈值则视为在底部
  return scrollHeight - scrollTop - clientHeight < SCROLL_THRESHOLD
}

const checkIsAtTop = () => {
  if (!listRef.value) return false
  return listRef.value.scrollTop < LOAD_MORE_THRESHOLD
}

const handleScroll = () => {
  if (scrollFramePending) {
    return
  }

  scrollFramePending = true
  requestAnimationFrame(() => {
    scrollFramePending = false
    scrollTop.value = listRef.value?.scrollTop ?? 0
    isUserAtBottom.value = checkIsAtBottom()
    showScrollToBottom.value = !isUserAtBottom.value

    const targetSessionId = props.sessionId || sessionStore.currentSessionId
    if (checkIsAtTop() && hasMoreMessages.value && !isLoadingMore.value && targetSessionId) {
      savedScrollHeight.value = listRef.value?.scrollHeight ?? 0
      void messageStore.loadMoreMessages(targetSessionId)
    }
  })
}

const scrollToBottom = (smooth = true) => {
  if (!listRef.value) return

  if (smooth && 'scrollBehavior' in document.documentElement.style) {
    listRef.value.scrollTo({
      top: listRef.value.scrollHeight + BOTTOM_SCROLL_BUFFER,
      behavior: 'smooth'
    })
  } else {
    listRef.value.scrollTop = listRef.value.scrollHeight + BOTTOM_SCROLL_BUFFER
  }
}

// 点击回到底部按钮
const handleScrollToBottom = () => {
  scrollToBottom()
  isUserAtBottom.value = true
  showScrollToBottom.value = false
}

const updateViewportMetrics = () => {
  viewportHeight.value = listRef.value?.clientHeight ?? 0
  scrollTop.value = listRef.value?.scrollTop ?? 0
}

const bindMessageElement = (messageId: string, element: Element | null) => {
  const existingObserver = resizeObservers.get(messageId)

  if (!element || !(element instanceof HTMLElement)) {
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

watch(() => props.sessionId || sessionStore.currentSessionId, async (sessionId) => {
  if (sessionId) {
    isUserAtBottom.value = true
    previousMessageCount.value = 0
    messageHeights.value = {}
    await messageStore.loadMessages(sessionId)
    await nextTick()
    updateViewportMetrics()
    scrollToBottom(false)
  }
}, { immediate: true })

watch(latestMessageActivity, async () => {
  const messages = currentMessages.value
  const currentCount = messages.length
  const hasNewMessage = currentCount > previousMessageCount.value
  const latestMessage = messages[messages.length - 1]
  const hasStreamingMessage = latestMessage?.status === 'streaming'

  if (isLoadingMore.value && savedScrollHeight.value > 0 && listRef.value) {
    await nextTick()
    const newScrollHeight = listRef.value.scrollHeight
    listRef.value.scrollTop = newScrollHeight - savedScrollHeight.value
    savedScrollHeight.value = 0
    previousMessageCount.value = currentCount
    updateViewportMetrics()
    return
  }

  previousMessageCount.value = currentCount

  await nextTick()
  updateViewportMetrics()

  if (isUserAtBottom.value && (hasNewMessage || hasStreamingMessage)) {
    scrollToBottom(hasNewMessage)
  }
})

watch(currentIsSending, async (sending, wasSending) => {
  if (!sending || wasSending) {
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
  if (currentHeight === previousHeight) {
    return
  }

  await nextTick()
  updateViewportMetrics()

  if (isUserAtBottom.value) {
    scrollToBottom(false)
  }
})

onMounted(() => {
  if (listRef.value) {
    listRef.value.addEventListener('scroll', handleScroll, { passive: true })
  }
  window.addEventListener('resize', updateViewportMetrics, { passive: true })
  updateViewportMetrics()
})

onUnmounted(() => {
  if (listRef.value) {
    listRef.value.removeEventListener('scroll', handleScroll)
  }
  window.removeEventListener('resize', updateViewportMetrics)
  resizeObservers.forEach(observer => observer.disconnect())
  resizeObservers.clear()
})

const handleRetry = (message: Message) => {
  emit('retry', message)
}

const handleFormSubmit = (formId: string, values: Record<string, unknown>) => {
  emit('formSubmit', formId, values)
}

const handleOpenEditTrace = (messageId: string, traceId: string) => {
  emit('openEditTrace', messageId, traceId)
}
</script>

<template>
  <div
    ref="listRef"
    class="message-list"
  >
    <!-- 加载更多历史消息提示 -->
    <div
      v-if="hasMoreMessages && currentMessages.length > 0"
      class="message-list__load-more"
    >
      <div
        v-if="isLoadingMore"
        class="message-list__loading"
      >
        <span class="message-list__loading-spinner" />
        <span>{{ t('message.loadingMore') }}</span>
      </div>
      <div
        v-else
        class="message-list__load-hint"
      >
        {{ t('message.scrollUpLoadMore') }}
      </div>
    </div>

    <template v-if="!shouldVirtualize">
      <TransitionGroup name="message">
        <MessageBubble
          v-for="message in currentMessages"
          :key="message.id"
          :message="message"
          :session-id="props.sessionId || sessionStore.currentSessionId || undefined"
          @retry="handleRetry"
          @form-submit="handleFormSubmit"
          @open-edit-trace="handleOpenEditTrace"
        />
      </TransitionGroup>
    </template>

    <div
      v-else
      class="message-list__virtual"
      :style="{ minHeight: `${totalMessageHeight}px` }"
    >
      <div
        class="message-list__virtual-spacer"
        :style="{ height: `${virtualWindow.topSpacer}px` }"
      />

      <div
        v-for="item in visibleMessages"
        :key="item.message.id"
        :ref="(element) => bindMessageElement(item.message.id, element as Element | null)"
        class="message-list__virtual-item"
      >
        <MessageBubble
          :message="item.message"
          :session-id="props.sessionId || sessionStore.currentSessionId || undefined"
          @retry="handleRetry"
          @form-submit="handleFormSubmit"
          @open-edit-trace="handleOpenEditTrace"
        />
      </div>

      <div
        class="message-list__virtual-spacer"
        :style="{ height: `${virtualWindow.bottomSpacer}px` }"
      />
    </div>

    <!-- 空状态 -->
    <div
      v-if="currentMessages.length === 0"
      class="message-list__empty"
    >
      <EaIcon
        name="sparkles"
        :size="48"
        class="message-list__empty-icon"
      />
      <h3 class="message-list__empty-title">
        {{ t('message.emptyWelcome') }}
      </h3>
      <p class="message-list__empty-hint">
        {{ t('message.emptyHint') }}
      </p>
      <div class="message-list__empty-tips">
        <div class="message-list__empty-tip">
          <EaIcon
            name="keyboard"
            :size="16"
          />
          <span>{{ t('message.emptyTip1') }}</span>
        </div>
        <div class="message-list__empty-tip">
          <EaIcon
            name="layout-panel-left"
            :size="16"
          />
          <span>{{ t('message.emptyTip2') }}</span>
        </div>
      </div>
    </div>

    <!-- 回到底部按钮 -->
    <Transition name="scroll-btn">
      <button
        v-if="showScrollToBottom"
        class="scroll-to-bottom-btn"
        :title="t('message.scrollToBottom')"
        @click="handleScrollToBottom"
      >
        <EaIcon
          name="arrow-down"
          :size="16"
        />
        <span class="scroll-to-bottom-btn__ring" />
      </button>
    </Transition>
  </div>
</template>

<style scoped>
.message-list {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-3);
  padding-bottom: calc(112px + env(safe-area-inset-bottom, 0px));
  scroll-padding-bottom: 112px;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  min-height: 0;
  position: relative;
}

.message-list__virtual {
  display: flex;
  flex-direction: column;
  min-height: 0;
  width: 100%;
  min-width: 0;
}

.message-list__virtual-item {
  contain: layout style paint;
  width: 100%;
  min-width: 0;
}

.message-list__virtual-spacer {
  flex-shrink: 0;
}

.message-list::-webkit-scrollbar {
  width: var(--scrollbar-size);
}

.message-list::-webkit-scrollbar-track {
  background: var(--scrollbar-track);
}

.message-list::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: var(--radius-full);
  border: 1px solid transparent;
  background-clip: padding-box;
}

.message-list::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}

.message-list__load-more {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: var(--spacing-2);
  min-height: 40px;
}

.message-list__loading {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-sm);
  color: var(--color-text-tertiary);
}

.message-list__loading-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--color-border);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

/* 回到底部按钮 */
.scroll-to-bottom-btn {
  position: sticky;
  bottom: 8px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 10;
  align-self: center;
  margin-top: auto;
  width: 28px;
  height: 28px;
  min-width: 28px;
  min-height: 28px;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: var(--color-surface-elevated);
  color: var(--color-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow:
    0 1px 3px rgba(0, 0, 0, 0.1),
    0 2px 8px rgba(0, 0, 0, 0.12);
  transition: all 0.2s ease;
  overflow: visible;
}

.scroll-to-bottom-btn:hover {
  background: var(--color-primary);
  color: white;
  transform: translateX(-50%) scale(1.08);
  box-shadow:
    0 2px 4px rgba(0, 0, 0, 0.15),
    0 4px 12px rgba(var(--color-primary-rgb, 59, 130, 246), 0.2);
}

.scroll-to-bottom-btn:active {
  transform: translateX(-50%) scale(0.95);
}

/* 按钮光环动画 */
.scroll-to-bottom-btn__ring {
  position: absolute;
  inset: -3px;
  border-radius: 50%;
  border: 1.5px solid var(--color-border-strong);
  opacity: 0;
  animation: ring-pulse 2s ease-out infinite;
}

/* 按钮动画 */
.scroll-btn-enter-active {
  transition: all 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.scroll-btn-leave-active {
  transition: all 0.15s ease;
}

.scroll-btn-enter-from,
.scroll-btn-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(8px) scale(0.8);
}

@keyframes ring-pulse {
  0% {
    transform: scale(0.95);
    opacity: 0.5;
  }
  100% {
    transform: scale(1.25);
    opacity: 0;
  }
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.message-list__load-hint {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  opacity: 0.6;
}

.message-list__empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: var(--spacing-8);
}

.message-list__empty-icon {
  color: var(--color-primary);
  margin-bottom: var(--spacing-4);
  opacity: 0.8;
}

.message-list__empty-title {
  font-size: var(--font-size-xl);
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0 0 var(--spacing-2);
}

.message-list__empty-hint {
  font-size: var(--font-size-base);
  color: var(--color-text-secondary);
  margin: 0 0 var(--spacing-6);
  max-width: 360px;
}

.message-list__empty-tips {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.message-list__empty-tip {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-sm);
  color: var(--color-text-tertiary);
}

/* 消息动画 */
.message-enter-active {
  transition: all var(--transition-normal) var(--easing-out);
}

.message-enter-from {
  opacity: 0;
  transform: translateY(10px);
}
</style>
