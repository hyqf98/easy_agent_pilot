<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAgentStore } from '@/stores/agent'
import { EaIcon } from '@/components/common'
import CliInstallerSection from '@/components/settings/cli/CliInstallerSection.vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { CliInstallerInfo, InstallLogEvent, VersionInfo } from '@/components/settings/cli/types'

withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false
})

const { t } = useI18n()
const agentStore = useAgentStore()

const installLogs = ref<InstallLogEvent[]>([])
const isInstalling = ref(false)
const currentInstallingCli = ref<string | null>(null)

const isCheckingUpdate = ref<string | null>(null)
const isUpgrading = ref<string | null>(null)
const claudeVersionInfo = ref<VersionInfo | null>(null)
const codexVersionInfo = ref<VersionInfo | null>(null)

const claudeInstallInfo = ref<CliInstallerInfo | null>(null)
const codexInstallInfo = ref<CliInstallerInfo | null>(null)

let unlistenLog: (() => void) | null = null
let unlistenComplete: (() => void) | null = null

const installerCards = computed(() => [
  {
    key: 'claude',
    label: 'Claude CLI',
    info: claudeInstallInfo.value,
    versionInfo: claudeVersionInfo.value
  },
  {
    key: 'codex',
    label: 'Codex CLI',
    info: codexInstallInfo.value,
    versionInfo: codexVersionInfo.value
  }
])

async function loadInstallOptions() {
  try {
    claudeInstallInfo.value = await invoke<CliInstallerInfo>('get_cli_install_options', { cliName: 'claude' })
    codexInstallInfo.value = await invoke<CliInstallerInfo>('get_cli_install_options', { cliName: 'codex' })
  } catch (error) {
    console.error('Failed to load install options:', error)
  }
}

async function checkAllUpdates() {
  if (claudeInstallInfo.value?.installed) {
    try {
      claudeVersionInfo.value = await invoke<VersionInfo>('check_cli_update', { cliName: 'claude' })
    } catch (error) {
      console.error('Failed to check claude update:', error)
    }
  }

  if (codexInstallInfo.value?.installed) {
    try {
      codexVersionInfo.value = await invoke<VersionInfo>('check_cli_update', { cliName: 'codex' })
    } catch (error) {
      console.error('Failed to check codex update:', error)
    }
  }
}

async function handleCheckUpdate(cliName: string) {
  isCheckingUpdate.value = cliName
  try {
    const info = await invoke<VersionInfo>('check_cli_update', { cliName })
    if (cliName === 'claude') {
      claudeVersionInfo.value = info
    } else {
      codexVersionInfo.value = info
    }
  } finally {
    isCheckingUpdate.value = null
  }
}

async function handleInstall(cliName: string, method: string) {
  if (isInstalling.value) {
    return
  }

  isInstalling.value = true
  currentInstallingCli.value = cliName
  installLogs.value = []

  try {
    await invoke('install_cli', { cliName, method })
  } catch (error) {
    console.error('Install failed:', error)
  }
}

async function handleUpgrade(cliName: string) {
  if (isUpgrading.value) {
    return
  }

  isUpgrading.value = cliName
  installLogs.value = []

  try {
    await invoke('upgrade_cli', { cliName })
  } catch (error) {
    console.error('Upgrade failed:', error)
  }
}

onMounted(async () => {
  await loadInstallOptions()
  await checkAllUpdates()

  unlistenLog = await listen('cli-install-log', (event) => {
    const log = event.payload as InstallLogEvent
    installLogs.value.push(log)
    nextTick(() => {
      const container = document.querySelector('.install-logs__content')
      if (container) {
        container.scrollTop = container.scrollHeight
      }
    })
  })

  unlistenComplete = await listen('cli-install-complete', async (event) => {
    const result = event.payload as { cli_name: string; success: boolean }
    isInstalling.value = false
    isUpgrading.value = null
    currentInstallingCli.value = null

    if (result.success) {
      await Promise.all([
        agentStore.scanCliTools(),
        loadInstallOptions(),
        checkAllUpdates()
      ])
    }
  })
})

onUnmounted(() => {
  if (unlistenLog) {
    unlistenLog()
  }

  if (unlistenComplete) {
    unlistenComplete()
  }
})
</script>

<template>
  <div
    class="settings-page"
    :class="{ 'settings-page--embedded': embedded }"
  >
    <div class="settings-page__header">
      <h3
        class="settings-page__title"
        :class="{ 'settings-page__title--embedded': embedded }"
      >
        {{ t('settings.cli.title') }}
      </h3>
    </div>

    <div class="settings-page__hint">
      <EaIcon
        name="info"
        :size="16"
      />
      <span>{{ t('settings.agentList.detectedTools') }}，安装完成后会自动出现在上方，可直接一键添加为 Agent。</span>
    </div>

    <CliInstallerSection
      :cards="installerCards"
      :install-logs="installLogs"
      :is-installing="isInstalling"
      :current-installing-cli="currentInstallingCli"
      :is-checking-update="isCheckingUpdate"
      :is-upgrading="isUpgrading"
      @install="handleInstall"
      @upgrade="handleUpgrade"
      @check-update="handleCheckUpdate"
      @clear-logs="installLogs = []"
    />
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-6);
  width: 100%;
  min-width: 0;
}

.settings-page--embedded {
  padding-top: var(--spacing-2);
  border-top: 1px solid var(--color-border);
}

.settings-page__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.settings-page__title {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.settings-page__title--embedded {
  font-size: var(--font-size-lg);
}

.settings-page__hint {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  background-color: var(--color-primary-light);
  border-radius: var(--radius-lg);
}

.settings-page__hint :deep(svg),
.settings-page__hint .ea-icon {
  color: var(--color-primary);
}

.settings-page__hint span {
  font-size: var(--font-size-sm);
  color: var(--color-primary);
}
</style>
