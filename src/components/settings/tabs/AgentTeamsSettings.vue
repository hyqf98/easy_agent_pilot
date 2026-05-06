<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAgentStore } from '@/stores/agent'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { useAgentTeamsStore, type AgentExpert, type AgentExpertCategory } from '@/stores/agentTeams'
import { useNotificationStore } from '@/stores/notification'
import { inferAgentProvider, type CliTool } from '@/stores/agent'
import DetectedCliToolsBanner from '@/components/settings/agent-settings/DetectedCliToolsBanner.vue'

interface ExpertFormState {
  id?: string
  name: string
  description: string
  prompt: string
  runtimeAgentId: string
  defaultModelId: string
  category: AgentExpertCategory
  tags: string
  recommendedScenes: string
  isEnabled: boolean
  sortOrder: number
}

const agentStore = useAgentStore()
const agentConfigStore = useAgentConfigStore()
const teamsStore = useAgentTeamsStore()
const notificationStore = useNotificationStore()
const { t } = useI18n()

const searchQuery = ref('')
const isCreating = ref(false)
const isSaving = ref(false)
const isBulkSwitching = ref(false)
const addingToolName = ref<string | null>(null)
const bulkRuntimeAgentId = ref('')

const emptyForm = (): ExpertFormState => ({
  name: '',
  description: '',
  prompt: '',
  runtimeAgentId: '',
  defaultModelId: '',
  category: 'custom',
  tags: '',
  recommendedScenes: '',
  isEnabled: true,
  sortOrder: 100
})

const form = reactive<ExpertFormState>(emptyForm())

const categoryOptions = computed<Array<{ label: string; value: AgentExpertCategory }>>(() => [
  { label: t('settings.agentTeams.categories.general'), value: 'general' },
  { label: t('settings.agentTeams.categories.planner'), value: 'planner' },
  { label: t('settings.agentTeams.categories.architect'), value: 'architect' },
  { label: t('settings.agentTeams.categories.developer'), value: 'developer' },
  { label: t('settings.agentTeams.categories.tester'), value: 'tester' },
  { label: t('settings.agentTeams.categories.writer'), value: 'writer' },
  { label: t('settings.agentTeams.categories.designer'), value: 'designer' },
  { label: t('settings.agentTeams.categories.reviewer'), value: 'reviewer' },
  { label: t('settings.agentTeams.categories.ops'), value: 'ops' },
  { label: t('settings.agentTeams.categories.custom'), value: 'custom' }
])

const categoryLabelMap = computed(
  () => new Map(categoryOptions.value.map(option => [option.value, option.label]))
)

const filteredExperts = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  if (!query) {
    return teamsStore.experts
  }

  return teamsStore.experts.filter(expert => {
    const haystack = [
      expert.name,
      expert.description,
      expert.category,
      expert.tags.join(' '),
      expert.recommendedScenes.join(' ')
    ].join(' ').toLowerCase()
    return haystack.includes(query)
  })
})

const runtimeAgentOptions = computed(() =>
  agentStore.agents.filter(agent => agent.type === 'cli')
)

const runtimeAgentIds = computed(() =>
  runtimeAgentOptions.value.map(agent => agent.id)
)

const runtimeModelOptions = computed(() => {
  if (!form.runtimeAgentId) return []
  return agentConfigStore.getModelsConfigs(form.runtimeAgentId).filter(model => model.enabled)
})

const bulkRuntimeModelOptions = computed(() => {
  if (!bulkRuntimeAgentId.value) return []
  return agentConfigStore.getModelsConfigs(bulkRuntimeAgentId.value).filter(model => model.enabled)
})

const bulkSwitchPreviewCount = computed(() => {
  if (!bulkRuntimeAgentId.value) return 0
  return teamsStore.experts.filter(expert => expert.runtimeAgentId !== bulkRuntimeAgentId.value).length
})

const selectedExpert = computed(() =>
  teamsStore.getExpertById(teamsStore.selectedExpertId)
)

function applyExpertToForm(expert: AgentExpert | null) {
  const next = expert
    ? {
        id: expert.id,
        name: expert.name,
        description: expert.description || '',
        prompt: expert.prompt,
        runtimeAgentId: expert.runtimeAgentId || '',
        defaultModelId: expert.defaultModelId || '',
        category: expert.category,
        tags: expert.tags.join(', '),
        recommendedScenes: expert.recommendedScenes.join(', '),
        isEnabled: expert.isEnabled,
        sortOrder: expert.sortOrder
      }
    : emptyForm()

  Object.assign(form, next)
}

