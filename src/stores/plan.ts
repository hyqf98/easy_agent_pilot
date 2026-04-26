import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { useTaskExecutionStore } from './taskExecution'
import { useTaskStore } from './task'
import { useTaskSplitStore } from './taskSplit'
import { getErrorMessage } from '@/utils/api'
import type { Plan, PlanStatus, PlanExecutionStatus, ScheduleStatus, CreatePlanInput, UpdatePlanInput, AgentRole, PlanSplitMode } from '@/types/plan'
import { DEFAULT_SPLIT_GRANULARITY } from '@/constants/plan'

// Rust 后端返回的 snake_case 结构
interface RustPlan {
  id: string
  project_id: string
  name: string
  description?: string
  memory_library_ids?: string[]
  execution_overview?: string
  execution_overview_updated_at?: string
  split_mode: string
  split_expert_id?: string
  split_agent_id?: string
  split_model_id?: string
  status: string
  execution_status?: string
  current_task_id?: string
  agent_team?: string // JSON 字符串
  granularity: number
  max_retry_count: number
  scheduled_at?: string
  schedule_status?: string
  created_at: string
  updated_at: string
}

export interface PlanSplitDialogContext {
  planId: string
  expertId?: string
  agentId: string
  modelId: string
  entry: 'create_start_split' | 'list_split' | 'resume_split'
}

// 将 Rust 返回的 snake_case 转换为 camelCase
function transformPlan(rustPlan: RustPlan): Plan {
  let agentTeam: AgentRole[] | undefined
  if (rustPlan.agent_team) {
    try {
      agentTeam = JSON.parse(rustPlan.agent_team)
    } catch {
      // ignore parse error
    }
  }

  return {
    id: rustPlan.id,
    projectId: rustPlan.project_id,
    name: rustPlan.name,
    description: rustPlan.description,
    memoryLibraryIds: rustPlan.memory_library_ids ?? [],
    executionOverview: rustPlan.execution_overview,
    executionOverviewUpdatedAt: rustPlan.execution_overview_updated_at,
    splitMode: (rustPlan.split_mode || 'ai') as PlanSplitMode,
    splitExpertId: rustPlan.split_expert_id,
    splitAgentId: rustPlan.split_agent_id,
    splitModelId: rustPlan.split_model_id,
    status: rustPlan.status as PlanStatus,
    executionStatus: rustPlan.execution_status as PlanExecutionStatus | undefined,
    currentTaskId: rustPlan.current_task_id,
    agentTeam,
    granularity: rustPlan.granularity,
    maxRetryCount: rustPlan.max_retry_count,
    scheduledAt: rustPlan.scheduled_at,
    scheduleStatus: rustPlan.schedule_status as ScheduleStatus | undefined,
    createdAt: rustPlan.created_at,
    updatedAt: rustPlan.updated_at
  }
}

