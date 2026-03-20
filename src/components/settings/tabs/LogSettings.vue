<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import type { UnwatchFn } from '@tauri-apps/plugin-fs'
import { EaButton, EaStateBlock } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import { startFsWatcher } from '@/utils/fsWatcher'

interface RuntimeLogFileInfo {
  name: string
  path: string
  sizeBytes: number
  modifiedAt?: string | null
}

interface RuntimeLogSummary {
  logDir: string
  fileCount: number
  totalSizeBytes: number
  latestFile?: RuntimeLogFileInfo | null
}

interface RuntimeLogReadResult {
  file: RuntimeLogFileInfo
  content: string
  truncated: boolean
  lineCount: number
}

const { t } = useI18n()

const isLoading = ref(false)
const isClearing = ref(false)
const isSidebarVisible = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const summary = ref<RuntimeLogSummary | null>(null)
const files = ref<RuntimeLogFileInfo[]>([])
const selectedFileName = ref('')
const logContent = ref<RuntimeLogReadResult | null>(null)
let logWatcherCleanup: UnwatchFn | null = null
let watchGeneration = 0
let refreshTimer: number | null = null
let pollingTimer: number | null = null

const selectedFile = computed(() =>
  files.value.find((item) => item.name === selectedFileName.value) || null
)

