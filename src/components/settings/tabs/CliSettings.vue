<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, reactive, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSettingsStore, type CliPathEntry } from '@/stores/settings'
import { EaButton, EaIcon } from '@/components/common'
import CliCustomPathList from '@/components/settings/cli/CliCustomPathList.vue'
import CliDeleteDialog from '@/components/settings/cli/CliDeleteDialog.vue'
import CliDetectedList from '@/components/settings/cli/CliDetectedList.vue'
import CliInstallerSection from '@/components/settings/cli/CliInstallerSection.vue'
import CliPathModal from '@/components/settings/cli/CliPathModal.vue'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { CliInstallerInfo, VersionInfo, InstallLogEvent } from '@/components/settings/cli/types'

const { t } = useI18n()
const settingsStore = useSettingsStore()

// CLI 名称选项
const cliNameOptions = computed(() => [
  { value: 'claude', label: 'Claude CLI' },
  { value: 'codex', label: 'Codex CLI' },
  { value: 'aider', label: 'Aider' },
  { value: 'cursor', label: 'Cursor' },
  { value: 'other', label: t('settings.cli.cliNameOther') }
])

// 模态框状态
const showModal = ref(false)
const modalMode = ref<'add' | 'edit'>('add')
const editingEntry = ref<CliPathEntry | null>(null)

// 表单状态
const formData = reactive({
  name: '',
  path: ''
})
const formError = ref('')
const isValidating = ref(false)
const validationResult = ref<{ valid: boolean; version: string | null } | null>(null)

// 删除确认状态
const showDeleteConfirm = ref(false)
const deletingEntry = ref<CliPathEntry | null>(null)

// CLI 安装器状态
const installLogs = ref<InstallLogEvent[]>([])
const isInstalling = ref(false)
const currentInstallingCli = ref<string | null>(null)

// 版本检测状态
const isCheckingUpdate = ref<string | null>(null)
const isUpgrading = ref<string | null>(null)
const claudeVersionInfo = ref<VersionInfo | null>(null)
const codexVersionInfo = ref<VersionInfo | null>(null)

// CLI 安装信息
const claudeInstallInfo = ref<CliInstallerInfo | null>(null)
const codexInstallInfo = ref<CliInstallerInfo | null>(null)

// 事件监听器取消函数
let unlistenLog: (() => void) | null = null
let unlistenComplete: (() => void) | null = null

// 计算属性
const hasCliTools = computed(() => settingsStore.cliTools.length > 0)
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

// 处理检测
const handleDetect = async () => {
  try {
    await settingsStore.detectCliTools()
  } catch (error) {
    console.error('Detection failed:', error)
  }
}

// 打开添加模态框
const openAddModal = () => {
  modalMode.value = 'add'
  editingEntry.value = null
  formData.name = ''
  formData.path = ''
  formError.value = ''
  validationResult.value = null
  showModal.value = true
}

// 打开编辑模态框
const openEditModal = (entry: CliPathEntry) => {
  modalMode.value = 'edit'
  editingEntry.value = entry
  formData.name = entry.name
  formData.path = entry.path
  formError.value = ''
  validationResult.value = entry.version
    ? { valid: true, version: entry.version }
    : null
  showModal.value = true
}

// 关闭模态框
const closeModal = () => {
  showModal.value = false
  editingEntry.value = null
}

// 浏览文件
const handleBrowse = async () => {
  const selected = await open({
    title: '选择 CLI 可执行文件',
    multiple: false,
    directory: false,
    filters: [
      { name: '可执行文件', extensions: ['*'] }
    ]
  })

  if (selected && typeof selected === 'string') {
    formData.path = selected
    // 自动验证
    await validatePath()
  }
}