function getCategoryLabel(category: AgentExpertCategory): string {
  return categoryLabelMap.value.get(category) || category
}

function parseCsv(value: string): string[] {
  return value
    .split(',')
    .map(item => item.trim())
    .filter(Boolean)
}

function selectExpert(expertId: string) {
  isCreating.value = false
  teamsStore.setSelectedExpert(expertId)
}

function handleCreate() {
  isCreating.value = true
  teamsStore.setSelectedExpert(null)
  applyExpertToForm(null)
  form.runtimeAgentId = runtimeAgentOptions.value[0]?.id || ''
}

function handleCopy(expert: AgentExpert) {
  isCreating.value = true
  teamsStore.setSelectedExpert(null)
  applyExpertToForm(expert)
  form.id = undefined
  form.name = `${expert.name} ${t('settings.agentTeams.duplicateSuffix')}`
  form.category = 'custom'
}

async function ensureModels(agentId?: string) {
  if (!agentId) return
  const agent = agentStore.agents.find(item => item.id === agentId)
  const provider = inferAgentProvider(agent)
  await agentConfigStore.ensureModelsConfigs(agentId, provider)
}

function syncRuntimeAgentSelection() {
  if (runtimeAgentIds.value.length === 0) {
    form.runtimeAgentId = ''
    bulkRuntimeAgentId.value = ''
    form.defaultModelId = ''
    return
  }

  if (!runtimeAgentIds.value.includes(form.runtimeAgentId)) {
    form.runtimeAgentId = runtimeAgentIds.value[0]
  }

  if (!runtimeAgentIds.value.includes(bulkRuntimeAgentId.value)) {
    bulkRuntimeAgentId.value = form.runtimeAgentId || runtimeAgentIds.value[0]
  }
}

function resolveBulkDefaultModelId(expert: AgentExpert): string | undefined {
  const availableModels = bulkRuntimeModelOptions.value
  if (availableModels.length === 0) {
    return undefined
  }

  const preservedModel = expert.defaultModelId
    ? availableModels.find(model => model.modelId === expert.defaultModelId)
    : null
  const fallbackModel = availableModels.find(model => model.isDefault) || availableModels[0]
  return preservedModel?.modelId ?? fallbackModel?.modelId ?? undefined
}

async function handleSave() {
  if (!form.name.trim() || !form.prompt.trim()) {
    return
  }

  isSaving.value = true
  try {
    const payload = {
      name: form.name.trim(),
      description: form.description.trim() || undefined,
      prompt: form.prompt.trim(),
      runtimeAgentId: form.runtimeAgentId || undefined,
      defaultModelId: form.defaultModelId || undefined,
      category: form.category,
      tags: parseCsv(form.tags),
      recommendedScenes: parseCsv(form.recommendedScenes),
      isEnabled: form.isEnabled,
      sortOrder: Number(form.sortOrder) || 100
    }

    if (isCreating.value || !form.id) {
      const created = await teamsStore.createExpert(payload)
      teamsStore.setSelectedExpert(created.id)
      isCreating.value = false
    } else {
      const updated = await teamsStore.updateExpert(form.id, payload)
      teamsStore.setSelectedExpert(updated.id)
    }
    notificationStore.success(t('settings.agentTeams.saveSuccess'))
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err)
    notificationStore.error(t('settings.agentTeams.saveFailed'), msg)
  } finally {
    isSaving.value = false
  }
}

async function handleQuickAdd(tool: CliTool) {
  addingToolName.value = tool.name
  try {
    await agentStore.addDetectedTool(tool)
    notificationStore.success(t('settings.agentList.addSuccess'))
    syncRuntimeAgentSelection()
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err)
    notificationStore.error(t('settings.agentList.addError'), msg)
  } finally {
    addingToolName.value = null
  }
}

