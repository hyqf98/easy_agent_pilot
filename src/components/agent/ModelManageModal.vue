<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import {
  useAgentConfigStore,
  type AgentModelConfig
} from '@/stores/agentConfig'
import { useAgentStore } from '@/stores/agent'
import { EaButton, EaIcon } from '@/components/common'
import ModelEditModal from './ModelEditModal.vue'

const agentConfigStore = useAgentConfigStore()
const agentStore = useAgentStore()

const props = defineProps<{
  agentId: string
}>()

const emit = defineEmits<{
  close: []
}>()

const models = computed(() => agentConfigStore.getModelsConfigs(props.agentId))
const isLoading = ref(false)
const isInitializing = ref(false)
const isSyncing = ref(false)
const syncResult = ref<{ success: boolean; added: number; removed: number; message?: string } | null>(null)

const currentAgent = computed(() => {
  return agentStore.agents.find(a => a.id === props.agentId)
})

const isOpenCodeAgent = computed(() => {
  return currentAgent.value?.provider === 'opencode'
})

interface ProviderGroup {
  provider: string
  models: AgentModelConfig[]
}

const providerGroups = computed<ProviderGroup[]>(() => {
  if (!isOpenCodeAgent.value) return []

  const defaultModel = models.value.find(m => m.modelId === '')
  const syncedModels = models.value.filter(m => m.modelId !== '')

  const groupMap = new Map<string, AgentModelConfig[]>()
  for (const model of syncedModels) {
    const slashIdx = model.modelId.indexOf('/')
    const provider = slashIdx > 0 ? model.modelId.substring(0, slashIdx) : 'other'
    if (!groupMap.has(provider)) {
      groupMap.set(provider, [])
    }
    groupMap.get(provider)!.push(model)
  }

  const groups: ProviderGroup[] = []
  for (const [provider, providerModels] of groupMap) {
    groups.push({ provider, models: providerModels })
  }

  if (defaultModel) {
    groups.unshift({ provider: '', models: [defaultModel] })
  }

  return groups
})

const formatProviderName = (provider: string): string => {
  if (!provider) return ''
  return provider
    .split(/[-.]/)
    .filter(w => w !== 'plan')
    .map(w => {
      const upper = ['ai', 'api', 'sdk', 'llm', 'cpu', 'gpu', 'db', 'io', 'url', 'gpt']
      if (upper.includes(w.toLowerCase())) return w.toUpperCase()
      if (w === 'opencode') return 'OpenCode'
      return w.charAt(0).toUpperCase() + w.slice(1)
    })
    .join(' ')
}

const displayModelName = (model: AgentModelConfig): string => {
  if (!model.modelId || !model.modelId.includes('/')) return model.displayName
  const slashIdx = model.modelId.indexOf('/')
  const provider = model.modelId.substring(0, slashIdx)
  if (model.displayName.startsWith(provider + '/')) {
    return model.displayName.substring(slashIdx + 1)
  }
  return model.displayName
}

const syncButtonLabel = computed(() => {
  if (isSyncing.value) return '正在同步...'
  return isOpenCodeAgent.value ? '从 CLI 同步模型' : '同步最新模型'
})

const showEditModal = ref(false)
const editingModel = ref<AgentModelConfig | null>(null)

const initBuiltinModelsIfNeeded = async () => {
  const provider = currentAgent.value?.provider || 'claude'

  isInitializing.value = true
  try {
    await agentConfigStore.initBuiltinModels(props.agentId, provider)
  } catch (error) {
    console.error('Failed to initialize builtin models:', error)
  } finally {
    isInitializing.value = false
  }
}

onMounted(async () => {
  await agentConfigStore.loadModelsConfigs(props.agentId)
  await initBuiltinModelsIfNeeded()
})

watch(() => props.agentId, async (newAgentId) => {
  if (newAgentId) {
    await agentConfigStore.loadModelsConfigs(newAgentId)
    await initBuiltinModelsIfNeeded()
  }
})

const handleAdd = () => {
  editingModel.value = null
  showEditModal.value = true
}

const handleEdit = (model: AgentModelConfig) => {
  editingModel.value = model
  showEditModal.value = true
}

const handleDelete = async (model: AgentModelConfig) => {
  try {
    await agentConfigStore.deleteModelConfig(model.id, props.agentId)
  } catch (error) {
    console.error('Failed to delete model:', error)
  }
}

const handleSetDefault = async (model: AgentModelConfig) => {
  try {
    for (const m of models.value) {
      if (m.id !== model.id && m.isDefault) {
        await agentConfigStore.updateModelConfig(m.id, props.agentId, { isDefault: false })
      }
    }
    await agentConfigStore.updateModelConfig(model.id, props.agentId, { isDefault: true })
  } catch (error) {
    console.error('Failed to set default model:', error)
  }
}

