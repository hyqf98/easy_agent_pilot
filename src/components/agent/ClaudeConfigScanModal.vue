<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { EaButton, EaIcon } from '@/components/common'
import ClaudeScanMcpList from '@/components/agent/claude-scan/ClaudeScanMcpList.vue'
import ClaudeScanPluginsList from '@/components/agent/claude-scan/ClaudeScanPluginsList.vue'
import ClaudeScanSkillsList from '@/components/agent/claude-scan/ClaudeScanSkillsList.vue'
import ClaudeScanTabs from '@/components/agent/claude-scan/ClaudeScanTabs.vue'
import type { ClaudeConfigScanResult } from '@/stores/skillConfigShared'
import type { ClaudeScanTab, SelectedItems } from '@/components/agent/claude-scan/shared'

const emit = defineEmits<{
  close: []
  import: [items: SelectedItems]
}>()

const { t } = useI18n()

const isScanning = ref(false)
const scanResult = ref<ClaudeConfigScanResult | null>(null)
const scanError = ref('')

// 选中的项目
const selectedMcpServers = ref<string[]>([])
const selectedSkills = ref<string[]>([])
const selectedPlugins = ref<string[]>([])

// 当前标签页
const activeTab = ref<ClaudeScanTab>('mcp')

// 计算选中的总数
const selectedCount = computed(() => {
  return selectedMcpServers.value.length + selectedSkills.value.length + selectedPlugins.value.length
})

// 是否可以导入
const canImport = computed(() => selectedCount.value > 0)
const tabCounts = computed(() => ({
  mcp: scanResult.value?.mcp_servers.length ?? 0,
  skills: scanResult.value?.skills.length ?? 0,
  plugins: scanResult.value?.plugins.length ?? 0
}))

// 扫描配置
const scanConfig = async () => {
  isScanning.value = true
  scanError.value = ''
  scanResult.value = null

  try {
    const result = await invoke<ClaudeConfigScanResult>('scan_claude_config')
    scanResult.value = result

    if (!result.scan_success && result.error_message) {
      scanError.value = result.error_message
    }
  } catch (error) {
    scanError.value = String(error)
  } finally {
    isScanning.value = false
  }
}

// 全选/取消全选 MCP
const toggleAllMcp = () => {
  if (!scanResult.value) return

  if (selectedMcpServers.value.length === scanResult.value.mcp_servers.length) {
    selectedMcpServers.value = []
  } else {
    selectedMcpServers.value = scanResult.value.mcp_servers.map(s => s.name)
  }
}

// 全选/取消全选 Skills
const toggleAllSkills = () => {
  if (!scanResult.value) return

  if (selectedSkills.value.length === scanResult.value.skills.length) {
    selectedSkills.value = []
  } else {
    selectedSkills.value = scanResult.value.skills.map(s => s.name)
  }
}

// 全选/取消全选 Plugins
const toggleAllPlugins = () => {
  if (!scanResult.value) return

  if (selectedPlugins.value.length === scanResult.value.plugins.length) {
    selectedPlugins.value = []
  } else {
    selectedPlugins.value = scanResult.value.plugins.map(s => s.name)
  }
}

// 处理导入
const handleImport = () => {
  emit('import', {
    mcpServers: selectedMcpServers.value,
    skills: selectedSkills.value,
    plugins: selectedPlugins.value
  })
}

// 关闭弹窗
const handleClose = () => {
  emit('close')
}

// 切换 MCP 选中状态
const toggleMcpServer = (name: string) => {
  const index = selectedMcpServers.value.indexOf(name)
  if (index === -1) {
    selectedMcpServers.value.push(name)
  } else {
    selectedMcpServers.value.splice(index, 1)
  }
}

// 切换 Skill 选中状态
const toggleSkill = (name: string) => {
  const index = selectedSkills.value.indexOf(name)
  if (index === -1) {
    selectedSkills.value.push(name)
  } else {
    selectedSkills.value.splice(index, 1)
  }
}

// 切换 Plugin 选中状态
const togglePlugin = (name: string) => {
  const index = selectedPlugins.value.indexOf(name)
  if (index === -1) {
    selectedPlugins.value.push(name)
  } else {
    selectedPlugins.value.splice(index, 1)
  }
}

