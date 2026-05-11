import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { setLocale } from '@/i18n'
import {
  defaultSettings,
  parseStoredSettings,
  serializeSettings,
  type AppSettings
} from './settingsSchema'

// CLI 相关类型定义
export type CliStatus = 'available' | 'not_found' | 'error'

export interface CliTool {
  name: string
  command: string
  version: string | null
  status: CliStatus
}

export interface DetectionResult {
  tools: CliTool[]
  total_found: number
}

// Install operation type
export type InstallOperationType = 'create_file' | 'create_dir' | 'modify_file' | 'delete_file'

// Install operation record
export interface InstallOperation {
  sequence: number
  operation_type: InstallOperationType
  target_path: string
  backup_path: string | null
  timestamp: string
}

// Install session
export interface InstallSession {
  id: string
  backup_dir: string
  operations: InstallOperation[]
  status: 'active' | 'rolling_back' | 'rolled_back' | 'rollback_failed' | 'completed' | 'cancelled' | 'cancel_rollback_failed'
  created_at: string
  error_message: string | null
}

// Install result
export interface InstallResult {
  success: boolean
  message: string
  session_id: string | null
  rollback_performed: boolean
  rollback_error: string | null
  backup_location: string | null
}

export type { AppSettings } from './settingsSchema'

