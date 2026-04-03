<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useConfirmDialog } from '@/composables'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { useAgentStore } from '@/stores/agent'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { useProjectStore } from '@/stores/project'
import { useSoloExecutionStore } from '@/stores/soloExecution'
import { useSoloRunStore } from '@/stores/soloRun'
import { compactSoloSummary } from '@/services/solo/prompts'
import { resolveExpertRuntime } from '@/services/agentTeams/runtime'
import type { SoloRun, SoloStep } from '@/types/solo'
import SoloExecutionLogPanel from './SoloExecutionLogPanel.vue'
import SoloRunCreateDialog from './SoloRunCreateDialog.vue'
import SoloRunList from './SoloRunList.vue'
import type { SoloAgentOption, SoloCreateFormState, SoloModelOption } from './soloShared'

const projectStore = useProjectStore()
const soloRunStore = useSoloRunStore()
const soloExecutionStore = useSoloExecutionStore()
const agentStore = useAgentStore()
const agentTeamsStore = useAgentTeamsStore()
const agentConfigStore = useAgentConfigStore()
const confirmDialog = useConfirmDialog()

const showCreateDialog = ref(false)
const createDialogModelOptions = ref<SoloModelOption[]>([])
const selectedStepId = ref<string | null>(null)

const createForm = reactive<SoloCreateFormState>({
  projectId: '',
  executionPath: '',
  name: '',
  requirement: '',
  goal: '',
  maxDispatchDepth: 100,
  participantExpertIds: [],
  coordinatorExpertId: null,
  coordinatorModelId: ''
})

const runs = computed(() => {
  if (!projectStore.currentProjectId) return []
  return soloRunStore.runs
    .filter((run) => run.projectId === projectStore.currentProjectId)
    .sort((left, right) => new Date(right.updatedAt).getTime() - new Date(left.updatedAt).getTime())
})

const currentRun = computed(() => soloRunStore.currentRun)
const currentSteps = computed(() => currentRun.value ? soloExecutionStore.getSteps(currentRun.value.id) : [])
const currentExecutionState = computed(() => currentRun.value ? soloExecutionStore.getExecutionState(currentRun.value.id) : undefined)
const selectedStep = computed(() => currentSteps.value.find((step) => step.id === selectedStepId.value) || null)

const builtinExpertPool = computed(() => {
  const enabledBuiltins = agentTeamsStore.enabledExperts.filter((expert) => expert.isBuiltin)
  return enabledBuiltins.length > 0 ? enabledBuiltins : agentTeamsStore.enabledExperts
})

const participantExpertOptions = computed<SoloAgentOption[]>(() =>
  builtinExpertPool.value
    .filter((expert) => expert.builtinCode !== 'builtin-solo-coordinator')
    .map((expert) => ({
    label: expert.name,
    value: expert.id,
    description: expert.description || `${expert.category} · 内置专家`
    }))
)

const coordinatorExpertOptions = computed<SoloAgentOption[]>(() => {
  const coordinatorExperts = builtinExpertPool.value.filter((expert) =>
    expert.builtinCode === 'builtin-solo-coordinator'
  )
  const fallbackExperts = coordinatorExperts.length > 0
    ? coordinatorExperts
    : builtinExpertPool.value.filter((expert) => expert.category === 'planner' && expert.isBuiltin)

  return fallbackExperts.map((expert) => ({
    label: expert.name,
    value: expert.id,
    description: expert.description || 'SOLO 规划智能体'
  }))
})

const currentRunParticipants = computed(() => {
  if (!currentRun.value) return []
  const idSet = new Set(currentRun.value.participantExpertIds)
  const availableParticipants = builtinExpertPool.value.filter((expert) => expert.builtinCode !== 'builtin-solo-coordinator')
  const matched = availableParticipants.filter((expert) => idSet.has(expert.id))
  return matched.length > 0 ? matched : availableParticipants
})

const currentRunCoordinatorLabel = computed(() => {
  if (!currentRun.value?.coordinatorExpertId) return '未指定智能体'
  return builtinExpertPool.value.find((expert) => expert.id === currentRun.value?.coordinatorExpertId)?.name
    || currentRun.value.coordinatorExpertId
})

const completedCount = computed(() => currentSteps.value.filter((step) => step.status === 'completed').length)
const blockedCount = computed(() => currentSteps.value.filter((step) => step.status === 'blocked').length)
const failedCount = computed(() => currentSteps.value.filter((step) => step.status === 'failed').length)

