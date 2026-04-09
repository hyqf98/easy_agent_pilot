<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAgentStore, type AgentConfig } from '@/stores/agent'
import { useSkillConfigStore, type UnifiedMcpConfig, type UnifiedSkillConfig, type UnifiedPluginConfig } from '@/stores/skillConfig'
import AgentSelector from './AgentSelector.vue'
import McpConfigTab from './tabs/McpConfigTab.vue'
import SkillsConfigTab from './tabs/SkillsConfigTab.vue'
import PluginsConfigTab from './tabs/PluginsConfigTab.vue'
import CliConfigSyncModal from './modals/CliConfigSyncModal.vue'
import AddModePickerModal from './modals/AddModePickerModal.vue'
import SkillEditModal from './modals/SkillEditModal.vue'
import PluginEditModal from './modals/PluginEditModal.vue'
import SkillCreateView from './skills/SkillCreateView.vue'
import SkillDetailView from './views/SkillDetailView.vue'
import PluginDetailView from './views/PluginDetailView.vue'
import SkillGitInstallModal from '@/components/marketplace/skills/SkillGitInstallModal.vue'
import PluginGitInstallModal from '@/components/marketplace/plugins/PluginGitInstallModal.vue'
import { EaButton, EaIcon } from '@/components/common'
import { useOverlayDismiss } from '@/composables/useOverlayDismiss'
import type { CliSyncResult, CreateVisualSkillInput, SyncConfigType } from '@/stores/skillConfig'

const { t } = useI18n()
const agentStore = useAgentStore()
const skillConfigStore = useSkillConfigStore()

const activeTab = ref<'mcp' | 'skills' | 'plugins'>('mcp')

const showPluginsTab = computed(() => skillConfigStore.supportsPlugins)

// 内容区域引用，用于重置滚动位置
const contentRef = ref<HTMLElement | null>(null)
const showSyncModal = ref(false)
const syncType = ref<SyncConfigType>('mcp')
const showSkillModal = ref(false)
const showPluginModal = ref(false)
const showSkillAddModeModal = ref(false)
const showPluginAddModeModal = ref(false)
const showSkillGitInstallModal = ref(false)
const showPluginGitInstallModal = ref(false)
const showSkillBuilder = ref(false)
const isCreatingSkill = ref(false)
const editingSkill = ref<UnifiedSkillConfig | null>(null)
const editingPlugin = ref<UnifiedPluginConfig | null>(null)
const {
  handleOverlayPointerDown: handleDeleteOverlayPointerDown,
  handleOverlayClick: handleDeleteOverlayClick
} = useOverlayDismiss(() => {
  showDeleteConfirm.value = false
})

watch(activeTab, () => {
  if (activeTab.value !== 'skills') {
    showSkillBuilder.value = false
  }

  nextTick(() => {
    if (contentRef.value) {
      contentRef.value.scrollTop = 0
    }
  })
})

watch(
  () => [activeTab.value, skillConfigStore.selectedAgent?.id, skillConfigStore.selectedAgent?.type] as const,
  ([tab, agentId, agentType]) => {
    if (!agentId || agentType !== 'cli') {
      return
    }

    if (tab === 'skills' || tab === 'plugins') {
      void skillConfigStore.ensureCliInventoryLoaded()
    }
  },
  { immediate: true }
)

watch(showSkillModal, (value) => {
  if (!value) {
    editingSkill.value = null
  }
})

watch(showPluginModal, (value) => {
  if (!value) {
    editingPlugin.value = null
  }
})

const showDeleteConfirm = ref(false)
const deletingConfig = ref<{ type: 'mcp' | 'skills' | 'plugins'; config: UnifiedMcpConfig | UnifiedSkillConfig | UnifiedPluginConfig } | null>(null)

// 加载智能体列表
onMounted(async () => {
  await agentStore.loadAgents()
})

// 选择智能体
async function handleSelectAgent(agent: any) {
  showSkillBuilder.value = false
  showSkillModal.value = false
  showPluginModal.value = false
  showSkillAddModeModal.value = false
  showPluginAddModeModal.value = false
  showSkillGitInstallModal.value = false
  showPluginGitInstallModal.value = false
  editingSkill.value = null
  editingPlugin.value = null
  await skillConfigStore.selectAgent(agent)
}

