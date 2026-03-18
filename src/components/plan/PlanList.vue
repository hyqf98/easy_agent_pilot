<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useConfirmDialog } from '@/composables'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { inferAgentProvider, useAgentStore } from '@/stores/agent'
import { usePlanStore } from '@/stores/plan'
import { useProjectStore } from '@/stores/project'
import type { Plan, PlanStatus, TaskStatus, UpdatePlanInput } from '@/types/plan'
import PlanCreateDialog from './PlanCreateDialog.vue'
import PlanEditDialog from './PlanEditDialog.vue'
import PlanListEmptyState from './PlanListEmptyState.vue'
import PlanListHeader from './PlanListHeader.vue'
import PlanListItem from './PlanListItem.vue'
import PlanListStatusTabs from './PlanListStatusTabs.vue'
import PlanSplitConfigDialog from './PlanSplitConfigDialog.vue'
import TaskSplitDialog from './TaskSplitDialog.vue'
import type {
  AgentOption,
  ModelOption,
  PlanListItemViewModel,
  PlanCreateFormState,
  PlanEditFormState,
  PlanTabKey,
  PlanSplitConfigFormState,
  ProjectOption,
  PlanTaskStats
} from './planListShared'

interface TaskStatusItem {
  status: TaskStatus
}

const EMPTY_PLAN_TASK_STATS: PlanTaskStats = {
  total: 0,
  executionQueue: 0,
  completed: 0,
  failed: 0
}

const planStore = usePlanStore()
const projectStore = useProjectStore()
const agentStore = useAgentStore()
const agentConfigStore = useAgentConfigStore()
const confirmDialog = useConfirmDialog()
const emit = defineEmits<{
  (e: 'plan-click', plan: Plan): void
}>()

const planTaskStats = ref<Record<string, PlanTaskStats>>({})
const selectedProjectIdForList = ref<string | null>(null)
const activeStatusTab = ref<PlanTabKey>('draft')
const showCreateDialog = ref(false)
const showSplitConfigDialog = ref(false)
const showEditDialog = ref(false)
const splitConfigPlan = ref<Plan | null>(null)
const editingPlan = ref<Plan | null>(null)
let planTaskStatsRequestId = 0

const createDialogModelOptions = ref<ModelOption[]>([])
const splitConfigModelOptions = ref<ModelOption[]>([])
const editDialogModelOptions = ref<ModelOption[]>([])

const createForm = reactive<PlanCreateFormState>({
  name: '',
  description: '',
  splitMode: 'ai',
  granularity: 20,
  maxRetryCount: 3,
  splitAgentId: null,
  splitModelId: '',
  executionMode: 'immediate',
  scheduledDateTime: ''
})

const splitConfigForm = reactive<PlanSplitConfigFormState>({
  agentId: null,
  modelId: ''
})

const editForm = reactive<PlanEditFormState>({
  name: '',
  description: '',
  splitMode: 'ai',
  granularity: 20,
  maxRetryCount: 3,
  splitAgentId: null,
  splitModelId: '',
  executionMode: 'immediate',
  scheduledDateTime: ''
})

const statusTabs: Array<{ key: PlanTabKey, label: string }> = [
  { key: 'draft', label: '草稿状态' },
  { key: 'splitting', label: '拆分中' },
  { key: 'executing', label: '执行中' },
  { key: 'completed', label: '执行完成' }
]

const tabStatusMap: Record<PlanTabKey, PlanStatus[]> = {
  draft: ['draft'],
  splitting: ['planning', 'ready'],
  executing: ['executing', 'paused'],
  completed: ['completed']
}

const statusLabels: Record<PlanStatus, string> = {
  draft: '草稿',
  planning: '规划中',
  ready: '已拆分',
  executing: '执行中',
  completed: '已完成',
  paused: '已暂停'
}

const statusColors: Record<PlanStatus, string> = {
  draft: 'gray',
  planning: 'orange',
  ready: 'yellow',
  executing: 'blue',
  completed: 'green',
  paused: 'orange'
}

const projectOptions = computed<ProjectOption[]>(() =>
  projectStore.projects.map(project => ({
    label: project.name,
    value: project.id,
    path: project.path
  }))
)

const selectedListProject = computed(() => {
  if (!selectedProjectIdForList.value) return null
  return projectStore.projects.find(project => project.id === selectedProjectIdForList.value) || null
})