const canCreate = computed(() =>
  Boolean(
    createForm.executionPath.trim()
    && createForm.name.trim()
    && createForm.requirement.trim()
    && createForm.goal.trim()
    && createForm.participantExpertIds.length > 0
    && createForm.coordinatorExpertId
  )
)

function runStatusLabel(status: SoloRun['status']): string {
  switch (status) {
    case 'draft': return '草稿'
    case 'running': return '执行中'
    case 'blocked': return '待输入'
    case 'completed': return '已完成'
    case 'failed': return '失败'
    case 'paused': return '已暂停'
    case 'stopped': return '已停止'
    default: return status
  }
}

function stepStatusLabel(status: SoloStep['status']): string {
  switch (status) {
    case 'pending': return '等待'
    case 'running': return '执行中'
    case 'completed': return '完成'
    case 'failed': return '失败'
    case 'blocked': return '待输入'
    case 'skipped': return '跳过'
    default: return status
  }
}

function formatTime(value?: string): string {
  if (!value) return '暂无'
  const date = new Date(value)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

function getDefaultParticipantExpertIds(): string[] {
  return participantExpertOptions.value.map((option) => option.value)
}

function getDefaultCoordinatorExpertId(): string | null {
  return agentTeamsStore.builtinSoloCoordinatorExpert?.id
    || coordinatorExpertOptions.value[0]?.value
    || null
}

function extractNameFromPath(path: string): string {
  const normalized = path.trim().replace(/[\\/]+$/, '').replace(/\\/g, '/')
  if (!normalized) return 'SOLO 项目'
  const segments = normalized.split('/').filter(Boolean)
  return segments[segments.length - 1] || 'SOLO 项目'
}

async function loadCoordinatorModelOptions(coordinatorExpertId?: string | null): Promise<SoloModelOption[]> {
  const runtimeExpert = builtinExpertPool.value.find((expert) => expert.id === coordinatorExpertId)
    || agentTeamsStore.builtinSoloCoordinatorExpert
    || agentTeamsStore.builtinPlannerExpert
    || agentTeamsStore.builtinGeneralExpert
    || builtinExpertPool.value[0]
    || null
  const runtime = resolveExpertRuntime(runtimeExpert, agentStore.agents)
  const agent = runtime?.agent
    || agentStore.agents.find((item) => item.type === 'cli')
    || agentStore.agents[0]

  if (!agent) {
    return []
  }

  const configs = await agentConfigStore.ensureModelsConfigs(agent.id, agent.provider)
  const enabledOptions = configs
    .filter((item) => item.enabled)
    .map((item) => ({
      label: item.displayName,
      value: item.modelId,
      isDefault: item.isDefault
    }))

  if (enabledOptions.length > 0) {
    return enabledOptions
  }

  return agent.modelId
    ? [{ label: agent.modelId, value: agent.modelId, isDefault: true }]
    : []
}

function pickDefaultModel(models: SoloModelOption[]): string {
  return models.find((model) => model.isDefault)?.value || models[0]?.value || ''
}

async function syncCreateDialogModels() {
  createDialogModelOptions.value = await loadCoordinatorModelOptions(createForm.coordinatorExpertId)
  createForm.coordinatorModelId = pickDefaultModel(createDialogModelOptions.value)
}

function updateCreateForm(patch: Partial<SoloCreateFormState>) {
  Object.assign(createForm, patch)
}

function resetCreateForm() {
  updateCreateForm({
    projectId: projectStore.currentProjectId || '',
    executionPath: projectStore.currentProject?.path || '',
    name: '',
    requirement: '',
    goal: '',
    maxDispatchDepth: 100,
    participantExpertIds: [],
    coordinatorExpertId: getDefaultCoordinatorExpertId(),
    coordinatorModelId: ''
  })
  createDialogModelOptions.value = []
}

async function openCreateDialog() {
  await Promise.all([
    projectStore.loadProjects(),
    agentStore.loadAgents(),
    agentTeamsStore.loadExperts()
  ])
  createForm.projectId = projectStore.currentProjectId || projectStore.projects[0]?.id || ''
  createForm.executionPath = projectStore.currentProject?.path || projectStore.projects.find((project) => project.id === createForm.projectId)?.path || ''
  createForm.participantExpertIds = getDefaultParticipantExpertIds()
  createForm.coordinatorExpertId = getDefaultCoordinatorExpertId()
  await syncCreateDialogModels()
  showCreateDialog.value = true
}

function closeCreateDialog() {
  showCreateDialog.value = false
  resetCreateForm()
}

async function createRun(startImmediately: boolean) {
  const selectedProject = projectStore.projects.find((project) => project.id === createForm.projectId)
  const trimmedExecutionPath = createForm.executionPath.trim() || selectedProject?.path || ''
  if (!trimmedExecutionPath) return

  let targetProjectId = createForm.projectId || selectedProject?.id || ''
  const matchedProject = projectStore.projects.find((project) => project.path === trimmedExecutionPath)
  if (matchedProject) {
    targetProjectId = matchedProject.id
  } else if (!targetProjectId || selectedProject?.path !== trimmedExecutionPath) {
    const createdProject = await projectStore.createProject({
      name: extractNameFromPath(trimmedExecutionPath),
      path: trimmedExecutionPath,
      description: 'SOLO 执行路径自动创建的项目',
      memoryLibraryIds: []
    })
    targetProjectId = createdProject.id
  }

  if (!targetProjectId) return

  const run = await soloRunStore.createRun({
    projectId: targetProjectId,
    executionPath: trimmedExecutionPath,
    name: createForm.name.trim(),
    requirement: createForm.requirement.trim(),
    goal: createForm.goal.trim(),
    participantExpertIds: createForm.participantExpertIds,
    coordinatorExpertId: createForm.coordinatorExpertId || undefined,
    coordinatorModelId: createForm.coordinatorModelId || undefined,
    maxDispatchDepth: Math.max(1, Math.min(100, createForm.maxDispatchDepth))
  })
  await selectRun(run.id)
  closeCreateDialog()

  if (startImmediately) {
    await soloExecutionStore.startRun(run.id)
  }
}

async function selectRun(runId: string) {
  soloRunStore.setCurrentRun(runId)
  await Promise.all([
    soloRunStore.getRun(runId),
    soloExecutionStore.loadSteps(runId),
    soloExecutionStore.loadLogs(runId)
  ])
  syncSelectedStep()
}

function syncSelectedStep() {
  if (!currentRun.value) {
    selectedStepId.value = null
    return
  }

  const candidateId = currentRun.value.currentStepId || currentSteps.value[currentSteps.value.length - 1]?.id || null
  if (candidateId && currentSteps.value.some((step) => step.id === candidateId)) {
    selectedStepId.value = candidateId
    return
  }

  selectedStepId.value = currentSteps.value[0]?.id || null
}

function selectStep(stepId: string) {
  selectedStepId.value = stepId
}

function getStepLogCount(stepId: string): number {
  return currentExecutionState.value?.logs.filter((log) => log.stepId === stepId).length ?? 0
}

function getStepExpertLabel(step: SoloStep): string {
  const expert = builtinExpertPool.value.find((item) => item.id === step.selectedExpertId)
  return expert?.name || step.selectedExpertId || '未指定专家'
}

async function handleStart() {
  if (!currentRun.value) return
  await soloExecutionStore.startRun(currentRun.value.id)
}

async function handlePause() {
  if (!currentRun.value) return
  await soloExecutionStore.pauseRun(currentRun.value.id)
}

async function handleResume() {
  if (!currentRun.value) return
  await soloExecutionStore.resumeRun(currentRun.value.id)
}

async function handleStop() {
  if (!currentRun.value) return
  const confirmed = await confirmDialog.danger(
    `确定要停止运行「${currentRun.value.name}」吗？停止后将清除当前续接会话。`,
    '停止 SOLO 运行'
  )
  if (!confirmed) return
  await soloExecutionStore.stopRun(currentRun.value.id)
}

async function handleReset() {
  if (!currentRun.value) return
  const confirmed = await confirmDialog.warning(
    `确定要清空运行「${currentRun.value.name}」的步骤和日志吗？`,
    '重置 SOLO 进度'
  )
  if (!confirmed) return
  await soloRunStore.clearRunProgress(currentRun.value.id)
  await Promise.all([
    soloRunStore.getRun(currentRun.value.id),
    soloExecutionStore.loadSteps(currentRun.value.id),
    soloExecutionStore.loadLogs(currentRun.value.id)
  ])
  syncSelectedStep()
}

async function handleDelete() {
  if (!currentRun.value) return
  const confirmed = await confirmDialog.danger(
    `确定要删除运行「${currentRun.value.name}」吗？`,
    '删除 SOLO 运行'
  )
  if (!confirmed) return
  const deletedId = currentRun.value.id
  await soloRunStore.deleteRun(deletedId)
  const nextRun = runs.value[0]
  if (nextRun) {
    await selectRun(nextRun.id)
  } else {
    selectedStepId.value = null
  }
}

async function handleBrowseExecutionPath() {
  const selected = await open({
    title: '选择 SOLO 执行目录',
    multiple: false,
    directory: true
  })

  if (!selected || typeof selected !== 'string') {
    return
  }

  const matchedProject = projectStore.projects.find((project) => project.path === selected)
  updateCreateForm({
    executionPath: selected,
    projectId: matchedProject?.id || createForm.projectId
  })
}

watch(
  () => projectStore.currentProjectId,
  async (projectId) => {
    soloRunStore.setCurrentRun(null)
    selectedStepId.value = null
    if (!projectId) return
    await soloRunStore.loadRuns(projectId)
    if (runs.value[0]) {
      await selectRun(runs.value[0].id)
    }
  },
  { immediate: true }
)

watch(
  () => [currentRun.value?.id, currentRun.value?.currentStepId, currentSteps.value.map((step) => step.id).join(':')].join('|'),
  () => {
    if (!currentRun.value) return
    if (!selectedStepId.value || !currentSteps.value.some((step) => step.id === selectedStepId.value)) {
      syncSelectedStep()
    }
  }
)

watch(
  () => createForm.projectId,
  (projectId, previousProjectId) => {
    const project = projectStore.projects.find((item) => item.id === projectId)
    if (!project) return
    const previousProjectPath = projectStore.projects.find((item) => item.id === previousProjectId)?.path
    if (!createForm.executionPath || createForm.executionPath === previousProjectPath) {
      createForm.executionPath = project.path
    }
  }
)

watch(
  () => createForm.coordinatorExpertId,
  async (expertId, previousExpertId) => {
    if (!showCreateDialog.value || expertId === previousExpertId) return
    await syncCreateDialogModels()
  }
)

onMounted(async () => {
  await Promise.all([
    projectStore.loadProjects(),
    agentStore.loadAgents(),
    agentTeamsStore.loadExperts()
  ])
  if (projectStore.currentProjectId) {
    await soloRunStore.loadRuns(projectStore.currentProjectId)
    if (!soloRunStore.currentRunId && runs.value[0]) {
      await selectRun(runs.value[0].id)
    }
  }
})
</script>

<template>
  <div class="solo-mode-panel">
    <div class="solo-mode-panel__list">
      <SoloRunList
        :runs="runs"
        :current-run-id="soloRunStore.currentRunId"
        @select="selectRun"
        @create="openCreateDialog"
      />
    </div>

    <div class="solo-mode-panel__main">
      <template v-if="currentRun">
        <div class="solo-run-header">
          <div class="solo-run-header__meta">
            <span class="solo-run-header__tag">执行路径</span>
            <strong>{{ currentRun.executionPath }}</strong>
          </div>

          <div class="solo-run-header__actions">
            <button
              v-if="['draft', 'failed', 'stopped'].includes(currentRun.status)"
              class="solo-run-header__button solo-run-header__button--primary"
              @click="handleStart"
            >
              启动
            </button>
            <button
              v-if="currentRun.status === 'running'"
              class="solo-run-header__button solo-run-header__button--secondary"
              @click="handlePause"
            >
              暂停
            </button>
            <button
              v-if="['paused', 'blocked'].includes(currentRun.status)"
              class="solo-run-header__button solo-run-header__button--primary"
              :disabled="currentRun.status === 'blocked' && Boolean(currentRun.inputRequest)"
              @click="handleResume"
            >
              继续
            </button>
            <button
              v-if="['running', 'paused', 'blocked'].includes(currentRun.status)"
              class="solo-run-header__button solo-run-header__button--danger"
              @click="handleStop"
            >
              停止
            </button>
            <button
              class="solo-run-header__button solo-run-header__button--ghost"
              @click="handleReset"
            >
              清空进度
            </button>
            <button
              class="solo-run-header__button solo-run-header__button--ghost"
              @click="handleDelete"
            >
              删除
            </button>
          </div>
        </div>

        <div class="solo-summary-card">
          <div class="solo-summary-card__content">
            <p class="solo-summary-card__eyebrow">
              SOLO Timeline
            </p>
            <h2>{{ currentRun.name }}</h2>
            <p class="solo-summary-card__goal">
              {{ currentRun.goal }}
            </p>
            <div class="solo-summary-card__chips">
              <span class="solo-summary-card__chip">{{ runStatusLabel(currentRun.status) }}</span>
              <span class="solo-summary-card__chip">深度 {{ currentRun.currentDepth }}/{{ currentRun.maxDispatchDepth }}</span>
              <span class="solo-summary-card__chip">步骤 {{ currentSteps.length }}</span>
              <span class="solo-summary-card__chip">专家 {{ currentRunParticipants.length }}</span>
              <span class="solo-summary-card__chip">{{ currentRunCoordinatorLabel }}</span>
              <span
                v-if="currentRun.coordinatorModelId"
                class="solo-summary-card__chip"
              >
                {{ currentRun.coordinatorModelId }}
              </span>
            </div>
          </div>

          <div class="solo-summary-card__stats">
            <article class="solo-stat-card">
              <span>已完成</span>
              <strong>{{ completedCount }}</strong>
            </article>
            <article class="solo-stat-card solo-stat-card--warning">
              <span>待输入</span>
              <strong>{{ blockedCount }}</strong>
            </article>
            <article class="solo-stat-card solo-stat-card--danger">
              <span>失败</span>
              <strong>{{ failedCount }}</strong>
            </article>
          </div>
        </div>

        <div class="solo-brief-grid">
          <section class="solo-brief-card">
            <header>
              <span>需求说明</span>
              <small>{{ formatTime(currentRun.updatedAt) }}</small>
            </header>
            <p>{{ currentRun.requirement }}</p>
          </section>
          <section class="solo-brief-card solo-brief-card--experts">
            <header>
              <span>执行路径与专家</span>
              <small>{{ currentRun.executionPath }}</small>
            </header>
            <div class="solo-brief-card__chips">
              <span>{{ currentRunCoordinatorLabel }}</span>
              <span
                v-for="expert in currentRunParticipants"
                :key="expert.id"
              >
                {{ expert.name }}
              </span>
            </div>
          </section>
        </div>

        <div class="solo-timeline">
          <div class="solo-timeline__header">
            <div>
              <h3>任务过程时间线</h3>
              <p>点击任一步骤卡片，右侧会切换到该步骤的执行日志流程。</p>
            </div>
            <div
              v-if="currentExecutionState?.status"
              class="solo-timeline__runtime"
            >
              <span>运行态</span>
              <strong>{{ currentExecutionState.status }}</strong>
            </div>
          </div>

          <div
            v-if="currentSteps.length === 0"
            class="solo-timeline__empty"
          >
            <p>还没有任何步骤。</p>
            <span>启动后，内置协调 AI 会自动派发第一步并持续轮询执行。</span>
          </div>

          <div
            v-else
            class="solo-timeline__track"
          >
            <article
              v-for="step in currentSteps"
              :key="step.id"
              class="solo-step-card"
              :class="[
                `solo-step-card--${step.status}`,
                { 'solo-step-card--active': selectedStepId === step.id }
              ]"
              @click="selectStep(step.id)"
            >
              <div class="solo-step-card__connector" />
              <div class="solo-step-card__marker" />
              <div class="solo-step-card__surface">
                <div class="solo-step-card__top">
                  <div>
                    <span class="solo-step-card__depth">Depth {{ step.depth }}</span>
                    <h4>{{ step.title }}</h4>
                  </div>
                  <span class="solo-step-card__status">{{ stepStatusLabel(step.status) }}</span>
                </div>

                <p class="solo-step-card__summary">
                  {{ compactSoloSummary(step.resultSummary || step.summary || step.description, '等待执行摘要') }}
                </p>

                <div class="solo-step-card__meta">
                  <span>{{ getStepExpertLabel(step) }}</span>
                  <span>{{ getStepLogCount(step.id) }} 条日志</span>
                  <span>{{ formatTime(step.updatedAt) }}</span>
                </div>

                <div
                  v-if="step.resultFiles.length > 0"
                  class="solo-step-card__files"
                >
                  <span
                    v-for="file in step.resultFiles.slice(0, 4)"
                    :key="file"
                  >
                    {{ file }}
                  </span>
                </div>
              </div>
            </article>
          </div>

          <div
            v-if="currentRun.lastError"
            class="solo-timeline__error"
          >
            {{ currentRun.lastError }}
          </div>
        </div>
      </template>

      <div
        v-else
        class="solo-mode-panel__placeholder"
      >
        <div>
          <p class="solo-mode-panel__placeholder-eyebrow">
            SOLO Mode
          </p>
          <h2>选择一个运行，或者新建一个统一调度任务。</h2>
          <span>左侧查看当前项目下的 SOLO 运行，中间查看步骤时间线，右侧查看选中步骤日志。</span>
        </div>
      </div>
    </div>

    <div
      v-if="currentRun"
      class="solo-mode-panel__log"
    >
      <SoloExecutionLogPanel
        :run-id="currentRun.id"
        :step-id="selectedStep?.id ?? null"
      />
    </div>

    <SoloRunCreateDialog
      :visible="showCreateDialog"
      :form="createForm"
      :coordinator-options="coordinatorExpertOptions"
      :expert-options="participantExpertOptions"
      :model-options="createDialogModelOptions"
      :can-create="canCreate"
      @browse-execution-path="handleBrowseExecutionPath"
      @close="closeCreateDialog"
      @create-draft="createRun(false)"
      @create-and-start="createRun(true)"
      @update:form="updateCreateForm"
    />
  </div>
