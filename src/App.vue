<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { NMessageProvider } from 'naive-ui'
import { useThemeStore } from './stores/theme'
import { useSettingsStore } from './stores/settings'
import { useUIStore } from './stores/ui'
import { useWindowStateStore } from './stores/windowState'
import { useSessionStore } from './stores/session'
import { useProjectStore } from './stores/project'
import { useWindowManagerStore } from './stores/windowManager'
import { useAppUpdateStore } from './stores/appUpdate'
import { useAppStateStore } from './stores/appState'
import { usePlanStore } from './stores/plan'
import { useTaskStore } from './stores/task'
import { useTaskExecutionStore } from './stores/taskExecution'
import { useUnattendedStore } from './stores/unattended'
import { useNotificationStore } from './stores/notification'
import { useConfirmDialog, useWindowEvents } from './composables'
import { useMiniPanelShortcut } from './composables/useMiniPanelShortcut'
import { createMockUpdaterAdapter } from './services/appUpdate'
import { readCrashLog, writeCrashLog, clearCrashLog } from './services/runtimeLog/crashLog'
import { SettingsModal } from './components/settings'
import { EaToast, EaLoadingOverlay, EaConfirmDialog } from './components/common'

const themeStore = useThemeStore()
const settingsStore = useSettingsStore()
const uiStore = useUIStore()
const windowStateStore = useWindowStateStore()
const sessionStore = useSessionStore()
const projectStore = useProjectStore()
const windowManagerStore = useWindowManagerStore()
const appUpdateStore = useAppUpdateStore()
const appStateStore = useAppStateStore()
const planStore = usePlanStore()
const taskStore = useTaskStore()
const taskExecutionStore = useTaskExecutionStore()
const unattendedStore = useUnattendedStore()
const notificationStore = useNotificationStore()
const confirmDialog = useConfirmDialog()
const confirmDialogState = confirmDialog.state
const { t, locale } = useI18n()

function installGlobalCrashHandlers() {
  const originalOnError = window.onerror
  window.onerror = (event, source, lineno, colno, error) => {
    const message = typeof event === 'string' ? event : String(event)
    const location = source ? `${source}:${lineno}:${colno}` : 'unknown'
    const stackTrace = error?.stack ?? undefined
    void writeCrashLog('js-error', `${message}\nLocation: ${location}`, stackTrace)
    if (originalOnError) {
      return originalOnError(event, source, lineno, colno, error)
    }
    return false
  }

  window.addEventListener('unhandledrejection', (event: PromiseRejectionEvent) => {
    const reason = event.reason
    const message = reason instanceof Error ? reason.message : String(reason)
    const stackTrace = reason instanceof Error ? reason.stack : undefined
    void writeCrashLog('js-unhandled-rejection', message, stackTrace)
  })
}

async function checkAndNotifyCrashOnStartup() {
  if (!windowManagerStore.isMainWindow) {
    return
  }

  try {
    const status = await readCrashLog()
    if (!status.hasCrashLog || status.entries.length === 0) {
      return
    }

    const latestEntry = status.entries[status.entries.length - 1]
    const sourceLabel = latestEntry.source === 'rust-panic'
      ? t('crashNotification.rustPanic')
      : t('crashNotification.jsError')
    const timeLabel = latestEntry.timestamp
      ? new Date(latestEntry.timestamp).toLocaleString(locale.value === 'zh-CN' ? 'zh-CN' : 'en-US')
      : ''

    notificationStore.warning(
      t('crashNotification.title'),
      t('crashNotification.message', {
        source: sourceLabel,
        time: timeLabel,
        detail: latestEntry.message.slice(0, 200)
      })
    )

    await clearCrashLog()
  } catch {
    // crash log is best-effort, never block startup
  }
}

useWindowEvents()
useMiniPanelShortcut()