const agentOptions = computed<AgentOption[]>(() =>
  agentStore.agents.map(agent => ({
    label: `${agent.name} (${agent.type.toUpperCase()}${agent.provider ? ` / ${agent.provider}` : ''})`,
    value: agent.id
  }))
)

const plans = computed(() => {
  if (!projectStore.currentProject) return []
  return planStore.plansByProject(projectStore.currentProject.id)
})

const statusTabCounts = computed<Record<PlanTabKey, number>>(() => ({
  draft: plans.value.filter(plan => tabStatusMap.draft.includes(plan.status)).length,
  splitting: plans.value.filter(plan => tabStatusMap.splitting.includes(plan.status)).length,
  executing: plans.value.filter(plan => tabStatusMap.executing.includes(plan.status)).length,
  completed: plans.value.filter(plan => tabStatusMap.completed.includes(plan.status)).length
}))

const filteredPlans = computed(() =>
  plans.value.filter(plan => tabStatusMap[activeStatusTab.value].includes(plan.status))
)

const visiblePlanCount = computed(() => filteredPlans.value.length)

const planItems = computed<PlanListItemViewModel[]>(() =>
  filteredPlans.value.map(plan => ({
    plan,
    isActive: planStore.currentPlanId === plan.id,
    statusLabel: statusLabels[plan.status],
    statusColor: statusColors[plan.status],
    relativeTimeLabel: formatRelativeTime(plan.updatedAt),
    scheduledLabel: formatScheduledTime(plan.scheduledAt),
    taskStats: getPlanTaskStats(plan.id),
    canSplit: canSplit(plan),
    canResumeSplit: plan.status === 'planning',
    canEdit: canEdit(plan)
  }))
)

const activeStatusTabLabel = computed(() =>
  statusTabs.find(tab => tab.key === activeStatusTab.value)?.label ?? ''
)

const canSaveDraft = computed(() =>
  Boolean(projectStore.currentProjectId && createForm.name.trim())
)

const canStartSplitFromCreate = computed(() =>
  Boolean(
    canSaveDraft.value &&
    createForm.splitAgentId !== null &&
    createDialogModelOptions.value.length > 0 &&
    isModelSelectionValid(createForm.splitAgentId, createForm.splitModelId)
  )
)

const canStartSplitFromList = computed(() =>
  Boolean(
    splitConfigPlan.value &&
    splitConfigForm.agentId &&
    splitConfigModelOptions.value.length > 0 &&
    isModelSelectionValid(splitConfigForm.agentId, splitConfigForm.modelId)
  )
)

async function loadEnabledModels(agentId: string): Promise<ModelOption[]> {
  const provider = inferAgentProvider(agentStore.agents.find(agent => agent.id === agentId))
  const configs = await agentConfigStore.ensureModelsConfigs(agentId, provider)
  return configs
    .filter(model => model.enabled)
    .map(model => ({
      label: model.displayName,
      value: model.modelId,
      isDefault: model.isDefault
    }))
}

function pickDefaultModel(models: ModelOption[]): string {
  const defaultModel = models.find(model => model.isDefault)
  if (defaultModel) return defaultModel.value
  return models[0]?.value ?? ''
}

function isModelSelectionValid(agentId: string | null, modelId: string): boolean {
  if (!agentId) return false
  const agent = agentStore.agents.find(item => item.id === agentId)
  if (!agent) return false
  if (agent.type === 'sdk') {
    return modelId.trim().length > 0
  }
  return true
}

function buildPlanTaskStats(tasks: TaskStatusItem[]): PlanTaskStats {
  return {
    total: tasks.length,
    executionQueue: tasks.filter(task => task.status === 'pending' || task.status === 'in_progress').length,
    completed: tasks.filter(task => task.status === 'completed').length,
    failed: tasks.filter(task => task.status === 'failed' || task.status === 'cancelled').length
  }
}

function getPlanTaskStats(planId: string): PlanTaskStats {
  return planTaskStats.value[planId] || EMPTY_PLAN_TASK_STATS
}

