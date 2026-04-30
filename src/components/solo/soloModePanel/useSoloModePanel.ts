import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useConfirmDialog } from '@/composables'
import { resolveExpertRuntime } from '@/services/agentTeams/runtime'
import { compactSoloSummary } from '@/services/solo/prompts'
import { useAgentStore } from '@/stores/agent'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { useProjectStore } from '@/stores/project'
import { useSoloExecutionStore } from '@/stores/soloExecution'
import { useSoloRunStore } from '@/stores/soloRun'
import type { SoloRun, SoloStep } from '@/types/solo'
import type { SoloAgentOption, SoloCreateFormState, SoloModelOption, SoloRunFormMode } from '../soloShared'

/**
 * 管理 SOLO 面板的运行列表、创建表单和执行时间线状态。
 */
export function useSoloModePanel() {
  const projectStore = useProjectStore()
  const soloRunStore = useSoloRunStore()
  const soloExecutionStore = useSoloExecutionStore()
  const agentStore = useAgentStore()
  const agentTeamsStore = useAgentTeamsStore()
  const agentConfigStore = useAgentConfigStore()
  const confirmDialog = useConfirmDialog()

  const showCreateDialog = ref(false)
  const dialogMode = ref<SoloRunFormMode>('create')
  const editingRunId = ref<string | null>(null)
  const createDialogModelOptions = ref<SoloModelOption[]>([])
  const selectedStepId = ref<string | null>(null)
  const isLogPanelOpen = ref(false)
  const logPanelWidth = ref(380)
  const isLogPanelResizing = ref(false)
  const minLogPanelWidth = 280
  const maxLogPanelWidth = 620

  const createForm = reactive<SoloCreateFormState>({
    projectId: '',
    executionPath: '',
    name: '',
    requirement: '',
    goal: '',
    memoryLibraryIds: [],
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
  const timelineSteps = computed(() => (
    [...currentSteps.value].sort((left, right) => (
      new Date(right.updatedAt || right.createdAt).getTime()
      - new Date(left.updatedAt || left.createdAt).getTime()
    ))
  ))
  const currentExecutionState = computed(() => currentRun.value ? soloExecutionStore.getExecutionState(currentRun.value.id) : undefined)
  const currentRunLogs = computed(() => currentExecutionState.value?.logs ?? [])
  const selectedStep = computed(() => currentSteps.value.find((step) => step.id === selectedStepId.value) || null)

  const builtinExpertPool = computed(() => {
    const enabledBuiltins = agentTeamsStore.enabledExperts.filter((expert) => expert.isBuiltin)
    return enabledBuiltins.length > 0 ? enabledBuiltins : agentTeamsStore.enabledExperts
  })

  const participantExpertOptions = computed<SoloAgentOption[]>(() => {
    const selectedIds = new Set((createForm.participantExpertIds ?? []).filter(Boolean))
    const selectableExperts = agentTeamsStore.enabledExperts.filter((expert) =>
      expert.builtinCode !== 'builtin-solo-coordinator'
    )
    const selectedExperts = agentTeamsStore.experts.filter((expert) =>
      selectedIds.has(expert.id) && expert.builtinCode !== 'builtin-solo-coordinator'
    )

    return Array.from(
      new Map(
        [...selectedExperts, ...selectableExperts].map((expert) => [
          expert.id,
          {
            label: expert.name,
            value: expert.id,
            description: expert.description || `${expert.category} · ${expert.isBuiltin ? '内置专家' : '自定义专家'}`
          }
        ])
      ).values()
    )
  })

  const coordinatorExpertOptions = computed<SoloAgentOption[]>(() => {
    const selectedCoordinator = agentTeamsStore.getExpertById(createForm.coordinatorExpertId)
    const coordinatorExperts = builtinExpertPool.value.filter((expert) =>
      expert.builtinCode === 'builtin-solo-coordinator'
    )
    const fallbackExperts = coordinatorExperts.length > 0
      ? coordinatorExperts
      : builtinExpertPool.value.filter((expert) => expert.category === 'planner' && expert.isBuiltin)

    return Array.from(
      new Map(
        [selectedCoordinator, ...fallbackExperts]
          .filter((expert): expert is NonNullable<typeof expert> => Boolean(expert))
          .map((expert) => [
            expert.id,
            {
              label: expert.name,
              value: expert.id,
              description: expert.description || 'SOLO 规划智能体'
            }
          ])
      ).values()
    )
  })

  const currentRunParticipants = computed(() => {
    if (!currentRun.value) return []
    const idSet = new Set(currentRun.value.participantExpertIds)
    const availableParticipants = agentTeamsStore.enabledExperts.filter((expert) => expert.builtinCode !== 'builtin-solo-coordinator')
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
  const uniqueResultFiles = computed(() => Array.from(
    new Set(
      currentSteps.value.flatMap((step) => step.resultFiles).filter(Boolean)
    )
  ))
  const latestCompletedStep = computed(() => (
    [...currentSteps.value]
      .reverse()
      .find((step) => step.resultSummary || step.summary || step.failReason) || null
  ))
  const currentRunHistoryRows = computed(() => {
    if (!currentRun.value) return []

    return [
      { label: '创建时间', value: formatFullTime(currentRun.value.createdAt) },
      { label: '开始执行', value: formatFullTime(currentRun.value.startedAt) },
      { label: '最后更新', value: formatFullTime(currentRun.value.updatedAt) },
      { label: '结束时间', value: formatFullTime(currentRun.value.completedAt || currentRun.value.stoppedAt) }
    ]
  })
  const currentRunHistoryMetrics = computed(() => {
    const stepCount = currentSteps.value.length
    const logs = currentRunLogs.value
    const toolCallCount = logs.filter((log) => log.type === 'tool_use').length

    return [
      { label: '总步骤', value: String(stepCount) },
      { label: '日志条数', value: String(logs.length) },
      { label: '工具调用', value: String(toolCallCount) },
      { label: '变更文件', value: String(uniqueResultFiles.value.length) }
    ]
  })
  const currentRunDurationLabel = computed(() => {
    if (!currentRun.value?.startedAt) return '尚未开始'
    const endAt = currentRun.value.completedAt || currentRun.value.stoppedAt || currentRun.value.updatedAt
    return formatDuration(currentRun.value.startedAt, endAt)
  })
  const currentRunHistorySummary = computed(() => {
    if (latestCompletedStep.value?.resultSummary) {
      return latestCompletedStep.value.resultSummary
    }
    if (latestCompletedStep.value?.summary) {
      return latestCompletedStep.value.summary
    }
    if (currentRun.value?.lastError) {
      return currentRun.value.lastError
    }
    return '当前运行还没有形成可回看的阶段结论。'
  })

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
  const canEditCurrentRun = computed(() =>
    Boolean(currentRun.value && ['draft', 'failed', 'stopped', 'completed'].includes(currentRun.value.status))
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

  function formatFullTime(value?: string): string {
    if (!value) return '暂无'
    const date = new Date(value)
    return date.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    })
  }

  function formatDuration(start?: string, end?: string): string {
    if (!start || !end) return '暂无'

    const diff = Math.max(0, new Date(end).getTime() - new Date(start).getTime())
    const minutes = Math.floor(diff / 60000)
    const hours = Math.floor(minutes / 60)
    const days = Math.floor(hours / 24)

    if (days > 0) {
      return `${days} 天 ${hours % 24} 小时`
    }
    if (hours > 0) {
      return `${hours} 小时 ${minutes % 60} 分钟`
    }
    if (minutes > 0) {
      return `${minutes} 分钟`
    }
    return '1 分钟内'
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

  async function loadCoordinatorModelOptions(
    coordinatorExpertId?: string | null,
    preferredModelId?: string | null
  ): Promise<SoloModelOption[]> {
    const runtimeExpert = builtinExpertPool.value.find((expert) => expert.id === coordinatorExpertId)
      || agentTeamsStore.getExpertById(coordinatorExpertId)
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

    if (preferredModelId?.trim() && !enabledOptions.some((item) => item.value === preferredModelId.trim())) {
      enabledOptions.unshift({
        label: preferredModelId.trim(),
        value: preferredModelId.trim(),
        isDefault: false
      })
    }

    if (enabledOptions.length > 0) {
      return enabledOptions
    }

    return (preferredModelId?.trim() || agent.modelId)
      ? [{
        label: preferredModelId?.trim() || agent.modelId || '',
        value: preferredModelId?.trim() || agent.modelId || '',
        isDefault: true
      }]
      : []
  }

  function pickDefaultModel(models: SoloModelOption[]): string {
    return models.find((model) => model.isDefault)?.value || models[0]?.value || ''
  }

  async function syncCreateDialogModels(preferredModelId?: string | null) {
    createDialogModelOptions.value = await loadCoordinatorModelOptions(
      createForm.coordinatorExpertId,
      preferredModelId ?? createForm.coordinatorModelId
    )

    const hasCurrentModel = createForm.coordinatorModelId
      && createDialogModelOptions.value.some((model) => model.value === createForm.coordinatorModelId)

    createForm.coordinatorModelId = hasCurrentModel
      ? createForm.coordinatorModelId
      : pickDefaultModel(createDialogModelOptions.value)
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
      memoryLibraryIds: [],
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
    createForm.memoryLibraryIds = [...(projectStore.currentProject?.memoryLibraryIds || [])]
    createForm.participantExpertIds = getDefaultParticipantExpertIds()
    createForm.coordinatorExpertId = getDefaultCoordinatorExpertId()
    await syncCreateDialogModels()
    dialogMode.value = 'create'
    editingRunId.value = null
    showCreateDialog.value = true
  }

  async function openEditDialog() {
    if (!currentRun.value || !canEditCurrentRun.value) return

    await Promise.all([
      projectStore.loadProjects(),
      agentStore.loadAgents(),
      agentTeamsStore.loadExperts()
    ])

    const run = currentRun.value
    updateCreateForm({
      projectId: run.projectId,
      executionPath: run.executionPath,
      name: run.name,
      requirement: run.requirement,
      goal: run.goal,
      memoryLibraryIds: [...run.memoryLibraryIds],
      maxDispatchDepth: run.maxDispatchDepth,
      participantExpertIds: [...run.participantExpertIds],
      coordinatorExpertId: run.coordinatorExpertId || getDefaultCoordinatorExpertId(),
      coordinatorModelId: run.coordinatorModelId || ''
    })
    await syncCreateDialogModels(run.coordinatorModelId)
    dialogMode.value = 'edit'
    editingRunId.value = run.id
    showCreateDialog.value = true
  }

  function closeCreateDialog() {
    showCreateDialog.value = false
    dialogMode.value = 'create'
    editingRunId.value = null
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
        memoryLibraryIds: [...createForm.memoryLibraryIds]
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
      memoryLibraryIds: [...createForm.memoryLibraryIds],
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

  async function saveRunEdits() {
    if (dialogMode.value !== 'edit' || !editingRunId.value) {
      return
    }

    const run = await soloRunStore.updateRun(editingRunId.value, {
      executionPath: createForm.executionPath.trim(),
      name: createForm.name.trim(),
      requirement: createForm.requirement.trim(),
      goal: createForm.goal.trim(),
      memoryLibraryIds: [...createForm.memoryLibraryIds],
      participantExpertIds: [...createForm.participantExpertIds],
      coordinatorExpertId: createForm.coordinatorExpertId || null,
      coordinatorModelId: createForm.coordinatorModelId || null,
      maxDispatchDepth: Math.max(1, Math.min(100, createForm.maxDispatchDepth))
    })

    await selectRun(run.id)
    closeCreateDialog()
  }

  async function selectRun(runId: string) {
    soloRunStore.setCurrentRun(runId)
    isLogPanelOpen.value = false
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
    isLogPanelOpen.value = true
  }

  function closeLogPanel() {
    isLogPanelOpen.value = false
  }

  function startLogPanelResize(event: MouseEvent) {
    isLogPanelResizing.value = true
    event.preventDefault()
  }

  function handleLogPanelResize(event: MouseEvent) {
    if (!isLogPanelResizing.value) {
      return
    }

    const containerRect = document.querySelector('.solo-mode-panel')?.getBoundingClientRect()
    if (!containerRect) {
      return
    }

    const nextWidth = containerRect.right - event.clientX
    logPanelWidth.value = Math.min(maxLogPanelWidth, Math.max(minLogPanelWidth, nextWidth))
  }

  function stopLogPanelResize() {
    isLogPanelResizing.value = false
  }

  function getStepLogCount(stepId: string): number {
    return currentExecutionState.value?.logs.filter((log) => log.stepId === stepId).length ?? 0
  }

  function getStepExpertLabel(step: SoloStep): string {
    const expert = agentTeamsStore.experts.find((item) => item.id === step.selectedExpertId)
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
      isLogPanelOpen.value = false
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
    document.addEventListener('mousemove', handleLogPanelResize)
    document.addEventListener('mouseup', stopLogPanelResize)
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

  watch(
    () => currentRun.value?.id ?? null,
    (runId, previousRunId) => {
      if (runId !== previousRunId) {
        isLogPanelOpen.value = false
      }
    }
  )

  watch(
    () => selectedStep.value?.id ?? null,
    (stepId) => {
      if (!stepId) {
        isLogPanelOpen.value = false
      }
    }
  )

  watch(
    () => currentSteps.value.some((step) => step.id === selectedStepId.value),
    (exists) => {
      if (!exists) {
        isLogPanelOpen.value = false
      }
    }
  )

  onUnmounted(() => {
    document.removeEventListener('mousemove', handleLogPanelResize)
    document.removeEventListener('mouseup', stopLogPanelResize)
  })

  watch(
    () => showCreateDialog.value,
    (visible) => {
      if (visible) {
        isLogPanelOpen.value = false
      }
    }
  )

  return {
    canCreate,
    canEditCurrentRun,
    completedCount,
    dialogMode,
    coordinatorExpertOptions,
    createDialogModelOptions,
    createForm,
    currentExecutionState,
    currentRunDurationLabel,
    currentRunHistoryMetrics,
    currentRunHistoryRows,
    currentRunHistorySummary,
    currentRun,
    currentRunCoordinatorLabel,
    currentRunParticipants,
    currentSteps,
    timelineSteps,
    failedCount,
    blockedCount,
    formatTime,
    getStepExpertLabel,
    getStepLogCount,
    handleBrowseExecutionPath,
    handleDelete,
    openEditDialog,
    handlePause,
    handleReset,
    handleResume,
    handleStart,
    handleStop,
    closeLogPanel,
    openCreateDialog,
    participantExpertOptions,
    runStatusLabel,
    runs,
    selectRun,
    selectedStep,
    selectedStepId,
    selectStep,
    isLogPanelOpen,
    logPanelWidth,
    isLogPanelResizing,
    startLogPanelResize,
    showCreateDialog,
    soloRunStore,
    stepStatusLabel,
    updateCreateForm,
    createRun,
    saveRunEdits,
    closeCreateDialog,
    compactSoloSummary
  }
}