async function handleBulkSwitchRuntime() {
  if (!bulkRuntimeAgentId.value || bulkSwitchPreviewCount.value === 0) {
    return
  }

  isBulkSwitching.value = true
  try {
    await ensureModels(bulkRuntimeAgentId.value)
    const targetAgent = runtimeAgentOptions.value.find(agent => agent.id === bulkRuntimeAgentId.value)
    const expertsToUpdate = teamsStore.experts.filter(expert => expert.runtimeAgentId !== bulkRuntimeAgentId.value)

    for (const expert of expertsToUpdate) {
      await teamsStore.updateExpert(expert.id, {
        runtimeAgentId: bulkRuntimeAgentId.value,
        defaultModelId: resolveBulkDefaultModelId(expert)
      })
    }

    if (selectedExpert.value) {
      applyExpertToForm(teamsStore.getExpertById(selectedExpert.value.id))
    }

    notificationStore.success(
      t('settings.agentTeams.bulkSwitch.success', {
        count: expertsToUpdate.length,
        agent: targetAgent?.name || t('settings.agentTeams.bulkSwitch.unknownAgent')
      })
    )
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err)
    notificationStore.error(t('settings.agentTeams.bulkSwitch.failed'), msg)
  } finally {
    isBulkSwitching.value = false
  }
}

async function handleDelete(expert: AgentExpert) {
  if (expert.isBuiltin) {
    return
  }
  const refs = await teamsStore.countReferences(expert.id)
  const blocked = refs.plans > 0 || refs.tasks > 0 || refs.sessions > 0
  if (blocked) {
    window.alert(t('settings.agentTeams.deleteBlocked', { ...refs }))
    return
  }
  if (!window.confirm(t('settings.agentTeams.deleteConfirm', { name: expert.name }))) {
    return
  }
  await teamsStore.deleteExpert(expert.id)
  handleCreate()
}

watch(
  selectedExpert,
  (expert) => {
    applyExpertToForm(expert)
    syncRuntimeAgentSelection()
  },
  { immediate: true }
)

watch(runtimeAgentIds, () => {
  syncRuntimeAgentSelection()
})

watch(
  () => form.runtimeAgentId,
  async (agentId) => {
    await ensureModels(agentId)
    if (agentId && !runtimeModelOptions.value.some(model => model.modelId === form.defaultModelId)) {
      form.defaultModelId = runtimeModelOptions.value.find(model => model.isDefault)?.modelId
        || runtimeModelOptions.value[0]?.modelId
        || ''
    }
  }
)

onMounted(async () => {
  await agentStore.loadAgents()
  void agentStore.scanCliTools()
  await teamsStore.loadExperts(true)

  const initialExpert = teamsStore.selectedExpert || teamsStore.builtinGeneralExpert || teamsStore.experts[0] || null
  if (initialExpert) {
    teamsStore.setSelectedExpert(initialExpert.id)
  } else {
    handleCreate()
  }

  syncRuntimeAgentSelection()
})
</script>

