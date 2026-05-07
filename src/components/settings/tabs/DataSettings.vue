<script setup lang="ts">
import { onMounted, reactive, ref, computed } from 'vue'
import { save, open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { EaButton } from '@/components/common'
import { useI18n } from 'vue-i18n'
import { useProjectStore } from '@/stores/project'
import { useSessionStore } from '@/stores/session'
import { useMessageStore } from '@/stores/message'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import DataClearConfirmDialog from '@/components/settings/data/DataClearConfirmDialog.vue'
import DataStorageCard from '@/components/settings/data/DataStorageCard.vue'
import DataTransferCard from '@/components/settings/data/DataTransferCard.vue'
import type {
  DataManagementStats,
  ExportOptions,
  ExportOptionItem,
  ExportOptionKey,
  ImportStatItem,
  ImportStats
} from '@/components/settings/data/types'

const { t } = useI18n()
const projectStore = useProjectStore()
const sessionStore = useSessionStore()
const messageStore = useMessageStore()

const exportOptions = reactive<ExportOptions>({
  include_projects: true,
  include_sessions: true,
  include_messages: true,
  include_agents: true,
  include_mcp_servers: true,
  include_cli_paths: true,
  include_app_settings: true
})

const exportOptionItems = computed<ExportOptionItem[]>(() => [
  { key: 'include_projects', label: t('settings.data.statsProjects'), checked: exportOptions.include_projects },
  { key: 'include_sessions', label: t('settings.data.statsSessions'), checked: exportOptions.include_sessions },
  { key: 'include_messages', label: t('settings.data.statsMessages'), checked: exportOptions.include_messages },
  { key: 'include_agents', label: t('settings.data.statsAgents'), checked: exportOptions.include_agents },
  { key: 'include_mcp_servers', label: t('settings.data.statsMcpServers'), checked: exportOptions.include_mcp_servers },
  { key: 'include_cli_paths', label: t('settings.data.statsCliPaths'), checked: exportOptions.include_cli_paths },
  { key: 'include_app_settings', label: t('settings.data.statsAppSettings'), checked: exportOptions.include_app_settings }
])

const allSelected = computed(() => Object.values(exportOptions).every(Boolean))
const hasAnySelected = computed(() => Object.values(exportOptions).some(Boolean))

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
    { key: 'app_settings_imported', label: t('settings.data.statsAppSettings'), value: importStats.value.app_settings_imported }
  ]

  return items.filter((item) => item.value > 0)
})

const showClearConfirm = ref(false)
const clearConfirmText = ref('')
const isClearing = ref(false)
const clearMessage = ref('')
const clearSuccess = ref(false)
const isLoadingStats = ref(false)
const statsError = ref<string | null>(null)
const dataStats = ref<DataManagementStats | null>(null)

function updateExportOption(key: ExportOptionKey, checked: boolean) {
  exportOptions[key] = checked
}

function toggleAllExportOptions(select: boolean) {
  Object.keys(exportOptions).forEach((key) => {
    exportOptions[key as ExportOptionKey] = select
  })
}

async function loadDataStats() {
  isLoadingStats.value = true
  statsError.value = null
  try {
    dataStats.value = await invoke<DataManagementStats>('get_data_management_stats')
  } catch (error) {
    console.error('Failed to load data stats:', error)
    statsError.value = error instanceof Error ? error.message : String(error)
  } finally {
    isLoadingStats.value = false
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
    await projectStore.loadProjects()
    sessionStore.sessions = []
    sessionStore.setCurrentSession(null)
    messageStore.messages = []
    await loadDataStats()
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
    await loadDataStats()

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
  void loadDataStats()
})
</script>

<template>
  <div class="settings-page">
    <h3 class="settings-page__title">
      {{ t('settings.nav.data') }}
    </h3>

    <DataStorageCard
      :loading="isLoadingStats"
      :error="statsError"
      :stats="dataStats"
      @refresh="loadDataStats"
    />

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

    <SettingsSectionCard
      :title="t('settings.data.dangerZone')"
      :description="t('settings.data.dangerZoneHint')"
      class="danger-card"
    >
      <div class="danger-card__content">
        <div class="danger-card__info">
          <span class="danger-card__badge">{{ t('settings.data.dangerZoneTag') }}</span>
          <div class="danger-card__title">
            {{ t('settings.data.clearAllData') }}
          </div>
          <div class="danger-card__desc">
            {{ t('settings.data.clearAllDataDesc') }}
          </div>
        </div>
        <EaButton
          type="danger"
          size="small"
          @click="handleClearData"
        >
          {{ t('settings.data.clearData') }}
        </EaButton>
      </div>
    </SettingsSectionCard>

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

.danger-card {
  border-color: color-mix(in srgb, var(--color-error) 16%, var(--color-border) 84%);
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--color-error-light) 38%, transparent) 0%, transparent 42%),
    linear-gradient(180deg, color-mix(in srgb, var(--color-bg-secondary) 94%, white 6%) 0%, var(--color-bg-secondary) 100%);
}

.danger-card__content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-4);
  padding: var(--spacing-4);
  border: 1px solid color-mix(in srgb, var(--color-error) 14%, var(--color-border) 86%);
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--color-bg-secondary) 84%, white 16%);
}

.danger-card__info {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: var(--spacing-2);
}

.danger-card__badge {
  display: inline-flex;
  width: fit-content;
  padding: 5px 10px;
  border: 1px solid color-mix(in srgb, var(--color-error) 18%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-error-light) 62%, white 38%);
  color: color-mix(in srgb, var(--color-error) 78%, #7f1d1d);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.danger-card__title {
  color: var(--color-text-primary);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
}

.danger-card__desc {
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  line-height: 1.6;
}

@media (max-width: 640px) {
  .danger-card__content {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
