<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSessionStore, type Session } from '@/stores/session'
import { useProjectStore } from '@/stores/project'
import { useUIStore } from '@/stores/ui'
import { useMessageStore } from '@/stores/message'
import { useNotificationStore } from '@/stores/notification'
import { useSessionView } from '@/composables'
import { EaIcon, EaButton, EaSkeleton, EaStateBlock } from '@/components/common'
import PanelHeader from './PanelHeader.vue'
import SessionPanelDialogs from './SessionPanelDialogs.vue'
import SessionPanelItem from './SessionPanelItem.vue'

const { t } = useI18n()

export interface SessionPanelProps {
  collapsed?: boolean
  showHeaderToggle?: boolean
}
interface SessionActionItem {
  key: string
  title: string
  icon: string
  danger?: boolean
  warning?: boolean
  handler: (session: Session) => void | Promise<void>
}

defineProps<SessionPanelProps>()

defineEmits<{
  toggle: []
}>()

const sessionStore = useSessionStore()
const projectStore = useProjectStore()
const uiStore = useUIStore()
const messageStore = useMessageStore()
const notificationStore = useNotificationStore()
const { openSessionTarget } = useSessionView()

const showDeleteConfirm = ref(false)
const deletingSession = ref<Session | null>(null)
const showErrorModal = ref(false)
const errorSession = ref<Session | null>(null)
const showSummaryModal = ref(false)
const summarySession = ref<Session | null>(null)
const searchInput = ref('')
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null

// 清空消息确认对话框状态
const showClearMessagesConfirm = ref(false)
const clearingSession = ref<Session | null>(null)
const isClearingMessages = ref(false)

// 创建会话表单状态
const newSessionName = ref('')

// 编辑会话名称状态
const editingSessionId = ref<string | null>(null)
const editingSessionName = ref('')

// 项目选择
const selectedProjectId = ref<string | null>(null)

// 表单有效性校验
const isNewSessionFormValid = computed(() => {
  return newSessionName.value.trim().length > 0
})

// 当前项目的会话列表
const currentProjectSessions = computed(() => {
  if (!projectStore.currentProjectId) return []
  return sessionStore.sessionsByProject(projectStore.currentProjectId)
})

const sessionActionMap = computed(() => new Map(
  currentProjectSessions.value.map(session => [session.id, getSessionActions(session)])
))

// 切换项目
const handleProjectChange = (projectId: string) => {
  selectedProjectId.value = projectId
  projectStore.setCurrentProject(projectId)
}

// 是否有搜索词
const hasSearchQuery = computed(() => sessionStore.searchQuery.trim().length > 0)

// 清除搜索
const clearSearch = () => {
  searchInput.value = ''
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
    searchDebounceTimer = null
  }
  sessionStore.setSearchQuery('')
}

const handleSearchInput = (value: string) => {
  searchInput.value = value

  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
  }

  // 搜索框本地即时响应，实际过滤延迟一点提交，避免大列表上每个按键都触发重排。
  searchDebounceTimer = setTimeout(() => {
    sessionStore.setSearchQuery(value)
    searchDebounceTimer = null
  }, 160)
}

// 手动刷新会话列表
const handleRefreshSessions = () => {
  if (projectStore.currentProjectId) {
    sessionStore.loadSessions(projectStore.currentProjectId)
  }
}

// 监听项目切换，加载会话
watch(() => projectStore.currentProjectId, async (projectId, oldProjectId) => {
  // 项目切换时，先清空当前会话（确保消息区域正确更新）
  if (oldProjectId !== undefined) {
    sessionStore.setCurrentSession(null)
  }

  if (projectId) {
    await sessionStore.loadSessions(projectId)
    // 自动选中第一个会话（添加到打开列表）
    const sessions = sessionStore.sessionsByProject(projectId)
    if (sessions.length > 0) {
      sessionStore.openSession(sessions[0].id)
    }
  }
}, { immediate: true })

onMounted(() => {
  // 初始化选中的项目ID
  selectedProjectId.value = projectStore.currentProjectId
  searchInput.value = sessionStore.searchQuery

  if (projectStore.currentProjectId) {
    sessionStore.loadSessions(projectStore.currentProjectId)
  }
  // 添加 ESC 键关闭模态框
  document.addEventListener('keydown', handleModalKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleModalKeydown)
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
    searchDebounceTimer = null
  }
})