async function loadPlanTaskStats(planList: Plan[]) {
  const requestId = ++planTaskStatsRequestId

  if (planList.length === 0) {
    planTaskStats.value = {}
    return
  }

  const entries = await Promise.all(
    planList.map(async (plan) => {
      try {
        const tasks = await invoke<TaskStatusItem[]>('list_tasks', { planId: plan.id })
        return [plan.id, buildPlanTaskStats(tasks)] as const
      } catch (error) {
        console.error(`Failed to load task stats for plan ${plan.id}:`, error)
        return [plan.id, { ...EMPTY_PLAN_TASK_STATS }] as const
      }
    })
  )

  if (requestId !== planTaskStatsRequestId) return

  const nextStats: Record<string, PlanTaskStats> = {}
  entries.forEach(([planId, stats]) => {
    nextStats[planId] = stats
  })
  planTaskStats.value = nextStats
}

function selectPlan(plan: Plan) {
  planStore.setCurrentPlan(plan.id)
  emit('plan-click', plan)
}

function handleListProjectChange(projectId: string) {
  if (!projectId) return
  if (projectStore.currentProjectId !== projectId) {
    projectStore.setCurrentProject(projectId)
    return
  }
  void planStore.loadPlans(projectId)
}

function updateCreateForm(patch: Partial<PlanCreateFormState>) {
  Object.assign(createForm, patch)
}

function resetCreateForm() {
  updateCreateForm({
    name: '',
    description: '',
    splitMode: 'ai',
    granularity: 20,
    maxRetryCount: 3,
    splitAgentId: null,
    splitModelId: '',
    executionMode: 'immediate',
    scheduledDateTime: ''
  })
  createDialogModelOptions.value = []
}

async function createPlan(startSplit: boolean) {
  if (!projectStore.currentProjectId || !createForm.name.trim()) return
  if (startSplit && (!createForm.splitAgentId || createDialogModelOptions.value.length === 0)) return

  let scheduledAt: string | undefined
  if (createForm.executionMode === 'scheduled' && createForm.scheduledDateTime) {
    scheduledAt = new Date(createForm.scheduledDateTime).toISOString()
  }

  try {
    const plan = await planStore.createPlan({
      projectId: projectStore.currentProjectId,
      name: createForm.name.trim(),
      description: createForm.description.trim() || undefined,
      splitMode: createForm.splitMode,
      splitAgentId: createForm.splitMode === 'ai' ? (createForm.splitAgentId ?? undefined) : undefined,
      splitModelId: createForm.splitMode === 'ai' && createForm.splitAgentId !== null ? createForm.splitModelId : undefined,
      granularity: createForm.granularity,
      maxRetryCount: createForm.maxRetryCount,
      scheduledAt
    })

    planStore.setCurrentPlan(plan.id)

    if (startSplit && createForm.splitAgentId !== null && createForm.splitMode === 'ai') {
      await planStore.updatePlan(plan.id, { status: 'planning' })
      planStore.openSplitDialog({
        planId: plan.id,
        agentId: createForm.splitAgentId,
        modelId: createForm.splitModelId,
        entry: 'create_start_split'
      })
    }

    closeCreateDialog()
  } catch (error) {
    console.error('Failed to create plan:', error)
  }
}

// 手动模式创建计划
async function createManualPlan() {
  if (!projectStore.currentProjectId || !createForm.name.trim()) return

  let scheduledAt: string | undefined
  if (createForm.executionMode === 'scheduled' && createForm.scheduledDateTime) {
    scheduledAt = new Date(createForm.scheduledDateTime).toISOString()
  }

  try {
    const plan = await planStore.createPlan({
      projectId: projectStore.currentProjectId,
      name: createForm.name.trim(),
      description: createForm.description.trim() || undefined,
      splitMode: 'manual',
      granularity: createForm.granularity,
      maxRetryCount: createForm.maxRetryCount,
      scheduledAt
    })

    // 手动模式：直接进入 planning 状态
    await planStore.updatePlan(plan.id, { status: 'planning' })
    planStore.setCurrentPlan(plan.id)
    closeCreateDialog()
  } catch (error) {
    console.error('Failed to create manual plan:', error)
  }
}

function closeCreateDialog() {
  showCreateDialog.value = false
  resetCreateForm()
}

async function openCreateDialog() {
  if (agentStore.agents.length === 0) {
    await agentStore.loadAgents()
  }
  updateCreateForm({ splitAgentId: agentOptions.value[0]?.value ?? null })
  showCreateDialog.value = true
}

function updateEditForm(patch: Partial<PlanEditFormState>) {
  Object.assign(editForm, patch)
}

