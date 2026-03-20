<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { EaButton, EaIcon, EaStateBlock } from '@/components/common'
import { useAgentStore } from '@/stores/agent'
import { useAgentConfigStore } from '@/stores/agentConfig'
import {
  useAgentStudioStore,
  type AgentResourceCatalogItem,
  type StudioAgent,
  type StudioAgentDraft,
  type StudioAgentResourceBinding
} from '@/stores/agentStudio'
import { useAgentTeamStore } from '@/stores/agentTeam'
import { inferAgentProvider } from '@/stores/agent'
import AgentTeamManager from '@/components/settings/agent-settings/AgentTeamManager.vue'

interface EditorState {
  id?: string
  name: string
  description: string
  specialization: string
  roleType: StudioAgent['roleType']
  runtimeAgentId: string
  runtimeModelId: string
  systemPrompt: string
  instructionNote: string
  readonlyResearcher: boolean
  planningCapability: boolean
  executionCapability: boolean
  enabled: boolean
  resourceBindings: StudioAgentResourceBinding[]
}

const agentStore = useAgentStore()
const agentConfigStore = useAgentConfigStore()
const agentStudioStore = useAgentStudioStore()
const agentTeamStore = useAgentTeamStore()

const selectedAgentId = ref<string | null>(null)
const isSaving = ref(false)
const editor = ref<EditorState | null>(null)

function emptyEditor(): EditorState {
  return {
    name: '',
    description: '',
    specialization: '',
    roleType: 'custom',
    runtimeAgentId: agentStore.agents[0]?.id || '',
    runtimeModelId: '',
    systemPrompt: '',
    instructionNote: '',
    readonlyResearcher: false,
    planningCapability: false,
    executionCapability: true,
    enabled: true,
    resourceBindings: []
  }
}

const selectedAgent = computed(() => agentStudioStore.getAgent(selectedAgentId.value))

const runtimeOptions = computed(() =>
  agentStore.agents.map(agent => ({
    value: agent.id,
    label: `${agent.name}${agent.provider ? ` / ${agent.provider}` : ''}${agent.type ? ` / ${agent.type.toUpperCase()}` : ''}`
  }))
)

const runtimeModels = computed(() => {
  if (!editor.value?.runtimeAgentId) return []
  return agentConfigStore
    .getModelsConfigs(editor.value.runtimeAgentId)
    .filter(model => model.enabled)
    .map(model => ({
      value: model.modelId,
      label: `${model.displayName}${model.isDefault ? ' (默认)' : ''}`
    }))
})

const roleOptions: Array<{ value: StudioAgent['roleType']; label: string; desc: string }> = [
  { value: 'orchestrator', label: '主控', desc: '适合作为规划与协调的父 Agent' },
  { value: 'researcher', label: '研究员', desc: '偏分析、检索与只读研究' },
  { value: 'executor', label: '执行者', desc: '偏实现、改动与结果交付' },
  { value: 'custom', label: '自定义', desc: '手工定义职责边界与专业方向' }
]

const resourceGroups = computed(() => [
  { key: 'mcp', title: 'MCP', items: agentStudioStore.resourceCatalog.mcpServers },
  { key: 'skill', title: 'Skills', items: agentStudioStore.resourceCatalog.skills },
  { key: 'plugin', title: 'Plugins', items: agentStudioStore.resourceCatalog.plugins }
])

const linkedTeamCount = computed(() => {
  if (!selectedAgentId.value) return 0
  return agentTeamStore.teams.filter(team =>
    team.members.some(member => member.agentId === selectedAgentId.value)
  ).length
})

function fillEditor(agent?: StudioAgent | null) {
  if (!agent) {
    editor.value = emptyEditor()
    return
  }

  editor.value = {
    id: agent.id,
    name: agent.name,
    description: agent.description || '',
    specialization: agent.specialization || '',
    roleType: agent.roleType,
    runtimeAgentId: agent.runtimeAgentId,
    runtimeModelId: agent.runtimeModelId || '',
    systemPrompt: agent.systemPrompt || '',
    instructionNote: agent.instructionNote || '',
    readonlyResearcher: agent.readonlyResearcher,
    planningCapability: agent.planningCapability,
    executionCapability: agent.executionCapability,
    enabled: agent.enabled,
    resourceBindings: [...agent.resourceBindings]
  }
}

function isBound(item: AgentResourceCatalogItem) {
  return Boolean(editor.value?.resourceBindings.some(binding =>
    binding.resourceType === item.resourceType && binding.resourceId === item.resourceId
  ))
}