watch(() => sessionStore.searchQuery, (value) => {
  if (value !== searchInput.value) {
    searchInput.value = value
  }
})

// ESC 键关闭模态框
const handleModalKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    // 按照优先级关闭模态框
    if (showSummaryModal.value) {
      showSummaryModal.value = false
    } else if (showErrorModal.value) {
      showErrorModal.value = false
    } else if (showClearMessagesConfirm.value) {
      showClearMessagesConfirm.value = false
    } else if (showDeleteConfirm.value) {
      showDeleteConfirm.value = false
    } else if (uiStore.sessionCreateModalVisible) {
      uiStore.closeSessionCreateModal()
    }
  }
}

const handleAdd = async () => {
  if (!projectStore.currentProjectId) return

  try {
    // 直接创建未命名会话，不需要弹框
    const newSession = await sessionStore.createSession({
      projectId: projectStore.currentProjectId,
      name: '未命名会话',
      agentType: 'claude',
      status: 'idle'
    })
    projectStore.incrementSessionCount(projectStore.currentProjectId)
    // 自动选中新创建的会话（添加到打开列表）
    sessionStore.openSession(newSession.id)
  } catch (error) {
    // 错误已在 sessionStore.createSession 中处理并显示通知
    console.error('Session creation failed in component:', error)
  }
}

const handleSelectSession = async (id: string) => {
  await openSessionTarget(id)
}

const handleTogglePin = (id: string) => {
  sessionStore.togglePin(id)
}

const handleDeleteSession = (session: Session) => {
  deletingSession.value = session
  showDeleteConfirm.value = true
}

const confirmDelete = () => {
  if (deletingSession.value) {
    sessionStore.deleteSession(deletingSession.value.id)
    if (projectStore.currentProjectId) {
      projectStore.decrementSessionCount(projectStore.currentProjectId)
    }
  }
  showDeleteConfirm.value = false
  deletingSession.value = null
}

const closeDeleteConfirm = () => {
  showDeleteConfirm.value = false
  deletingSession.value = null
}

const handleCreateSession = async (name: string) => {
  if (!projectStore.currentProjectId) return

  try {
    const newSession = await sessionStore.createSession({
      projectId: projectStore.currentProjectId,
      name,
      agentType: 'claude',
      status: 'idle'
    })
    projectStore.incrementSessionCount(projectStore.currentProjectId)
    // 清空表单
    newSessionName.value = ''
    uiStore.closeSessionCreateModal()
    // 自动选中新创建的会话（添加到打开列表）
    sessionStore.openSession(newSession.id)
  } catch (error) {
    // 错误已在 sessionStore.createSession 中处理并显示通知
    // 这里只需阻止错误继续传播，避免未处理的 Promise rejection
    console.error('Session creation failed in component:', error)
  }
}

// 暂停会话
const handlePauseSession = async (session: Session) => {
  await sessionStore.updateSession(session.id, { status: 'paused' })
}

// 继续会话
const handleResumeSession = async (session: Session) => {
  await sessionStore.updateSession(session.id, { status: 'running' })
}

// 停止会话
const handleStopSession = async (session: Session) => {
  await sessionStore.updateSession(session.id, { status: 'idle' })
}

// 查看错误详情
const handleShowErrorDetails = (session: Session) => {
  errorSession.value = session
  showErrorModal.value = true
}

// 重试会话
const handleRetrySession = async (session: Session) => {
  // 清除错误信息并重新开始
  await sessionStore.updateSession(session.id, { status: 'running', errorMessage: undefined })
}

// 查看执行摘要
const handleShowSummary = (session: Session) => {
  summarySession.value = session
  showSummaryModal.value = true
}

// 重新运行完成的会话
const handleRerunSession = async (session: Session) => {
  // 重置状态为运行中
  await sessionStore.updateSession(session.id, { status: 'running' })
}

// 开始编辑会话名称
const startEditSessionName = async (session: Session) => {
  editingSessionId.value = session.id
  editingSessionName.value = session.name
  // 等待 DOM 更新后自动聚焦输入框
  await nextTick()
  const input = document.querySelector('.session-item__name-input') as HTMLInputElement
  if (input) {
    input.focus()
    input.select()
  }
}

// 取消编辑会话名称
const cancelEditSessionName = () => {
  editingSessionId.value = null
  editingSessionName.value = ''
}