// MCP 操作
async function handleSaveMcp(config: Partial<UnifiedMcpConfig>, originalId?: string) {
  if (originalId) {
    await skillConfigStore.updateMcpConfig(originalId, config)
  } else {
    await skillConfigStore.createMcpConfig({
      ...config,
      id: '',
      source: 'database',
      isReadOnly: false,
    } as any)
  }
}

function handleDeleteMcp(config: UnifiedMcpConfig) {
  deletingConfig.value = { type: 'mcp', config }
  showDeleteConfirm.value = true
}

async function confirmDelete() {
  if (!deletingConfig.value) return

  try {
    switch (deletingConfig.value.type) {
      case 'mcp':
        await skillConfigStore.deleteMcpConfig((deletingConfig.value.config as UnifiedMcpConfig).id)
        break
      case 'skills':
        await skillConfigStore.deleteSkillWithFiles(deletingConfig.value.config as UnifiedSkillConfig)
        break
      case 'plugins':
        await skillConfigStore.deletePluginWithFiles(deletingConfig.value.config as UnifiedPluginConfig)
        break
    }
  } finally {
    showDeleteConfirm.value = false
    deletingConfig.value = null
  }
}

// Skills 鎿嶄綔
async function handleAddSkill() {
  const agent = skillConfigStore.selectedAgent
  if (!agent) {
    return
  }

  if (agent.type === 'cli' && (agent.provider === 'claude' || agent.provider === 'codex' || agent.provider === 'opencode')) {
    showSkillAddModeModal.value = true
    return
  }

  await openSkillManualCreate(agent)
}

async function openSkillManualCreate(agent: AgentConfig) {
  if (agent.type === 'cli') {
    await skillConfigStore.resolveCliConfigPaths(agent)
    showSkillBuilder.value = true
    return
  }

  editingSkill.value = null
  showSkillModal.value = true
}

async function handleSelectSkillAddMode(mode: string) {
  const agent = skillConfigStore.selectedAgent
  if (!agent) {
    return
  }

  if (mode === 'git') {
    showSkillGitInstallModal.value = true
    return
  }

  await openSkillManualCreate(agent)
}

async function handleSkillGitInstallComplete() {
  showSkillGitInstallModal.value = false
  if (skillConfigStore.selectedAgent?.type === 'cli') {
    await skillConfigStore.refreshCliConfigs()
  }
}

function handleCloseSkillGitInstall() {
  showSkillGitInstallModal.value = false
}

// Plugins 鎿嶄綔
function handleAddPlugin() {
  const agent = skillConfigStore.selectedAgent
  if (!agent) {
    return
  }

  if (agent.type === 'cli' && (agent.provider === 'claude' || agent.provider === 'codex' || agent.provider === 'opencode')) {
    showPluginAddModeModal.value = true
    return
  }

  openPluginManualCreate()
}

function openPluginManualCreate() {
  editingPlugin.value = null
  showPluginModal.value = true
}

async function handleSelectPluginAddMode(mode: string) {
  if (mode === 'git') {
    showPluginGitInstallModal.value = true
    return
  }

  openPluginManualCreate()
}

async function handlePluginGitInstallComplete() {
  showPluginGitInstallModal.value = false
  if (skillConfigStore.selectedAgent?.type === 'cli') {
    await skillConfigStore.refreshCliConfigs()
  }
}

function handleClosePluginGitInstall() {
  showPluginGitInstallModal.value = false
}

const addModeOptions = computed(() => ({
  skills: [
    {
      id: 'manual',
      label: t('settings.sdkConfig.addMode.manualSkillLabel'),
      description: t('settings.sdkConfig.addMode.manualSkillDescription'),
      icon: 'lucide:wand-sparkles'
    },
    {
      id: 'git',
      label: t('settings.sdkConfig.addMode.gitSkillLabel'),
      description: t('settings.sdkConfig.addMode.gitSkillDescription'),
      icon: 'lucide:git-branch'
    }
  ],
  plugins: [
    {
      id: 'manual',
      label: t('settings.sdkConfig.addMode.manualPluginLabel'),
      description: t('settings.sdkConfig.addMode.manualPluginDescription'),
      icon: 'lucide:puzzle'
    },
    {
      id: 'git',
      label: t('settings.sdkConfig.addMode.gitPluginLabel'),
      description: t('settings.sdkConfig.addMode.gitPluginDescription'),
      icon: 'lucide:git-branch'
    }
  ]
}))