</template>

<style scoped>
.solo-mode-panel {
  display: grid;
  grid-template-columns: 300px minmax(760px, 1.34fr) minmax(300px, 0.62fr);
  height: 100%;
  background: var(--color-bg-secondary);
  overflow: hidden;
}

.solo-mode-panel__list,
.solo-mode-panel__main,
.solo-mode-panel__log {
  min-width: 0;
  min-height: 0;
}

.solo-mode-panel__list,
.solo-mode-panel__main {
  border-right: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
}

.solo-mode-panel__list,
.solo-mode-panel__log {
  overflow: hidden;
}

.solo-mode-panel__main {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  padding: 14px;
  gap: 10px;
}

.solo-mode-panel__log {
  display: flex;
  width: 100%;
  min-width: 0;
}

.solo-mode-panel__placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
  text-align: center;
}

.solo-mode-panel__placeholder-eyebrow {
  margin: 0 0 10px;
  font-size: 12px;
  letter-spacing: 0.24em;
  text-transform: uppercase;
  color: var(--color-primary);
}

.solo-mode-panel__placeholder h2 {
  margin: 0 0 12px;
  color: var(--color-text-primary);
}

.solo-mode-panel__placeholder span {
  color: var(--color-text-secondary);
}

.solo-run-header,
.solo-summary-card,
.solo-brief-card,
.solo-timeline {
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
  background: color-mix(in srgb, var(--color-surface) 96%, transparent);
  border-radius: 26px;
}

