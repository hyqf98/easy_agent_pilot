import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSessionView } from '@/composables'
import { useAgentStore } from '@/stores/agent'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { useMessageStore } from '@/stores/message'
import { useNotificationStore } from '@/stores/notification'
import { useProjectStore } from '@/stores/project'
import { useSessionStore, type Session } from '@/stores/session'
import { useUIStore } from '@/stores/ui'
import { resolveExpertRuntime } from '@/services/agentTeams/runtime'

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

/**
 * 会话侧栏视图状态。
 * 负责项目内会话加载、筛选、增删改、状态操作以及相关对话框编排。
 */
export function useSessionPanelView() {
  const { t } = useI18n()

  const sessionStore = useSessionStore()
  const projectStore = useProjectStore()
  const uiStore = useUIStore()
  const messageStore = useMessageStore()
  const notificationStore = useNotificationStore()
  const agentStore = useAgentStore()
  const agentTeamsStore = useAgentTeamsStore()
  const { openSessionTarget } = useSessionView()

  const showDeleteConfirm = ref(false)
  const deletingSession = ref<Session | null>(null)
  const pendingDeleteSessionIds = ref<string[]>([])
  const isDeletingSessions = ref(false)
  const showErrorModal = ref(false)
  const errorSession = ref<Session | null>(null)
  const showSummaryModal = ref(false)
  const summarySession = ref<Session | null>(null)
  const searchInput = ref('')
  let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null

  const showClearMessagesConfirm = ref(false)
  const clearingSession = ref<Session | null>(null)
  const isClearingMessages = ref(false)

  const newSessionName = ref('')
  const editingSessionId = ref<string | null>(null)
  const editingSessionName = ref('')
  const selectedProjectId = ref<string | null>(null)
  const selectedSessionIds = ref<Set<string>>(new Set())

  const isNewSessionFormValid = computed(() => newSessionName.value.trim().length > 0)

  const currentProjectSessions = computed(() => {
    if (!projectStore.currentProjectId) return []
    return sessionStore.sessionsByProject(projectStore.currentProjectId)
  })

  const sessionActionMap = computed(() => new Map(
    currentProjectSessions.value.map(session => [session.id, getSessionActions(session)])
  ))
  const selectedSessions = computed(() => currentProjectSessions.value.filter(session =>
    selectedSessionIds.value.has(session.id)
  ))
  const selectedSessionCount = computed(() => selectedSessions.value.length)
  const pendingDeleteSessionCount = computed(() => pendingDeleteSessionIds.value.length)
  const hasSelectedSessions = computed(() => selectedSessionCount.value > 0)
  const allVisibleSessionsSelected = computed(() =>
    currentProjectSessions.value.length > 0 && selectedSessionCount.value === currentProjectSessions.value.length
  )

  const handleProjectChange = (projectId: string) => {
    selectedProjectId.value = projectId
    selectedSessionIds.value = new Set()
    projectStore.setCurrentProject(projectId)
  }

  const hasSearchQuery = computed(() => sessionStore.searchQuery.trim().length > 0)

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

    searchDebounceTimer = setTimeout(() => {
      sessionStore.setSearchQuery(value)
      searchDebounceTimer = null
    }, 160)
  }

  const handleRefreshSessions = () => {
    if (projectStore.currentProjectId) {
      sessionStore.loadSessions(projectStore.currentProjectId, { force: true })
    }
  }

  watch(() => projectStore.currentProjectId, async (projectId, oldProjectId) => {
    selectedSessionIds.value = new Set()
    if (oldProjectId !== undefined) {
      sessionStore.setCurrentSession(null)
    }

    if (projectId) {
      await sessionStore.loadSessions(projectId, { force: true })
      const sessions = sessionStore.sessionsByProject(projectId)
      if (sessions.length > 0) {
        uiStore.setAppMode('chat')
        uiStore.setMainContentMode('chat')
        await sessionStore.openSession(sessions[0].id)
      }
    }
  }, { immediate: true })

  onMounted(() => {
    selectedProjectId.value = projectStore.currentProjectId
    searchInput.value = sessionStore.searchQuery

    if (projectStore.currentProjectId) {
      sessionStore.loadSessions(projectStore.currentProjectId, { force: true })
    }

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

  watch(currentProjectSessions, (sessions) => {
    const visibleIds = new Set(sessions.map(session => session.id))
    const nextSelected = new Set(
      Array.from(selectedSessionIds.value).filter(sessionId => visibleIds.has(sessionId))
    )
    if (nextSelected.size !== selectedSessionIds.value.size) {
      selectedSessionIds.value = nextSelected
    }
  })

  const handleModalKeydown = (event: KeyboardEvent) => {
    if (event.key !== 'Escape') return

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

  const handleAdd = async () => {
    if (!projectStore.currentProjectId) return

    try {
      await Promise.all([
        agentStore.loadAgents(),
        agentTeamsStore.loadExperts(true)
      ])
      const expert = agentTeamsStore.builtinGeneralExpert || agentTeamsStore.enabledExperts[0] || null
      const runtime = resolveExpertRuntime(expert, agentStore.agents)

      const newSession = await sessionStore.createSession({
        projectId: projectStore.currentProjectId,
        name: '未命名会话',
        expertId: expert?.id,
        agentId: runtime?.agent.id,
        agentType: runtime?.agent.provider || runtime?.agent.type || 'claude',
        status: 'idle'
      })
      projectStore.incrementSessionCount(projectStore.currentProjectId)
      uiStore.setAppMode('chat')
      uiStore.setMainContentMode('chat')
      await sessionStore.openSession(newSession.id)
    } catch (error) {
      console.error('Session creation failed in component:', error)
    }
  }

  const handleSelectSession = async (id: string) => {
    await openSessionTarget(id)
  }

  const handleTogglePin = (id: string) => {
    sessionStore.togglePin(id)
  }

  const toggleSessionSelection = (id: string) => {
    const next = new Set(selectedSessionIds.value)
    if (next.has(id)) {
      next.delete(id)
    } else {
      next.add(id)
    }
    selectedSessionIds.value = next
  }

  const clearSelectedSessions = () => {
    selectedSessionIds.value = new Set()
  }

  const selectAllVisibleSessions = () => {
    selectedSessionIds.value = new Set(currentProjectSessions.value.map(session => session.id))
  }

  const handleDeleteSession = (session: Session) => {
    pendingDeleteSessionIds.value = [session.id]
    deletingSession.value = session
    showDeleteConfirm.value = true
  }

  const handleDeleteSelectedSessions = () => {
    if (!hasSelectedSessions.value) {
      return
    }

    pendingDeleteSessionIds.value = selectedSessions.value.map(session => session.id)
    deletingSession.value = selectedSessionCount.value === 1 ? selectedSessions.value[0] ?? null : null
    showDeleteConfirm.value = true
  }

  const confirmDelete = async () => {
    if (pendingDeleteSessionIds.value.length === 0 || isDeletingSessions.value) {
      return
    }

    isDeletingSessions.value = true
    const sessionIds = [...pendingDeleteSessionIds.value]
    try {
      for (const sessionId of sessionIds) {
        await sessionStore.deleteSession(sessionId)
        if (projectStore.currentProjectId) {
          projectStore.decrementSessionCount(projectStore.currentProjectId)
        }
      }

      if (sessionIds.length > 1) {
        notificationStore.success(t('session.batchDeleteSuccess', { count: sessionIds.length }))
      }
      clearSelectedSessions()
      showDeleteConfirm.value = false
      deletingSession.value = null
      pendingDeleteSessionIds.value = []
    } finally {
      isDeletingSessions.value = false
    }
  }

  const closeDeleteConfirm = () => {
    if (isDeletingSessions.value) {
      return
    }
    showDeleteConfirm.value = false
    deletingSession.value = null
    pendingDeleteSessionIds.value = []
  }

  const handleCreateSession = async (name: string) => {
    if (!projectStore.currentProjectId) return

    try {
      await Promise.all([
        agentStore.loadAgents(),
        agentTeamsStore.loadExperts(true)
      ])
      const expert = agentTeamsStore.builtinGeneralExpert || agentTeamsStore.enabledExperts[0] || null
      const runtime = resolveExpertRuntime(expert, agentStore.agents)

      const newSession = await sessionStore.createSession({
        projectId: projectStore.currentProjectId,
        name,
        expertId: expert?.id,
        agentId: runtime?.agent.id,
        agentType: runtime?.agent.provider || runtime?.agent.type || 'claude',
        status: 'idle'
      })
      projectStore.incrementSessionCount(projectStore.currentProjectId)
      newSessionName.value = ''
      uiStore.closeSessionCreateModal()
      uiStore.setAppMode('chat')
      uiStore.setMainContentMode('chat')
      await sessionStore.openSession(newSession.id)
    } catch (error) {
      console.error('Session creation failed in component:', error)
    }
  }

  const handlePauseSession = async (session: Session) => {
    await sessionStore.updateSession(session.id, { status: 'paused' })
  }

  const handleResumeSession = async (session: Session) => {
    await sessionStore.updateSession(session.id, { status: 'running' })
  }

  const handleStopSession = async (session: Session) => {
    await sessionStore.updateSession(session.id, { status: 'idle' })
  }

  const handleShowErrorDetails = (session: Session) => {
    errorSession.value = session
    showErrorModal.value = true
  }

  const handleRetrySession = async (session: Session) => {
    await sessionStore.updateSession(session.id, { status: 'running', errorMessage: undefined })
  }

  const handleShowSummary = (session: Session) => {
    summarySession.value = session
    showSummaryModal.value = true
  }

  const handleRerunSession = async (session: Session) => {
    await sessionStore.updateSession(session.id, { status: 'running' })
  }

  const startEditSessionName = async (session: Session) => {
    editingSessionId.value = session.id
    editingSessionName.value = session.name
    await nextTick()
    const input = document.querySelector('.session-item__name-input') as HTMLInputElement | null
    if (input) {
      input.focus()
      input.select()
    }
  }

  const cancelEditSessionName = () => {
    editingSessionId.value = null
    editingSessionName.value = ''
  }

  const saveEditSessionName = async (session: Session) => {
    const trimmedName = editingSessionName.value.trim()
    if (trimmedName && trimmedName !== session.name) {
      await sessionStore.updateSession(session.id, { name: trimmedName })
    }
    cancelEditSessionName()
  }

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

  return {
    cancelEditSessionName,
    clearingSession,
    closeClearMessagesConfirm,
    closeDeleteConfirm,
    closeErrorModal,
    closeSummaryModal,
    confirmClearMessages,
    confirmDelete,
    currentProjectSessions,
    deletingSession,
    editingSessionId,
    editingSessionName,
    errorSession,
    allVisibleSessionsSelected,
    clearSelectedSessions,
    handleAdd,
    handleCreateSession,
    handleDeleteSelectedSessions,
    handleProjectChange,
    handleRefreshSessions,
    handleSearchInput,
    handleSelectSession,
    hasSelectedSessions,
    hasSearchQuery,
    isClearingMessages,
    isDeletingSessions,
    isNewSessionFormValid,
    newSessionName,
    projectStore,
    rerunSummarySession,
    retryErroredSession,
    saveEditSessionName,
    searchInput,
    pendingDeleteSessionCount,
    selectedSessionCount,
    selectedSessionIds,
    selectedProjectId,
    selectAllVisibleSessions,
    sessionActionMap,
    sessionStore,
    showClearMessagesConfirm,
    showDeleteConfirm,
    showErrorModal,
    showSummaryModal,
    summarySession,
    t,
    toggleSessionSelection,
    uiStore,
    clearSearch
  }
}