function toggleBinding(item: AgentResourceCatalogItem) {
  if (!editor.value) return
  const index = editor.value.resourceBindings.findIndex(binding =>
    binding.resourceType === item.resourceType && binding.resourceId === item.resourceId
  )
  if (index >= 0) {
    editor.value.resourceBindings.splice(index, 1)
    return
  }

  editor.value.resourceBindings.push({
    id: `draft-${item.resourceType}-${item.resourceId}`,
    studioAgentId: editor.value.id || '',
    resourceType: item.resourceType,
    resourceId: item.resourceId,
    resourceName: item.name,
    enabled: item.enabled,
    sortOrder: editor.value.resourceBindings.length,
    createdAt: '',
    updatedAt: ''
  })
}

function createDraft(): StudioAgentDraft | null {
  if (!editor.value || !editor.value.name.trim() || !editor.value.runtimeAgentId) {
    return null
  }

  return {
    name: editor.value.name.trim(),
    description: editor.value.description.trim() || undefined,
    specialization: editor.value.specialization.trim() || undefined,
    roleType: editor.value.roleType,
    runtimeAgentId: editor.value.runtimeAgentId,
    runtimeModelId: editor.value.runtimeModelId || undefined,
    systemPrompt: editor.value.systemPrompt.trim() || undefined,
    instructionNote: editor.value.instructionNote.trim() || undefined,
    readonlyResearcher: editor.value.readonlyResearcher,
    planningCapability: editor.value.planningCapability,
    executionCapability: editor.value.executionCapability,
    enabled: editor.value.enabled,
    resourceBindings: editor.value.resourceBindings.map((binding, index) => ({
      resourceType: binding.resourceType,
      resourceId: binding.resourceId,
      resourceName: binding.resourceName,
      enabled: binding.enabled,
      sortOrder: index
    }))
  }
}

function handleCreate() {
  selectedAgentId.value = null
  fillEditor(null)
}

async function ensureRuntimeModels(runtimeAgentId: string) {
  const provider = inferAgentProvider(agentStore.agents.find(agent => agent.id === runtimeAgentId))
  await agentConfigStore.ensureModelsConfigs(runtimeAgentId, provider)
}

async function handleSave() {
  const draft = createDraft()
  if (!draft) return

  isSaving.value = true
  try {
    if (editor.value?.id) {
      const updated = await agentStudioStore.updateAgent(editor.value.id, draft)
      selectedAgentId.value = updated.id
      fillEditor(updated)
    } else {
      const created = await agentStudioStore.createAgent(draft)
      selectedAgentId.value = created.id
      fillEditor(created)
    }
  } finally {
    isSaving.value = false
  }
}

async function handleDelete() {
  if (!selectedAgent.value) return
  const confirmed = window.confirm(`确定删除 Agent「${selectedAgent.value.name}」吗？`)
  if (!confirmed) return
  await agentStudioStore.deleteAgent(selectedAgent.value.id)
  const next = agentStudioStore.agents[0] || null
  selectedAgentId.value = next?.id || null
  fillEditor(next)
}

watch(selectedAgent, (agent) => {
  fillEditor(agent)
}, { immediate: true })

watch(
  () => editor.value?.runtimeAgentId,
  async (runtimeAgentId) => {
    if (!runtimeAgentId || !editor.value) return
    await ensureRuntimeModels(runtimeAgentId)
    const models = agentConfigStore.getModelsConfigs(runtimeAgentId).filter(model => model.enabled)
    if (!editor.value.runtimeModelId || !models.some(model => model.modelId === editor.value?.runtimeModelId)) {
      editor.value.runtimeModelId = models.find(model => model.isDefault)?.modelId || models[0]?.modelId || ''
    }
  },
  { immediate: true }
)

onMounted(async () => {
  await Promise.all([
    agentStore.agents.length === 0 ? agentStore.loadAgents() : Promise.resolve(),
    agentStudioStore.loadAgents(),
    agentStudioStore.loadResourceCatalog(),
    agentTeamStore.loadTeams().catch(() => undefined)
  ])

  const first = agentStudioStore.agents[0] || null
  selectedAgentId.value = first?.id || null
  fillEditor(first)
})
</script>