function formatBytes(value: number): string {
  if (value <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let size = value
  let index = 0
  while (size >= 1024 && index < units.length - 1) {
    size /= 1024
    index += 1
  }
  return `${size.toFixed(size >= 10 || index === 0 ? 0 : 1)} ${units[index]}`
}

function formatDate(value?: string | null): string {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

async function loadLogFile(fileName?: string) {
  if (!fileName) {
    logContent.value = null
    return
  }

  try {
    logContent.value = await invoke<RuntimeLogReadResult>('read_runtime_log_file_command', {
      fileName,
      tailLines: 1200
    })
  } catch (error) {
    logContent.value = null
    errorMessage.value = `${t('settings.logs.readFailed')}: ${error}`
  }
}

async function loadLogs() {
  isLoading.value = true
  errorMessage.value = ''
  successMessage.value = ''

  try {
    const [nextSummary, nextFiles] = await Promise.all([
      invoke<RuntimeLogSummary>('get_runtime_log_summary_command'),
      invoke<RuntimeLogFileInfo[]>('list_runtime_log_files_command')
    ])

    summary.value = nextSummary
    files.value = nextFiles

    if (!nextFiles.length) {
      selectedFileName.value = ''
      logContent.value = null
      return
    }

    const preferredName = nextFiles.some((item) => item.name === selectedFileName.value)
      ? selectedFileName.value
      : nextFiles[0].name

    selectedFileName.value = preferredName
    await loadLogFile(preferredName)
  } catch (error) {
    errorMessage.value = `${t('settings.logs.loadFailed')}: ${error}`
  } finally {
    isLoading.value = false
  }
}

async function refreshLogsSilently() {
  try {
    const [nextSummary, nextFiles] = await Promise.all([
      invoke<RuntimeLogSummary>('get_runtime_log_summary_command'),
      invoke<RuntimeLogFileInfo[]>('list_runtime_log_files_command')
    ])

    summary.value = nextSummary
    files.value = nextFiles

    if (!nextFiles.length) {
      selectedFileName.value = ''
      logContent.value = null
      return
    }

    const preferredName = nextFiles.some((item) => item.name === selectedFileName.value)
      ? selectedFileName.value
      : nextFiles[0].name

    selectedFileName.value = preferredName
    await loadLogFile(preferredName)
  } catch (error) {
    console.error('[LogSettings] auto refresh failed:', error)
  }
}

async function bindLogWatcher(logDir?: string) {
  watchGeneration += 1
  const generation = watchGeneration

  if (logWatcherCleanup) {
    logWatcherCleanup()
    logWatcherCleanup = null
  }

  if (refreshTimer !== null) {
    window.clearTimeout(refreshTimer)
    refreshTimer = null
  }

  if (pollingTimer !== null) {
    window.clearInterval(pollingTimer)
    pollingTimer = null
  }

  if (!logDir) {
    return
  }

  const unwatch = await startFsWatcher(logDir, () => {
    if (refreshTimer !== null) {
      window.clearTimeout(refreshTimer)
    }
    refreshTimer = window.setTimeout(() => {
      void refreshLogsSilently()
    }, 150)
  }, {
    recursive: false,
    delayMs: 120
  })

  if (generation === watchGeneration) {
    logWatcherCleanup = unwatch
    pollingTimer = window.setInterval(() => {
      void refreshLogsSilently()
    }, 1200)
  } else if (unwatch) {
    unwatch()
  }
}

async function handleClearLogs() {
  if (isClearing.value) {
    return
  }

  if (!window.confirm(t('settings.logs.clearConfirm'))) {
    return
  }

  isClearing.value = true
  errorMessage.value = ''
  successMessage.value = ''

  try {
    const removed = await invoke<number>('clear_runtime_log_files_command')
    successMessage.value = t('settings.logs.clearSuccess', { count: removed })
    await loadLogs()
  } catch (error) {
    errorMessage.value = `${t('settings.logs.clearFailed')}: ${error}`
  } finally {
    isClearing.value = false
  }
}

watch(selectedFileName, async (fileName, previous) => {
  if (!fileName || fileName === previous) {
    return
  }
  errorMessage.value = ''
  await loadLogFile(fileName)
})

watch(
  () => summary.value?.logDir,
  (logDir) => {
    void bindLogWatcher(logDir)
  },
  { immediate: true }
)

onMounted(() => {
  void loadLogs()
})

onBeforeUnmount(() => {
  if (logWatcherCleanup) {
    logWatcherCleanup()
    logWatcherCleanup = null
  }
  if (refreshTimer !== null) {
    window.clearTimeout(refreshTimer)
    refreshTimer = null
  }
  if (pollingTimer !== null) {
    window.clearInterval(pollingTimer)
    pollingTimer = null
  }
})
</script>

<template>
  <div class="log-settings">
    <SettingsSectionCard
      :title="t('settings.logs.title')"
      :description="t('settings.logs.description')"
    >
      <template #actions>
        <EaButton
          type="secondary"
          :disabled="!files.length"
          @click="isSidebarVisible = !isSidebarVisible"
        >
          {{ isSidebarVisible ? t('settings.logs.hideFiles') : t('settings.logs.showFiles') }}
        </EaButton>
        <EaButton
          type="danger"
          :loading="isClearing"
          :disabled="!files.length"
          @click="handleClearLogs"
        >
          {{ t('settings.logs.clear') }}
        </EaButton>
      </template>

      <EaStateBlock
        v-if="isLoading && !summary"
        variant="loading"
        :title="t('settings.logs.loadingTitle')"
        :description="t('settings.logs.loadingDescription')"
      />
      <EaStateBlock
        v-else-if="errorMessage"
        variant="error"
        :title="t('settings.logs.errorTitle')"
        :description="errorMessage"
      />
      <EaStateBlock
        v-else-if="!files.length"
        variant="empty"
        icon="scroll-text"
        :title="t('settings.logs.emptyTitle')"
        :description="t('settings.logs.emptyDescription')"
      />

      <div
        v-if="files.length"
        :class="['log-settings__workspace', { 'log-settings__workspace--full': !isSidebarVisible }]"
      >
        <div
          v-if="isSidebarVisible"
          class="log-settings__sidebar"
        >
          <button
            v-for="item in files"
            :key="item.name"
            :class="['log-settings__file', { 'log-settings__file--active': item.name === selectedFileName }]"
            @click="selectedFileName = item.name"
          >
            <div class="log-settings__file-name">
              {{ item.name }}
            </div>
            <div class="log-settings__file-meta">
              <span>{{ formatDate(item.modifiedAt) }}</span>
              <span>{{ formatBytes(item.sizeBytes) }}</span>
            </div>
          </button>
        </div>

        <div class="log-settings__viewer">
          <div
            v-if="selectedFile"
            class="log-settings__viewer-header"
          >
            <div>
              <div class="log-settings__viewer-title">
                {{ selectedFile.name }}
              </div>
              <div class="log-settings__viewer-subtitle">
                {{ formatDate(selectedFile.modifiedAt) }}
              </div>
            </div>
            <div class="log-settings__viewer-badges">
              <span class="badge">{{ formatBytes(selectedFile.sizeBytes) }}</span>
              <span
                v-if="logContent"
                class="badge"
              >
                {{ t('settings.logs.lineCount', { count: logContent.lineCount }) }}
              </span>
            </div>
          </div>

          <div
            v-if="logContent?.truncated"
            class="log-settings__truncated"
          >
            {{ t('settings.logs.truncatedNotice') }}
          </div>

          <pre class="log-settings__content">{{ logContent?.content || '' }}</pre>
        </div>
      </div>

      <p
        v-if="successMessage"
        class="log-settings__success"
      >
        {{ successMessage }}
      </p>
    </SettingsSectionCard>
  </div>
</template>

<style scoped>
.log-settings {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  min-height: 100%;
  height: 100%;
}

.log-settings__workspace {
  display: grid;
  grid-template-columns: 220px minmax(0, 1fr);
  gap: var(--spacing-4);
  min-height: 780px;
  height: 100%;
}

.log-settings__workspace--full {
  grid-template-columns: minmax(0, 1fr);
}

.log-settings__sidebar {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding-right: var(--spacing-2);
  overflow-y: auto;
  border-right: 1px solid var(--color-border);
}

.log-settings__file {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  text-align: left;
  transition: all var(--transition-fast) var(--easing-default);
}

.log-settings__file:hover {
  border-color: var(--color-primary);
  background: var(--color-surface-hover);
}

.log-settings__file--active {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
}

.log-settings__file-name {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.log-settings__file-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.log-settings__viewer {
  display: flex;
  flex-direction: column;
  min-height: 780px;
  min-width: 0;
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  background: #07111f;
}

.log-settings__viewer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  padding: var(--spacing-3) var(--spacing-4);
  border-bottom: 1px solid rgba(148, 163, 184, 0.18);
  background: rgba(15, 23, 42, 0.92);
}

.log-settings__viewer-title {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: #f8fafc;
}

.log-settings__viewer-subtitle {
  font-size: var(--font-size-xs);
  color: #94a3b8;
}

.log-settings__viewer-badges {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.badge {
  padding: 4px 8px;
  border-radius: var(--radius-full);
  background: rgba(59, 130, 246, 0.16);
  color: #bfdbfe;
  font-size: 11px;
}

.log-settings__truncated {
  padding: var(--spacing-2) var(--spacing-4);
  background: rgba(245, 158, 11, 0.12);
  color: #fbbf24;
  font-size: var(--font-size-xs);
  border-bottom: 1px solid rgba(245, 158, 11, 0.22);
}

.log-settings__content {
  flex: 1;
  margin: 0;
  padding: var(--spacing-4);
  overflow: auto;
  color: #dbeafe;
  font-size: 12px;
  line-height: 1.6;
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
  white-space: pre-wrap;
  word-break: break-word;
}

.log-settings__success {
  margin: 0;
  color: var(--color-success);
  font-size: var(--font-size-sm);
}

@media (max-width: 960px) {
  .log-settings__workspace {
    grid-template-columns: 1fr;
    min-height: 680px;
  }

  .log-settings__sidebar {
    max-height: 180px;
    padding-right: 0;
    padding-bottom: var(--spacing-2);
    border-right: none;
    border-bottom: 1px solid var(--color-border);
  }

  .log-settings__viewer {
    min-height: 620px;
  }
}
</style>
