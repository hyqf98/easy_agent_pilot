<script setup lang="ts">
import { ref, onMounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSessionStore, type SessionStatus } from '@/stores/session'
import { useWindowManagerStore } from '@/stores/windowManager'
import { EaIcon } from '@/components/common'
import { useMessage } from 'naive-ui'
import { useSessionView } from '@/composables'

const { t } = useI18n()
const sessionStore = useSessionStore()
const windowManagerStore = useWindowManagerStore()
const message = useMessage()
const { openSessionTarget } = useSessionView()

// 标签栏容器引用
const tabsContainerRef = ref<HTMLElement | null>(null)
// 正在切换的标签 ID（用于视觉反馈）
const switchingTabId = ref<string | null>(null)

// 拖拽状态
const isDragging = ref(false)
const dragSessionId = ref<string | null>(null)

// 拖拽开始
function onDragStart(e: DragEvent, sessionId: string) {
  if (!e.dataTransfer) return

  isDragging.value = true
  dragSessionId.value = sessionId

  e.dataTransfer.effectAllowed = 'move'
  e.dataTransfer.setData('text/plain', sessionId)

  // 设置拖拽图像（可选）
  const target = e.target as HTMLElement
  if (target) {
    const rect = target.getBoundingClientRect()
    e.dataTransfer.setDragImage(target, rect.width / 2, rect.height / 2)
  }
}

// 拖拽结束
function onDragEnd() {
  isDragging.value = false
  dragSessionId.value = null
}

// 拖拽离开窗口
async function onDragLeave(e: DragEvent) {
  // 检查是否离开窗口边界
  if (e.clientX < 0 || e.clientX > window.innerWidth ||
      e.clientY < 0 || e.clientY > window.innerHeight) {
    // 离开窗口边界，分离会话到新窗口
    if (dragSessionId.value) {
      await detachSessionToNewWindow(dragSessionId.value)
    }
  }
}

// 分离会话到新窗口
async function detachSessionToNewWindow(sessionId: string) {
  // 获取会话所属的项目
  const session = sessionStore.sessions.find(s => s.id === sessionId)
  if (!session) return

  try {
    // 在新窗口中打开项目
    await windowManagerStore.openProjectInNewWindow(session.projectId)
    // 关闭当前窗口的会话
    sessionStore.closeSession(sessionId)
    message.success('会话已分离到新窗口')
  } catch (error) {
    console.error('Failed to detach session:', error)
    message.error('分离会话失败')
  }
}

// 获取会话状态的颜色
const getStatusColor = (status: SessionStatus): string => {
  switch (status) {
    case 'running':
      return 'var(--color-success)'
    case 'paused':
      return 'var(--color-warning)'
    case 'error':
      return 'var(--color-danger)'
    case 'completed':
      return 'var(--color-primary)'
    default:
      return 'var(--color-text-tertiary)'
  }
}

// 获取会话状态的图标
const getStatusIcon = (status: SessionStatus): string => {
  switch (status) {
    case 'running':
      return 'loader'
    case 'paused':
      return 'pause-circle'
    case 'error':
      return 'alert-circle'
    case 'completed':
      return 'check-circle'
    default:
      return 'circle'
  }
}

// 切换到指定会话
const switchToSession = async (sessionId: string) => {
  if (sessionId === sessionStore.currentSessionId) return

  if (sessionStore.isSessionOpen(sessionId)) {
    sessionStore.setCurrentSession(sessionId)
    return
  }

  switchingTabId.value = sessionId

  try {
    await openSessionTarget(sessionId)
  } finally {
    switchingTabId.value = null
  }
}

// 关闭指定会话标签
const closeTab = (sessionId: string, event: MouseEvent) => {
  event.stopPropagation() // 阻止触发切换会话
  sessionStore.closeSession(sessionId)
}

// 处理鼠标滚轮滚动（横向滚动）
const handleWheel = (event: WheelEvent) => {
  if (tabsContainerRef.value) {
    tabsContainerRef.value.scrollLeft += event.deltaY
  }
}

// 滚动到活动标签
const scrollToActiveTab = async () => {
  await nextTick()
  if (!tabsContainerRef.value) return

  const activeTab = tabsContainerRef.value.querySelector('.session-tabs__tab--active') as HTMLElement
  if (!activeTab) return

  const container = tabsContainerRef.value
  const containerRect = container.getBoundingClientRect()
  const tabRect = activeTab.getBoundingClientRect()

  // 如果标签在可视区域外，滚动到标签位置
  if (tabRect.left < containerRect.left) {
    container.scrollLeft -= containerRect.left - tabRect.left + 20
  } else if (tabRect.right > containerRect.right) {
    container.scrollLeft += tabRect.right - containerRect.right + 20
  }
}

