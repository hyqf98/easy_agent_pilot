<script setup lang="ts">
import { computed, ref, onMounted, nextTick, watch, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSessionStore, type SessionStatus } from '@/stores/session'
import { useProjectStore } from '@/stores/project'
import { useLayoutStore } from '@/stores/layout'
import { useWindowManagerStore } from '@/stores/windowManager'
import { useSplitPaneStore } from '@/stores/splitPane'
import { EaIcon } from '@/components/common'
import { useMessage } from 'naive-ui'
import { useSessionView } from '@/composables'
import { useAgentStore } from '@/stores/agent'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { resolveExpertRuntime } from '@/services/agentTeams/runtime'

const { t } = useI18n()
const sessionStore = useSessionStore()
const projectStore = useProjectStore()
const layoutStore = useLayoutStore()
const windowManagerStore = useWindowManagerStore()
const splitPaneStore = useSplitPaneStore()
const agentStore = useAgentStore()
const agentTeamsStore = useAgentTeamsStore()
const message = useMessage()
const { openSessionTarget } = useSessionView()

// 标签栏容器引用
const tabsContainerRef = ref<HTMLElement | null>(null)
// 正在切换的标签 ID（用于视觉反馈）
const switchingTabId = ref<string | null>(null)

const contextMenuState = ref<{
  visible: boolean
  x: number
  y: number
  sessionId: string | null
}>({
  visible: false,
  x: 0,
  y: 0,
  sessionId: null
})

// 拖拽状态
const isDragging = ref(false)
const dragSessionId = ref<string | null>(null)

const contextTargetIndex = computed(() => {
  if (!contextMenuState.value.sessionId) {
    return -1
  }

  return sessionStore.openSessionIds.indexOf(contextMenuState.value.sessionId)
})

const canCloseOthers = computed(() =>
  contextTargetIndex.value !== -1 && sessionStore.openSessionIds.length > 1
)

const canCloseLeft = computed(() => contextTargetIndex.value > 0)

const canCloseRight = computed(() => (
  contextTargetIndex.value !== -1
  && contextTargetIndex.value < sessionStore.openSessionIds.length - 1
))

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

const switchToSession = async (sessionId: string) => {
  if (sessionId === sessionStore.currentSessionId) return

  if (sessionStore.isSessionOpen(sessionId)) {
    sessionStore.setCurrentSession(sessionId)
    syncSidebarToSession(sessionId)
    return
  }

  switchingTabId.value = sessionId

  try {
    await openSessionTarget(sessionId)
  } finally {
    switchingTabId.value = null
  }
}

const syncSidebarToSession = (sessionId: string) => {
  const session = sessionStore.sessions.find(s => s.id === sessionId)
  if (!session?.projectId) return

  projectStore.setCurrentProject(session.projectId)
  if (!projectStore.isProjectExpanded(session.projectId)) {
    projectStore.expandProject(session.projectId)
  }
  layoutStore.setProjectTab(session.projectId, 'sessions')
  void sessionStore.loadSessions(session.projectId).catch(() => {})
}

// 关闭指定会话标签
const closeTab = (sessionId: string, event: MouseEvent) => {
  event.stopPropagation() // 阻止触发切换会话
  sessionStore.closeSession(sessionId)
  if (contextMenuState.value.sessionId === sessionId) {
    hideContextMenu()
  }
}

const hideContextMenu = () => {
  contextMenuState.value.visible = false
  contextMenuState.value.sessionId = null
}

const showContextMenu = (event: MouseEvent, sessionId: string) => {
  event.preventDefault()
  event.stopPropagation()

  contextMenuState.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    sessionId
  }
}

const handleContextMenuAction = (action: 'closeAll' | 'closeOthers' | 'closeLeft' | 'closeRight' | 'splitPane') => {
  const sessionId = contextMenuState.value.sessionId
  if (!sessionId) {
    return
  }

  switch (action) {
    case 'closeAll':
      sessionStore.closeAllSessions()
      break
    case 'closeOthers':
      sessionStore.closeOtherSessions(sessionId)
      break
    case 'closeLeft':
      sessionStore.closeSessionsToLeft(sessionId)
      break
    case 'closeRight':
      sessionStore.closeSessionsToRight(sessionId)
      break
    case 'splitPane':
      handleContextMenuSplit()
      return
  }

  hideContextMenu()
}

const handleGlobalPointer = () => {
  if (contextMenuState.value.visible) {
    hideContextMenu()
  }
}

const handleGlobalKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape' && contextMenuState.value.visible) {
    hideContextMenu()
  }
}

async function resolveDefaultExpertSession(projectId: string) {
  await Promise.all([
    agentStore.loadAgents(),
    agentTeamsStore.loadExperts(true)
  ])
  const expert = agentTeamsStore.builtinGeneralExpert || agentTeamsStore.enabledExperts[0] || null
  const runtime = resolveExpertRuntime(expert, agentStore.agents)
  return sessionStore.createSession({
    projectId,
    name: '',
    expertId: expert?.id,
    agentId: runtime?.agent.id,
    agentType: runtime?.agent.provider || runtime?.agent.type || 'cli',
    status: 'idle'
  })
}

const handleSplitPane = async (sessionId?: string) => {
  const targetId = sessionId ?? sessionStore.currentSessionId
  if (!targetId) return

  if (splitPaneStore.isSplitActive) {
    splitPaneStore.addPane(targetId)
  } else {
    const session = sessionStore.sessions.find(s => s.id === targetId)
    const newSession = await resolveDefaultExpertSession(session?.projectId ?? '')
    splitPaneStore.enterSplitMode(targetId, newSession.id)
  }
  splitPaneStore.focusPane(splitPaneStore.focusedPaneId!)
}

