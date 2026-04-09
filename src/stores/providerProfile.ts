import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'

// ============================================================================
// 类型定义
// ============================================================================

/** CLI 类型 */
export type CliType = 'claude' | 'codex' | 'opencode'

/** CLI 连接信息 */
export interface CliConnectionInfo {
  cliType: string
  displayName: string
  configFile: string
  settingsFile: string
  baseUrl: string | null
  mainModel: string | null
  providerName: string | null
  apiKeyMasked: string | null
  apiKey: string | null
  isValid: boolean
  errorMessage: string | null
}

/** Provider 配置 */
export interface ProviderProfile {
  id: string
  name: string
  cliType: CliType
  isActive: boolean
  // Claude CLI 配置字段
  apiKey?: string
  baseUrl?: string
  providerName?: string
  mainModel?: string
  reasoningModel?: string
  haikuModel?: string
  sonnetDefault?: string
  opusDefault?: string
  // Codex 配置字段
  codexModel?: string
  // 元数据
  createdAt: string
  updatedAt: string
}

/** 创建 Provider 配置输入 */
export interface CreateProviderProfileInput {
  name: string
  cliType: CliType
  apiKey?: string
  baseUrl?: string
  providerName?: string
  mainModel?: string
  reasoningModel?: string
  haikuModel?: string
  sonnetDefault?: string
  opusDefault?: string
  codexModel?: string
}

/** 更新 Provider 配置输入 */
export interface UpdateProviderProfileInput {
  name?: string
  apiKey?: string
  baseUrl?: string
  providerName?: string
  mainModel?: string
  reasoningModel?: string
  haikuModel?: string
  sonnetDefault?: string
  opusDefault?: string
  codexModel?: string
}

// 后端返回的原始数据结构（snake_case）
interface RawProviderProfile {
  id: string
  name: string
  cli_type: string
  is_active: boolean
  api_key?: string
  base_url?: string
  provider_name?: string
  main_model?: string
  reasoning_model?: string
  haiku_model?: string
  sonnet_default?: string
  opus_default?: string
  codex_model?: string
  created_at: string
  updated_at: string
}

// ============================================================================
// 转换函数
// ============================================================================