// 保存编辑的会话名称
const saveEditSessionName = async (session: Session) => {
  const trimmedName = editingSessionName.value.trim()
  if (trimmedName && trimmedName !== session.name) {
    await sessionStore.updateSession(session.id, { name: trimmedName })
  }
  cancelEditSessionName()
}

// 清空会话消息
const handleClearMessages = (session: Session) => {
  clearingSession.value = session
  showClearMessagesConfirm.value = true
}

const closeClearMessagesConfirm = () => {
  if (isClearingMessages.value) return
  showClearMessagesConfirm.value = false
  clearingSession.value = null
}

const confirmClearMessages = async () => {
  if (!clearingSession.value) return

  isClearingMessages.value = true
  try {
    await messageStore.clearSessionMessages(clearingSession.value.id)
    notificationStore.success(t('message.clearMessagesSuccess'))
    closeClearMessagesConfirm()
  } catch (error) {
    console.error('Failed to clear messages:', error)
  } finally {
    isClearingMessages.value = false
  }
}

const closeErrorModal = () => {
  showErrorModal.value = false
  errorSession.value = null
}

const retryErroredSession = async () => {
  if (!errorSession.value) return
  const session = errorSession.value
  closeErrorModal()
  await handleRetrySession(session)
}

const closeSummaryModal = () => {
  showSummaryModal.value = false
  summarySession.value = null
}

const rerunSummarySession = async () => {
  if (!summarySession.value) return
  const session = summarySession.value
  closeSummaryModal()
  await handleRerunSession(session)
}

function getSessionActions(session: Session): SessionActionItem[] {
  const statusActions: Record<Session['status'], SessionActionItem[]> = {
    idle: [],
    running: [
      {
        key: 'pause',
        title: t('session.pause'),
        icon: 'pause',
        handler: handlePauseSession
      },
      {
        key: 'stop',
        title: t('session.stop'),
        icon: 'square',
        danger: true,
        handler: handleStopSession
      }
    ],
    paused: [
      {
        key: 'resume',
        title: t('session.resume'),
        icon: 'play',
        handler: handleResumeSession
      }
    ],
    error: [
      {
        key: 'details',
        title: t('session.viewErrorDetails'),
        icon: 'info',
        handler: handleShowErrorDetails
      },
      {
        key: 'retry',
        title: t('common.retry'),
        icon: 'refresh-cw',
        handler: handleRetrySession
      }
    ],
    completed: [
      {
        key: 'summary',
        title: t('session.viewSummary'),
        icon: 'file-text',
        handler: handleShowSummary
      },
      {
        key: 'rerun',
        title: t('session.rerun'),
        icon: 'rotate-ccw',
        handler: handleRerunSession
      }
    ]
  }

  const commonActions: SessionActionItem[] = [
    {
      key: 'pin',
      title: session.pinned ? t('session.unpin') : t('session.pin'),
      icon: session.pinned ? 'pin-off' : 'pin',
      handler: (target) => handleTogglePin(target.id)
    },
    {
      key: 'edit',
      title: t('common.edit'),
      icon: 'edit-2',
      handler: startEditSessionName
    }
  ]

  if (session.messageCount && session.messageCount > 0) {
    commonActions.push({
      key: 'clear',
      title: t('message.clearMessages'),
      icon: 'eraser',
      warning: true,
      handler: handleClearMessages
    })
  }

  commonActions.push({
    key: 'delete',
    title: t('common.delete'),
    icon: 'trash-2',
    danger: true,
    handler: handleDeleteSession
  })

  return [...statusActions[session.status], ...commonActions]
}

</script>