// 验证路径
const validatePath = async () => {
  if (!formData.path) {
    validationResult.value = null
    return
  }

  isValidating.value = true
  formError.value = ''

  try {
    const tool = await settingsStore.verifyCliPath(formData.path)
    validationResult.value = {
      valid: tool.status === 'available',
      version: tool.version
    }
    if (tool.status !== 'available' && formData.name === '') {
      formData.name = tool.name
    }
  } catch {
    formError.value = '验证失败'
    validationResult.value = { valid: false, version: null }
  } finally {
    isValidating.value = false
  }
}

// 提交表单
const handleSubmit = async () => {
  if (!formData.name || !formData.path) {
    formError.value = t('settings.cli.nameAndPathRequired')
    return
  }

  formError.value = ''

  try {
    if (modalMode.value === 'add') {
      await settingsStore.addCustomCliPath(formData.name, formData.path)
    } else if (editingEntry.value) {
      await settingsStore.updateCustomCliPath(
        editingEntry.value.id,
        formData.name,
        formData.path
      )
    }
    closeModal()
  } catch (error) {
    formError.value = t('settings.cli.saveFailed')
    console.error('Save failed:', error)
  }
}

// 请求删除
const requestDelete = (entry: CliPathEntry) => {
  deletingEntry.value = entry
  showDeleteConfirm.value = true
}

// 确认删除
const confirmDelete = async () => {
  if (!deletingEntry.value) return

  try {
    await settingsStore.deleteCustomCliPath(deletingEntry.value.id)
    showDeleteConfirm.value = false
    deletingEntry.value = null
  } catch (error) {
    console.error('Delete failed:', error)
  }
}

// 组件挂载时加载数据
onMounted(async () => {
  if (settingsStore.cliTools.length === 0) {
    handleDetect()
  }
  settingsStore.loadCustomCliPaths()

  // 加载安装选项
  await loadInstallOptions()

  // 自动检测版本更新
  await checkAllUpdates()

  // 监听实时日志
  unlistenLog = await listen('cli-install-log', (event) => {
    const log = event.payload as InstallLogEvent
    installLogs.value.push(log)
    // 自动滚动到底部
    nextTick(() => {
      const container = document.querySelector('.install-logs__content')
      if (container) {
        container.scrollTop = container.scrollHeight
      }
    })
  })

  // 监听安装完成
  unlistenComplete = await listen('cli-install-complete', async (event) => {
    const result = event.payload as { cli_name: string; success: boolean }
    isInstalling.value = false
    isUpgrading.value = null
    currentInstallingCli.value = null

    if (result.success) {
      // 重新检测 CLI
      await settingsStore.detectCliTools()
      // 重新加载安装选项
      await loadInstallOptions()
      // 重新检测版本
      await checkAllUpdates()
    }
  })
})

onUnmounted(() => {
  if (unlistenLog) unlistenLog()
  if (unlistenComplete) unlistenComplete()
})

// 加载安装选项
const loadInstallOptions = async () => {
  try {
    claudeInstallInfo.value = await invoke<CliInstallerInfo>('get_cli_install_options', { cliName: 'claude' })
    codexInstallInfo.value = await invoke<CliInstallerInfo>('get_cli_install_options', { cliName: 'codex' })
  } catch (error) {
    console.error('Failed to load install options:', error)
  }
}