const currentCliAgentId = computed(() =>
  skillConfigStore.selectedAgent?.type === 'cli'
    ? skillConfigStore.selectedAgent.id
    : undefined
)

function handleViewSkillDetail(config: UnifiedSkillConfig) {
  skillConfigStore.viewSkillDetail(config)
}

function handleEditSkill(config: UnifiedSkillConfig) {
  editingSkill.value = config
  showSkillModal.value = true
}

function handleDeleteSkill(config: UnifiedSkillConfig) {
  deletingConfig.value = { type: 'skills', config }
  showDeleteConfirm.value = true
}

function handleBackFromSkill() {
  skillConfigStore.clearDetailState()
}

async function handleDeleteSkillFromDetail(skill: UnifiedSkillConfig) {
  deletingConfig.value = { type: 'skills', config: skill }
  showDeleteConfirm.value = true
}

async function handleSaveSkill(config: Partial<UnifiedSkillConfig>, originalId?: string) {
  if (originalId) {
    await skillConfigStore.updateSkillsConfig(originalId, config)
  } else {
    const payload: Omit<UnifiedSkillConfig, 'id' | 'source' | 'isReadOnly'> = {
      ...config,
      enabled: true,
    } as Omit<UnifiedSkillConfig, 'id' | 'source' | 'isReadOnly'>
    await skillConfigStore.createSkillsConfig(payload)
  }

  showSkillModal.value = false
  editingSkill.value = null
}

function handleBackFromSkillBuilder() {
  showSkillBuilder.value = false
}

async function handleCreateVisualSkill(input: CreateVisualSkillInput) {
  isCreatingSkill.value = true
  try {
    await skillConfigStore.createVisualSkill(input)
    showSkillBuilder.value = false
  } finally {
    isCreatingSkill.value = false
  }
}

function handleViewPluginDetail(config: UnifiedPluginConfig) {
  skillConfigStore.viewPluginDetail(config)
}

function handleEditPlugin(config: UnifiedPluginConfig) {
  editingPlugin.value = config
  showPluginModal.value = true
}

async function handleSavePlugin(config: Partial<UnifiedPluginConfig>, originalId?: string) {
  if (originalId) {
    await skillConfigStore.updatePluginsConfig(originalId, config)
  } else {
    const payload: Omit<UnifiedPluginConfig, 'id' | 'source' | 'isReadOnly'> = {
      ...config,
      enabled: true,
    } as Omit<UnifiedPluginConfig, 'id' | 'source' | 'isReadOnly'>
    await skillConfigStore.createPluginsConfig(payload)
  }

  showPluginModal.value = false
  editingPlugin.value = null
}

function handleDeletePlugin(config: UnifiedPluginConfig) {
  deletingConfig.value = { type: 'plugins', config }
  showDeleteConfirm.value = true
}

function handleBackFromPlugin() {
  skillConfigStore.clearDetailState()
}

async function handleDeletePluginFromDetail(plugin: UnifiedPluginConfig) {
  deletingConfig.value = { type: 'plugins', config: plugin }
  showDeleteConfirm.value = true
}

async function handleRefresh() {
  await skillConfigStore.refreshCliConfigs()
}

async function handleOpenFile() {
  await skillConfigStore.openConfigFile()
}

const canSyncCliConfigs = computed(() => {
  const agent = skillConfigStore.selectedAgent
  if (!agent || agent.type !== 'cli') {
    return false
  }

  if (agent.provider !== 'claude' && agent.provider !== 'codex' && agent.provider !== 'opencode') {
    return false
  }

  return agentStore.agents.some(
    item =>
      item.id !== agent.id
      && item.type === 'cli'
      && !!item.cliPath
      && item.provider
      && item.provider !== agent.provider
      && (item.provider === 'claude' || item.provider === 'codex' || item.provider === 'opencode')
  )
})

function openSyncModal(type: SyncConfigType) {
  syncType.value = type
  showSyncModal.value = true
}