<template>
  <div class="agent-teams-page">
    <aside class="agent-teams-sidebar">
      <div class="agent-teams-sidebar__header">
        <div>
          <h3>{{ t('settings.nav.agentTeams') }}</h3>
          <p>{{ t('settings.agentTeams.subtitle') }}</p>
        </div>
        <button
          class="primary-button"
          :title="t('settings.agentTeams.createExpert')"
          :aria-label="t('settings.agentTeams.createExpert')"
          @click="handleCreate"
        >
          +
        </button>
      </div>

      <input
        v-model="searchQuery"
        class="search-input"
        type="text"
        :placeholder="t('settings.agentTeams.searchPlaceholder')"
      >

      <div class="expert-list">
        <button
          v-for="expert in filteredExperts"
          :key="expert.id"
          class="expert-list__item"
          :class="{ 'expert-list__item--active': expert.id === teamsStore.selectedExpertId && !isCreating }"
          @click="selectExpert(expert.id)"
        >
          <div class="expert-list__title-row">
            <span class="expert-list__title">{{ expert.name }}</span>
            <span
              v-if="expert.isBuiltin"
              class="expert-badge"
            >{{ t('settings.agentTeams.builtinBadge') }}</span>
          </div>
          <div class="expert-list__meta">
            <span>{{ getCategoryLabel(expert.category) }}</span>
            <span>{{ expert.isEnabled ? t('settings.agentTeams.enabled') : t('settings.agentTeams.disabled') }}</span>
          </div>
          <p class="expert-list__description">
            {{ expert.description || t('settings.agentTeams.emptyDescription') }}
          </p>
        </button>
      </div>
    </aside>

    <section class="agent-teams-editor">
      <div class="agent-teams-editor__header">
        <div>
          <h3>{{ isCreating ? t('settings.agentTeams.createTitle') : (selectedExpert?.name || t('settings.agentTeams.editTitle')) }}</h3>
        </div>
        <div
          v-if="selectedExpert"
          class="agent-teams-editor__actions"
        >
          <button
            class="ghost-button"
            @click="handleCopy(selectedExpert)"
          >
            {{ t('settings.agentTeams.copy') }}
          </button>
          <button
            class="danger-button"
            :disabled="selectedExpert.isBuiltin"
            @click="handleDelete(selectedExpert)"
          >
            {{ t('settings.agentTeams.delete') }}
          </button>
        </div>
      </div>

      <DetectedCliToolsBanner
        class="agent-teams-editor__banner"
        :tools="agentStore.availableToolsToAdd"
        :adding-tool-name="addingToolName"
        @quick-add="handleQuickAdd"
      />

      <section class="bulk-runtime-card">
        <div class="bulk-runtime-card__content">
          <div>
            <h4>{{ t('settings.agentTeams.bulkSwitch.title') }}</h4>
            <p>{{ t('settings.agentTeams.bulkSwitch.description') }}</p>
          </div>
          <div class="bulk-runtime-card__controls">
            <label class="bulk-runtime-card__select editor-grid__select">
              <span>{{ t('settings.agentTeams.bulkSwitch.targetLabel') }}</span>
              <select
                v-model="bulkRuntimeAgentId"
                class="select-input"
                :disabled="runtimeAgentOptions.length === 0 || isBulkSwitching"
              >
                <option
                  v-for="agent in runtimeAgentOptions"
                  :key="agent.id"
                  :value="agent.id"
                >
                  {{ agent.name }} ({{ agent.provider || agent.type }})
                </option>
              </select>
            </label>
            <button
              class="primary-button"
              :disabled="runtimeAgentOptions.length === 0 || bulkSwitchPreviewCount === 0 || isBulkSwitching"
              @click="handleBulkSwitchRuntime"
            >
              {{ isBulkSwitching ? t('settings.agentTeams.bulkSwitch.switching') : t('settings.agentTeams.bulkSwitch.action') }}
            </button>
          </div>
        </div>
        <p class="bulk-runtime-card__hint">
          {{ t('settings.agentTeams.bulkSwitch.hint', { count: bulkSwitchPreviewCount }) }}
        </p>
      </section>

      <div class="editor-grid">
        <label>
          {{ t('settings.agentTeams.fields.name') }}
          <input
            v-model="form.name"
            type="text"
            class="text-input"
          >
        </label>
        <label class="editor-grid__select">
          {{ t('settings.agentTeams.fields.category') }}
          <select
            v-model="form.category"
            class="select-input"
          >
            <option
              v-for="option in categoryOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </option>
          </select>
        </label>
        <label class="editor-grid__full">
          {{ t('settings.agentTeams.fields.description') }}
          <input
            v-model="form.description"
            type="text"
            class="text-input"
            :placeholder="t('settings.agentTeams.placeholders.description')"
          >
        </label>
        <label class="editor-grid__select">
          {{ t('settings.agentTeams.fields.runtimeAgent') }}
          <select
            v-model="form.runtimeAgentId"
            class="select-input"
            :disabled="runtimeAgentOptions.length === 0"
          >
            <option
              v-for="agent in runtimeAgentOptions"
              :key="agent.id"
              :value="agent.id"
            >
              {{ agent.name }} ({{ agent.provider || agent.type }})
            </option>
          </select>
        </label>
        <label class="editor-grid__select">
          {{ t('settings.agentTeams.fields.defaultModel') }}
          <select
            v-model="form.defaultModelId"
            class="select-input"
            :disabled="!form.runtimeAgentId"
          >
            <option
              v-for="model in runtimeModelOptions"
              :key="model.id"
              :value="model.modelId"
            >
              {{ model.displayName }}
            </option>
          </select>
        </label>
        <label>
          {{ t('settings.agentTeams.fields.tags') }}
          <input
            v-model="form.tags"
            type="text"
            class="text-input"
            :placeholder="t('settings.agentTeams.placeholders.tags')"
          >
        </label>
        <label>
          {{ t('settings.agentTeams.fields.recommendedScenes') }}
          <input
            v-model="form.recommendedScenes"
            type="text"
            class="text-input"
            :placeholder="t('settings.agentTeams.placeholders.recommendedScenes')"
          >
        </label>
        <label>
          {{ t('settings.agentTeams.fields.sortOrder') }}
          <input
            v-model.number="form.sortOrder"
            type="number"
            class="text-input"
          >
        </label>
        <label class="switch-label">
          <input
            v-model="form.isEnabled"
            type="checkbox"
          >
          {{ t('settings.agentTeams.fields.enabled') }}
        </label>
        <label class="editor-grid__full">
          {{ t('settings.agentTeams.fields.prompt') }}
          <textarea
            v-model="form.prompt"
            class="prompt-input"
            rows="18"
          />
        </label>
      </div>

      <div class="editor-footer">
        <button
          class="primary-button"
          :disabled="isSaving"
          @click="handleSave"
        >
          {{ isSaving ? t('settings.agentTeams.saving') : t('settings.agentTeams.save') }}
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.agent-teams-page {
  --agent-teams-scrollbar-track: var(--scrollbar-track);
  --agent-teams-scrollbar-thumb: var(--scrollbar-thumb);
  --agent-teams-scrollbar-thumb-hover: var(--scrollbar-thumb-hover);
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  gap: 16px;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  overscroll-behavior: none;
}