<template>
  <div class="agent-studio">
    <div class="agent-studio__header">
      <div>
        <h2 class="agent-studio__title">
          Agent 管理
        </h2>
        <p class="agent-studio__subtitle">
          创建专业化 Agent，绑定模型运行配置、固定提示词、专业方向与 Skill / MCP / Plugin 能力。
        </p>
      </div>
      <div class="agent-studio__actions">
        <EaButton
          type="ghost"
          size="small"
          @click="handleCreate"
        >
          <EaIcon
            name="plus"
            :size="14"
          />
          新建 Agent
        </EaButton>
        <EaButton
          type="primary"
          size="small"
          :loading="isSaving"
          @click="handleSave"
        >
          <EaIcon
            name="save"
            :size="14"
          />
          保存
        </EaButton>
      </div>
    </div>

    <div class="agent-studio__layout">
      <aside class="agent-studio__sidebar">
        <button
          v-for="agent in agentStudioStore.agents"
          :key="agent.id"
          class="agent-studio__agent-card"
          :class="{ 'agent-studio__agent-card--active': selectedAgentId === agent.id }"
          @click="selectedAgentId = agent.id"
        >
          <div class="agent-studio__agent-card-top">
            <strong>{{ agent.name }}</strong>
            <span class="agent-studio__agent-role">{{ roleOptions.find(option => option.value === agent.roleType)?.label || agent.roleType }}</span>
          </div>
          <span class="agent-studio__agent-specialization">
            {{ agent.specialization || '未定义专业方向' }}
          </span>
          <span class="agent-studio__agent-meta">
            {{ agent.resourceBindings.length }} 资源
          </span>
        </button>

        <EaStateBlock
          v-if="agentStudioStore.agents.length === 0"
          icon="bot"
          title="还没有 Agent"
          description="先创建一个专业化 Agent，再给它绑定模型、提示词和能力。"
        />
      </aside>

      <section class="agent-studio__main">
        <EaStateBlock
          v-if="!editor"
          icon="bot"
          title="选择 Agent"
          description="从左侧选择一个 Agent，或新建一个。"
        />

        <template v-else>
          <div class="agent-studio__summary">
            <div class="summary-card">
              <span class="summary-card__label">绑定模型</span>
              <strong>{{ runtimeOptions.find(option => option.value === editor?.runtimeAgentId)?.label || '未选择' }}</strong>
            </div>
            <div class="summary-card">
              <span class="summary-card__label">默认模型</span>
              <strong>{{ editor.runtimeModelId || '跟随运行配置' }}</strong>
            </div>
            <div class="summary-card">
              <span class="summary-card__label">能力数</span>
              <strong>{{ editor.resourceBindings.length }}</strong>
            </div>
            <div class="summary-card">
              <span class="summary-card__label">团队归属</span>
              <strong>{{ linkedTeamCount }}</strong>
            </div>
          </div>

          <div class="agent-studio__grid">
            <label class="form-field">
              <span>名称</span>
              <input
                v-model="editor.name"
                type="text"
                placeholder="例如：Rust 架构师 / 只读研究员 / 前端实现 Agent"
              >
            </label>

            <label class="form-field">
              <span>角色类型</span>
              <select v-model="editor.roleType">
                <option
                  v-for="option in roleOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </select>
            </label>

            <label class="form-field form-field--full">
              <span>专业方向</span>
              <input
                v-model="editor.specialization"
                type="text"
                placeholder="例如：代码审查、需求澄清、Rust 后端、前端交互实现"
              >
            </label>

            <label class="form-field form-field--full">
              <span>描述</span>
              <textarea
                v-model="editor.description"
                rows="3"
                placeholder="描述这个 Agent 适合处理什么任务。"
              />
            </label>

            <label class="form-field">
              <span>运行配置</span>
              <select v-model="editor.runtimeAgentId">
                <option
                  v-for="option in runtimeOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </select>
            </label>

            <label class="form-field">
              <span>默认模型</span>
              <select v-model="editor.runtimeModelId">
                <option value="">
                  跟随运行配置默认模型
                </option>
                <option
                  v-for="option in runtimeModels"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </select>
            </label>

            <label class="form-field form-field--full">
              <span>系统提示词</span>
              <textarea
                v-model="editor.systemPrompt"
                rows="6"
                placeholder="定义长期角色、行为边界、专业口径与稳定约束。"
              />
            </label>

            <label class="form-field form-field--full">
              <span>固定工作边界</span>
              <textarea
                v-model="editor.instructionNote"
                rows="4"
                placeholder="补充特定规则，例如只读研究、先分析后执行、优先输出方案等。"
              />
            </label>
          </div>

          <div class="agent-studio__toggles">
            <label class="switch-item">
              <input
                v-model="editor.readonlyResearcher"
                type="checkbox"
              >
              <span>只读研究型</span>
            </label>
            <label class="switch-item">
              <input
                v-model="editor.planningCapability"
                type="checkbox"
              >
              <span>可做规划协调</span>
            </label>
            <label class="switch-item">
              <input
                v-model="editor.executionCapability"
                type="checkbox"
              >
              <span>可做执行交付</span>
            </label>
            <label class="switch-item">
              <input
                v-model="editor.enabled"
                type="checkbox"
              >
              <span>启用</span>
            </label>
          </div>

          <div class="agent-studio__resource-panel">
            <div class="panel-header">
              <div>
                <h3>能力挂载</h3>
                <p>这里引用的是全局资源目录，不复制资源本体。</p>
              </div>
            </div>

            <div class="resource-groups">
              <section
                v-for="group in resourceGroups"
                :key="group.key"
                class="resource-group"
              >
                <h4>{{ group.title }}</h4>
                <div
                  v-if="group.items.length > 0"
                  class="resource-list"
                >
                  <label
                    v-for="item in group.items"
                    :key="`${item.resourceType}-${item.resourceId}`"
                    class="resource-item"
                  >
                    <input
                      :checked="isBound(item)"
                      type="checkbox"
                      @change="toggleBinding(item)"
                    >
                    <div class="resource-item__content">
                      <strong>{{ item.name }}</strong>
                      <span>{{ item.description || item.source || '无附加描述' }}</span>
                    </div>
                  </label>
                </div>
                <EaStateBlock
                  v-else
                  icon="box"
                  title="暂无资源"
                  description="当前分组还没有可绑定资源。"
                />
              </section>
            </div>
          </div>

          <div class="agent-studio__footer">
            <EaButton
              v-if="selectedAgent"
              type="ghost"
              size="small"
              @click="handleDelete"
            >
              <EaIcon
                name="trash-2"
                :size="14"
              />
              删除当前 Agent
            </EaButton>
          </div>
        </template>
      </section>
    </div>

    <AgentTeamManager />
  </div>