function resetEditForm() {
  updateEditForm({
    name: '',
    description: '',
    splitMode: 'ai',
    granularity: 20,
    maxRetryCount: 3,
    splitAgentId: null,
    splitModelId: '',
    executionMode: 'immediate',
    scheduledDateTime: ''
  })
  editDialogModelOptions.value = []
}

function openEditDialog(plan: Plan) {
  editingPlan.value = plan
  updateEditForm({
    name: plan.name,
    description: plan.description || '',
    splitMode: plan.splitMode,
    granularity: plan.granularity,
    maxRetryCount: plan.maxRetryCount,
    splitAgentId: plan.splitAgentId ?? null,
    splitModelId: plan.splitModelId ?? '',
    executionMode: plan.scheduledAt ? 'scheduled' : 'immediate',
    scheduledDateTime: plan.scheduledAt ? new Date(plan.scheduledAt).toISOString().slice(0, 16) : ''
  })
  showEditDialog.value = true
}

function closeEditDialog() {
  showEditDialog.value = false
  editingPlan.value = null
  resetEditForm()
}

async function saveEdit() {
  if (!editingPlan.value || !editForm.name.trim()) return

  try {
    const updates: UpdatePlanInput = {
      name: editForm.name.trim(),
      description: editForm.description.trim() || undefined
    }

     if (editingPlan.value.status === 'draft') {
      updates.splitMode = editForm.splitMode
      updates.granularity = editForm.granularity
      updates.maxRetryCount = editForm.maxRetryCount
      updates.splitAgentId = editForm.splitMode === 'ai'
        ? (editForm.splitAgentId ?? undefined)
        : undefined
      updates.splitModelId = editForm.splitMode === 'ai' && editForm.splitAgentId
        ? (editForm.splitModelId || undefined)
        : undefined
    }

    const canEditScheduleBeforeExecution = ['draft', 'planning', 'ready'].includes(editingPlan.value.status)
    if (canEditScheduleBeforeExecution) {
      if (editForm.executionMode === 'scheduled' && editForm.scheduledDateTime) {
        updates.scheduledAt = new Date(editForm.scheduledDateTime).toISOString()
      } else {
        updates.scheduledAt = undefined
        updates.scheduleStatus = 'none'
      }
    }

    await planStore.updatePlan(editingPlan.value.id, updates)
    closeEditDialog()
  } catch (error) {
    console.error('Failed to update plan:', error)
  }
}

function updateSplitConfigForm(patch: Partial<PlanSplitConfigFormState>) {
  Object.assign(splitConfigForm, patch)
}

function resetSplitConfigForm() {
  updateSplitConfigForm({
    agentId: null,
    modelId: ''
  })
  splitConfigModelOptions.value = []
}

async function startSplitTasks(plan: Plan) {
  if (agentStore.agents.length === 0) {
    await agentStore.loadAgents()
  }

  const configuredAgent = plan.splitAgentId
    ? agentStore.agents.find(agent => agent.id === plan.splitAgentId)
    : null

  const hasSplitConfig = Boolean(
    plan.splitAgentId !== undefined &&
    plan.splitModelId !== undefined &&
    isModelSelectionValid(plan.splitAgentId, plan.splitModelId)
  )

  if (hasSplitConfig && plan.splitAgentId && configuredAgent) {
    await planStore.updatePlan(plan.id, { status: 'planning' })
    planStore.openSplitDialog({
      planId: plan.id,
      agentId: plan.splitAgentId,
      modelId: plan.splitModelId ?? '',
      entry: plan.status === 'planning' ? 'resume_split' : 'list_split'
    })
    return
  }

  splitConfigPlan.value = plan
  updateSplitConfigForm({ agentId: plan.splitAgentId || agentOptions.value[0]?.value || null })
  showSplitConfigDialog.value = true
}

function closeSplitConfigDialog() {
  showSplitConfigDialog.value = false
  splitConfigPlan.value = null
  resetSplitConfigForm()
}

async function confirmSplitConfigAndStart() {
  if (!splitConfigPlan.value || !splitConfigForm.agentId) return
  if (splitConfigModelOptions.value.length === 0) return

  try {
    const updatedPlan = await planStore.updatePlan(splitConfigPlan.value.id, {
      splitAgentId: splitConfigForm.agentId,
      splitModelId: splitConfigForm.modelId,
      status: 'planning'
    })

    planStore.openSplitDialog({
      planId: updatedPlan.id,
      agentId: splitConfigForm.agentId,
      modelId: splitConfigForm.modelId,
      entry: splitConfigPlan.value.status === 'planning' ? 'resume_split' : 'list_split'
    })

    closeSplitConfigDialog()
  } catch (error) {
    console.error('Failed to save split config:', error)
  }
}