export const usePlanStore = defineStore('plan', () => {
  // State
  const plans = ref<Plan[]>([])
  const currentPlanId = ref<string | null>(null)
  const isLoading = ref(false)
  const loadError = ref<string | null>(null)
  const splitDialogVisible = ref(false)
  const splitDialogContexts = ref<Map<string, PlanSplitDialogContext>>(new Map())
  const activeSplitPlanId = ref<string | null>(null)

  const splitDialogContext = computed<PlanSplitDialogContext | null>(() => {
    if (!activeSplitPlanId.value) return null
    return splitDialogContexts.value.get(activeSplitPlanId.value) ?? null
  })

  const splitDialogPlanIds = computed<string[]>(() =>
    Array.from(splitDialogContexts.value.keys())
  )

  // Getters
  const currentPlan = computed(() =>
    plans.value.find(p => p.id === currentPlanId.value)
  )

  const plansByProject = computed(() => {
    return (projectId: string) => {
      return plans.value
        .filter(p => p.projectId === projectId)
        .sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())
    }
  })

  const plansByStatus = computed(() => {
    return (status: PlanStatus) => {
      return plans.value
        .filter(p => p.status === status)
        .sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())
    }
  })

  function replaceProjectPlans(projectId: string, nextPlans: Plan[]): void {
    plans.value = [
      ...plans.value.filter(plan => plan.projectId !== projectId),
      ...nextPlans
    ]

    if (currentPlanId.value) {
      const currentPlan = plans.value.find(plan => plan.id === currentPlanId.value)
      if (!currentPlan) {
        currentPlanId.value = null
      }
    }
  }

  // Actions
  async function loadPlans(projectId: string) {
    isLoading.value = true
    loadError.value = null
    const notificationStore = useNotificationStore()
    try {
      const rustPlans = await invoke<RustPlan[]>('list_plans', { projectId })
      replaceProjectPlans(projectId, rustPlans.map(transformPlan))
    } catch (error) {
      console.error('Failed to load plans:', error)
      loadError.value = getErrorMessage(error)
      notificationStore.networkError(
        '加载计划列表',
        getErrorMessage(error),
        () => loadPlans(projectId)
      )
    } finally {
      isLoading.value = false
    }
  }

  async function getPlan(id: string): Promise<Plan | null> {
    try {
      const rustPlan = await invoke<RustPlan>('get_plan', { id })
      return transformPlan(rustPlan)
    } catch (error) {
      console.error('Failed to get plan:', error)
      return null
    }
  }

  async function createPlan(input: CreatePlanInput): Promise<Plan> {
    const notificationStore = useNotificationStore()
    const rustInput = {
      project_id: input.projectId,
      name: input.name,
      description: input.description ?? null,
      memory_library_ids: input.memoryLibraryIds ?? [],
      split_mode: input.splitMode ?? 'ai',
      split_expert_id: input.splitExpertId ?? null,
      split_agent_id: input.splitAgentId ?? null,
      split_model_id: input.splitModelId ?? null,
      agent_team: input.agentTeam ?? null,
      granularity: input.granularity ?? DEFAULT_SPLIT_GRANULARITY,
      max_retry_count: input.maxRetryCount ?? 3,
      scheduled_at: input.scheduledAt ?? null
    }

    try {
      const rustPlan = await invoke<RustPlan>('create_plan', { input: rustInput })
      const newPlan = transformPlan(rustPlan)
      plans.value.unshift(newPlan)
      return newPlan
    } catch (error) {
      console.error('Failed to create plan:', error)
      notificationStore.databaseError(
        '创建计划失败',
        getErrorMessage(error),
        async () => { await createPlan(input) }
      )
      throw error
    }
  }

  async function updatePlan(id: string, updates: UpdatePlanInput): Promise<Plan> {
    const notificationStore = useNotificationStore()
    const input: Record<string, unknown> = {}

    if ('name' in updates) input.name = updates.name ?? null
    if ('description' in updates) input.description = updates.description ?? null
    if ('memoryLibraryIds' in updates) input.memory_library_ids = updates.memoryLibraryIds ?? []
    if ('executionOverview' in updates) input.execution_overview = updates.executionOverview ?? null
    if ('executionOverviewUpdatedAt' in updates) input.execution_overview_updated_at = updates.executionOverviewUpdatedAt ?? null
    if ('splitMode' in updates) input.split_mode = updates.splitMode ?? null
    if ('splitExpertId' in updates) input.split_expert_id = updates.splitExpertId ?? null
    if ('splitAgentId' in updates) input.split_agent_id = updates.splitAgentId ?? null
    if ('splitModelId' in updates) input.split_model_id = updates.splitModelId ?? null
    if ('status' in updates) input.status = updates.status ?? null
    if ('executionStatus' in updates) input.execution_status = updates.executionStatus ?? null
    if ('currentTaskId' in updates) input.current_task_id = updates.currentTaskId ?? null
    if ('agentTeam' in updates) input.agent_team = updates.agentTeam ?? null
    if ('granularity' in updates) input.granularity = updates.granularity ?? null
    if ('maxRetryCount' in updates) input.max_retry_count = updates.maxRetryCount ?? null
    if ('scheduledAt' in updates) input.scheduled_at = updates.scheduledAt ?? null
    if ('scheduleStatus' in updates) input.schedule_status = updates.scheduleStatus ?? null

    try {
      const rustPlan = await invoke<RustPlan>('update_plan', { id, input })
      const updatedPlan = transformPlan(rustPlan)

      const index = plans.value.findIndex(p => p.id === id)
      if (index !== -1) {
        plans.value[index] = updatedPlan
      }

      return updatedPlan
    } catch (error) {
      console.error('Failed to update plan:', error)
      notificationStore.databaseError(
        '更新计划失败',
        getErrorMessage(error),
        async () => { await updatePlan(id, updates) }
      )
      throw error
    }
  }

  // 将计划状态设置为"就绪"（任务拆分完成，等待用户确认开始执行）
  async function markPlanAsReady(planId: string): Promise<Plan> {
    const plan = plans.value.find(p => p.id === planId)
    if (!plan) {
      throw new Error('Plan not found')
    }

    // 先更新状态为 ready
    let updatedPlan = await updatePlan(planId, { status: 'ready' })

    // 检查是否需要设置为定时计划
    if (updatedPlan.scheduledAt) {
      const scheduledTime = new Date(updatedPlan.scheduledAt)
      const now = new Date()

      if (scheduledTime > now) {
        // 定时时间在未来，设置为 scheduled 状态
        updatedPlan = await updatePlan(planId, { scheduleStatus: 'scheduled' })
        console.log(`Plan ${planId} set as scheduled for ${scheduledTime.toLocaleString('zh-CN')}`)
      } else {
        // 时间已过，清除定时设置
        updatedPlan = await updatePlan(planId, { scheduledAt: undefined, scheduleStatus: 'none' })
        console.log(`Plan ${planId} scheduled time has passed, cleared schedule`)
      }
    }

    return updatedPlan
  }

  // 开始执行计划
  async function startPlanExecution(planId: string): Promise<Plan> {
    return updatePlan(planId, {
      status: 'executing',
      executionStatus: 'running'
    })
  }

  // 暂停计划执行
  async function pausePlanExecution(planId: string): Promise<Plan> {
    return updatePlan(planId, { executionStatus: 'paused' })
  }

  // 恢复计划执行
  async function resumePlanExecution(planId: string): Promise<Plan> {
    return updatePlan(planId, { executionStatus: 'running' })
  }

  // 完成计划执行
  async function completePlanExecution(planId: string): Promise<Plan> {
    return updatePlan(planId, {
      status: 'completed',
      executionStatus: 'completed'
    })
  }

  // 设置当前执行的任务
  async function setCurrentTask(planId: string, taskId: string | undefined): Promise<Plan> {
    return updatePlan(planId, { currentTaskId: taskId })
  }

  async function deletePlan(id: string): Promise<void> {
    const notificationStore = useNotificationStore()

    try {
      // 先清理前端 store 中的相关数据
      // 1. 清理任务执行状态
      const taskExecutionStore = useTaskExecutionStore()
      taskExecutionStore.clearPlanExecution(id)

      // 2. 清理任务列表中该计划的任务
      const taskStore = useTaskStore()
      taskStore.clearPlanTasks(id)

      // 3. 清理任务拆分会话
      const taskSplitStore = useTaskSplitStore()
      taskSplitStore.clearPlanSplitSessions(id)

      // 4. 调用后端删除计划（会级联删除 tasks、task_execution_logs、task_split_sessions）
      await invoke('delete_plan', { id })

      // 5. 从本地列表中移除
      const index = plans.value.findIndex(p => p.id === id)
      if (index !== -1) {
        plans.value.splice(index, 1)
      }

      // 6. 如果是当前计划，清除当前计划 ID
      if (currentPlanId.value === id) {
        currentPlanId.value = null
      }
    } catch (error) {
      console.error('Failed to delete plan:', error)
      notificationStore.databaseError(
        '删除计划失败',
        getErrorMessage(error),
        async () => { await deletePlan(id) }
      )
      throw error
    }
  }

  function setCurrentPlan(id: string | null) {
    currentPlanId.value = id
  }

  function clearProjectPlans(projectId: string): string[] {
    const clearedPlanIds = plans.value
      .filter(plan => plan.projectId === projectId)
      .map(plan => plan.id)

    if (clearedPlanIds.length === 0) {
      return []
    }

    const clearedPlanIdSet = new Set(clearedPlanIds)
    plans.value = plans.value.filter(plan => !clearedPlanIdSet.has(plan.id))

    if (currentPlanId.value && clearedPlanIdSet.has(currentPlanId.value)) {
      currentPlanId.value = null
    }

    return clearedPlanIds
  }

  // 根据状态分组获取计划
  function getPlansByStatus(status: PlanStatus): Plan[] {
    return plans.value
      .filter(p => p.status === status)
      .sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())
  }

  // 打开任务拆分对话框
  function openSplitDialog(context: PlanSplitDialogContext) {
    splitDialogContexts.value.set(context.planId, context)
    activeSplitPlanId.value = context.planId
    splitDialogVisible.value = true
  }

  function switchSplitDialogTab(planId: string) {
    if (splitDialogContexts.value.has(planId)) {
      activeSplitPlanId.value = planId
    }
  }

  function closeSplitDialogTab(planId: string) {
    splitDialogContexts.value.delete(planId)
    if (activeSplitPlanId.value === planId) {
      const remaining = Array.from(splitDialogContexts.value.keys())
      activeSplitPlanId.value = remaining.length > 0 ? remaining[remaining.length - 1] : null
    }
    if (splitDialogContexts.value.size === 0) {
      splitDialogVisible.value = false
    }
  }

  function closeSplitDialog() {
    splitDialogVisible.value = false
    activeSplitPlanId.value = null
    splitDialogContexts.value.clear()
  }

  // 取消计划定时
  async function cancelPlanSchedule(planId: string): Promise<Plan> {
    const notificationStore = useNotificationStore()
    try {
      const rustPlan = await invoke<RustPlan>('cancel_plan_schedule', { id: planId })
      const updatedPlan = transformPlan(rustPlan)

      const index = plans.value.findIndex(p => p.id === planId)
      if (index !== -1) {
        plans.value[index] = updatedPlan
      }

      return updatedPlan
    } catch (error) {
      console.error('Failed to cancel plan schedule:', error)
      notificationStore.databaseError(
        '取消计划定时失败',
        getErrorMessage(error),
        async () => { await cancelPlanSchedule(planId) }
      )
      throw error
    }
  }

  return {
    plans,
    currentPlanId,
    isLoading,
    loadError,
    splitDialogVisible,
    splitDialogContexts,
    splitDialogContext,
    activeSplitPlanId,
    splitDialogPlanIds,
    currentPlan,
    plansByProject,
    plansByStatus,
    loadPlans,
    getPlan,
    createPlan,
    updatePlan,
    deletePlan,
    clearProjectPlans,
    setCurrentPlan,
    getPlansByStatus,
    markPlanAsReady,
    startPlanExecution,
    pausePlanExecution,
    resumePlanExecution,
    completePlanExecution,
    setCurrentTask,
    openSplitDialog,
    closeSplitDialog,
    switchSplitDialogTab,
    closeSplitDialogTab,
    cancelPlanSchedule
  }
})