const handleKeydown = (event: KeyboardEvent) => {
  if ((event.metaKey || event.ctrlKey) && event.key === 'n') {
    event.preventDefault()
    uiStore.openProjectCreateModal()
  }

  if ((event.metaKey || event.ctrlKey) && event.key === 't') {
    event.preventDefault()
    uiStore.openSessionCreateModal()
  }

  if ((event.metaKey || event.ctrlKey) && event.key >= '1' && event.key <= '5') {
    event.preventDefault()
    const index = parseInt(event.key) - 1
    const openSessions = sessionStore.openSessions
    if (index < openSessions.length) {
      sessionStore.setCurrentSession(openSessions[index].id)
    }
  }
}

let hasCheckedInterruptedPlans = false

function registerDevAppUpdateHooks() {
  if (!import.meta.env.DEV) {
    return
  }

  const globalWindow = window as Window & {
    __EASY_AGENT_PILOT_TEST_HOOKS__?: Record<string, unknown>
  }

  globalWindow.__EASY_AGENT_PILOT_TEST_HOOKS__ = {
    ...(globalWindow.__EASY_AGENT_PILOT_TEST_HOOKS__ || {}),
    appUpdate: {
      async useScenario(name: string) {
        switch (name) {
          case 'available':
            await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
              currentVersion: '1.2.1',
              availableUpdate: {
                version: '1.2.1',
                publishedAt: '2026-03-22T12:00:00Z',
                notes: 'Mock release notes from Tauri MCP regression.'
              },
              relaunchAfterInstall: false
            }))
            break
          case 'check-failed':
            await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
              currentVersion: '1.2.1',
              checkError: 'Mock check failure'
            }))
            break
          case 'install-failed':
            await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
              currentVersion: '1.2.1',
              availableUpdate: {
                version: '1.2.1',
                publishedAt: '2026-03-22T12:00:00Z',
                notes: 'Mock release notes from Tauri MCP regression.'
              },
              installError: 'Mock install failure'
            }))
            break
          case 'none':
            await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
              currentVersion: '1.2.1',
              availableUpdate: null
            }))
            break
          default:
            await appUpdateStore.__restoreDefaultAdapterFactory()
            break
        }

        await appUpdateStore.initialize()
        return {
          scenario: name,
          status: appUpdateStore.status,
          currentVersion: appUpdateStore.currentVersion
        }
      },
      async check() {
        await appUpdateStore.checkForUpdates()
        return {
          status: appUpdateStore.status,
          availableUpdate: appUpdateStore.availableUpdate,
          errorMessage: appUpdateStore.errorMessage
        }
      },
      async install() {
        const success = await appUpdateStore.installUpdate()
        return {
          success,
          status: appUpdateStore.status,
          progress: appUpdateStore.progress,
          errorMessage: appUpdateStore.errorMessage
        }
      },
      async restore() {
        await appUpdateStore.__restoreDefaultAdapterFactory()
        return true
      },
      getState() {
        return {
          status: appUpdateStore.status,
          currentVersion: appUpdateStore.currentVersion,
          availableUpdate: appUpdateStore.availableUpdate,
          progress: appUpdateStore.progress,
          errorMessage: appUpdateStore.errorMessage
        }
      }
    }
  }
}

async function promptInterruptedPlanRecovery(projectId: string | null) {
  if (
    hasCheckedInterruptedPlans
    || !windowManagerStore.isMainWindow
    || !projectId
  ) {
    return
  }

  hasCheckedInterruptedPlans = true
  await planStore.loadPlans(projectId)

  const candidatePlans = [...planStore.plans]
    .filter(plan =>
      plan.projectId === projectId
      && (plan.status === 'executing' || plan.executionStatus === 'running')
    )
    .sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())

  for (const plan of candidatePlans) {
    const progress = await taskExecutionStore.getPlanExecutionProgress(plan.id)
    if ((progress?.in_progress_count ?? 0) <= 0) {
      continue
    }

    const confirmed = await confirmDialog.show({
      type: 'info',
      title: t('planRecovery.title'),
      message: t('planRecovery.message', {
        name: plan.name,
        time: new Date(plan.updatedAt).toLocaleString(locale.value === 'zh-CN' ? 'zh-CN' : 'en-US')
      }),
      confirmLabel: t('planRecovery.confirm'),
      cancelLabel: t('planRecovery.cancel'),
      confirmButtonType: 'primary'
    })

    if (confirmed) {
      projectStore.setCurrentProject(projectId)
      uiStore.setAppMode('plan')
      planStore.setCurrentPlan(plan.id)
      await taskStore.loadTasks(plan.id)
    }

    return
  }
}

