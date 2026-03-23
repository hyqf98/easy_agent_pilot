<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
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
import { useConfirmDialog, useWindowEvents } from './composables'
import { useMiniPanelShortcut } from './composables/useMiniPanelShortcut'
import { createMockUpdaterAdapter } from './services/appUpdate'
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
const confirmDialog = useConfirmDialog()
const confirmDialogState = confirmDialog.state

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
              currentVersion: '1.0.0',
              availableUpdate: {
                version: '1.1.0',
                publishedAt: '2026-03-22T12:00:00Z',
                notes: 'Mock release notes from Tauri MCP regression.'
              },
              relaunchAfterInstall: false
            }))
            break
          case 'check-failed':
            await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
              currentVersion: '1.0.0',
              checkError: 'Mock check failure'
            }))
            break
          case 'install-failed':
            await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
              currentVersion: '1.0.0',
              availableUpdate: {
                version: '1.1.0',
                publishedAt: '2026-03-22T12:00:00Z',
                notes: 'Mock release notes from Tauri MCP regression.'
              },
              installError: 'Mock install failure'
            }))
            break
          case 'none':
            await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
              currentVersion: '1.0.0',
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
      title: '检测到中断的计划执行',
      message: `计划“${plan.name}”在 ${new Date(plan.updatedAt).toLocaleString('zh-CN')} 前仍有执行中的任务。是否跳转到计划面板继续查看？`,
      confirmLabel: '继续查看',
      cancelLabel: '暂不处理',
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
  await windowManagerStore.initWindowContext()
  await themeStore.loadTheme()
  await settingsStore.loadSettings()

  if (!windowManagerStore.isMiniPanelWindow) {
    await windowStateStore.initWindowState()
  }

  if (windowManagerStore.isMainWindow) {
    await appStateStore.loadState()
    await appUpdateStore.initialize()
  }

  if (!windowManagerStore.isMiniPanelWindow) {
    await projectStore.loadProjects()

    if (windowManagerStore.projectId) {
      projectStore.setCurrentProject(windowManagerStore.projectId)
    } else if (windowManagerStore.isMainWindow && appStateStore.lastProjectId) {
      const projectExists = projectStore.projects.some(
        project => project.id === appStateStore.lastProjectId
      )

      if (projectExists) {
        projectStore.setCurrentProject(appStateStore.lastProjectId)

        for (const sessionId of appStateStore.lastSessionIds) {
          await sessionStore.openSession(sessionId)
        }
      }
    }
  }

  await promptInterruptedPlanRecovery(projectStore.currentProjectId)

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
