import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'
import { useSessionExecutionStore } from './sessionExecution'
import { useWindowManagerStore } from './windowManager'
import { useAppStateStore } from './appState'
import { useMessageStore } from './message'
import { useTokenStore } from './token'
import { useAiEditTraceStore } from './aiEditTrace'

export type SessionStatus = 'idle' | 'running' | 'paused' | 'error' | 'completed'

export interface Session {
  id: string
  projectId: string
  name: string
  expertId?: string
  agentId?: string
  agentType: string
  cliSessionId?: string
  cliSessionProvider?: string
  status: SessionStatus
  pinned: boolean
  lastMessage?: string
  errorMessage?: string
  messageCount: number
  createdAt: string
  updatedAt: string
}

// Rust 后端返回的 snake_case 结构
interface RustSession {
  id: string
  project_id: string
  name: string
  expert_id?: string
  agent_id?: string
  agent_type: string
  cli_session_id?: string
  cli_session_provider?: string
  status: string
  pinned: boolean
  last_message?: string
  error_message?: string
  message_count: number
  created_at: string
  updated_at: string
}

// 将 Rust 返回的 snake_case 转换为 camelCase
function transformSession(rustSession: RustSession): Session {
  return {
    id: rustSession.id,
    projectId: rustSession.project_id,
    name: rustSession.name,
    expertId: rustSession.expert_id,
    agentId: rustSession.agent_id,
    agentType: rustSession.agent_type,
    cliSessionId: rustSession.cli_session_id,
    cliSessionProvider: rustSession.cli_session_provider,
    status: rustSession.status as SessionStatus,
    pinned: rustSession.pinned,
    lastMessage: rustSession.last_message,
    errorMessage: rustSession.error_message,
    messageCount: rustSession.message_count,
    createdAt: rustSession.created_at,
    updatedAt: rustSession.updated_at
  }
}

function shouldDisplaySession(session: Session): boolean {
  const isUnattendedShell = session.name.startsWith('无人值守')
  if (!isUnattendedShell) {
    return true
  }

  const hasPreview = Boolean(session.lastMessage?.trim())
  return hasPreview || session.messageCount > 0
}

// 最大同时打开的会话数量
export const MAX_OPEN_SESSIONS = 5