async function deletePlan(plan: Plan) {
  const confirmed = await confirmDialog.danger(
    `确定要删除计划「${plan.name}」吗？`,
    '删除计划'
  )
  if (!confirmed) return

  try {
    await planStore.deletePlan(plan.id)
  } catch (error) {
    console.error('Failed to delete plan:', error)
  }
}

function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)

  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  if (hours < 24) return `${hours}小时前`
  if (days < 7) return `${days}天前`

  return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
}

function formatScheduledTime(dateStr: string | undefined): string {
  if (!dateStr) return ''
  const date = new Date(dateStr)
  const now = new Date()
  const diff = date.getTime() - now.getTime()

  if (diff < 0) return '已到期'

  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)

  if (minutes < 60) return `${minutes}分钟后执行`
  if (hours < 24) return `${hours}小时后执行`
  if (days < 7) return `${days}天后执行`

  return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

function canSplit(plan: Plan): boolean {
  return plan.status === 'draft'
}

function canEdit(plan: Plan): boolean {
  return plan.status === 'draft' || plan.status === 'planning'
}

function getPreferredStatusTab(planList: Plan[], preferredPlanId: string | null): PlanTabKey {
  const preferredPlan = preferredPlanId
    ? planList.find(plan => plan.id === preferredPlanId)
    : null

  if (preferredPlan) {
    const matchedTab = statusTabs.find(tab => tabStatusMap[tab.key].includes(preferredPlan.status))
    if (matchedTab) return matchedTab.key
  }

  const firstNonEmptyTab = statusTabs.find(tab =>
    planList.some(plan => tabStatusMap[tab.key].includes(plan.status))
  )

  return firstNonEmptyTab?.key ?? 'draft'
}

onMounted(() => {
  void agentStore.loadAgents()
})

watch(
  () => projectStore.currentProjectId,
  (projectId) => {
    selectedProjectIdForList.value = projectId
    if (projectId) {
      void planStore.loadPlans(projectId)
    }
  },
  { immediate: true }
)

watch(
  () => createForm.splitAgentId,
  async (agentId) => {
    if (!agentId) {
      createDialogModelOptions.value = []
      createForm.splitModelId = ''
      return
    }

    createDialogModelOptions.value = await loadEnabledModels(agentId)
    createForm.splitModelId = pickDefaultModel(createDialogModelOptions.value)
  },
  { immediate: true }
)

watch(
  () => splitConfigForm.agentId,
  async (agentId) => {
    if (!agentId) {
      splitConfigModelOptions.value = []
      splitConfigForm.modelId = ''
      return
    }

    splitConfigModelOptions.value = await loadEnabledModels(agentId)
    const preferredModelId = splitConfigPlan.value?.splitAgentId === agentId
      ? (splitConfigPlan.value.splitModelId ?? '')
      : ''

    splitConfigForm.modelId = splitConfigModelOptions.value.some(option => option.value === preferredModelId)
      ? preferredModelId
      : pickDefaultModel(splitConfigModelOptions.value)
  },
  { immediate: true }
)

watch(
  () => editForm.splitAgentId,
  async (agentId) => {
    if (!showEditDialog.value || editForm.splitMode !== 'ai') return

    if (!agentId) {
      editDialogModelOptions.value = []
      editForm.splitModelId = ''
      return
    }

    editDialogModelOptions.value = await loadEnabledModels(agentId)
    const preferredModelId = editingPlan.value?.splitAgentId === agentId
      ? (editingPlan.value.splitModelId ?? '')
      : editForm.splitModelId

    editForm.splitModelId = editDialogModelOptions.value.some(option => option.value === preferredModelId)
      ? preferredModelId
      : pickDefaultModel(editDialogModelOptions.value)
  },
  { immediate: true }
)

watch(
  () => editForm.splitMode,
  (splitMode) => {
    if (splitMode !== 'ai') {
      editDialogModelOptions.value = []
      editForm.splitAgentId = null
      editForm.splitModelId = ''
    }
  }
)