const handleSyncRemoteModels = async () => {
  const beforeCount = models.value.filter(m => m.modelId !== '').length

  isSyncing.value = true
  syncResult.value = null
  try {
    if (isOpenCodeAgent.value) {
      await agentConfigStore.syncOpencodeModels(props.agentId)
    } else {
      const provider = currentAgent.value?.provider || 'claude'
      await agentConfigStore.syncRemoteModels(props.agentId, provider)
    }

    const afterModels = agentConfigStore.getModelsConfigs(props.agentId)
    const afterCount = afterModels.filter(m => m.modelId !== '').length

    syncResult.value = {
      success: true,
      added: Math.max(0, afterCount - beforeCount),
      removed: Math.max(0, beforeCount - afterCount)
    }
  } catch (error: unknown) {
    console.error('Failed to sync models:', error)
    const message = error instanceof Error ? error.message : (typeof error === 'string' ? error : String(error))
    syncResult.value = { success: false, added: 0, removed: 0, message }
  } finally {
    isSyncing.value = false
  }

  if (syncResult.value) {
    setTimeout(() => {
      syncResult.value = null
    }, 8000)
  }
}

const handleEditComplete = async () => {
  showEditModal.value = false
  editingModel.value = null
  await agentConfigStore.loadModelsConfigs(props.agentId)
}

const handleClose = () => {
  emit('close')
}
</script>

<template>
  <div class="model-manage-modal">
    <div class="modal-header">
      <h3 class="modal-title">
        模型管理
      </h3>
      <button
        class="modal-close"
        @click="handleClose"
      >
        <EaIcon
          name="x"
          :size="16"
        />
      </button>
    </div>

    <div class="modal-body">
      <!-- 操作栏 -->
      <div class="model-actions">
        <EaButton
          size="small"
          :loading="isSyncing"
          @click="handleSyncRemoteModels"
        >
          <EaIcon
            name="refresh-cw"
            :size="14"
          />
          {{ syncButtonLabel }}
        </EaButton>
        <EaButton
          size="small"
          @click="handleAdd"
        >
          <EaIcon
            name="plus"
            :size="14"
          />
          添加模型
        </EaButton>
      </div>

      <!-- 同步结果提示 -->
      <div
        v-if="syncResult"
        class="sync-result"
        :class="syncResult.success ? 'sync-result--success' : 'sync-result--error'"
      >
        <EaIcon
          :name="syncResult.success ? 'check-circle' : 'x-circle'"
          :size="16"
        />
        <span>
          {{ syncResult.success
            ? `同步成功：新增 ${syncResult.added} 个，移除 ${syncResult.removed} 个模型`
            : (syncResult.message || '同步失败，请检查网络连接后重试') }}
        </span>
      </div>

      <!-- 模型列表 -->
      <div class="model-list">
        <div
          v-if="isLoading || isInitializing"
          class="loading-state"
        >
          <EaIcon
            name="loader"
            :size="24"
            class="spin"
          />
          {{ isInitializing ? '正在初始化内置模型...' : '加载中...' }}
        </div>

        <div
          v-else-if="models.length === 0"
          class="empty-state"
        >
          <EaIcon
            name="inbox"
            :size="48"
          />
          <p>暂无模型配置</p>
          <p class="hint">
            点击"添加模型"开始配置
          </p>
        </div>

        <div
          v-else
          class="model-items"
        >
          <!-- OpenCode: 分组展示 -->
          <template v-if="isOpenCodeAgent && providerGroups.length > 0">
            <div
              v-for="group in providerGroups"
              :key="group.provider || '__default__'"
              class="provider-group"
            >
              <div
                v-if="group.provider"
                class="provider-header"
              >
                <span class="provider-name">{{ formatProviderName(group.provider) }}</span>
                <span class="provider-count">{{ group.models.length }}</span>
              </div>
              <div
                v-for="model in group.models"
                :key="model.id"
                class="model-item"
                :class="{ 'is-default': model.isDefault, 'is-disabled': !model.enabled }"
              >
                <div class="model-info">
                  <div class="model-name">
                    <span class="name">{{ displayModelName(model) }}</span>
                    <span
                      v-if="model.isDefault"
                      class="badge default"
                    >默认</span>
                    <span
                      v-if="!model.enabled"
                      class="badge disabled"
                    >禁用</span>
                  </div>
                  <div class="model-id">
                    {{ model.modelId || '使用默认模型' }}
                  </div>
                </div>

                <div class="model-actions-row">
                  <button
                    v-if="!model.isDefault"
                    class="action-btn"
                    title="设为默认"
                    @click="handleSetDefault(model)"
                  >
                    <EaIcon
                      name="star"
                      :size="14"
                    />
                  </button>
                  <button
                    class="action-btn"
                    title="编辑"
                    @click="handleEdit(model)"
                  >
                    <EaIcon
                      name="pencil"
                      :size="14"
                    />
                  </button>
                  <button
                    v-if="!model.isBuiltin"
                    class="action-btn danger"
                    title="删除"
                    @click="handleDelete(model)"
                  >
                    <EaIcon
                      name="trash-2"
                      :size="14"
                    />
                  </button>
                </div>
              </div>
            </div>
          </template>

          <!-- 非 OpenCode: 平铺展示 -->
          <template v-else>
            <div
              v-for="model in models"
              :key="model.id"
              class="model-item"
              :class="{ 'is-default': model.isDefault, 'is-disabled': !model.enabled }"
            >
              <div class="model-info">
                <div class="model-name">
                  <span class="name">{{ model.displayName }}</span>
                  <span
                    v-if="model.isBuiltin"
                    class="badge builtin"
                  >内置</span>
                  <span
                    v-if="model.isDefault"
                    class="badge default"
                  >默认</span>
                  <span
                    v-if="!model.enabled"
                    class="badge disabled"
                  >禁用</span>
                </div>
                <div class="model-id">
                  {{ model.modelId || '使用默认模型' }}
                </div>
              </div>

              <div class="model-actions-row">
                <button
                  v-if="!model.isDefault"
                  class="action-btn"
                  title="设为默认"
                  @click="handleSetDefault(model)"
                >
                  <EaIcon
                    name="star"
                    :size="14"
                  />
                </button>
                <button
                  class="action-btn"
                  title="编辑"
                  @click="handleEdit(model)"
                >
                  <EaIcon
                    name="pencil"
                    :size="14"
                  />
                </button>
                <button
                  v-if="!model.isBuiltin"
                  class="action-btn danger"
                  title="删除"
                  @click="handleDelete(model)"
                >
                  <EaIcon
                    name="trash-2"
                    :size="14"
                  />
                </button>
              </div>
            </div>
          </template>
        </div>
      </div>
    </div>

    <!-- 编辑弹窗 -->
    <ModelEditModal
      v-if="showEditModal"
      :agent-id="agentId"
      :provider="currentAgent?.provider || 'claude'"
      :model="editingModel"
      @close="handleEditComplete"
    />
  </div>
