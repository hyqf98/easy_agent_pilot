<script setup lang="ts">
import { onMounted, reactive, ref, computed } from 'vue'
import { save, open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { EaButton } from '@/components/common'
import { useI18n } from 'vue-i18n'
import { useProjectStore } from '@/stores/project'
import { useSessionStore } from '@/stores/session'
import { useMessageStore } from '@/stores/message'
import { useSettingsStore } from '@/stores/settings'
import DataClearConfirmDialog from '@/components/settings/data/DataClearConfirmDialog.vue'
import DataStorageCard from '@/components/settings/data/DataStorageCard.vue'
import DataTransferCard from '@/components/settings/data/DataTransferCard.vue'
import InstallSessionsCard from '@/components/settings/data/InstallSessionsCard.vue'
import type { ExportOptions, ExportOptionItem, ExportOptionKey, ImportStatItem, ImportStats } from '@/components/settings/data/types'

const { t } = useI18n()
const projectStore = useProjectStore()
const sessionStore = useSessionStore()
const messageStore = useMessageStore()
const settingsStore = useSettingsStore()

const exportOptions = reactive<ExportOptions>({
  include_projects: true,
  include_sessions: true,
  include_messages: true,
  include_agents: true,
  include_mcp_servers: true,
  include_cli_paths: true,
  include_market_sources: true,
  include_app_settings: true
})

const exportOptionItems = computed<ExportOptionItem[]>(() => [
  { key: 'include_projects', label: t('settings.data.statsProjects'), checked: exportOptions.include_projects },
  { key: 'include_sessions', label: t('settings.data.statsSessions'), checked: exportOptions.include_sessions },
  { key: 'include_messages', label: t('settings.data.statsMessages'), checked: exportOptions.include_messages },
  { key: 'include_agents', label: t('settings.data.statsAgents'), checked: exportOptions.include_agents },
  { key: 'include_mcp_servers', label: t('settings.data.statsMcpServers'), checked: exportOptions.include_mcp_servers },
  { key: 'include_cli_paths', label: t('settings.data.statsCliPaths'), checked: exportOptions.include_cli_paths },
  { key: 'include_market_sources', label: t('settings.data.statsMarketSources'), checked: exportOptions.include_market_sources },
  { key: 'include_app_settings', label: t('settings.data.statsAppSettings'), checked: exportOptions.include_app_settings }
])

const allSelected = computed(() => Object.values(exportOptions).every(Boolean))
const hasAnySelected = computed(() => Object.values(exportOptions).some(Boolean))

const isCancellingSession = ref<string | null>(null)
const isCleaningUpSession = ref<string | null>(null)

const isExporting = ref(false)
const exportMessage = ref('')
const exportSuccess = ref(false)

const isImporting = ref(false)
const importMessage = ref('')
const importSuccess = ref(false)
const importStats = ref<ImportStats | null>(null)

const importStatItems = computed<ImportStatItem[]>(() => {
  if (!importStats.value) {
    return []
  }

  const items: ImportStatItem[] = [
    { key: 'projects_imported', label: t('settings.data.statsProjects'), value: importStats.value.projects_imported },
    { key: 'sessions_imported', label: t('settings.data.statsSessions'), value: importStats.value.sessions_imported },
    { key: 'messages_imported', label: t('settings.data.statsMessages'), value: importStats.value.messages_imported },
    { key: 'agents_imported', label: t('settings.data.statsAgents'), value: importStats.value.agents_imported },
    { key: 'mcp_servers_imported', label: t('settings.data.statsMcpServers'), value: importStats.value.mcp_servers_imported },
    { key: 'cli_paths_imported', label: t('settings.data.statsCliPaths'), value: importStats.value.cli_paths_imported },
    { key: 'market_sources_imported', label: t('settings.data.statsMarketSources'), value: importStats.value.market_sources_imported },
    { key: 'app_settings_imported', label: t('settings.data.statsAppSettings'), value: importStats.value.app_settings_imported }
  ]

  return items.filter((item) => item.value > 0)
})

const showClearConfirm = ref(false)
const clearConfirmText = ref('')
const isClearing = ref(false)
const clearMessage = ref('')
const clearSuccess = ref(false)

function updateExportOption(key: ExportOptionKey, checked: boolean) {
  exportOptions[key] = checked
}

function toggleAllExportOptions(select: boolean) {
  Object.keys(exportOptions).forEach((key) => {
    exportOptions[key as ExportOptionKey] = select
  })
}

async function loadSessions() {
  await settingsStore.loadPendingInstallSessions()
}

async function handleCancelSession(sessionId: string) {
  isCancellingSession.value = sessionId
  try {
    const result = await settingsStore.cancelInstallSession(sessionId)
    if (result.success) {
      await loadSessions()
    }
  } catch (error) {
    console.error('Failed to cancel session:', error)
  } finally {
    isCancellingSession.value = null
  }
}

async function handleCleanupSession(sessionId: string) {
  isCleaningUpSession.value = sessionId
  try {
    await settingsStore.cleanupInstallSession(sessionId)
  } catch (error) {
    console.error('Failed to cleanup session:', error)
  } finally {
    isCleaningUpSession.value = null
  }
}

async function handleExport() {
  if (isExporting.value) {
    return
  }

  if (!hasAnySelected.value) {
    exportSuccess.value = false
    exportMessage.value = t('settings.data.exportNoSelection')
    setTimeout(() => {
      exportMessage.value = ''
    }, 3000)
    return
  }

  isExporting.value = true
  exportMessage.value = ''
  exportSuccess.value = false

  try {
    const filePath = await save({
      defaultPath: `easy-agent-backup-${new Date().toISOString().slice(0, 10)}.json`,
      filters: [{ name: 'JSON', extensions: ['json'] }],
      title: t('settings.data.exportDialogTitle')
    })

    if (!filePath) {
      isExporting.value = false
      return
    }

    await invoke('export_selected_to_file', { filePath, options: exportOptions })
    exportSuccess.value = true
    exportMessage.value = t('settings.data.exportSuccess')
  } catch (error) {
    console.error('Export failed:', error)
    exportSuccess.value = false
    exportMessage.value = `${t('settings.data.exportFailed')}: ${error}`
  } finally {
    isExporting.value = false
    setTimeout(() => {
      exportMessage.value = ''
    }, 3000)
  }
}

async function handleImport() {
  if (isImporting.value) {
    return
  }

  isImporting.value = true
  importMessage.value = ''
  importSuccess.value = false
  importStats.value = null

  try {
    const filePath = await open({
      multiple: false,
      filters: [{ name: 'JSON', extensions: ['json'] }],
      title: t('settings.data.importDialogTitle')
    })

    if (!filePath) {
      isImporting.value = false
      return
    }

    try {
      await invoke('validate_import_data', { filePath })
    } catch (validateError) {
      console.error('Validation error:', validateError)
      importSuccess.value = false
      importMessage.value = `${t('settings.data.invalidFormat')}: ${validateError}`
      isImporting.value = false
      return
    }

    importStats.value = await invoke<ImportStats>('import_data_from_file', { filePath })
    importSuccess.value = true
    importMessage.value = t('settings.data.importSuccess')
  } catch (error) {
    console.error('Import failed:', error)
    importSuccess.value = false
    importMessage.value = `${t('settings.data.importFailed')}: ${error}`
  } finally {
    isImporting.value = false
    setTimeout(() => {
      importMessage.value = ''
    }, 5000)
  }
}

function handleClearData() {
  showClearConfirm.value = true
  clearConfirmText.value = ''
  clearMessage.value = ''
}

async function confirmClearData() {
  if (clearConfirmText.value !== 'CLEAR') {
    clearMessage.value = t('settings.data.clearConfirmError')
    clearSuccess.value = false
    return
  }

  isClearing.value = true
  clearMessage.value = ''

  try {
    await invoke('clear_all_data')

    projectStore.projects = []
    projectStore.setCurrentProject(null)
    sessionStore.sessions = []
    sessionStore.setCurrentSession(null)
    messageStore.messages = []

    clearSuccess.value = true
    clearMessage.value = t('settings.data.clearSuccess')

    setTimeout(() => {
      showClearConfirm.value = false
      clearMessage.value = ''
    }, 3000)
  } catch (error) {
    console.error('Clear data failed:', error)
    clearSuccess.value = false
    clearMessage.value = `${t('settings.data.clearFailed')}: ${error}`
  } finally {
    isClearing.value = false
  }
}

function handleClearDialogVisibleChange(visible: boolean) {
  showClearConfirm.value = visible
  if (!visible) {
    clearConfirmText.value = ''
    clearMessage.value = ''
  }
}

onMounted(() => {
  void loadSessions()
})
</script>

<template>
  <div class="settings-page">
    <h3 class="settings-page__title">
      {{ t('settings.nav.data') }}
    </h3>

    <DataStorageCard />

    <DataTransferCard
      :export-option-items="exportOptionItems"
      :is-exporting="isExporting"
      :export-message="exportMessage"
      :export-success="exportSuccess"
      :is-importing="isImporting"
      :import-message="importMessage"
      :import-success="importSuccess"
      :import-stat-items="importStatItems"
      :all-selected="allSelected"
      :has-any-selected="hasAnySelected"
      @export="handleExport"
      @import="handleImport"
      @toggle-all="toggleAllExportOptions"
      @update-option="updateExportOption"
    />

    <InstallSessionsCard
      :loading="settingsStore.isLoadingPendingSessions"
      :error="settingsStore.pendingSessionsError"
      :sessions="settingsStore.pendingInstallSessions"
      :cancelling-session-id="isCancellingSession"
      :cleaning-session-id="isCleaningUpSession"
      @refresh="loadSessions"
      @cancel-session="handleCancelSession"
      @cleanup-session="handleCleanupSession"
    />

    <div class="settings-card settings-card--danger">
      <h4 class="settings-card__title">
        {{ t('settings.data.dangerZone') }}
      </h4>
      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.data.clearAllData') }}</span>
          <span class="settings-item__desc">{{ t('settings.data.clearAllDataDesc') }}</span>
        </div>
        <EaButton
          type="danger"
          size="small"
          @click="handleClearData"
        >
          {{ t('settings.data.clearData') }}
        </EaButton>
      </div>
    </div>

    <DataClearConfirmDialog
      :visible="showClearConfirm"
      :confirm-text="clearConfirmText"
      :clearing="isClearing"
      :message="clearMessage"
      :success="clearSuccess"
      @update:visible="handleClearDialogVisibleChange"
      @update:confirm-text="clearConfirmText = $event"
      @confirm="confirmClearData"
    />
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-6);
}

.settings-page__title {
  margin: 0;
  color: var(--color-text-primary);
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
}

.settings-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  padding: var(--spacing-5);
  border-radius: var(--radius-lg);
  background-color: var(--color-bg-secondary);
}

.settings-card--danger {
  border: 1px solid var(--color-error-light);
}

.settings-card__title {
  margin: 0;
  padding-bottom: var(--spacing-3);
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-primary);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
}

.settings-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.settings-item__info {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.settings-item__label {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.settings-item__desc {
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}
</style>