watch(
  () => showEditDialog.value,
  async (visible) => {
    if (!visible || editForm.splitMode !== 'ai' || !editForm.splitAgentId) return
    editDialogModelOptions.value = await loadEnabledModels(editForm.splitAgentId)
    editForm.splitModelId = editDialogModelOptions.value.some(option => option.value === editForm.splitModelId)
      ? editForm.splitModelId
      : pickDefaultModel(editDialogModelOptions.value)
  }
)

watch(
  plans,
  (nextPlans) => {
    if (nextPlans.length === 0) {
      activeStatusTab.value = 'draft'
    } else if (!nextPlans.some(plan => tabStatusMap[activeStatusTab.value].includes(plan.status))) {
      activeStatusTab.value = getPreferredStatusTab(nextPlans, planStore.currentPlanId)
    }
    void loadPlanTaskStats(nextPlans)
  },
  { immediate: true }
)

watch(
  () => planStore.currentPlanId,
  (planId) => {
    if (!planId || plans.value.length === 0) return

    const matchedPlan = plans.value.find(plan => plan.id === planId)
    if (!matchedPlan) return

    const matchedTab = statusTabs.find(tab => tabStatusMap[tab.key].includes(matchedPlan.status))
    if (matchedTab && matchedTab.key !== activeStatusTab.value) {
      activeStatusTab.value = matchedTab.key
    }
  }
)
</script>

<template>
  <div class="plan-list">
    <PlanListHeader
      :visible-plan-count="visiblePlanCount"
      :selected-project-id="selectedProjectIdForList"
      :selected-project-path="selectedListProject?.path || ''"
      :project-options="projectOptions"
      @create="openCreateDialog"
      @update:selected-project-id="selectedProjectIdForList = $event; handleListProjectChange($event)"
    />

    <div class="list-body">
      <PlanListStatusTabs
        :tabs="statusTabs"
        :active-tab="activeStatusTab"
        :counts="statusTabCounts"
        @update:active-tab="activeStatusTab = $event"
      />

      <div
        v-if="planItems.length > 0"
        class="plan-items"
      >
        <PlanListItem
          v-for="item in planItems"
          :key="item.plan.id"
          :item="item"
          @select="selectPlan(item.plan)"
          @split="startSplitTasks(item.plan)"
          @edit="openEditDialog(item.plan)"
          @delete="deletePlan(item.plan)"
        />
      </div>

      <PlanListEmptyState
        v-else
        :title="plans.length === 0 ? '暂无计划' : `${activeStatusTabLabel}暂无计划`"
      />
    </div>

    <PlanCreateDialog
      :visible="showCreateDialog"
      :form="createForm"
      :agent-options="agentOptions"
      :model-options="createDialogModelOptions"
      :can-save-draft="canSaveDraft"
      :can-start-split="canStartSplitFromCreate"
      @update:form="updateCreateForm"
      @close="closeCreateDialog"
      @save-draft="createPlan(false)"
      @start-split="createPlan(true)"
      @create-manual="createManualPlan"
    />

    <PlanEditDialog
      :visible="showEditDialog"
      :plan="editingPlan"
      :form="editForm"
      :agent-options="agentOptions"
      :model-options="editDialogModelOptions"
      @update:form="updateEditForm"
      @close="closeEditDialog"
      @save="saveEdit"
    />

    <PlanSplitConfigDialog
      :visible="showSplitConfigDialog"
      :plan="splitConfigPlan"
      :form="splitConfigForm"
      :agent-options="agentOptions"
      :model-options="splitConfigModelOptions"
      :can-start="canStartSplitFromList"
      @update:form="updateSplitConfigForm"
      @close="closeSplitConfigDialog"
      @start="confirmSplitConfigAndStart"
    />

    <TaskSplitDialog />
  </div>
</template>

<style scoped>
.plan-list {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: var(--color-bg-secondary, #f8fafc);
}

.list-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-3, 0.75rem);
  scrollbar-width: thin;
  scrollbar-color: var(--color-border, #e2e8f0) transparent;
}

.list-body::-webkit-scrollbar {
  width: 6px;
}

.list-body::-webkit-scrollbar-track {
  background: transparent;
}

.list-body::-webkit-scrollbar-thumb {
  background-color: var(--color-border, #e2e8f0);
  border-radius: var(--radius-full, 9999px);
}

.list-body::-webkit-scrollbar-thumb:hover {
  background-color: var(--color-border-dark, #cbd5e1);
}

.plan-items {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1, 0.25rem);
}
</style>