</template>

<style scoped>
.agent-studio {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-5);
  min-width: 0;
}

.agent-studio__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-4);
}

.agent-studio__title {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: var(--color-text-primary);
}

.agent-studio__subtitle {
  margin: var(--spacing-2) 0 0;
  color: var(--color-text-secondary);
  font-size: 14px;
}

.agent-studio__actions {
  display: flex;
  gap: var(--spacing-2);
  flex-wrap: wrap;
}

.agent-studio__layout {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  gap: var(--spacing-4);
  min-width: 0;
}

.agent-studio__sidebar {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.agent-studio__agent-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: 14px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  text-align: left;
}

.agent-studio__agent-card--active {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 1px var(--color-primary-light);
}

.agent-studio__agent-card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
}

.agent-studio__agent-role,
.agent-studio__agent-meta,
.agent-studio__agent-specialization {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.agent-studio__main {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  min-width: 0;
}

.agent-studio__summary {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--spacing-3);
}

.summary-card {
  padding: 14px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.summary-card__label {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.agent-studio__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--spacing-4);
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.form-field--full {
  grid-column: 1 / -1;
}

.form-field span {
  font-size: 13px;
  color: var(--color-text-secondary);
}

.form-field input,
.form-field select,
.form-field textarea {
  width: 100%;
  min-width: 0;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text-primary);
}

.agent-studio__toggles {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-4);
  padding: 14px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-border);
  background: var(--color-surface);
}

.switch-item {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-2);
  color: var(--color-text-primary);
}

.agent-studio__resource-panel {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  padding: 16px;
}

.panel-header h3 {
  margin: 0;
}

.panel-header p {
  margin: 6px 0 0;
  color: var(--color-text-secondary);
  font-size: 13px;
}

.resource-groups {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--spacing-4);
  margin-top: var(--spacing-4);
}

.resource-group {
  min-width: 0;
}

.resource-group h4 {
  margin: 0 0 10px;
}

.resource-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.resource-item {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-2);
  padding: 10px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
}

.resource-item__content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.resource-item__content span {
  font-size: 12px;
  color: var(--color-text-secondary);
  word-break: break-word;
}

.agent-studio__footer {
  display: flex;
  justify-content: flex-end;
}

@media (max-width: 1100px) {
  .agent-studio__layout {
    grid-template-columns: 1fr;
  }

  .agent-studio__summary,
  .resource-groups {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 768px) {
  .agent-studio__header {
    flex-direction: column;
  }

  .agent-studio__summary,
  .agent-studio__grid,
  .resource-groups {
    grid-template-columns: 1fr;
  }
}
</style>
