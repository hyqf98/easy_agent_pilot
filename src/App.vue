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
import { useAppStateStore } from './stores/appState'
import { useConfirmDialog, useWindowEvents } from './composables'
import { SettingsModal } from './components/settings'
import { EaToast, EaLoadingOverlay, EaConfirmDialog } from './components/common'

const themeStore = useThemeStore()
const settingsStore = useSettingsStore()
const uiStore = useUIStore()
const windowStateStore = useWindowStateStore()
const sessionStore = useSessionStore()
const projectStore = useProjectStore()
const windowManagerStore = useWindowManagerStore()
const appStateStore = useAppStateStore()
const confirmDialog = useConfirmDialog()
const confirmDialogState = confirmDialog.state

// 初始化窗口事件监听
useWindowEvents()

const handleKeydown = (event: KeyboardEvent) => {
  // Cmd/Ctrl + N: 新建项目
  if ((event.metaKey || event.ctrlKey) && event.key === 'n') {
    event.preventDefault()
    uiStore.openProjectCreateModal()
  }
  // Cmd/Ctrl + T: 新建会话
  if ((event.metaKey || event.ctrlKey) && event.key === 't') {
    event.preventDefault()
    uiStore.openSessionCreateModal()
  }
  // Cmd/Ctrl + 1-5: 切换会话标签
  if ((event.metaKey || event.ctrlKey) && event.key >= '1' && event.key <= '5') {
    event.preventDefault()
    const index = parseInt(event.key) - 1
    const openSessions = sessionStore.openSessions
    if (index < openSessions.length) {
      sessionStore.setCurrentSession(openSessions[index].id)
    }
  }
}

onMounted(async () => {
  // 初始化窗口上下文
  await windowManagerStore.initWindowContext()

  // 初始化窗口状态（恢复窗口位置和大小）
  await windowStateStore.initWindowState()

  // 初始化主题
  await themeStore.loadTheme()

  // 加载应用设置
  await settingsStore.loadSettings()

  // 加载应用状态（如果是主窗口）
  if (windowManagerStore.isMainWindow) {
    await appStateStore.loadState()
  }

  // 加载项目列表
  await projectStore.loadProjects()

  // 如果是项目窗口，直接打开指定项目
  if (windowManagerStore.projectId) {
    projectStore.setCurrentProject(windowManagerStore.projectId)
  }
  // 如果是主窗口且有上次的项目，恢复状态
  else if (windowManagerStore.isMainWindow && appStateStore.lastProjectId) {
    const projectExists = projectStore.projects.some(
      p => p.id === appStateStore.lastProjectId
    )
    if (projectExists) {
      projectStore.setCurrentProject(appStateStore.lastProjectId)

      // 恢复上次的会话
      for (const sessionId of appStateStore.lastSessionIds) {
        await sessionStore.openSession(sessionId)
      }
    }
  }

  // 添加全局快捷键监听
  window.addEventListener('keydown', handleKeydown)
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