.solo-run-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 14px;
}

.solo-run-header__meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.solo-run-header__tag {
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.solo-run-header__meta strong {
  font-size: 13px;
  color: var(--color-text-primary);
  font-weight: 600;
  word-break: break-all;
}

.solo-run-header__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}

.solo-run-header__button {
  padding: 9px 14px;
  border: none;
  border-radius: 999px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;
}

.solo-run-header__button--primary {
  color: white;
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-primary) 82%, white) 0%, var(--color-primary) 100%);
}

.solo-run-header__button--secondary {
  background: color-mix(in srgb, #0ea5e9 16%, transparent);
  color: color-mix(in srgb, #0369a1 90%, white 10%);
}

.solo-run-header__button--danger {
  background: color-mix(in srgb, #ef4444 14%, transparent);
  color: color-mix(in srgb, #b91c1c 90%, white 10%);
}

.solo-run-header__button--ghost {
  background: color-mix(in srgb, var(--color-surface-hover) 82%, transparent);
  color: var(--color-text-primary);
}

.solo-run-header__button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.solo-summary-card {
  display: grid;
  grid-template-columns: minmax(0, 1.5fr) minmax(260px, 0.9fr);
  gap: 12px;
  padding: 12px 14px;
  background:
    radial-gradient(circle at top left, color-mix(in srgb, var(--color-primary) 14%, transparent) 0%, transparent 46%),
    linear-gradient(180deg, color-mix(in srgb, var(--color-surface) 92%, white 8%) 0%, var(--color-surface) 100%);
}

.solo-summary-card__eyebrow {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.24em;
  text-transform: uppercase;
  color: var(--color-primary);
}

.solo-summary-card__content h2 {
  margin: 4px 0 6px;
  font-size: 20px;
  line-height: 1.12;
  color: var(--color-text-primary);
}

.solo-summary-card__goal {
  margin: 0;
  font-size: 11px;
  line-height: 1.45;
  color: var(--color-text-secondary);
}

.solo-summary-card__chips,
.solo-brief-card__chips,
.solo-step-card__files {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.solo-summary-card__chips {
  margin-top: 8px;
}

.solo-summary-card__chip,
.solo-brief-card__chips span,
.solo-step-card__files span {
  display: inline-flex;
  align-items: center;
  padding: 5px 9px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-surface-hover) 82%, transparent);
  color: var(--color-text-secondary);
  font-size: 11px;
}

.solo-summary-card__stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.solo-stat-card {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border-radius: 16px;
  background: color-mix(in srgb, var(--color-surface) 88%, transparent);
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
}

.solo-stat-card span {
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.solo-stat-card strong {
  font-size: 20px;
  color: var(--color-text-primary);
}

.solo-stat-card--warning {
  background: color-mix(in srgb, #f59e0b 8%, var(--color-surface) 92%);
}

.solo-stat-card--danger {
  background: color-mix(in srgb, #ef4444 8%, var(--color-surface) 92%);
}

.solo-brief-grid {
  display: grid;
  grid-template-columns: 1.2fr 0.8fr;
  gap: 10px;
}

.solo-brief-card {
  padding: 10px 12px;
}

.solo-brief-card header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 6px;
  font-size: 11px;
  color: var(--color-text-secondary);
}

.solo-brief-card p {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
  color: var(--color-text-primary);
  white-space: pre-wrap;
}

.solo-brief-card--experts {
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-primary) 8%, transparent) 0%, color-mix(in srgb, var(--color-surface) 96%, transparent) 100%);
}

.solo-timeline {
  padding: 12px;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.solo-timeline__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 14px;
  margin-bottom: 10px;
}

.solo-timeline__header h3 {
  margin: 0;
  font-size: 18px;
  color: var(--color-text-primary);
}

.solo-timeline__header p {
  margin: 4px 0 0;
  font-size: 11px;
  line-height: 1.45;
  color: var(--color-text-secondary);
}

.solo-timeline__runtime {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  min-width: 110px;
  border-radius: 16px;
  background: color-mix(in srgb, var(--color-surface-hover) 84%, transparent);
}

.solo-timeline__runtime span {
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.solo-timeline__runtime strong {
  color: var(--color-text-primary);
}

.solo-timeline__empty {
  padding: 26px 18px;
  border-radius: 22px;
  text-align: center;
  background: color-mix(in srgb, var(--color-surface-hover) 86%, transparent);
}

.solo-timeline__empty p {
  margin: 0 0 8px;
  color: var(--color-text-primary);
}

.solo-timeline__empty span {
  font-size: 13px;
  line-height: 1.6;
  color: var(--color-text-secondary);
}

.solo-timeline__track {
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-left: 24px;
  min-height: 0;
  overflow: auto;
  padding-right: 6px;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--color-primary) 28%, transparent) transparent;
}

.solo-timeline__track::before {
  content: '';
  position: absolute;
  top: 4px;
  bottom: 4px;
  left: 8px;
  width: 2px;
  background: linear-gradient(180deg, color-mix(in srgb, var(--color-primary) 42%, transparent) 0%, color-mix(in srgb, var(--color-border) 82%, transparent) 100%);
}

.solo-step-card {
  position: relative;
  cursor: pointer;
}

.solo-step-card__connector {
  position: absolute;
  left: -16px;
  top: 26px;
  width: 16px;
  height: 2px;
  background: color-mix(in srgb, var(--color-primary) 36%, var(--color-border) 64%);
}

.solo-step-card__marker {
  position: absolute;
  left: -24px;
  top: 18px;
  width: 18px;
  height: 18px;
  border-radius: 999px;
  border: 4px solid color-mix(in srgb, var(--color-surface) 96%, transparent);
  background: color-mix(in srgb, var(--color-primary) 34%, transparent);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-border) 82%, transparent);
}

