import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// 应用状态键名常量
export const APP_STATE_KEYS = {
  LAST_PROJECT_ID: 'last_project_id',
  LAST_SESSION_IDS: 'last_session_ids',
  LAST_ACTIVE_SESSION_ID: 'last_active_session_id',
  PANEL_EXPANDED: 'panel_expanded'
} as const

interface AppStateEntry {
  key: string
  value: string
  updated_at: number
}

export const useAppStateStore = defineStore('appState', () => {
  // 状态
  const lastProjectId = ref<string | null>(null)
  const lastSessionIds = ref<string[]>([])
  const lastActiveSessionId = ref<string | null>(null)
  const panelExpanded = ref(true)
  const isInitialized = ref(false)

  // 加载所有状态
  async function loadState() {
    try {
      const entries = await invoke<AppStateEntry[]>('get_app_states', {
        keys: Object.values(APP_STATE_KEYS)
      })

      for (const entry of entries) {
        switch (entry.key) {
          case APP_STATE_KEYS.LAST_PROJECT_ID:
            lastProjectId.value = entry.value || null
            break
          case APP_STATE_KEYS.LAST_SESSION_IDS:
            try {
              lastSessionIds.value = JSON.parse(entry.value)
            } catch {
              lastSessionIds.value = []
            }
            break
          case APP_STATE_KEYS.LAST_ACTIVE_SESSION_ID:
            lastActiveSessionId.value = entry.value || null
            break
          case APP_STATE_KEYS.PANEL_EXPANDED:
            panelExpanded.value = entry.value === 'true'
            break
        }
      }

      isInitialized.value = true
    } catch (error) {
      console.error('Failed to load app state:', error)
    }
  }

  // 保存状态
  async function saveState(key: string, value: string) {
    try {
      await invoke('set_app_state', { key, value })
    } catch (error) {
      console.error('Failed to save app state:', error)
    }
  }

  // 监听状态变化并自动保存
  watch(lastProjectId, (newVal) => {
    if (isInitialized.value && newVal) {
      saveState(APP_STATE_KEYS.LAST_PROJECT_ID, newVal)
    }
  })

  watch(lastSessionIds, (newVal) => {
    if (isInitialized.value) {
      saveState(APP_STATE_KEYS.LAST_SESSION_IDS, JSON.stringify(newVal))
    }
  }, { deep: true })

  watch(lastActiveSessionId, (newVal) => {
    if (isInitialized.value) {
      saveState(APP_STATE_KEYS.LAST_ACTIVE_SESSION_ID, newVal ?? '')
    }
  })

  watch(panelExpanded, (newVal) => {
    if (isInitialized.value) {
      saveState(APP_STATE_KEYS.PANEL_EXPANDED, String(newVal))
    }
  })

  // 设置上次项目
  function setLastProject(projectId: string | null) {
    lastProjectId.value = projectId
  }

  // 设置上次会话列表
  function setLastSessions(sessionIds: string[]) {
    lastSessionIds.value = sessionIds
  }

  function setLastActiveSession(sessionId: string | null) {
    lastActiveSessionId.value = sessionId
  }

  // 设置面板展开状态
  function setPanelExpanded(expanded: boolean) {
    panelExpanded.value = expanded
  }

  return {
    // 状态
    lastProjectId,
    lastSessionIds,
    lastActiveSessionId,
    panelExpanded,
    isInitialized,
    // 方法
    loadState,
    setLastProject,
    setLastSessions,
    setLastActiveSession,
    setPanelExpanded
  }
})