function handleSyncCompleted(payload: { targetAgentId: string; result: CliSyncResult }) {
  if (payload.result.successCount === 0) {
    return
  }

  if (skillConfigStore.selectedAgent?.id === payload.targetAgentId) {
    void skillConfigStore.refreshCliConfigs()
  }
}
</script>

<template>
  <div class="skill-config-page">
    <!-- 智能体选择器 -->
    <AgentSelector
      :model-value="skillConfigStore.selectedAgent"
      @update:model-value="handleSelectAgent"
    />

    <!-- Skill 详情视图 -->
    <SkillDetailView
      v-if="skillConfigStore.selectedSkill && activeTab === 'skills'"
      :skill="skillConfigStore.selectedSkill"
      @back="handleBackFromSkill"
      @delete="handleDeleteSkillFromDetail"
    />

    <SkillCreateView
      v-else-if="showSkillBuilder && activeTab === 'skills'"
      :agent="skillConfigStore.selectedAgent"
      :cli-config-paths="skillConfigStore.cliConfigPaths"
      :is-saving="isCreatingSkill"
      @back="handleBackFromSkillBuilder"
      @save="handleCreateVisualSkill"
    />

    <!-- Plugin 详情视图 -->
    <PluginDetailView
      v-else-if="skillConfigStore.selectedPlugin && activeTab === 'plugins' && showPluginsTab"
      :plugin="skillConfigStore.selectedPlugin"
      @back="handleBackFromPlugin"
      @delete="handleDeletePluginFromDetail"
    />

    <template v-else>
      <div class="skill-config-page__tabs">
        <button
          class="skill-config-page__tab"
          :class="{ 'skill-config-page__tab--active': activeTab === 'mcp', 'skill-config-page__tab--mcp': activeTab === 'mcp' }"
          @click="activeTab = 'mcp'"
        >
          <EaIcon name="lucide:server" />
          {{ t('settings.integration.tabs.mcp') }}
        </button>
        <button
          class="skill-config-page__tab"
          :class="{ 'skill-config-page__tab--active': activeTab === 'skills', 'skill-config-page__tab--skills': activeTab === 'skills' }"
          @click="activeTab = 'skills'"
        >
          <EaIcon name="lucide:book-open" />
          {{ t('settings.integration.tabs.skills') }}
        </button>
        <button
          v-if="showPluginsTab"
          class="skill-config-page__tab"
          :class="{ 'skill-config-page__tab--active': activeTab === 'plugins', 'skill-config-page__tab--plugins': activeTab === 'plugins' }"
          @click="activeTab = 'plugins'"
        >
          <EaIcon name="lucide:puzzle" />
          {{ t('settings.integration.tabs.plugins') }}
        </button>
      </div>

      <div
        ref="contentRef"
        class="skill-config-page__content"
      >
        <McpConfigTab
          v-if="activeTab === 'mcp'"
          :configs="skillConfigStore.mcpConfigs"
          :is-read-only="skillConfigStore.isReadOnly"
          :is-loading="skillConfigStore.isLoading"
          :can-sync="canSyncCliConfigs"
          @refresh="handleRefresh"
          @sync="openSyncModal('mcp')"
          @open-file="handleOpenFile"
          @save="handleSaveMcp"
          @delete="handleDeleteMcp"
        />
        <SkillsConfigTab
          v-else-if="activeTab === 'skills'"
          :configs="skillConfigStore.skillsConfigs"
          :is-read-only="skillConfigStore.isReadOnly"
          :is-loading="skillConfigStore.isLoading"
          :can-sync="canSyncCliConfigs"
          @add="handleAddSkill"
          @sync="openSyncModal('skills')"
          @detail="handleViewSkillDetail"
          @edit="handleEditSkill"
          @delete="handleDeleteSkill"
        />
        <PluginsConfigTab
          v-else-if="activeTab === 'plugins' && showPluginsTab"
          :configs="skillConfigStore.pluginsConfigs"
          :is-read-only="skillConfigStore.isReadOnly"
          :is-loading="skillConfigStore.isLoading"
          @add="handleAddPlugin"
          @detail="handleViewPluginDetail"
          @edit="handleEditPlugin"
          @delete="handleDeletePlugin"
        />
      </div>
    </template>

    <CliConfigSyncModal
      :visible="showSyncModal"
      :sync-type="syncType"
      :agents="agentStore.agents"
      :selected-agent="skillConfigStore.selectedAgent"
      @close="showSyncModal = false"
      @completed="handleSyncCompleted"
    />

    <AddModePickerModal
      v-model:visible="showSkillAddModeModal"
      :title="t('settings.sdkConfig.addMode.skillTitle')"
      :description="t('settings.sdkConfig.addMode.skillDescription')"
      :options="addModeOptions.skills"
      @select="handleSelectSkillAddMode"
    />

    <AddModePickerModal
      v-model:visible="showPluginAddModeModal"
      :title="t('settings.sdkConfig.addMode.pluginTitle')"
      :description="t('settings.sdkConfig.addMode.pluginDescription')"
      :options="addModeOptions.plugins"
      @select="handleSelectPluginAddMode"
    />

    <SkillEditModal
      v-model:visible="showSkillModal"
      :config="editingSkill"
      @save="handleSaveSkill"
    />

    <PluginEditModal
      v-model:visible="showPluginModal"
      :config="editingPlugin"
      @save="handleSavePlugin"
    />

    <SkillGitInstallModal
      v-if="showSkillGitInstallModal"
      :default-agent-id="currentCliAgentId"
      @close="handleCloseSkillGitInstall"
      @complete="handleSkillGitInstallComplete"
    />

    <PluginGitInstallModal
      v-if="showPluginGitInstallModal"
      :default-agent-id="currentCliAgentId"
      @close="handleClosePluginGitInstall"
      @complete="handlePluginGitInstallComplete"
    />

    <div
      v-if="showDeleteConfirm"
      class="delete-confirm-overlay"
      @pointerdown.capture="handleDeleteOverlayPointerDown"
      @click.self="handleDeleteOverlayClick"
    >
      <div class="delete-confirm">
        <div class="delete-confirm__header">
          <EaIcon
            name="lucide:alert-triangle"
            class="delete-confirm__icon"
          />
          <h3>{{ t('common.confirmDelete') }}</h3>
        </div>
        <p class="delete-confirm__message">
          {{ t('settings.sdkConfig.confirmDeleteMessage') }}
        </p>
        <div class="delete-confirm__actions">
          <EaButton
            variant="ghost"
            @click="showDeleteConfirm = false"
          >
            {{ t('common.cancel') }}
          </EaButton>
          <EaButton
            variant="danger"
            @click="confirmDelete"
          >
            {{ t('common.delete') }}
          </EaButton>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.skill-config-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: var(--spacing-4);
  overflow: hidden;
}

