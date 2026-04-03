import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getErrorMessage } from '@/utils/api'
import { useNotificationStore } from './notification'
import type {
  CreateSoloRunInput,
  SoloInputRequest,
  SoloRun,
  UpdateSoloRunInput
} from '@/types/solo'

interface RustSoloRun {
  id: string
  project_id: string
  execution_path: string
  name: string
  requirement: string
  goal: string
  participant_expert_ids_json?: string | null
  coordinator_expert_id?: string | null
  coordinator_agent_id?: string | null
  coordinator_model_id?: string | null
  max_dispatch_depth: number
  current_depth: number
  current_step_id?: string | null
  status: SoloRun['status']
  execution_status: SoloRun['executionStatus']
  last_error?: string | null
  input_request_json?: string | null
  input_response_json?: string | null
  created_at: string
  updated_at: string
  started_at?: string | null
  completed_at?: string | null
  stopped_at?: string | null
}

function parseJson<T>(raw?: string | null): T | undefined {
  if (!raw) return undefined
  try {
    return JSON.parse(raw) as T
  } catch {
    return undefined
  }
}

function transformRun(raw: RustSoloRun): SoloRun {
  return {
    id: raw.id,
    projectId: raw.project_id,
    executionPath: raw.execution_path,
    name: raw.name,
    requirement: raw.requirement,
    goal: raw.goal,
    participantExpertIds: parseJson<string[]>(raw.participant_expert_ids_json) ?? [],
    coordinatorExpertId: raw.coordinator_expert_id || undefined,
    coordinatorAgentId: raw.coordinator_agent_id || undefined,
    coordinatorModelId: raw.coordinator_model_id || undefined,
    maxDispatchDepth: raw.max_dispatch_depth,
    currentDepth: raw.current_depth,
    currentStepId: raw.current_step_id || undefined,
    status: raw.status,
    executionStatus: raw.execution_status,
    lastError: raw.last_error || undefined,
    inputRequest: parseJson<SoloInputRequest>(raw.input_request_json),
    inputResponse: parseJson<Record<string, unknown>>(raw.input_response_json),
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
    startedAt: raw.started_at || undefined,
    completedAt: raw.completed_at || undefined,
    stoppedAt: raw.stopped_at || undefined
  }
}

