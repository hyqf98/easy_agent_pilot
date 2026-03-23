import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { getErrorMessage } from '@/utils/api'
import { useNotificationStore } from './notification'
import { useSettingsStore } from './settings'
import { useUIStore } from './ui'
import { AppUpdateService, TauriUpdaterAdapter } from '@/services/appUpdate'
import type {
  AppUpdateHandle,
  AppUpdateInfo,
  AppUpdateProgress,
  AppUpdateStatus,
  AppUpdaterAdapter
} from '@/types/appUpdate'

function buildDefaultAdapter(): AppUpdaterAdapter {
  return new TauriUpdaterAdapter()
}

/**
 * useAppUpdateStore 负责应用版本检查、安装进度和设置页展示状态。
 */
export const useAppUpdateStore = defineStore('appUpdate', () => {
  const notificationStore = useNotificationStore()
  const settingsStore = useSettingsStore()
  const uiStore = useUIStore()

  const currentVersion = ref<string>('...')
  const status = ref<AppUpdateStatus>('idle')
  const availableUpdate = ref<AppUpdateInfo | null>(null)
  const progress = ref<AppUpdateProgress | null>(null)
  const errorMessage = ref<string | null>(null)
  const hasInitialized = ref(false)

  let currentHandle: AppUpdateHandle | null = null
  let lastAnnouncedVersion: string | null = null
  let serviceFactory: () => AppUpdaterAdapter = buildDefaultAdapter
  let service: AppUpdateService | null = null

  const isBusy = computed(() =>
    status.value === 'checking'
    || status.value === 'downloading'
    || status.value === 'installing'
  )

  const lastCheckedAt = computed(() => settingsStore.settings.appUpdateLastCheckedAt)

  function getService(): AppUpdateService {
    service ??= new AppUpdateService(serviceFactory())
    return service
  }

  async function replaceHandle(nextHandle: AppUpdateHandle | null): Promise<void> {
    if (currentHandle && currentHandle !== nextHandle) {
      try {
        await currentHandle.close()
      } catch (error) {
        console.error('Failed to close updater handle:', error)
      }
    }

    currentHandle = nextHandle
  }

  function resetTransientState(nextStatus: AppUpdateStatus = 'idle'): void {
    status.value = nextStatus
    progress.value = null
    errorMessage.value = null
  }

  async function persistLastCheckedAt(value: string): Promise<void> {
    await settingsStore.updateSettings({
      appUpdateLastCheckedAt: value
    })
  }

  function notifyAvailableUpdate(version: string): void {
    if (lastAnnouncedVersion === version) {
      return
    }

    lastAnnouncedVersion = version
    notificationStore.show({
      type: 'info',
      title: `发现新版本 v${version}`,
      message: '打开设置页即可查看更新说明并开始安装。',
      retryAction: () => {
        uiStore.openSettings('appUpdate')
      },
      retryLabel: '立即查看'
    })
  }

  /**
   * 初始化当前应用版本，仅在主窗口启动后执行一次。
   */
  async function initialize(): Promise<void> {
    if (hasInitialized.value) {
      return
    }

    if (!getService().isSupported()) {
      resetTransientState('unsupported')
      hasInitialized.value = true
      return
    }

    try {
      currentVersion.value = await getService().getCurrentVersion()
      resetTransientState(status.value === 'unsupported' ? 'idle' : status.value)
    } catch (error) {
      errorMessage.value = getErrorMessage(error)
      status.value = 'error'
    } finally {
      hasInitialized.value = true
    }
  }

  /**
   * 根据用户设置执行静默启动检查。
   */
  async function runStartupCheck(): Promise<void> {
    if (!settingsStore.settings.autoCheckAppUpdate) {
      return
    }

    await checkForUpdates({ silent: true })
  }

  /**
   * 检查远端是否存在新版本。
   */
  async function checkForUpdates(options: { silent?: boolean } = {}): Promise<AppUpdateInfo | null> {
    const silent = options.silent ?? false

    await initialize()

    if (!getService().isSupported()) {
      status.value = 'unsupported'
      if (!silent) {
        notificationStore.warning('当前环境不支持自动升级', '该功能仅在 Tauri 桌面应用中可用。')
      }
      return null
    }

    if (isBusy.value) {
      return availableUpdate.value
    }

    progress.value = null
    errorMessage.value = null
    status.value = 'checking'

    try {
      const result = await getService().checkForUpdates()
      currentVersion.value = result.currentVersion
      await persistLastCheckedAt(result.checkedAt)

      if (!result.update) {
        await replaceHandle(null)
        availableUpdate.value = null
        status.value = 'up-to-date'

        if (!silent) {
          notificationStore.success('已是最新版本', `当前版本 v${result.currentVersion} 无需升级。`)
        }

        return null
      }

      await replaceHandle(result.update)
      availableUpdate.value = result.update.metadata
      status.value = 'available'

      if (silent) {
        notifyAvailableUpdate(result.update.metadata.version)
      } else {
        notificationStore.info(
          `检测到新版本 v${result.update.metadata.version}`,
          '更新已准备就绪，可以在当前页面直接安装。'
        )
      }

      return availableUpdate.value
    } catch (error) {
      errorMessage.value = getErrorMessage(error)
      status.value = 'error'

      if (!silent) {
        notificationStore.error('检查更新失败', errorMessage.value)
      }

      return null
    }
  }

  /**
   * 下载并安装当前检测到的更新。
   */
  async function installUpdate(): Promise<boolean> {
    if (!currentHandle || !availableUpdate.value) {
      notificationStore.warning('暂无可安装更新', '请先执行一次更新检查。')
      return false
    }

    errorMessage.value = null
    progress.value = {
      contentLength: null,
      downloadedBytes: 0,
      percent: null
    }
    status.value = 'downloading'

    try {
      const result = await getService().installUpdate(currentHandle, (event) => {
        if (!progress.value) {
          progress.value = {
            contentLength: null,
            downloadedBytes: 0,
            percent: null
          }
        }

        switch (event.event) {
          case 'Started':
            progress.value = {
              contentLength: event.contentLength,
              downloadedBytes: 0,
              percent: event.contentLength ? 0 : null
            }
            status.value = 'downloading'
            break
          case 'Progress': {
            const downloadedBytes = progress.value.downloadedBytes + event.chunkLength
            const contentLength = progress.value.contentLength
            progress.value = {
              contentLength,
              downloadedBytes,
              percent: contentLength && contentLength > 0
                ? Math.min(100, Math.round((downloadedBytes / contentLength) * 100))
                : null
            }
            status.value = 'downloading'
            break
          }
          case 'Finished':
            progress.value = {
              contentLength: progress.value.contentLength,
              downloadedBytes: progress.value.contentLength ?? progress.value.downloadedBytes,
              percent: 100
            }
            status.value = 'installing'
            break
        }
      })

      status.value = 'completed'

      if (result.relaunchTriggered) {
        notificationStore.info('更新安装完成', '应用将立即重启以完成升级。')
      } else {
        notificationStore.success('更新安装完成', '安装器已启动，请按系统提示完成升级。')
      }

      return true
    } catch (error) {
      errorMessage.value = getErrorMessage(error)
      status.value = 'error'
      notificationStore.error('安装更新失败', errorMessage.value)
      return false
    }
  }

  /**
   * 测试专用：注入 mock adapter，便于单测和 Tauri MCP 场景回归。
   */
  async function __setAdapterFactoryForTesting(factory: () => AppUpdaterAdapter): Promise<void> {
    serviceFactory = factory
    service = null
    hasInitialized.value = false
    currentVersion.value = '...'
    availableUpdate.value = null
    lastAnnouncedVersion = null
    resetTransientState('idle')
    await replaceHandle(null)
  }

  /**
   * 测试专用：恢复默认 Tauri adapter。
   */
  async function __restoreDefaultAdapterFactory(): Promise<void> {
    await __setAdapterFactoryForTesting(buildDefaultAdapter)
  }

  return {
    currentVersion,
    status,
    availableUpdate,
    progress,
    errorMessage,
    hasInitialized,
    isBusy,
    lastCheckedAt,
    initialize,
    runStartupCheck,
    checkForUpdates,
    installUpdate,
    __setAdapterFactoryForTesting,
    __restoreDefaultAdapterFactory
  }
})
