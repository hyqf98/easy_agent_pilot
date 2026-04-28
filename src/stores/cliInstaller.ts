import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useAgentStore } from '@/stores/agent'
import { useNotificationStore } from '@/stores/notification'
import { getErrorMessage } from '@/utils/api'
import type { CliInstallerInfo, InstallLogEvent, VersionInfo } from '@/components/settings/cli/types'

export type CliName = 'claude' | 'codex' | 'opencode'
type CliOperation = 'install' | 'upgrade'

const CLI_NAMES: CliName[] = ['claude', 'codex', 'opencode']

function createEmptyMap<T>(value: T): Record<CliName, T> {
  return {
    claude: value,
    codex: value,
    opencode: value
  }
}

export const useCliInstallerStore = defineStore('cliInstaller', () => {
  const agentStore = useAgentStore()
  const notificationStore = useNotificationStore()

  const installLogs = ref<InstallLogEvent[]>([])
  const installInfoMap = ref<Record<CliName, CliInstallerInfo | null>>(createEmptyMap<CliInstallerInfo | null>(null))
  const versionInfoMap = ref<Record<CliName, VersionInfo | null>>(createEmptyMap<VersionInfo | null>(null))
  const visibleLogsMap = ref<Record<CliName, boolean>>(createEmptyMap(false))
  const activeOperations = ref<Partial<Record<CliName, CliOperation>>>({})
  const checkingUpdates = ref<Partial<Record<CliName, boolean>>>({})
  const isReady = ref(false)
  let initPromise: Promise<void> | null = null

  const installerCards = computed(() => [
    {
      key: 'claude' as const,
      label: 'Claude CLI',
      info: installInfoMap.value.claude,
      versionInfo: versionInfoMap.value.claude
    },
    {
      key: 'codex' as const,
      label: 'Codex CLI',
      info: installInfoMap.value.codex,
      versionInfo: versionInfoMap.value.codex
    },
    {
      key: 'opencode' as const,
      label: 'OpenCode CLI',
      info: installInfoMap.value.opencode,
      versionInfo: versionInfoMap.value.opencode
    }
  ])

  function clearLogs() {
    installLogs.value = []
  }

  function clearLogsForCli(cliName: CliName) {
    installLogs.value = installLogs.value.filter(log => log.cli_name !== cliName)
  }

  function isInstalling(cliName: CliName) {
    return activeOperations.value[cliName] === 'install'
  }

  function isUpgrading(cliName: CliName) {
    return activeOperations.value[cliName] === 'upgrade'
  }

  function isCheckingUpdate(cliName: CliName) {
    return checkingUpdates.value[cliName] === true
  }

  async function ensureReady() {
    if (isReady.value) {
      return
    }

    if (!initPromise) {
      initPromise = (async () => {
        await listen('cli-install-log', (event) => {
          const log = event.payload as InstallLogEvent
          installLogs.value.push(log)
        })

        await listen('cli-install-complete', async (event) => {
          const result = event.payload as { cli_name: CliName; success: boolean; error?: string | null }
          delete activeOperations.value[result.cli_name]

          if (result.success) {
            visibleLogsMap.value[result.cli_name] = false
            await Promise.all([
              agentStore.scanCliTools({ force: true }),
              loadInstallOption(result.cli_name),
              checkUpdate(result.cli_name)
            ])
            return
          }

          visibleLogsMap.value[result.cli_name] = true
          await loadInstallOption(result.cli_name)
        })

        await loadInstallOptions()
        await checkAllUpdates()

        isReady.value = true
      })().finally(() => {
        initPromise = null
      })
    }

    await initPromise
  }

  async function loadInstallOption(cliName: CliName) {
    try {
      installInfoMap.value[cliName] = await invoke<CliInstallerInfo>('get_cli_install_options', { cliName })
    } catch (error) {
      console.error(`Failed to load ${cliName} install options:`, error)
    }
  }

  async function loadInstallOptions() {
    await Promise.all(CLI_NAMES.map(async cliName => {
      await loadInstallOption(cliName)
    }))
  }

  async function checkUpdate(cliName: CliName) {
    checkingUpdates.value[cliName] = true
    try {
      versionInfoMap.value[cliName] = await invoke<VersionInfo>('check_cli_update', { cliName })
    } catch (error) {
      console.error(`Failed to check ${cliName} update:`, error)
    } finally {
      delete checkingUpdates.value[cliName]
    }
  }

  async function checkAllUpdates() {
    await Promise.all(CLI_NAMES.map(async cliName => {
      if (installInfoMap.value[cliName]?.installed) {
        await checkUpdate(cliName)
        return
      }

      versionInfoMap.value[cliName] = null
    }))
  }

  async function installCli(cliName: CliName, method: string) {
    if (activeOperations.value[cliName]) {
      return
    }

    visibleLogsMap.value[cliName] = true
    activeOperations.value[cliName] = 'install'
    try {
      await invoke('install_cli', { cliName, method })
    } catch (error) {
      delete activeOperations.value[cliName]
      notificationStore.error('CLI 安装失败', getErrorMessage(error))
    }
  }

  async function upgradeCli(cliName: CliName) {
    if (activeOperations.value[cliName]) {
      return
    }

    visibleLogsMap.value[cliName] = true
    activeOperations.value[cliName] = 'upgrade'
    try {
      await invoke('upgrade_cli', { cliName })
    } catch (error) {
      delete activeOperations.value[cliName]
      notificationStore.error('CLI 升级失败', getErrorMessage(error))
    }
  }

  return {
    activeOperations,
    checkAllUpdates,
    checkUpdate,
    checkingUpdates,
    clearLogs,
    clearLogsForCli,
    ensureReady,
    installCli,
    installInfoMap,
    installLogs,
    installerCards,
    isCheckingUpdate,
    isInstalling,
    isReady,
    isUpgrading,
    loadInstallOptions,
    upgradeCli,
    versionInfoMap,
    visibleLogsMap
  }
})