.agent-teams-sidebar,
.agent-teams-editor {
  display: flex;
  flex-direction: column;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 16px;
  padding: 16px;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  overscroll-behavior: none;
}

.agent-teams-sidebar__header,
.agent-teams-editor__header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
  margin-bottom: 16px;
}

.agent-teams-sidebar__header h3,
.agent-teams-editor__header h3 {
  margin: 0 0 6px;
}

.agent-teams-sidebar__header p,
.agent-teams-editor__header p {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: 13px;
}

.agent-teams-editor__banner {
  margin-bottom: 16px;
}

.bulk-runtime-card {
  border: 1px solid var(--color-border);
  border-radius: 14px;
  background: var(--color-bg-primary);
  padding: 14px 16px;
  margin-bottom: 16px;
}

.bulk-runtime-card__content {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
}

.bulk-runtime-card h4,
.bulk-runtime-card p {
  margin: 0;
}

.bulk-runtime-card h4 {
  font-size: 15px;
}

.bulk-runtime-card__content p,
.bulk-runtime-card__hint {
  color: var(--color-text-secondary);
  font-size: 13px;
}

.bulk-runtime-card__controls {
  display: flex;
  align-items: flex-end;
  gap: 12px;
}

.bulk-runtime-card__select {
  min-width: 260px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.bulk-runtime-card__hint {
  margin-top: 10px;
}

.search-input,
.text-input,
.select-input,
.prompt-input {
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-bg-primary);
  color: var(--color-text-primary);
  padding: 10px 12px;
  font: inherit;
  transition: border-color 0.2s ease, box-shadow 0.2s ease, background-color 0.2s ease;
}

.search-input:focus,
.text-input:focus,
.select-input:focus,
.prompt-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px var(--color-primary-light);
}

.search-input {
  margin-bottom: 12px;
}

.select-input {
  appearance: none;
  -webkit-appearance: none;
  min-height: 42px;
  padding-right: 40px;
  cursor: pointer;
}

.select-input:disabled {
  cursor: not-allowed;
  color: var(--color-text-secondary);
  background: var(--color-bg-secondary);
}

.expert-list {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow: auto;
  scrollbar-gutter: stable;
  overscroll-behavior: contain;
  padding-right: 4px;
}

.expert-list__item {
  text-align: left;
  border: 1px solid var(--color-border);
  background: var(--color-bg-primary);
  border-radius: 12px;
  padding: 12px;
}

.expert-list__item--active {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 1px var(--color-primary-light);
}

.expert-list__title-row,
.expert-list__meta,
.agent-teams-editor__actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.expert-list__title-row {
  justify-content: space-between;
}

