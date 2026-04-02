import { useI18n } from 'vue-i18n'
import { useLayoutStore } from '@/stores/layout'
import { usePlanStore } from '@/stores/plan'
import { useProjectStore } from '@/stores/project'
import { useSessionStore, type SessionStatus } from '@/stores/session'
import { useTaskStore } from '@/stores/task'
import { useUIStore } from '@/stores/ui'

interface OpenSessionTargetOptions {
  onBeforeOpen?: () => void
}

export function useSessionView() {
  const { t } = useI18n()
  const sessionStore = useSessionStore()
  const projectStore = useProjectStore()
  const layoutStore = useLayoutStore()
  const uiStore = useUIStore()
  const taskStore = useTaskStore()
  const planStore = usePlanStore()

  async function openSessionTarget(id: string, options: OpenSessionTargetOptions = {}) {
    options.onBeforeOpen?.()

    const session = sessionStore.sessions.find(item => item.id === id)
    if (session?.projectId) {
      projectStore.setCurrentProject(session.projectId)
      projectStore.expandProject(session.projectId)
      layoutStore.setProjectTab(session.projectId, 'sessions')
      await sessionStore.loadSessions(session.projectId)
    }

    if (session?.agentType === 'planner') {
      if (session.projectId) {
        await planStore.loadPlans(session.projectId)
      }
      uiStore.setAppMode('plan')
      return
    }

    const task = await taskStore.getTaskBySessionId(id)
    if (task?.planId) {
      if (session?.projectId && planStore.plansByProject(session.projectId).length === 0) {
        await planStore.loadPlans(session.projectId)
      }
      planStore.setCurrentPlan(task.planId)
      await taskStore.loadTasks(task.planId)
      uiStore.setAppMode('plan')
      return
    }

    sessionStore.openSession(id)
  }

  function getStatusIcon(status: SessionStatus) {
    switch (status) {
      case 'running': return 'loader'
      case 'completed': return 'check-circle'
      case 'error': return 'alert-circle'
      case 'paused': return 'pause-circle'
      default: return 'circle'
    }
  }

  function getStatusText(status: SessionStatus) {
    switch (status) {
      case 'running': return t('session.statusRunning')
      case 'completed': return t('session.statusCompleted')
      case 'error': return t('session.statusError')
      case 'paused': return t('session.statusPaused')
      default: return t('session.statusIdle')
    }
  }

  function getStatusClass(status: SessionStatus) {
    return `session-item__status--${status}`
  }

  function isRunningStatus(status: SessionStatus) {
    return status === 'running'
  }

  function formatRelativeTime(dateStr: string) {
    const date = new Date(dateStr)
    const now = new Date()
    const diff = now.getTime() - date.getTime()
    const minutes = Math.floor(diff / 60000)
    const hours = Math.floor(diff / 3600000)
    const days = Math.floor(diff / 86400000)

    if (minutes < 1) return t('common.justNow')
    if (minutes < 60) return t('common.minutesAgo', { n: minutes })
    if (hours < 24) return t('common.hoursAgo', { n: hours })
    if (days < 7) return t('common.daysAgo', { n: days })
    return date.toLocaleDateString()
  }

  function formatSessionCreatedAt(dateStr: string) {
    const date = new Date(dateStr)
    const now = new Date()
    const isToday = date.toDateString() === now.toDateString()
    const yesterday = new Date(now)
    yesterday.setDate(yesterday.getDate() - 1)
    const isYesterday = date.toDateString() === yesterday.toDateString()
    const timeStr = date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })

    if (isToday) {
      return `${t('unified.today')} ${timeStr}`
    }
    if (isYesterday) {
      return `${t('unified.yesterday')} ${timeStr}`
    }
    if (date.getFullYear() === now.getFullYear()) {
      return date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' }) + ` ${timeStr}`
    }
    return date.toLocaleDateString('zh-CN') + ` ${timeStr}`
  }

  return {
    openSessionTarget,
    getStatusIcon,
    getStatusText,
    getStatusClass,
    isRunningStatus,
    formatRelativeTime,
    formatSessionCreatedAt
  }
}