onMounted(async () => {
  installGlobalCrashHandlers()

  await windowManagerStore.initWindowContext()
  await themeStore.loadTheme()
  await settingsStore.loadSettings()

  if (!windowManagerStore.isMiniPanelWindow) {
    await windowStateStore.initWindowState()
  }

  if (windowManagerStore.isMainWindow) {
    await appStateStore.loadState()
    await appUpdateStore.initialize()
    await unattendedStore.initialize()
  }

  if (!windowManagerStore.isMiniPanelWindow) {
    await projectStore.loadProjects()

    let restoredProjectId: string | null = null

    if (windowManagerStore.projectId) {
      restoredProjectId = windowManagerStore.projectId
    } else if (windowManagerStore.isMainWindow) {
      const candidateProjectId = appStateStore.lastProjectId ?? projectStore.currentProjectId
      const projectExists = candidateProjectId
        ? projectStore.projects.some(project => project.id === candidateProjectId)
        : false

      if (projectExists) {
        restoredProjectId = candidateProjectId
      }
    }

    if (restoredProjectId) {
      projectStore.setCurrentProject(restoredProjectId)
    }

    if (windowManagerStore.isMainWindow && restoredProjectId) {
      await sessionStore.loadSessions(restoredProjectId, { force: true })
      const preferredSessionId = appStateStore.lastActiveSessionId

      for (const sessionId of appStateStore.lastSessionIds) {
        await sessionStore.openSession(sessionId)
      }

      const hasPreferredSession = preferredSessionId
        ? sessionStore.openSessionIds.includes(preferredSessionId)
        : false

      if (hasPreferredSession) {
        sessionStore.setCurrentSession(preferredSessionId)
      } else if (!sessionStore.currentSessionId && sessionStore.openSessionIds.length > 0) {
        sessionStore.setCurrentSession(
          sessionStore.openSessionIds[sessionStore.openSessionIds.length - 1] ?? null
        )
      }
    }
  }

  await promptInterruptedPlanRecovery(projectStore.currentProjectId)

  await checkAndNotifyCrashOnStartup()

  if (windowManagerStore.isMainWindow) {
    registerDevAppUpdateHooks()
    await appUpdateStore.runStartupCheck()
  }

  if (!windowManagerStore.isMiniPanelWindow) {
    window.addEventListener('keydown', handleKeydown)
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  unattendedStore.dispose()
})
</script>

<template>
  <n-message-provider>
    <div class="app-container">
      <RouterView />
      <SettingsModal />
      <EaToast />
      <EaLoadingOverlay />
      <EaConfirmDialog
        :visible="confirmDialogState.visible"
        :type="confirmDialogState.type"
        :title="confirmDialogState.title"
        :message="confirmDialogState.message"
        :confirm-label="confirmDialogState.confirmLabel"
        :cancel-label="confirmDialogState.cancelLabel"
        :confirm-button-type="confirmDialogState.confirmButtonType"
        @confirm="confirmDialog.handleConfirm"
        @cancel="confirmDialog.handleCancel"
        @update:visible="confirmDialog.handleVisibleChange"
      />
    </div>
  </n-message-provider>
</template>

<style scoped>
.app-container {
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>