// 监听当前会话变化，滚动到活动标签
onMounted(() => {
  // 加载保存的打开会话列表
  sessionStore.loadOpenSessions()
  void scrollToActiveTab()
})

watch(() => sessionStore.currentSessionId, () => {
  void scrollToActiveTab()
})
</script>

<template>
  <div
    v-if="sessionStore.openSessions.length > 0"
    class="session-tabs"
  >
    <div
      ref="tabsContainerRef"
      class="session-tabs__container"
      @wheel.prevent="handleWheel"
    >
      <div
        v-for="(session, index) in sessionStore.openSessions"
        :key="session.id"
        class="session-tabs__tab"
        :class="{
          'session-tabs__tab--active': session.id === sessionStore.currentSessionId,
          'session-tabs__tab--switching': session.id === switchingTabId,
          'session-tabs__tab--dragging': isDragging && dragSessionId === session.id
        }"
        :title="index < 5 ? `${session.name} (Ctrl+${index + 1})` : session.name"
        draggable="true"
        @dragstart="onDragStart($event, session.id)"
        @dragend="onDragEnd"
        @dragleave="onDragLeave"
        @click="switchToSession(session.id)"
      >
        <!-- 状态指示器 -->
        <span
          class="session-tabs__status"
          :style="{ backgroundColor: getStatusColor(session.status) }"
        >
          <EaIcon
            :name="getStatusIcon(session.status)"
            :size="10"
            :spin="session.status === 'running'"
          />
        </span>

        <!-- 会话名称 -->
        <span class="session-tabs__name">
          {{ session.name }}
        </span>

        <!-- 关闭按钮 -->
        <button
          class="session-tabs__close"
          :title="t('sessionTabs.close')"
          @click="closeTab(session.id, $event)"
        >
          <EaIcon
            name="x"
            :size="12"
          />
        </button>
      </div>
    </div>

    <!-- 溢出指示器 -->
    <div class="session-tabs__overflow-indicator" />
  </div>
</template>

<style scoped>
.session-tabs {
  display: flex;
  align-items: center;
  height: 36px;
  flex-shrink: 0;
  background-color: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
  position: relative;
}

.session-tabs__container {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 8px;
  overflow-x: auto;
  overflow-y: hidden;
  flex: 1;
  scrollbar-width: none; /* Firefox */
  -ms-overflow-style: none; /* IE and Edge */
}

.session-tabs__container::-webkit-scrollbar {
  display: none; /* Chrome, Safari, Opera */
}

.session-tabs__tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px 4px 12px;
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
  cursor: pointer;
  flex-shrink: 0;
  max-width: 160px;
  transition: all var(--transition-fast) var(--easing-default);
  border: 1px solid transparent;
}

.session-tabs__tab:hover {
  background-color: var(--color-surface-hover);
}

.session-tabs__tab--active {
  background-color: var(--color-primary-light);
  border-color: var(--color-primary);
}

.session-tabs__tab--active .session-tabs__name {
  color: var(--color-primary-dark);
  font-weight: var(--font-weight-medium);
}

/* 暗色模式下的激活样式 */
[data-theme='dark'] .session-tabs__tab--active {
  background-color: var(--color-active-bg);
  border-color: var(--color-active-border);
}

[data-theme='dark'] .session-tabs__tab--active .session-tabs__name {
  color: var(--color-active-text);
}

[data-theme='dark'] .session-tabs__tab--active:hover {
  background-color: var(--color-active-bg-hover);
}

/* 切换时的视觉反馈 */
.session-tabs__tab--switching {
  transform: scale(0.95);
  box-shadow: 0 0 0 2px var(--color-primary-alpha);
}

.session-tabs__status {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  color: white;
  flex-shrink: 0;
}

.session-tabs__name {
  flex: 1;
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-tabs__close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  flex-shrink: 0;
  opacity: 0;
  transition: all var(--transition-fast) var(--easing-default);
}

.session-tabs__tab:hover .session-tabs__close {
  opacity: 1;
}

.session-tabs__close:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.session-tabs__overflow-indicator {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 24px;
  background: linear-gradient(90deg, transparent, var(--color-surface));
  pointer-events: none;
  opacity: 0;
  transition: opacity var(--transition-fast) var(--easing-default);
}

/* 当有滚动时显示溢出指示器 */
.session-tabs:has(.session-tabs__container:not(:hover)) .session-tabs__overflow-indicator {
  opacity: 0;
}

/* 拖拽样式 */
.session-tabs__tab--dragging {
  opacity: 0.5;
  transform: scale(0.95);
  cursor: grabbing;
}

/* 拖拽时的视觉提示 */
.session-tabs__tab[draggable="true"]:active {
  cursor: grabbing;
}
</style>