// 检测所有 CLI 更新
const checkAllUpdates = async () => {
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

// 手动检测更新
const handleCheckUpdate = async (cliName: string) => {
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

// 执行安装
const handleInstall = async (cliName: string, method: string) => {
  if (isInstalling.value) return

  isInstalling.value = true
  currentInstallingCli.value = cliName
  installLogs.value = []

  try {
    await invoke('install_cli', { cliName, method })
  } catch (error) {
    console.error('Install failed:', error)
  }
}

// 执行升级
const handleUpgrade = async (cliName: string) => {
  if (isUpgrading.value) return

  isUpgrading.value = cliName
  installLogs.value = []

  try {
    await invoke('upgrade_cli', { cliName })
  } catch (error) {
    console.error('Upgrade failed:', error)
  }
}

const handleModalVisibleChange = (visible: boolean) => {
  showModal.value = visible
  if (!visible) {
    editingEntry.value = null
  }
}

const handleDeleteVisibleChange = (visible: boolean) => {
  showDeleteConfirm.value = visible
  if (!visible) {
    deletingEntry.value = null
  }
}
</script>

<template>
  <div class="settings-page">
    <div class="settings-page__header">
      <h3 class="settings-page__title">
        {{ t('settings.cli.title') }}
      </h3>
      <div class="settings-page__actions">
        <EaButton
          type="secondary"
          size="small"
          :loading="settingsStore.isDetectingCli"
          @click="handleDetect"
        >
          <EaIcon
            v-if="!settingsStore.isDetectingCli"
            name="search"
            :size="16"
          />
          {{ t('settings.cli.autoDetect') }}
        </EaButton>
      </div>
    </div>

    <!-- 检测进度提示 -->
    <div
      v-if="settingsStore.isDetectingCli"
      class="detection-progress"
    >
      <div class="detection-progress__spinner">
        <svg
          viewBox="0 0 24 24"
          class="animate-spin"
        >
          <circle
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            stroke-width="3"
            fill="none"
            stroke-dasharray="31.416"
            stroke-dashoffset="10"
          />
        </svg>
      </div>
      <span class="detection-progress__text">{{ t('settings.cli.scanning') }}</span>
    </div>

    <!-- 检测结果提示 -->
    <div
      v-else-if="settingsStore.detectionComplete && hasCliTools"
      class="detection-result detection-result--success"
    >
      <EaIcon
        name="check-circle"
        :size="16"
      />
      <span>{{ t('settings.cli.foundTools', { n: settingsStore.foundCliCount }) }}</span>
    </div>

    <div
      v-else-if="settingsStore.detectionComplete && !hasCliTools"
      class="detection-result detection-result--empty"
    >
      <EaIcon
        name="info"
        :size="16"
      />
      <span>{{ t('settings.cli.noToolsFound') }}</span>
    </div>

    <CliDetectedList
      v-if="hasCliTools"
      :tools="settingsStore.cliTools"
    />

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

    <CliCustomPathList
      :entries="settingsStore.customCliPaths"
      @add="openAddModal"
      @edit="openEditModal"
      @delete="requestDelete"
    />

    <CliPathModal
      :visible="showModal"
      :mode="modalMode"
      :name="formData.name"
      :path="formData.path"
      :name-options="cliNameOptions"
      :form-error="formError"
      :is-validating="isValidating"
      :validation-result="validationResult"
      @update:visible="handleModalVisibleChange"
      @update:name="formData.name = $event"
      @update:path="formData.path = $event"
      @browse="handleBrowse"
      @validate="validatePath"
      @submit="handleSubmit"
    />

    <CliDeleteDialog
      :visible="showDeleteConfirm"
      :name="deletingEntry?.name"
      @update:visible="handleDeleteVisibleChange"
      @confirm="confirmDelete"
    />
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-6);
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

.settings-page__actions {
  display: flex;
  gap: var(--spacing-2);
}

/* 检测进度 */
.detection-progress {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  background-color: var(--color-primary-light);
  border-radius: var(--radius-lg);
}

.detection-progress__spinner {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  color: var(--color-primary);
}

.detection-progress__spinner svg {
  width: 100%;
  height: 100%;
}

.detection-progress__text {
  font-size: var(--font-size-sm);
  color: var(--color-primary);
}

/* 检测结果提示 */
.detection-result {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-3) var(--spacing-4);
  border-radius: var(--radius-lg);
  font-size: var(--font-size-sm);
}

.detection-result--success {
  background-color: var(--color-success-light, rgba(34, 197, 94, 0.1));
  color: var(--color-success);
}

.detection-result--empty {
  background-color: var(--color-surface-hover);
  color: var(--color-text-secondary);
}

/* CLI 区块 */
.cli-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.cli-section__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.cli-section__title {
  margin: 0;
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
}

/* 动画 */
.animate-spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