const handleExitSplit = () => {
  splitPaneStore.exitSplitMode()
}

const handleContextMenuSplit = () => {
  const sessionId = contextMenuState.value.sessionId
  if (sessionId) {
    handleSplitPane(sessionId)
  }
  hideContextMenu()
}

const handleAddPane = async () => {
  const session = sessionStore.currentSession
  if (!session) return

  const newSession = await resolveDefaultExpertSession(session.projectId)

  if (!splitPaneStore.isSplitActive) {
    splitPaneStore.enterSplitMode(session.id, newSession.id)
  } else {
    splitPaneStore.addPane(newSession.id)
  }
  splitPaneStore.focusPane(splitPaneStore.focusedPaneId!)
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
  window.addEventListener('click', handleGlobalPointer)
  window.addEventListener('blur', handleGlobalPointer)
  window.addEventListener('resize', handleGlobalPointer)
  window.addEventListener('keydown', handleGlobalKeydown)
  void scrollToActiveTab()
})

onUnmounted(() => {
  window.removeEventListener('click', handleGlobalPointer)
  window.removeEventListener('blur', handleGlobalPointer)
  window.removeEventListener('resize', handleGlobalPointer)
  window.removeEventListener('keydown', handleGlobalKeydown)
})

watch(() => sessionStore.currentSessionId, () => {
  void scrollToActiveTab()
})

watch(() => sessionStore.openSessionIds.join(':'), () => {
  if (!contextMenuState.value.sessionId) {
    return
  }

  if (!sessionStore.openSessionIds.includes(contextMenuState.value.sessionId)) {
    hideContextMenu()
  }
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
        @contextmenu="showContextMenu($event, session.id)"
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

    <!-- 分屏按钮 -->
    <div class="session-tabs__actions">
      <button
        v-if="splitPaneStore.isSplitActive"
        class="session-tabs__action-btn"
        :title="t('sessionTabs.addPane')"
        @click="handleAddPane"
      >
        <EaIcon
          name="plus"
          :size="14"
        />
      </button>
      <button
        v-if="splitPaneStore.isSplitActive"
        class="session-tabs__action-btn"
        :title="t('sessionTabs.exitSplit')"
        @click="handleExitSplit"
      >
        <EaIcon
          name="minimize-2"
          :size="14"
        />
      </button>
      <button
        v-else-if="sessionStore.openSessions.length >= 1 && sessionStore.currentSessionId"
        class="session-tabs__action-btn"
        :title="t('sessionTabs.splitPane')"
        @click="handleSplitPane()"
      >
        <EaIcon
          name="columns"
          :size="14"
        />
      </button>
    </div>

    <div
      v-if="contextMenuState.visible"
      class="session-tabs__context-menu"
      :style="{
        left: `${contextMenuState.x}px`,
        top: `${contextMenuState.y}px`
      }"
      @click.stop
    >
      <button
        class="session-tabs__context-action"
        type="button"
        @click="handleContextMenuAction('closeAll')"
      >
        {{ t('sessionTabs.closeAll') }}
      </button>
      <button
        class="session-tabs__context-action"
        type="button"
        :disabled="!canCloseOthers"
        @click="handleContextMenuAction('closeOthers')"
      >
        {{ t('sessionTabs.closeOthers') }}
      </button>
      <button
        class="session-tabs__context-action"
        type="button"
        :disabled="!canCloseLeft"
        @click="handleContextMenuAction('closeLeft')"
      >
        {{ t('sessionTabs.closeLeft') }}
      </button>
      <button
        class="session-tabs__context-action"
        type="button"
        :disabled="!canCloseRight"
        @click="handleContextMenuAction('closeRight')"
      >
        {{ t('sessionTabs.closeRight') }}
      </button>
      <div class="session-tabs__context-divider" />
      <button
        class="session-tabs__context-action session-tabs__context-action--split"
        type="button"
        @click="handleContextMenuAction('splitPane')"
      >
        <EaIcon
          name="columns"
          :size="14"
        />
        {{ t('sessionTabs.splitPane') }}
      </button>
    </div>
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

.session-tabs__context-menu {
  position: fixed;
  z-index: 3000;
  min-width: 180px;
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  background: color-mix(in srgb, var(--color-surface, #fff) 96%, white);
  border: 1px solid color-mix(in srgb, var(--color-border, #e2e8f0) 82%, transparent);
  border-radius: 10px;
  box-shadow: 0 14px 32px rgba(15, 23, 42, 0.14);
}

.session-tabs__context-action {
  display: flex;
  align-items: center;
  width: 100%;
  min-height: 32px;
  padding: 0 10px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  text-align: left;
  cursor: pointer;
  transition: background-color var(--transition-fast) var(--easing-default), color var(--transition-fast) var(--easing-default);
}

.session-tabs__context-action:hover:not(:disabled) {
  background: var(--color-surface-hover);
}

.session-tabs__context-action:disabled {
  color: var(--color-text-tertiary);
  cursor: not-allowed;
}

.session-tabs__context-divider {
  height: 1px;
  background: var(--color-border);
  margin: 4px 2px;
}

.session-tabs__context-action--split {
  color: var(--color-primary);
  gap: 6px;
}

.session-tabs__context-action--split:hover {
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
}

.session-tabs__actions {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 0 6px;
  flex-shrink: 0;
  border-left: 1px solid var(--color-border);
  margin-left: 4px;
}

.session-tabs__action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.session-tabs__action-btn:hover {
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}

[data-theme='dark'] .session-tabs__context-menu {
  background: color-mix(in srgb, var(--color-surface, #111827) 90%, #020617);
  border-color: rgba(148, 163, 184, 0.18);
  box-shadow: 0 16px 36px rgba(2, 6, 23, 0.38);
}
</style>