<template>
  <div :class="['session-panel', { 'session-panel--collapsed': collapsed }]">
    <PanelHeader
      :title="t('panel.sessions')"
      icon="message-square"
      :collapsed="collapsed"
      :show-toggle="showHeaderToggle"
      show-add
      @toggle="$emit('toggle')"
      @add="handleAdd"
    />

    <div
      v-if="!collapsed"
      class="session-panel__content"
    >
      <!-- 无项目提示 -->
      <div
        v-if="!projectStore.currentProjectId"
        class="session-empty"
      >
        <EaStateBlock
          icon="folder"
          :description="t('session.noProjectSelected')"
        />
      </div>

      <!-- 加载状态 -->
      <template v-else-if="sessionStore.isLoading">
        <!-- 搜索框骨架屏 -->
        <div class="session-search session-search--loading">
          <EaSkeleton
            variant="circle"
            height="14px"
            width="14px"
            animation="wave"
          />
          <EaSkeleton
            variant="text"
            height="14px"
            width="60%"
            animation="wave"
          />
        </div>
        <!-- 会话列表骨架屏 -->
        <div class="session-loading">
          <div
            v-for="i in 3"
            :key="i"
            class="session-skeleton"
          >
            <div class="session-skeleton__header">
              <EaSkeleton
                variant="circle"
                height="14px"
                width="14px"
                animation="wave"
              />
              <EaSkeleton
                variant="text"
                height="14px"
                :width="`${40 + Math.random() * 30}%`"
                animation="wave"
              />
            </div>
            <div class="session-skeleton__meta">
              <EaSkeleton
                variant="text"
                height="12px"
                width="50px"
                animation="wave"
              />
            </div>
            <div class="session-skeleton__preview">
              <EaSkeleton
                variant="text"
                height="12px"
                :width="`${70 + Math.random() * 20}%`"
                animation="wave"
              />
            </div>
          </div>
        </div>
      </template>

      <!-- 错误状态 -->
      <div
        v-else-if="sessionStore.loadError"
        class="session-error"
      >
        <EaStateBlock
          variant="error"
          :title="t('common.loadFailed')"
          :description="sessionStore.loadError"
        >
          <template #actions>
            <EaButton
              type="primary"
              size="small"
              @click="handleRefreshSessions"
            >
              <EaIcon
                name="refresh-cw"
                :size="14"
              />
              {{ t('common.retry') }}
            </EaButton>
          </template>
        </EaStateBlock>
      </div>

      <!-- 项目和智能体选择 -->
      <div
        v-else
        class="session-filters"
      >
        <!-- 项目选择 -->
        <div class="session-filter">
          <select
            v-model="selectedProjectId"
            class="session-filter__select"
            @change="handleProjectChange(($event.target as HTMLSelectElement).value)"
          >
            <option
              value=""
              disabled
            >
              {{ t('session.selectProject') }}
            </option>
            <option
              v-for="project in projectStore.projects"
              :key="project.id"
              :value="project.id"
            >
              {{ project.name }}
            </option>
          </select>
          <EaIcon
            name="chevron-down"
            :size="14"
            class="session-filter__icon"
          />
        </div>
      </div>

      <!-- 搜索框 -->
      <div class="session-search">
        <EaIcon
          name="search"
          :size="14"
          class="session-search__icon"
        />
        <input
          :value="searchInput"
          type="text"
          class="session-search__input"
          :placeholder="t('session.searchSessions')"
          @input="handleSearchInput(($event.target as HTMLInputElement).value)"
        >
        <button
          v-if="hasSearchQuery"
          class="session-search__clear"
          :title="t('common.clearSearch')"
          @click="clearSearch"
        >
          <EaIcon
            name="x"
            :size="14"
          />
        </button>
      </div>

      <!-- 搜索无结果 -->
      <div
        v-if="projectStore.currentProjectId && hasSearchQuery && currentProjectSessions.length === 0"
        class="session-empty"
      >
        <EaStateBlock
          icon="search"
          :description="t('session.noMatchingSessions')"
        >
          <template #actions>
            <EaButton
              type="secondary"
              size="small"
              @click="clearSearch"
            >
              {{ t('common.clearSearch') }}
            </EaButton>
          </template>
        </EaStateBlock>
      </div>

      <!-- 空状态（无会话） -->
      <div
        v-else-if="projectStore.currentProjectId && !hasSearchQuery && currentProjectSessions.length === 0"
        class="session-empty"
      >
        <EaStateBlock
          icon="message-square-plus"
          :description="t('session.noSessions')"
        >
          <template #actions>
            <EaButton
              type="primary"
              size="small"
              @click="handleAdd"
            >
              <EaIcon
                name="plus"
                :size="14"
              />
              {{ t('session.createSession') }}
            </EaButton>
          </template>
        </EaStateBlock>
      </div>

      <!-- 会话列表 -->
      <div
        v-else-if="projectStore.currentProjectId"
        class="session-list"
        role="list"
      >
        <SessionPanelItem
          v-for="session in currentProjectSessions"
          :key="session.id"
          :session="session"
          :active="session.id === sessionStore.currentSessionId"
          :editing-session-id="editingSessionId"
          :editing-session-name="editingSessionName"
          :search-query="sessionStore.searchQuery"
          :actions="(sessionActionMap.get(session.id) ?? []).map(({ key, title, icon, danger, warning }) => ({ key, title, icon, danger, warning }))"
          @select="handleSelectSession"
          @save-name="saveEditSessionName"
          @cancel-edit="cancelEditSessionName"
          @update-name="editingSessionName = $event"
          @action="(key, targetSession) => sessionActionMap.get(targetSession.id)?.find(action => action.key === key)?.handler(targetSession)"
        />
      </div>
    </div>

    <SessionPanelDialogs
      :session-create-modal-visible="uiStore.sessionCreateModalVisible"
      :new-session-name="newSessionName"
      :is-new-session-form-valid="isNewSessionFormValid"
      :show-delete-confirm="showDeleteConfirm"
      :deleting-session="deletingSession"
      :show-clear-messages-confirm="showClearMessagesConfirm"
      :is-clearing-messages="isClearingMessages"
      :show-error-modal="showErrorModal"
      :error-session="errorSession"
      :show-summary-modal="showSummaryModal"
      :summary-session="summarySession"
      @close-create="uiStore.closeSessionCreateModal()"
      @submit-create="handleCreateSession(newSessionName.trim())"
      @update-new-session-name="newSessionName = $event"
      @close-delete="closeDeleteConfirm"
      @confirm-delete="confirmDelete"
      @close-clear-messages="closeClearMessagesConfirm"
      @confirm-clear-messages="confirmClearMessages"
      @close-error="closeErrorModal"
      @retry-error="retryErroredSession"
      @close-summary="closeSummaryModal"
      @rerun-summary="rerunSummarySession"
    />
  </div>