</template>

<style scoped>
.model-manage-modal {
  display: flex;
  flex-direction: column;
  max-height: 85vh;
  background-color: var(--color-surface);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4) var(--spacing-5);
  border-bottom: 1px solid var(--color-border);
}

.modal-title {
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--color-text-primary);
}

.modal-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  background: none;
  cursor: pointer;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.modal-close:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.modal-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: var(--spacing-4) var(--spacing-5);
}

.model-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-2);
  margin-bottom: var(--spacing-4);
}

.sync-result {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  margin-bottom: var(--spacing-3);
  animation: slide-in 0.3s ease-out;
}

.sync-result--success {
  background-color: rgba(34, 197, 94, 0.1);
  border: 1px solid var(--color-success, #22c55e);
  color: var(--color-success, #22c55e);
}

.sync-result--error {
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-error, #ef4444);
  color: var(--color-error, #ef4444);
}

.model-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  min-height: 0;
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-8);
  color: var(--color-text-tertiary);
}

.empty-state .hint {
  font-size: var(--font-size-sm);
  margin-top: var(--spacing-2);
}

.model-items {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.provider-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.provider-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-1) var(--spacing-2);
  margin-top: var(--spacing-2);
}

.provider-group:first-child .provider-header {
  margin-top: 0;
}

.provider-name {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.provider-count {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  background-color: var(--color-surface-hover);
  padding: 1px 8px;
  border-radius: 999px;
}

.model-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3) var(--spacing-4);
  background-color: var(--color-bg-secondary);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  transition: all var(--transition-fast);
}

.model-item:hover {
  background-color: var(--color-surface-hover);
}

.model-item.is-default {
  border-color: var(--color-primary);
  background-color: var(--color-primary-light);
}

.model-item.is-disabled {
  opacity: 0.6;
}

.model-info {
  flex: 1;
  min-width: 0;
}

.model-name {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  margin-bottom: var(--spacing-1);
}

.model-name .name {
  font-weight: 500;
  color: var(--color-text-primary);
}

.model-name .badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  font-weight: 500;
}

.model-name .badge.builtin {
  background-color: var(--color-info-light);
  color: var(--color-info);
}

.model-name .badge.default {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.model-name .badge.disabled {
  background-color: var(--color-surface-hover);
  color: var(--color-text-tertiary);
}

.model-id {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  font-family: monospace;
}

.model-actions-row {
  display: flex;
  gap: var(--spacing-1);
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: none;
  cursor: pointer;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.action-btn:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.action-btn.danger:hover {
  color: var(--color-danger);
}

.spin {
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