export const useSoloRunStore = defineStore('soloRun', () => {
  const runs = ref<SoloRun[]>([])
  const currentRunId = ref<string | null>(null)
  const isLoading = ref(false)

  const currentRun = computed(() => runs.value.find((run) => run.id === currentRunId.value) || null)

  function setCurrentRun(id: string | null) {
    currentRunId.value = id
  }

  function replaceProjectRuns(projectId: string, nextRuns: SoloRun[]) {
    runs.value = [
      ...runs.value.filter((run) => run.projectId !== projectId),
      ...nextRuns
    ]
    if (currentRunId.value && !runs.value.some((run) => run.id === currentRunId.value)) {
      currentRunId.value = nextRuns[0]?.id || null
    }
  }

  async function loadRuns(projectId: string): Promise<void> {
    const notificationStore = useNotificationStore()
    isLoading.value = true
    try {
      const rawRuns = await invoke<RustSoloRun[]>('list_solo_runs', { projectId })
      replaceProjectRuns(projectId, rawRuns.map(transformRun))
    } catch (error) {
      notificationStore.databaseError('加载 SOLO 运行失败', getErrorMessage(error))
      throw error
    } finally {
      isLoading.value = false
    }
  }

  async function getRun(id: string): Promise<SoloRun> {
    const rawRun = await invoke<RustSoloRun>('get_solo_run', { id })
    const run = transformRun(rawRun)
    const index = runs.value.findIndex((item) => item.id === id)
    if (index >= 0) {
      runs.value[index] = run
    } else {
      runs.value.unshift(run)
    }
    return run
  }

  async function createRun(input: CreateSoloRunInput): Promise<SoloRun> {
    const notificationStore = useNotificationStore()
    try {
      const rawRun = await invoke<RustSoloRun>('create_solo_run', {
        input: {
          project_id: input.projectId,
          execution_path: input.executionPath,
          name: input.name,
          requirement: input.requirement,
          goal: input.goal,
          participant_expert_ids_json: JSON.stringify(input.participantExpertIds ?? []),
          coordinator_expert_id: input.coordinatorExpertId ?? null,
          coordinator_agent_id: input.coordinatorAgentId ?? null,
          coordinator_model_id: input.coordinatorModelId ?? null,
          max_dispatch_depth: input.maxDispatchDepth
        }
      })
      const run = transformRun(rawRun)
      runs.value.unshift(run)
      currentRunId.value = run.id
      return run
    } catch (error) {
      notificationStore.databaseError('创建 SOLO 运行失败', getErrorMessage(error))
      throw error
    }
  }

  async function updateRun(id: string, updates: UpdateSoloRunInput): Promise<SoloRun> {
    const notificationStore = useNotificationStore()
    const input: Record<string, unknown> = {}

    if ('executionPath' in updates) input.execution_path = updates.executionPath ?? null
    if ('name' in updates) input.name = updates.name ?? null
    if ('requirement' in updates) input.requirement = updates.requirement ?? null
    if ('goal' in updates) input.goal = updates.goal ?? null
    if ('participantExpertIds' in updates) input.participant_expert_ids_json = updates.participantExpertIds ? JSON.stringify(updates.participantExpertIds) : null
    if ('coordinatorExpertId' in updates) input.coordinator_expert_id = updates.coordinatorExpertId ?? null
    if ('coordinatorAgentId' in updates) input.coordinator_agent_id = updates.coordinatorAgentId ?? null
    if ('coordinatorModelId' in updates) input.coordinator_model_id = updates.coordinatorModelId ?? null
    if ('maxDispatchDepth' in updates) input.max_dispatch_depth = updates.maxDispatchDepth ?? null
    if ('currentDepth' in updates) input.current_depth = updates.currentDepth ?? null
    if ('currentStepId' in updates) input.current_step_id = updates.currentStepId ?? null
    if ('status' in updates) input.status = updates.status ?? null
    if ('executionStatus' in updates) input.execution_status = updates.executionStatus ?? null
    if ('lastError' in updates) input.last_error = updates.lastError ?? null
    if ('inputRequest' in updates) input.input_request_json = updates.inputRequest ? JSON.stringify(updates.inputRequest) : null
    if ('inputResponse' in updates) input.input_response_json = updates.inputResponse ? JSON.stringify(updates.inputResponse) : null
    if ('startedAt' in updates) input.started_at = updates.startedAt ?? null
    if ('completedAt' in updates) input.completed_at = updates.completedAt ?? null
    if ('stoppedAt' in updates) input.stopped_at = updates.stoppedAt ?? null

    try {
      const rawRun = await invoke<RustSoloRun>('update_solo_run', { id, input })
      const run = transformRun(rawRun)
      const index = runs.value.findIndex((item) => item.id === id)
      if (index >= 0) {
        runs.value[index] = run
      } else {
        runs.value.unshift(run)
      }
      return run
    } catch (error) {
      notificationStore.databaseError('更新 SOLO 运行失败', getErrorMessage(error))
      throw error
    }
  }

  async function deleteRun(id: string): Promise<void> {
    const notificationStore = useNotificationStore()
    try {
      await invoke('delete_solo_run', { id })
      runs.value = runs.value.filter((run) => run.id !== id)
      if (currentRunId.value === id) {
        currentRunId.value = runs.value[0]?.id || null
      }
    } catch (error) {
      notificationStore.databaseError('删除 SOLO 运行失败', getErrorMessage(error))
      throw error
    }
  }

  async function clearRunProgress(id: string): Promise<void> {
    const notificationStore = useNotificationStore()
    try {
      await invoke('clear_solo_run_progress', { runId: id })
      await getRun(id)
    } catch (error) {
      notificationStore.databaseError('清除 SOLO 进度失败', getErrorMessage(error))
      throw error
    }
  }

  return {
    runs,
    currentRunId,
    currentRun,
    isLoading,
    setCurrentRun,
    loadRuns,
    getRun,
    createRun,
    updateRun,
    deleteRun,
    clearRunProgress
  }
})