</template>

<style scoped>
.session-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: var(--color-surface);
  border-right: 1px solid var(--color-border);
  overflow: hidden;
}

.session-panel--collapsed {
  width: 48px;
}

.session-panel__content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.session-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--spacing-8) var(--spacing-4);
  text-align: center;
}

.session-empty__icon {
  color: var(--color-text-tertiary);
  margin-bottom: var(--spacing-3);
}

.session-empty__text {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-4);
}

.session-loading {
  flex: 1;
  overflow-y: auto;
  padding: 0 var(--spacing-2) var(--spacing-2);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.session-skeleton {
  display: flex;
  flex-direction: column;
  padding: var(--spacing-3);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
}

.session-skeleton__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  margin-bottom: var(--spacing-2);
}

.session-skeleton__meta {
  padding-left: calc(16px + var(--spacing-3));
  margin-bottom: var(--spacing-2);
}

.session-skeleton__preview {
  padding-left: calc(16px + var(--spacing-3));
}

.session-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-8) var(--spacing-4);
  text-align: center;
  flex: 1;
}

.session-error__icon {
  color: var(--color-error);
  margin-bottom: var(--spacing-3);
}

.session-error__text {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  margin: 0 0 var(--spacing-1);
}

.session-error__detail {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  margin: 0 0 var(--spacing-4);
  max-width: 180px;
  line-height: 1.5;
}

.session-search--loading {
  cursor: default;
}

.session-search {
  display: flex;
  align-items: center;
  margin: var(--spacing-2);
  padding: var(--spacing-2);
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
}

.session-search__icon {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.session-search__input {
  flex: 1;
  margin-left: var(--spacing-2);
  padding: 0;
  background: none;
  border: none;
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  outline: none;
}

.session-search__input::placeholder {
  color: var(--color-text-tertiary);
}

.session-search__clear {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  transition: all var(--transition-fast) var(--easing-default);
  flex-shrink: 0;
  outline: none;
}

.session-search__clear:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-secondary);
}

.session-search__clear:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: -2px;
}

.session-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 var(--spacing-3) var(--spacing-3);
}

/* 弹框样式 */
/* 筛选器样式 */
.session-filters {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  border-bottom: 1px solid var(--color-border);
}

.session-filter {
  position: relative;
  display: flex;
  align-items: center;
}

.session-filter__select {
  width: 100%;
  padding: var(--spacing-2) var(--spacing-6) var(--spacing-2) var(--spacing-3);
  background-color: var(--color-bg-tertiary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  appearance: none;
  cursor: pointer;
  outline: none;
  transition: all var(--transition-fast) var(--easing-default);
}

.session-filter__select:hover {
  border-color: var(--color-primary);
}

.session-filter__select:focus {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-light);
}

.session-filter__icon {
  position: absolute;
  right: var(--spacing-2);
  pointer-events: none;
  color: var(--color-text-tertiary);
}
</style>