function transformProfile(raw: RawProviderProfile): ProviderProfile {
  return {
    id: raw.id,
    name: raw.name,
    cliType: raw.cli_type as CliType,
    isActive: raw.is_active,
    apiKey: raw.api_key,
    baseUrl: raw.base_url,
    providerName: raw.provider_name,
    mainModel: raw.main_model,
    reasoningModel: raw.reasoning_model,
    haikuModel: raw.haiku_model,
    sonnetDefault: raw.sonnet_default,
    opusDefault: raw.opus_default,
    codexModel: raw.codex_model,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

// ============================================================================
// Store 定义
// ============================================================================

export const useProviderProfileStore = defineStore('providerProfile', () => {
  // State
  const profiles = ref<ProviderProfile[]>([])
  const activeProfile = ref<ProviderProfile | null>(null)
  const currentConfig = ref<ProviderProfile | null>(null)
  const isLoading = ref(false)
  const currentCliType = ref<CliType>('claude')

  // CLI 连接信息
  const cliConnections = ref<CliConnectionInfo[]>([])
  const isLoadingConnections = ref(false)

  // Computed
  const claudeProfiles = computed(() =>
    profiles.value.filter(p => p.cliType === 'claude')
  )

  const codexProfiles = computed(() =>
    profiles.value.filter(p => p.cliType === 'codex')
  )

  const opencodeProfiles = computed(() =>
    profiles.value.filter(p => p.cliType === 'opencode')
  )

  const activeClaudeProfile = computed(() =>
    claudeProfiles.value.find(p => p.isActive) || null
  )

  const activeCodexProfile = computed(() =>
    codexProfiles.value.find(p => p.isActive) || null
  )

  const activeOpencodeProfile = computed(() =>
    opencodeProfiles.value.find(p => p.isActive) || null
  )

  // Claude CLI 连接信息
  const claudeConnection = computed(() =>
    cliConnections.value.find(c => c.cliType === 'claude') || null
  )

  // Codex CLI 连接信息
  const codexConnection = computed(() =>
    cliConnections.value.find(c => c.cliType === 'codex') || null
  )

  // OpenCode CLI 连接信息
  const opencodeConnection = computed(() =>
    cliConnections.value.find(c => c.cliType === 'opencode') || null
  )

  // Actions

  function upsertCliConnection(info: CliConnectionInfo) {
    const index = cliConnections.value.findIndex(connection => connection.cliType === info.cliType)
    if (index >= 0) {
      cliConnections.value[index] = info
      return
    }

    cliConnections.value.push(info)
  }

  /**
   * 刷新指定 CLI 类型的运行态信息。
   * 用于激活配置、编辑激活配置后同步更新当前激活项、当前文件配置和顶部连接信息。
   */
  async function refreshCliTypeState(cliType: CliType, options?: { reloadProfiles?: boolean }) {
    const { reloadProfiles = false } = options ?? {}

    if (reloadProfiles) {
      await loadProfiles()
    }

    isLoadingConnections.value = true
    try {
      const [active, current, connection] = await Promise.all([
        loadActiveProfile(cliType),
        readCurrentConfig(cliType),
        readCliConnectionInfo(cliType)
      ])

      if (activeProfile.value?.cliType === cliType || active?.cliType === cliType || !active) {
        activeProfile.value = active
      }

      if (currentConfig.value?.cliType === cliType || current?.cliType === cliType || !current) {
        currentConfig.value = current
      }

      if (connection) {
        upsertCliConnection(connection)
      }

      return {
        activeProfile: active,
        currentConfig: current,
        connection
      }
    } finally {
      isLoadingConnections.value = false
    }
  }

  /** 加载所有配置 */
  async function loadProfiles(cliType?: CliType) {
    isLoading.value = true
    const notificationStore = useNotificationStore()
    try {
      const rawProfiles = await invoke<RawProviderProfile[]>('list_provider_profiles', {
        cliType: cliType || null
      })
      profiles.value = rawProfiles.map(transformProfile)
      return profiles.value
    } catch (error) {
      console.error('Failed to load provider profiles:', error)
      notificationStore.databaseError(
        '加载配置失败',
        getErrorMessage(error),
        async () => { void await loadProfiles(cliType) }
      )
      return []
    } finally {
      isLoading.value = false
    }
  }

  /** 获取单个配置 */
  async function getProfile(id: string) {
    const notificationStore = useNotificationStore()
    try {
      const rawProfile = await invoke<RawProviderProfile>('get_provider_profile', { id })
      return transformProfile(rawProfile)
    } catch (error) {
      console.error('Failed to get provider profile:', error)
      notificationStore.databaseError(
        '获取配置失败',
        getErrorMessage(error),
        async () => { void await getProfile(id) }
      )
      return null
    }
  }

  /** 创建配置 */
  async function createProfile(input: CreateProviderProfileInput) {
    const notificationStore = useNotificationStore()
    try {
      const rawProfile = await invoke<RawProviderProfile>('create_provider_profile', {
        input: {
          name: input.name,
          cli_type: input.cliType,
          api_key: input.apiKey,
          base_url: input.baseUrl,
          provider_name: input.providerName,
          main_model: input.mainModel,
          reasoning_model: input.reasoningModel,
          haiku_model: input.haikuModel,
          sonnet_default: input.sonnetDefault,
          opus_default: input.opusDefault,
          codex_model: input.codexModel
        }
      })
      const newProfile = transformProfile(rawProfile)
      profiles.value.push(newProfile)
      return newProfile
    } catch (error) {
      console.error('Failed to create provider profile:', error)
      notificationStore.databaseError(
        '创建配置失败',
        getErrorMessage(error),
        async () => { void await createProfile(input) }
      )
      throw error
    }
  }

  /** 更新配置 */
  async function updateProfile(id: string, input: UpdateProviderProfileInput) {
    const notificationStore = useNotificationStore()
    try {
      const rawProfile = await invoke<RawProviderProfile>('update_provider_profile', {
        id,
        input: {
          name: input.name,
          api_key: input.apiKey,
          base_url: input.baseUrl,
          provider_name: input.providerName,
          main_model: input.mainModel,
          reasoning_model: input.reasoningModel,
          haiku_model: input.haikuModel,
          sonnet_default: input.sonnetDefault,
          opus_default: input.opusDefault,
          codex_model: input.codexModel
        }
      })
      const updatedProfile = transformProfile(rawProfile)
      const index = profiles.value.findIndex(p => p.id === id)
      if (index !== -1) {
        profiles.value[index] = updatedProfile
      }

      if (updatedProfile.isActive) {
        await refreshCliTypeState(updatedProfile.cliType, { reloadProfiles: true })
      } else if (activeProfile.value?.id === updatedProfile.id) {
        activeProfile.value = updatedProfile
      }

      return updatedProfile
    } catch (error) {
      console.error('Failed to update provider profile:', error)
      notificationStore.databaseError(
        '更新配置失败',
        getErrorMessage(error),
        async () => { void await updateProfile(id, input) }
      )
      throw error
    }
  }

  /** 删除配置 */
  async function deleteProfile(id: string) {
    const notificationStore = useNotificationStore()
    try {
      const deletedProfile = profiles.value.find(profile => profile.id === id) || null
      await invoke('delete_provider_profile', { id })
      const index = profiles.value.findIndex(p => p.id === id)
      if (index !== -1) {
        profiles.value.splice(index, 1)
      }

      if (deletedProfile?.isActive) {
        await refreshCliTypeState(deletedProfile.cliType, { reloadProfiles: true })
      } else if (activeProfile.value?.id === id) {
        activeProfile.value = null
      }
    } catch (error) {
      console.error('Failed to delete provider profile:', error)
      notificationStore.databaseError(
        '删除配置失败',
        getErrorMessage(error),
        async () => { void await deleteProfile(id) }
      )
      throw error
    }
  }

  /** 获取当前激活的配置 */
  async function loadActiveProfile(cliType: CliType) {
    const notificationStore = useNotificationStore()
    try {
      const rawProfile = await invoke<RawProviderProfile | null>('get_active_provider_profile', {
        cliType
      })
      if (rawProfile) {
        activeProfile.value = transformProfile(rawProfile)
      } else {
        activeProfile.value = null
      }
      return activeProfile.value
    } catch (error) {
      console.error('Failed to get active profile:', error)
      notificationStore.databaseError(
        '获取激活配置失败',
        getErrorMessage(error),
        async () => { void await loadActiveProfile(cliType) }
      )
      return null
    }
  }

  /** 一键切换配置 */
  async function switchProfile(id: string) {
    const notificationStore = useNotificationStore()
    try {
      const rawProfile = await invoke<RawProviderProfile>('switch_provider_profile', { id })
      const updatedProfile = transformProfile(rawProfile)

      // 更新本地状态
      profiles.value = profiles.value.map(p => ({
        ...p,
        isActive: p.cliType === updatedProfile.cliType ? p.id === id : p.isActive
      }))

      activeProfile.value = updatedProfile
      await refreshCliTypeState(updatedProfile.cliType, { reloadProfiles: true })
      return updatedProfile
    } catch (error) {
      console.error('Failed to switch provider profile:', error)
      notificationStore.databaseError(
        '切换配置失败',
        getErrorMessage(error),
        async () => { void await switchProfile(id) }
      )
      throw error
    }
  }

  /** 直接更新当前 CLI 配置文件中的默认配置 */
  async function updateCurrentConfig(cliType: CliType, input: UpdateProviderProfileInput) {
    const notificationStore = useNotificationStore()
    try {
      const rawProfile = await invoke<RawProviderProfile>('update_current_cli_config', {
        cliType,
        input: {
          api_key: input.apiKey,
          base_url: input.baseUrl,
          provider_name: input.providerName,
          main_model: input.mainModel,
          reasoning_model: input.reasoningModel,
          haiku_model: input.haikuModel,
          sonnet_default: input.sonnetDefault,
          opus_default: input.opusDefault,
          codex_model: input.codexModel
        }
      })

      const updatedProfile = transformProfile(rawProfile)
      currentConfig.value = updatedProfile
      return updatedProfile
    } catch (error) {
      console.error('Failed to update current CLI config:', error)
      notificationStore.databaseError(
        '更新默认配置失败',
        getErrorMessage(error),
        async () => { void await updateCurrentConfig(cliType, input) }
      )
      throw error
    }
  }

  /** 读取当前 CLI 配置文件 */
  async function readCurrentConfig(cliType: CliType) {
    const notificationStore = useNotificationStore()
    try {
      const rawProfile = await invoke<RawProviderProfile>('read_current_cli_config', {
        cliType
      })
      currentConfig.value = transformProfile(rawProfile)
      return currentConfig.value
    } catch (error) {
      console.error('Failed to read current CLI config:', error)
      notificationStore.databaseError(
        '读取当前配置失败',
        getErrorMessage(error),
        async () => { void await readCurrentConfig(cliType) }
      )
      return null
    }
  }

  /** 清除缓存 */
  function clearCache() {
    profiles.value = []
    activeProfile.value = null
    currentConfig.value = null
    cliConnections.value = []
  }

  /** 读取单个 CLI 连接信息 */
  async function readCliConnectionInfo(cliType: CliType) {
    try {
      const info = await invoke<CliConnectionInfo>('read_cli_connection_info', {
        cliType
      })
      upsertCliConnection(info)
      return info
    } catch (error) {
      console.error('Failed to read CLI connection info:', error)
      return null
    }
  }

  /** 读取所有 CLI 连接信息 */
  async function readAllCliConnections() {
    isLoadingConnections.value = true
    try {
      const connections = await invoke<CliConnectionInfo[]>('read_all_cli_connections')
      cliConnections.value = connections
      return connections
    } catch (error) {
      console.error('Failed to read all CLI connections:', error)
      // 即使调用失败，也要初始化默认的连接信息，以便 UI 能显示错误状态
      cliConnections.value = [
        {
          cliType: 'claude',
          displayName: 'Claude CLI',
          configFile: '',
          settingsFile: '',
          baseUrl: null,
          mainModel: null,
          providerName: null,
          apiKeyMasked: null,
          apiKey: null,
          isValid: false,
          errorMessage: `读取配置失败: ${error instanceof Error ? error.message : String(error)}`
        },
        {
          cliType: 'codex',
          displayName: 'Codex CLI',
          configFile: '',
          settingsFile: '',
          baseUrl: null,
          mainModel: null,
          providerName: null,
          apiKeyMasked: null,
          apiKey: null,
          isValid: false,
          errorMessage: `读取配置失败: ${error instanceof Error ? error.message : String(error)}`
        }
      ]
      return cliConnections.value
    } finally {
      isLoadingConnections.value = false
    }
  }

  return {
    // State
    profiles,
    activeProfile,
    currentConfig,
    isLoading,
    currentCliType,
    cliConnections,
    isLoadingConnections,
    // Computed
    claudeProfiles,
    codexProfiles,
    opencodeProfiles,
    activeClaudeProfile,
    activeCodexProfile,
    activeOpencodeProfile,
    claudeConnection,
    codexConnection,
    opencodeConnection,
    // Actions
    loadProfiles,
    getProfile,
    createProfile,
    updateProfile,
    deleteProfile,
    loadActiveProfile,
    switchProfile,
    updateCurrentConfig,
    readCurrentConfig,
    readCliConnectionInfo,
    readAllCliConnections,
    refreshCliTypeState,
    clearCache
  }
})