.skill-config-page__tabs {
  display: flex;
  gap: var(--spacing-1);
  padding: var(--spacing-1);
  background: var(--color-background-secondary);
  border-radius: var(--radius-lg);
  margin-bottom: var(--spacing-4);
}

.skill-config-page__tab {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-4);
  border: none;
  background: transparent;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.skill-config-page__tab:hover {
  color: var(--color-text);
}

.skill-config-page__tab--active {
  background: var(--color-surface);
  box-shadow: var(--shadow-sm);
}

.skill-config-page__tab--mcp {
  color: #8b5cf6;
}

.skill-config-page__tab--mcp.skill-config-page__tab--active {
  background: rgba(139, 92, 246, 0.1);
  color: #8b5cf6;
}

.skill-config-page__tab--skills {
  color: #10b981;
}

.skill-config-page__tab--skills.skill-config-page__tab--active {
  background: rgba(16, 185, 129, 0.1);
  color: #10b981;
}

.skill-config-page__tab--plugins {
  color: #f59e0b;
}

.skill-config-page__tab--plugins.skill-config-page__tab--active {
  background: rgba(245, 158, 11, 0.1);
  color: #f59e0b;
}

.skill-config-page__tab svg {
  width: 16px;
  height: 16px;
}

.skill-config-page__content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.delete-confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.delete-confirm {
  background: var(--color-surface);
  border-radius: var(--radius-xl);
  padding: var(--spacing-6);
  width: 400px;
  max-width: 90vw;
}

.delete-confirm__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  margin-bottom: var(--spacing-4);
}

.delete-confirm__icon {
  width: 24px;
  height: 24px;
  color: var(--color-danger);
}

.delete-confirm__header h3 {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.delete-confirm__message {
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-6);
}

.delete-confirm__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-2);
}
</style>