// 组件挂载时自动扫描
watch(() => true, () => {
  scanConfig()
}, { immediate: true })
</script>

<template>
  <div class="scan-modal">
    <div class="scan-modal__header">
      <h3 class="scan-modal__title">
        {{ t('settings.agent.scan.title') }}
      </h3>
      <button
        class="scan-modal__close"
        @click="handleClose"
      >
        <EaIcon
          name="close"
          :size="18"
        />
      </button>
    </div>

    <div class="scan-modal__body">
      <!-- 扫描中状态 -->
      <div
        v-if="isScanning"
        class="scan-modal__loading"
      >
        <EaIcon
          name="loader"
          :size="24"
          spin
        />
        <span>{{ t('settings.agent.scan.scanning') }}</span>
      </div>

      <!-- 扫描错误 -->
      <div
        v-else-if="scanError"
        class="scan-modal__error"
      >
        <EaIcon
          name="alert-circle"
          :size="24"
        />
        <span>{{ scanError }}</span>
        <EaButton
          type="secondary"
          @click="scanConfig"
        >
          {{ t('common.retry') }}
        </EaButton>
      </div>

      <!-- 扫描结果 -->
      <template v-else-if="scanResult">
        <!-- 扫描目录信息 -->
        <div class="scan-modal__info">
          <EaIcon
            name="folder"
            :size="16"
          />
          <span>{{ scanResult.claude_dir }}</span>
        </div>

        <ClaudeScanTabs
          v-model:active-tab="activeTab"
          :counts="tabCounts"
        />

        <ClaudeScanMcpList
          v-show="activeTab === 'mcp'"
          :items="scanResult.mcp_servers"
          :selected-names="selectedMcpServers"
          @toggle-all="toggleAllMcp"
          @toggle-item="toggleMcpServer"
        />

        <ClaudeScanSkillsList
          v-show="activeTab === 'skills'"
          :items="scanResult.skills"
          :selected-names="selectedSkills"
          @toggle-all="toggleAllSkills"
          @toggle-item="toggleSkill"
        />

        <ClaudeScanPluginsList
          v-show="activeTab === 'plugins'"
          :items="scanResult.plugins"
          :selected-names="selectedPlugins"
          @toggle-all="toggleAllPlugins"
          @toggle-item="togglePlugin"
        />
      </template>
    </div>

    <div class="scan-modal__footer">
      <span
        v-if="selectedCount > 0"
        class="scan-modal__selected-count"
      >
        {{ t('settings.agent.scan.selectedCount', { n: selectedCount }) }}
      </span>
      <span
        v-else
        class="scan-modal__selected-count scan-modal__selected-count--empty"
      >
        {{ t('settings.agent.scan.noSelection') }}
      </span>
      <div class="scan-modal__actions">
        <EaButton
          type="secondary"
          @click="handleClose"
        >
          {{ t('common.cancel') }}
        </EaButton>
        <EaButton
          type="primary"
          :disabled="!canImport"
          @click="handleImport"
        >
          {{ t('settings.agent.scan.importSelected') }}
        </EaButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.scan-modal {
  display: flex;
  flex-direction: column;
  background-color: var(--color-surface);
  border-radius: var(--radius-lg);
  width: 560px;
  max-height: 80vh;
  overflow: hidden;
}

.scan-modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4) var(--spacing-5);
  border-bottom: 1px solid var(--color-border);
}

.scan-modal__title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.scan-modal__close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.scan-modal__close:hover {
  background-color: var(--color-hover);
  color: var(--color-text-primary);
}

.scan-modal__body {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-4) var(--spacing-5);
}

.scan-modal__loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-3);
  padding: var(--spacing-8);
  color: var(--color-text-secondary);
}

.scan-modal__error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-3);
  padding: var(--spacing-8);
  color: var(--color-error);
}

.scan-modal__info {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-background);
  border-radius: var(--radius-md);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-4);
}

.scan-modal__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4) var(--spacing-5);
  border-top: 1px solid var(--color-border);
}

.scan-modal__selected-count {
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.scan-modal__selected-count--empty {
  color: var(--color-text-tertiary);
}

.scan-modal__actions {
  display: flex;
  gap: var(--spacing-3);
}
</style>