export const useSessionStore = defineStore('session', () => {
  // State
  const sessions = ref<Session[]>([])
  const currentSessionId = ref<string | null>(null)
  const isLoading = ref(false)
  const loadError = ref<string | null>(null)
  const searchQuery = ref('')
  // 打开的会话 ID 列表（用于标签栏）
  const openSessionIds = ref<string[]>([])
  const loadedProjectIds = ref<Set<string>>(new Set())
  const loadingProjectIds = ref<Set<string>>(new Set())
  const EMPTY_SESSIONS: Session[] = []

  // Getters
  const currentSession = computed(() =>
    sessions.value.find(s => s.id === currentSessionId.value)
  )

  const normalizedSearchQuery = computed(() => searchQuery.value.trim().toLowerCase())

  const projectSessionsByUpdatedAt = computed(() => {
    // 侧栏会频繁读取当前项目会话，先按项目聚合并排序，避免每次渲染都重新 filter/sort。
    const grouped = new Map<string, Session[]>()
    const query = normalizedSearchQuery.value

    for (const session of sessions.value) {
      if (!shouldDisplaySession(session)) {
        continue
      }

      if (query) {
        const name = session.name.toLowerCase()
        const lastMessage = session.lastMessage?.toLowerCase() ?? ''
        if (!name.includes(query) && !lastMessage.includes(query)) {
          continue
        }
      }

      const projectSessions = grouped.get(session.projectId)
      if (projectSessions) {
        projectSessions.push(session)
      } else {
        grouped.set(session.projectId, [session])
      }
    }

    for (const projectSessions of grouped.values()) {
      projectSessions.sort((a, b) => {
        if (a.pinned && !b.pinned) return -1
        if (!a.pinned && b.pinned) return 1
        return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
      })
    }

    return grouped
  })

  // 获取打开的会话列表
  const openSessions = computed(() => {
    return openSessionIds.value
      .map(id => sessions.value.find(s => s.id === id))
      .filter((s): s is Session => s !== undefined && shouldDisplaySession(s))
  })

  const sessionsByProject = computed(() => {
    return (projectId: string, sortBy: 'updatedAt' | 'createdAt' = 'updatedAt') => {
      if (sortBy === 'updatedAt') {
        return projectSessionsByUpdatedAt.value.get(projectId) ?? EMPTY_SESSIONS
      }

      let filtered = sessions.value.filter(s => s.projectId === projectId)
      filtered = filtered.filter(shouldDisplaySession)

      // 搜索过滤
      if (searchQuery.value) {
        const query = searchQuery.value.toLowerCase()
        filtered = filtered.filter(s =>
          s.name.toLowerCase().includes(query) ||
          s.lastMessage?.toLowerCase().includes(query)
        )
      }

      // 固定的排在前面，然后按指定字段排序（降序，最新的在前面）
      return filtered.sort((a, b) => {
        if (a.pinned && !b.pinned) return -1
        if (!a.pinned && b.pinned) return 1
        const aTime = sortBy === 'createdAt'
          ? new Date(a.createdAt).getTime()
          : new Date(a.updatedAt).getTime()
        const bTime = sortBy === 'createdAt'
          ? new Date(b.createdAt).getTime()
          : new Date(b.updatedAt).getTime()
        return bTime - aTime
      })
    }
  })

  // 按项目和智能体筛选会话
  // agentFilter: 'all' 表示全部，其他值为智能体 ID
  const sessionsByProjectAndAgentType = computed(() => {
    return (projectId: string, agentFilter?: string | 'all', sortBy: 'updatedAt' | 'createdAt' = 'updatedAt') => {
      if ((!agentFilter || agentFilter === 'all') && sortBy === 'updatedAt') {
        return projectSessionsByUpdatedAt.value.get(projectId) ?? EMPTY_SESSIONS
      }

      let filtered = sessions.value.filter(s => s.projectId === projectId)
      filtered = filtered.filter(shouldDisplaySession)

      // 智能体筛选（根据智能体 ID）
      if (agentFilter && agentFilter !== 'all') {
        // agentFilter 是智能体 ID，会话的 agentId 字段存储了创建该会话的智能体 ID
        filtered = filtered.filter(s => s.agentId === agentFilter)
      }

      // 搜索过滤
      if (searchQuery.value) {
        const query = searchQuery.value.toLowerCase()
        filtered = filtered.filter(s =>
          s.name.toLowerCase().includes(query) ||
          s.lastMessage?.toLowerCase().includes(query)
        )
      }

      // 固定的排在前面，然后按指定字段排序（降序，最新的在前面）
      return filtered.sort((a, b) => {
        if (a.pinned && !b.pinned) return -1
        if (!a.pinned && b.pinned) return 1
        const aTime = sortBy === 'createdAt'
          ? new Date(a.createdAt).getTime()
          : new Date(a.updatedAt).getTime()
        const bTime = sortBy === 'createdAt'
          ? new Date(b.createdAt).getTime()
          : new Date(b.updatedAt).getTime()
        return bTime - aTime
      })
    }
  })

  function replaceProjectSessions(projectId: string, projectSessions: Session[]) {
    const otherSessions = sessions.value.filter(session => session.projectId !== projectId)
    sessions.value = [...otherSessions, ...projectSessions]
  }

  function pruneStaleOpenSessions() {
    const validSessionIds = new Set(
      sessions.value
        .filter(shouldDisplaySession)
        .map(session => session.id)
    )
    const nextOpenSessionIds = openSessionIds.value.filter(sessionId => validSessionIds.has(sessionId))

    if (nextOpenSessionIds.length !== openSessionIds.value.length) {
      openSessionIds.value = nextOpenSessionIds
      const appStateStore = useAppStateStore()
      appStateStore.setLastSessions([...openSessionIds.value])
    }

    if (currentSessionId.value && !validSessionIds.has(currentSessionId.value)) {
      currentSessionId.value = openSessionIds.value[0] ?? null
    }
  }

  // Actions
  async function loadSessions(projectId: string, options: { force?: boolean } = {}) {
    const { force = false } = options
    if (!force && loadedProjectIds.value.has(projectId)) {
      return
    }

    if (loadingProjectIds.value.has(projectId)) {
      return
    }

    isLoading.value = true
    loadError.value = null
    loadingProjectIds.value.add(projectId)
    const notificationStore = useNotificationStore()
    try {
      const rustSessions = await invoke<RustSession[]>('list_sessions', { projectId })
      replaceProjectSessions(projectId, rustSessions.map(transformSession))
      pruneStaleOpenSessions()
      loadedProjectIds.value.add(projectId)
    } catch (error) {
      console.error('Failed to load sessions:', error)
      loadError.value = getErrorMessage(error)
      notificationStore.networkError(
        '加载会话列表',
        getErrorMessage(error),
        () => loadSessions(projectId, { force: true })
      )
    } finally {
      loadingProjectIds.value.delete(projectId)
      isLoading.value = false
    }
  }

  async function createSession(session: Omit<Session, 'id' | 'createdAt' | 'updatedAt' | 'pinned' | 'messageCount' | 'lastMessage'>) {
    const notificationStore = useNotificationStore()
    const input = {
      project_id: session.projectId,
      name: session.name || null, // 如果为空，后端会生成默认名称
      expert_id: session.expertId ?? null,
      agent_id: session.agentId ?? null,
      agent_type: session.agentType,
      status: session.status || null
    }

    try {
      const rustSession = await invoke<RustSession>('create_session', { input })
      const newSession = transformSession(rustSession)
      sessions.value.unshift(newSession)
      return newSession
    } catch (error) {
      console.error('Failed to create session:', error)
      notificationStore.databaseError(
        '创建会话失败',
        getErrorMessage(error),
        async () => { await createSession(session) }
      )
      throw error
    }
  }

  async function updateSession(
    id: string,
    updates: Partial<Pick<Session, 'name' | 'status' | 'pinned' | 'lastMessage' | 'errorMessage' | 'agentType' | 'expertId' | 'agentId' | 'cliSessionId' | 'cliSessionProvider'>>
  ) {
    const notificationStore = useNotificationStore()
    const input: Record<string, unknown> = {}

    if ('name' in updates) input.name = updates.name ?? null
    if ('status' in updates) input.status = updates.status ?? null
    if ('pinned' in updates) input.pinned = updates.pinned ?? null
    if ('lastMessage' in updates) input.last_message = updates.lastMessage ?? null
    if ('errorMessage' in updates) input.error_message = updates.errorMessage ?? null
    if ('agentType' in updates) input.agent_type = updates.agentType ?? null
    if ('expertId' in updates) input.expert_id = updates.expertId ?? null
    if ('agentId' in updates) input.agent_id = updates.agentId ?? null
    if ('cliSessionId' in updates) input.cli_session_id = updates.cliSessionId ?? null
    if ('cliSessionProvider' in updates) input.cli_session_provider = updates.cliSessionProvider ?? null

    try {
      const rustSession = await invoke<RustSession>('update_session', { id, input })
      const updatedSession = transformSession(rustSession)

      const index = sessions.value.findIndex(s => s.id === id)
      if (index !== -1) {
        sessions.value[index] = updatedSession
      }

      return updatedSession
    } catch (error) {
      console.error('Failed to update session:', error)
      notificationStore.databaseError(
        '更新会话失败',
        getErrorMessage(error),
        async () => { await updateSession(id, updates) }
      )
      throw error
    }
  }

  async function deleteSession(id: string) {
    const notificationStore = useNotificationStore()

    try {
      await invoke('delete_session', { id })

      const index = sessions.value.findIndex(s => s.id === id)
      if (index !== -1) {
        sessions.value.splice(index, 1)
      }

      // 清理该会话的执行状态
      const sessionExecutionStore = useSessionExecutionStore()
      sessionExecutionStore.clearExecutionState(id)

      useMessageStore().clearSessionMessagesCache(id)

      useTokenStore().clearSessionTokenCache(id)

      useAiEditTraceStore().resetSession(id)

      // 从打开列表中移除
      const openIndex = openSessionIds.value.indexOf(id)
      if (openIndex !== -1) {
        openSessionIds.value.splice(openIndex, 1)
      }

      if (currentSessionId.value === id) {
        currentSessionId.value = null
      }
    } catch (error) {
      console.error('Failed to delete session:', error)
      notificationStore.databaseError(
        '删除会话失败',
        getErrorMessage(error),
        async () => { await deleteSession(id) }
      )
      throw error
    }
  }

  function setCurrentSession(id: string | null) {
    currentSessionId.value = id
  }

  async function clearProjectSessions(projectId: string) {
    const projectSessionIds = sessions.value
      .filter(session => session.projectId === projectId)
      .map(session => session.id)

    if (projectSessionIds.length === 0) {
      return
    }

    const projectSessionIdSet = new Set(projectSessionIds)
    const windowManager = useWindowManagerStore()
    await Promise.all(
      projectSessionIds.map(sessionId =>
        windowManager.releaseSession(sessionId).catch(console.error)
      )
    )

    sessions.value = sessions.value.filter(session => session.projectId !== projectId)
    openSessionIds.value = openSessionIds.value.filter(sessionId => !projectSessionIdSet.has(sessionId))
    loadedProjectIds.value.delete(projectId)
    loadingProjectIds.value.delete(projectId)
    pruneStaleOpenSessions()

    const sessionExecutionStore = useSessionExecutionStore()
    projectSessionIds.forEach(sessionId => sessionExecutionStore.clearExecutionState(sessionId))

    const appStateStore = useAppStateStore()
    appStateStore.setLastSessions([...openSessionIds.value])
  }

  async function togglePin(id: string) {
    const notificationStore = useNotificationStore()

    try {
      const rustSession = await invoke<RustSession>('toggle_session_pin', { id })
      const updatedSession = transformSession(rustSession)

      const index = sessions.value.findIndex(s => s.id === id)
      if (index !== -1) {
        sessions.value[index] = updatedSession
      }

      return updatedSession
    } catch (error) {
      console.error('Failed to toggle session pin:', error)
      notificationStore.databaseError(
        '切换会话固定状态失败',
        getErrorMessage(error),
        async () => { await togglePin(id) }
      )
      throw error
    }
  }

  function setSearchQuery(query: string) {
    searchQuery.value = query
  }

  function updateLastMessage(id: string, message: string) {
    const session = sessions.value.find(s => s.id === id)
    if (session) {
      session.lastMessage = message
      session.messageCount = (session.messageCount || 0) + 1
      session.updatedAt = new Date().toISOString()
    }
  }

  // 打开会话（添加到标签栏）
  async function openSession(sessionId: string): Promise<boolean> {
    // 检查会话是否在其他窗口中打开
    const windowManager = useWindowManagerStore()
    const lockedBy = await windowManager.isSessionLocked(sessionId)

    if (lockedBy && lockedBy !== windowManager.windowLabel) {
      // 会话在其他窗口中打开，不允许在此窗口打开
      console.warn(`Session ${sessionId} is locked by window ${lockedBy}`)
      return false
    }

    // 如果已经打开，只切换
    if (openSessionIds.value.includes(sessionId)) {
      currentSessionId.value = sessionId
      return true
    }

    // 检查是否达到最大数量
    if (openSessionIds.value.length >= MAX_OPEN_SESSIONS) {
      // 关闭最早打开的会话（列表第一个）
      const closedSessionId = openSessionIds.value[0]
      // 释放会话锁定
      windowManager.releaseSession(closedSessionId).catch(console.error)
      openSessionIds.value.shift()
    }

    // 添加到打开列表
    openSessionIds.value.push(sessionId)
    currentSessionId.value = sessionId

    // 锁定会话到当前窗口
    await windowManager.lockSession(sessionId)

    // 更新应用状态
    const appStateStore = useAppStateStore()
    appStateStore.setLastSessions([...openSessionIds.value])

    return true
  }

  // 关闭会话（从标签栏移除）
  function closeSession(sessionId: string) {
    const index = openSessionIds.value.indexOf(sessionId)
    if (index === -1) return

    openSessionIds.value.splice(index, 1)

    // 释放会话锁定
    const windowManager = useWindowManagerStore()
    windowManager.releaseSession(sessionId).catch(console.error)

    // 清理该会话的执行状态
    const sessionExecutionStore = useSessionExecutionStore()
    sessionExecutionStore.clearExecutionState(sessionId)

    // 更新应用状态
    const appStateStore = useAppStateStore()
    appStateStore.setLastSessions([...openSessionIds.value])

    // 如果关闭的是当前会话，切换到相邻的会话
    if (currentSessionId.value === sessionId) {
      if (openSessionIds.value.length > 0) {
        // 切换到关闭会话的下一个，如果没有下一个则切换到上一个
        const newIndex = Math.min(index, openSessionIds.value.length - 1)
        currentSessionId.value = openSessionIds.value[newIndex]
      } else {
        currentSessionId.value = null
      }
    }
  }

  // 检查会话是否已打开
  function isSessionOpen(sessionId: string): boolean {
    return openSessionIds.value.includes(sessionId)
  }

  // 从 localStorage 加载打开的会话列表
  function loadOpenSessions() {
    try {
      const saved = localStorage.getItem('ea-open-sessions')
      if (saved) {
        const parsed = JSON.parse(saved)
        if (Array.isArray(parsed)) {
          openSessionIds.value = parsed.slice(0, MAX_OPEN_SESSIONS)
        }
      }
    } catch {
      // ignore
    }
  }

  // 保存打开的会话列表到 localStorage
  function saveOpenSessions() {
    try {
      localStorage.setItem('ea-open-sessions', JSON.stringify(openSessionIds.value))
    } catch {
      // ignore
    }
  }

  // 监听 openSessionIds 变化并自动保存
  watch(openSessionIds, () => {
    saveOpenSessions()
  }, { deep: true })

  return {
    // State
    sessions,
    currentSessionId,
    isLoading,
    loadError,
    searchQuery,
    openSessionIds,
    loadedProjectIds,
    loadingProjectIds,
    // Getters
    currentSession,
    sessionsByProject,
    sessionsByProjectAndAgentType,
    openSessions,
    // Actions
    loadSessions,
    createSession,
    updateSession,
    deleteSession,
    clearProjectSessions,
    setCurrentSession,
    togglePin,
    setSearchQuery,
    updateLastMessage,
    // 多会话管理
    openSession,
    closeSession,
    isSessionOpen,
    loadOpenSessions,
    saveOpenSessions
  }
})