.solo-step-card__surface {
  padding: 16px 16px 14px;
  border-radius: 20px;
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
  background: color-mix(in srgb, var(--color-surface) 92%, transparent);
  transition: transform 160ms ease, border-color 160ms ease, box-shadow 160ms ease;
}

.solo-step-card:hover .solo-step-card__surface {
  transform: translateY(-1px);
}

.solo-step-card--active .solo-step-card__surface {
  border-color: color-mix(in srgb, var(--color-primary) 52%, var(--color-border) 48%);
  box-shadow: 0 18px 30px color-mix(in srgb, var(--color-primary) 12%, transparent);
}

.solo-step-card--completed .solo-step-card__marker {
  background: color-mix(in srgb, #22c55e 70%, white 30%);
}

.solo-step-card--failed .solo-step-card__marker {
  background: color-mix(in srgb, #ef4444 70%, white 30%);
}

.solo-step-card--blocked .solo-step-card__marker {
  background: color-mix(in srgb, #f59e0b 70%, white 30%);
}

.solo-step-card__top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.solo-step-card__depth {
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.solo-step-card__top h4 {
  margin: 4px 0 0;
  font-size: 16px;
  color: var(--color-text-primary);
}

.solo-step-card__status {
  display: inline-flex;
  align-items: center;
  padding: 5px 9px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-surface-hover) 84%, transparent);
  color: var(--color-text-secondary);
  font-size: 11px;
  white-space: nowrap;
}

.solo-step-card__summary {
  margin: 10px 0 12px;
  font-size: 13px;
  line-height: 1.62;
  color: var(--color-text-secondary);
}

.solo-step-card__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 10px;
  font-size: 11px;
  color: var(--color-text-tertiary);
}

.solo-timeline__error {
  margin-top: 16px;
  padding: 14px 16px;
  border-radius: 18px;
  background: color-mix(in srgb, #ef4444 10%, transparent);
  color: color-mix(in srgb, #b91c1c 90%, white 10%);
}

@media (max-width: 1380px) {
  .solo-mode-panel {
    grid-template-columns: 270px minmax(0, 1fr) minmax(280px, 0.56fr);
  }

  .solo-summary-card,
  .solo-brief-grid {
    grid-template-columns: 1fr;
  }
}

.solo-timeline__track::-webkit-scrollbar {
  width: 10px;
}

.solo-timeline__track::-webkit-scrollbar-thumb {
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 28%, transparent);
}

.solo-timeline__track::-webkit-scrollbar-track {
  background: transparent;
}

[data-theme='dark'] .solo-mode-panel {
  background: color-mix(in srgb, var(--color-bg-secondary) 92%, #020617 8%);
}

[data-theme='dark'] .solo-run-header,
[data-theme='dark'] .solo-summary-card,
[data-theme='dark'] .solo-brief-card,
[data-theme='dark'] .solo-timeline {
  border-color: color-mix(in srgb, var(--color-border) 58%, rgba(148, 163, 184, 0.24) 42%);
  background: color-mix(in srgb, var(--color-surface) 86%, #020617 14%);
}

[data-theme='dark'] .solo-summary-card {
  background:
    radial-gradient(circle at top left, color-mix(in srgb, var(--color-primary) 20%, transparent) 0%, transparent 44%),
    linear-gradient(180deg, color-mix(in srgb, var(--color-surface) 82%, #020617 18%) 0%, color-mix(in srgb, var(--color-surface) 90%, #020617 10%) 100%);
}

[data-theme='dark'] .solo-brief-card--experts {
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-primary) 14%, #020617 16%) 0%, color-mix(in srgb, var(--color-surface) 88%, #020617 12%) 100%);
}

[data-theme='dark'] .solo-run-header__button--secondary {
  background: color-mix(in srgb, #0ea5e9 24%, rgba(14, 165, 233, 0.12));
  color: #bae6fd;
}

[data-theme='dark'] .solo-run-header__button--danger {
  background: color-mix(in srgb, #ef4444 22%, rgba(239, 68, 68, 0.14));
  color: #fecaca;
}

[data-theme='dark'] .solo-run-header__button--ghost,
[data-theme='dark'] .solo-summary-card__chip,
[data-theme='dark'] .solo-brief-card__chips span,
[data-theme='dark'] .solo-step-card__files span,
[data-theme='dark'] .solo-step-card__status,
[data-theme='dark'] .solo-timeline__runtime {
  background: color-mix(in srgb, var(--color-surface-hover) 54%, rgba(15, 23, 42, 0.46) 46%);
  color: color-mix(in srgb, var(--color-text-secondary) 90%, white 10%);
}

[data-theme='dark'] .solo-stat-card {
  background: color-mix(in srgb, var(--color-surface) 76%, #020617 24%);
  border-color: color-mix(in srgb, var(--color-border) 54%, rgba(148, 163, 184, 0.2) 46%);
}

[data-theme='dark'] .solo-stat-card--warning {
  background: color-mix(in srgb, #f59e0b 14%, color-mix(in srgb, var(--color-surface) 78%, #020617 22%));
}

[data-theme='dark'] .solo-stat-card--danger {
  background: color-mix(in srgb, #ef4444 14%, color-mix(in srgb, var(--color-surface) 78%, #020617 22%));
}

[data-theme='dark'] .solo-timeline__empty,
[data-theme='dark'] .solo-step-card__surface {
  background: color-mix(in srgb, var(--color-surface) 78%, #020617 22%);
  border-color: color-mix(in srgb, var(--color-border) 54%, rgba(148, 163, 184, 0.18) 46%);
}

[data-theme='dark'] .solo-step-card--active .solo-step-card__surface {
  border-color: color-mix(in srgb, var(--color-primary) 48%, rgba(59, 130, 246, 0.26) 52%);
  box-shadow: 0 18px 30px rgba(15, 23, 42, 0.42);
}

[data-theme='dark'] .solo-timeline__track::before {
  background: linear-gradient(180deg, color-mix(in srgb, var(--color-primary) 54%, transparent) 0%, color-mix(in srgb, rgba(148, 163, 184, 0.24) 76%, transparent) 100%);
}
</style>