.expert-list__title {
  font-weight: 600;
}

.expert-list__meta,
.expert-list__description {
  color: var(--color-text-secondary);
  font-size: 13px;
}

.expert-badge {
  font-size: 12px;
  color: var(--color-primary);
}

.editor-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
  overflow: auto;
  scrollbar-gutter: stable;
  overscroll-behavior: contain;
  padding-right: 6px;
}

.editor-grid label {
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 13px;
}

.editor-grid__select {
  position: relative;
}

.editor-grid__select::after {
  content: '';
  position: absolute;
  right: 14px;
  bottom: 16px;
  width: 8px;
  height: 8px;
  border-right: 1.5px solid var(--color-text-secondary);
  border-bottom: 1.5px solid var(--color-text-secondary);
  transform: rotate(45deg);
  pointer-events: none;
}

.editor-grid__full {
  grid-column: 1 / -1;
}

.prompt-input {
  min-height: 360px;
  resize: vertical;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  scrollbar-gutter: stable;
}

.switch-label {
  justify-content: flex-end;
  flex-direction: row !important;
  align-items: center;
  gap: 8px !important;
}

.editor-footer {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
  flex-shrink: 0;
}

.primary-button,
.ghost-button,
.danger-button {
  border-radius: 10px;
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  cursor: pointer;
}

.primary-button {
  background: var(--color-primary);
  color: white;
  border-color: var(--color-primary);
}

.agent-teams-sidebar__header .primary-button {
  min-width: 40px;
  min-height: 40px;
  padding: 0;
  font-size: 24px;
  line-height: 1;
}

.agent-teams-page,
.expert-list,
.editor-grid,
.prompt-input {
  scrollbar-width: thin;
  scrollbar-color: var(--agent-teams-scrollbar-thumb) var(--agent-teams-scrollbar-track);
}

.agent-teams-page::-webkit-scrollbar,
.expert-list::-webkit-scrollbar,
.editor-grid::-webkit-scrollbar,
.prompt-input::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.agent-teams-page::-webkit-scrollbar-track,
.expert-list::-webkit-scrollbar-track,
.editor-grid::-webkit-scrollbar-track,
.prompt-input::-webkit-scrollbar-track {
  background: var(--agent-teams-scrollbar-track);
  border-radius: 999px;
}

.agent-teams-page::-webkit-scrollbar-thumb,
.expert-list::-webkit-scrollbar-thumb,
.editor-grid::-webkit-scrollbar-thumb,
.prompt-input::-webkit-scrollbar-thumb {
  background: var(--agent-teams-scrollbar-thumb);
  border-radius: 999px;
  border: 1px solid var(--agent-teams-scrollbar-track);
  background-clip: padding-box;
  min-height: 30px;
}

.agent-teams-page::-webkit-scrollbar-thumb:hover,
.expert-list::-webkit-scrollbar-thumb:hover,
.editor-grid::-webkit-scrollbar-thumb:hover,
.prompt-input::-webkit-scrollbar-thumb:hover {
  background: var(--agent-teams-scrollbar-thumb-hover);
}

.agent-teams-page::-webkit-scrollbar-thumb:active,
.expert-list::-webkit-scrollbar-thumb:active,
.editor-grid::-webkit-scrollbar-thumb:active,
.prompt-input::-webkit-scrollbar-thumb:active {
  background: var(--scrollbar-thumb-active);
}

:global([data-theme='dark']) .agent-teams-page,
:global(.dark) .agent-teams-page {
  --agent-teams-scrollbar-track: rgba(15, 23, 42, 0.5);
  --agent-teams-scrollbar-thumb: rgba(148, 163, 184, 0.92);
  --agent-teams-scrollbar-thumb-hover: rgba(203, 213, 225, 0.98);
}

.ghost-button {
  background: transparent;
}

.danger-button {
  color: var(--color-danger, #dc2626);
}

@media (max-width: 1280px) {
  .agent-teams-page {
    grid-template-columns: 240px minmax(0, 1fr);
  }

  .bulk-runtime-card__content {
    flex-direction: column;
    align-items: stretch;
  }

  .bulk-runtime-card__controls {
    flex-direction: column;
    align-items: stretch;
  }

  .bulk-runtime-card__select {
    min-width: 0;
  }
}
</style>