export const useSettingsStore = defineStore('settings', () => {
  // State
  const settings = ref<AppSettings>({ ...defaultSettings })
  const isLoading = ref(false)
  const hasLoaded = ref(false)

  // 监听语言设置变化，同步到 i18n
  watch(
    () => settings.value.language,
    (newLang) => {
      setLocale(newLang as 'zh-CN' | 'en-US')
    },
    { immediate: true }
  )

  // 应用字体大小到 CSS 变量
  const applyFontSize = (size: number) => {
    const root = document.documentElement
    // 基于基准字体大小计算缩放比例
    const baseFontSize = 14
    const ratio = size / baseFontSize
    // 设置基准字体大小
    root.style.setProperty('--font-size-base', `${size}px`)
    // 根据缩放比例调整其他字体大小
    root.style.setProperty('--font-size-xs', `${Math.round(12 * ratio)}px`)
    root.style.setProperty('--font-size-sm', `${Math.round(13 * ratio)}px`)
    root.style.setProperty('--font-size-lg', `${Math.round(16 * ratio)}px`)
    root.style.setProperty('--font-size-xl', `${Math.round(18 * ratio)}px`)
    root.style.setProperty('--font-size-2xl', `${Math.round(20 * ratio)}px`)
    root.style.setProperty('--font-size-3xl', `${Math.round(24 * ratio)}px`)
    root.style.setProperty('--font-size-4xl', `${Math.round(30 * ratio)}px`)
  }

  // 监听字体大小变化，应用到界面
  watch(
    () => settings.value.fontSize,
    (newSize) => {
      applyFontSize(newSize)
    },
    { immediate: true }
  )

  // 自动保存设置到数据库（深度监听所有设置变化）
  watch(
    settings,
    async (newSettings) => {
      // 跳过加载期间的保存
      if (isLoading.value) return

      // 保存到数据库
      try {
        const settingsToSave = serializeSettings(newSettings)
        await invoke('save_app_settings', { settings: settingsToSave })
      } catch (error) {
        console.error('Failed to auto-save settings:', error)
      }
    },
    { deep: true }
  )

  // Pending Install Sessions state
  const pendingInstallSessions = ref<InstallSession[]>([])
  const isLoadingPendingSessions = ref(false)
  const pendingSessionsError = ref<string | null>(null)

  // Getters
  const isDebugMode = computed(() => settings.value.enableDebugMode)

  // Actions
  async function loadSettings() {
    isLoading.value = true
    hasLoaded.value = false
    try {
      const savedSettings = await invoke<Record<string, string>>('get_all_app_settings')
      if (savedSettings && Object.keys(savedSettings).length > 0) {
        const parsedSettings = parseStoredSettings(savedSettings)
        settings.value = { ...defaultSettings, ...parsedSettings }

        // 确保语言设置同步到 i18n
        if (parsedSettings.language) {
          setLocale(parsedSettings.language as 'zh-CN' | 'en-US')
        }
      }
    } catch (error) {
      console.error('Failed to load settings:', error)
      // 加载失败时使用默认设置
    } finally {
      isLoading.value = false
      hasLoaded.value = true
    }
  }

  async function updateSettings(updates: Partial<AppSettings>) {
    settings.value = {
      ...settings.value,
      ...updates
    }
    // 保存到数据库
    try {
      const settingsToSave = serializeSettings(updates)
      await invoke('save_app_settings', { settings: settingsToSave })
    } catch (error) {
      console.error('Failed to save settings:', error)
    }
  }

  async function resetSettings() {
    settings.value = { ...defaultSettings }
    // 保存默认设置到数据库
    try {
      const settingsToSave = serializeSettings(defaultSettings)
      await invoke('save_app_settings', { settings: settingsToSave })
    } catch (error) {
      console.error('Failed to reset settings:', error)
    }
  }

  // Install session management
  async function createInstallSession(): Promise<InstallSession> {
    try {
      const session = await invoke<InstallSession>('create_install_session')
      return session
    } catch (error) {
      console.error('Failed to create install session:', error)
      throw error
    }
  }

  async function recordCreateFile(sessionId: string, filePath: string): Promise<InstallSession> {
    try {
      const session = await invoke<InstallSession>('record_create_file', { sessionId, filePath })
      return session
    } catch (error) {
      console.error('Failed to record create file:', error)
      throw error
    }
  }

  async function recordCreateDir(sessionId: string, dirPath: string): Promise<InstallSession> {
    try {
      const session = await invoke<InstallSession>('record_create_dir', { sessionId, dirPath })
      return session
    } catch (error) {
      console.error('Failed to record create dir:', error)
      throw error
    }
  }

  async function recordModifyFile(sessionId: string, filePath: string): Promise<InstallSession> {
    try {
      const session = await invoke<InstallSession>('record_modify_file', { sessionId, filePath })
      return session
    } catch (error) {
      console.error('Failed to record modify file:', error)
      throw error
    }
  }

  async function recordDeleteFile(sessionId: string, filePath: string): Promise<InstallSession> {
    try {
      const session = await invoke<InstallSession>('record_delete_file', { sessionId, filePath })
      return session
    } catch (error) {
      console.error('Failed to record delete file:', error)
      throw error
    }
  }

  async function rollbackInstall(sessionId: string, errorReason: string): Promise<InstallResult> {
    try {
      const result = await invoke<InstallResult>('rollback_install', { sessionId, errorReason })
      return result
    } catch (error) {
      console.error('Failed to rollback install:', error)
      throw error
    }
  }

  async function completeInstall(sessionId: string): Promise<InstallResult> {
    try {
      const result = await invoke<InstallResult>('complete_install', { sessionId })
      return result
    } catch (error) {
      console.error('Failed to complete install:', error)
      throw error
    }
  }

  async function getInstallSessionStatus(sessionId: string): Promise<InstallSession> {
    try {
      const session = await invoke<InstallSession>('get_install_session_status', { sessionId })
      return session
    } catch (error) {
      console.error('Failed to get install session status:', error)
      throw error
    }
  }

  async function cancelInstallSession(sessionId: string): Promise<InstallResult> {
    try {
      const result = await invoke<InstallResult>('cancel_install_session', { sessionId })
      if (result.success) {
        // Remove from local state
        pendingInstallSessions.value = pendingInstallSessions.value.filter(s => s.id !== sessionId)
      }
      return result
    } catch (error) {
      console.error('Failed to cancel install session:', error)
      throw error
    }
  }

  async function loadPendingInstallSessions(): Promise<void> {
    isLoadingPendingSessions.value = true
    pendingSessionsError.value = null

    try {
      // 使用 list_all_install_sessions 获取所有会话（包括可清理的）
      const sessions = await invoke<InstallSession[]>('list_all_install_sessions')
      pendingInstallSessions.value = sessions
    } catch (error) {
      console.error('Failed to list install sessions:', error)
      pendingSessionsError.value = error instanceof Error ? error.message : '获取安装会话列表失败'
    } finally {
      isLoadingPendingSessions.value = false
    }
  }

  async function cleanupInstallSession(sessionId: string): Promise<void> {
    try {
      await invoke('cleanup_install_session', { sessionId })
      // Remove from local state
      pendingInstallSessions.value = pendingInstallSessions.value.filter(s => s.id !== sessionId)
    } catch (error) {
      console.error('Failed to cleanup install session:', error)
      throw error
    }
  }

  function clearPendingInstallSessions(): void {
    pendingInstallSessions.value = []
    pendingSessionsError.value = null
  }

  return {
    // State
    settings,
    isLoading,
    hasLoaded,
    // Pending Install Sessions state
    pendingInstallSessions,
    isLoadingPendingSessions,
    pendingSessionsError,
    // Getters
    isDebugMode,
    // Actions
    loadSettings,
    updateSettings,
    resetSettings,
    // Install session actions
    createInstallSession,
    recordCreateFile,
    recordCreateDir,
    recordModifyFile,
    recordDeleteFile,
    rollbackInstall,
    completeInstall,
    getInstallSessionStatus,
    cancelInstallSession,
    loadPendingInstallSessions,
    cleanupInstallSession,
    clearPendingInstallSessions
  }
})
