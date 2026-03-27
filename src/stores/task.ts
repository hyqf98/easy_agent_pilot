import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { getErrorMessage } from '@/utils/api'
import type {
  Task,
  TaskStatus,
  TaskPriority,
  CreateTaskInput,
  UpdateTaskInput,
  AgentRole,
  TaskOrderItem
} from '@/types/plan'

// Rust 后端返回的 snake_case 结构
interface RustTask {
  id: string
  plan_id: string
  parent_id?: string
  title: string
  description?: string
  status: string
  priority: string
  assignee?: string
  agent_id?: string
  model_id?: string
  session_id?: string
  progress_file?: string
  dependencies?: string | string[] // JSON 字符串或数组
  task_order?: number
  order?: number
  retry_count: number
  max_retries: number
  error_message?: string
  implementation_steps?: string | string[] // JSON 字符串或数组
  test_steps?: string | string[] // JSON 字符串或数组
  acceptance_criteria?: string | string[] // JSON 字符串或数组
  block_reason?: string
  input_request?: string | Record<string, unknown> // JSON 字符串或对象
  input_response?: string | Record<string, unknown> // JSON 字符串或对象
  created_at: string
  updated_at: string
}

// 将 Rust 返回的 snake_case 转换为 camelCase
function transformTask(rustTask: RustTask): Task {
  const parseStringArray = (value?: string | string[] | null): string[] | undefined => {
    if (Array.isArray(value)) {
      return value
    }
    if (typeof value === 'string') {
      try {
        const parsed = JSON.parse(value)
        if (Array.isArray(parsed)) return parsed
      } catch {
        // ignore parse error
      }
    }
    return undefined
  }

  const order = typeof rustTask.order === 'number'
    ? rustTask.order
    : (typeof rustTask.task_order === 'number' ? rustTask.task_order : 0)

  const dependencies = parseStringArray(rustTask.dependencies)
  const implementationSteps = parseStringArray(rustTask.implementation_steps)
  const testSteps = parseStringArray(rustTask.test_steps)
  const acceptanceCriteria = parseStringArray(rustTask.acceptance_criteria)

  // 解析 input_request 和 input_response
  const parseJsonValue = (
    value?: string | Record<string, unknown> | null
  ): Record<string, unknown> | undefined => {
    if (!value) return undefined
    if (typeof value === 'object') {
      return value
    }
    try {
      const parsed = JSON.parse(value)
      if (typeof parsed === 'object' && parsed !== null) return parsed
    } catch {
      // ignore parse error
    }
    return undefined
  }

  const inputRequest = parseJsonValue(rustTask.input_request)
  const inputResponse = parseJsonValue(rustTask.input_response)

  return {
    id: rustTask.id,
    planId: rustTask.plan_id,
    parentId: rustTask.parent_id,
    title: rustTask.title,
    description: rustTask.description,
    status: rustTask.status as TaskStatus,
    priority: rustTask.priority as TaskPriority,
    assignee: rustTask.assignee as AgentRole | undefined,
    agentId: rustTask.agent_id,
    modelId: rustTask.model_id,
    sessionId: rustTask.session_id,
    progressFile: rustTask.progress_file,
    dependencies,
    order,
    retryCount: rustTask.retry_count,
    maxRetries: rustTask.max_retries,
    errorMessage: rustTask.error_message,
    implementationSteps,
    testSteps,
    acceptanceCriteria,
    blockReason: rustTask.block_reason as 'waiting_input' | undefined,
    inputRequest: inputRequest as any,
    inputResponse,
    createdAt: rustTask.created_at,
    updatedAt: rustTask.updated_at
  }
}

function collectTaskAndDescendantIds(taskId: string, taskList: Task[]): Set<string> {
  const ids = new Set<string>([taskId])
  let changed = true

  while (changed) {
    changed = false
    for (const task of taskList) {
      if (task.parentId && ids.has(task.parentId) && !ids.has(task.id)) {
        ids.add(task.id)
        changed = true
      }
    }
  }

  return ids
}

export const useTaskStore = defineStore('task', () => {
  // State
  const tasks = ref<Task[]>([])
  const currentTaskId = ref<string | null>(null)
  const isLoading = ref(false)
  const loadError = ref<string | null>(null)
  // 拖拽状态 - 全局共享
  const draggingTaskId = ref<string | null>(null)

  // Getters
  const currentTask = computed(() =>
    tasks.value.find(t => t.id === currentTaskId.value)
  )

  // 按计划分组的任务
  const tasksByPlan = computed(() => {
    return (planId: string) => {
      return tasks.value
        .filter(t => t.planId === planId)
        .sort((a, b) => a.order - b.order)
    }
  })

  // 按状态分组的任务（看板视图）
  const tasksByStatus = computed(() => {
    return (planId: string) => {
      const result: Record<TaskStatus, Task[]> = {
        pending: [],
        in_progress: [],
        completed: [],
        blocked: [],
        failed: [],
        cancelled: []
      }

      tasks.value
        .filter(t => t.planId === planId)
        .forEach(t => {
          if (result[t.status]) {
            result[t.status].push(t)
          }
        })

      // 每个分组内按顺序排序
      Object.keys(result).forEach(status => {
        result[status as TaskStatus].sort((a, b) => a.order - b.order)
      })

      return result
    }
  })

  // 获取根任务（没有父任务的任务）
  const rootTasks = computed(() => {
    return (planId: string) => {
      return tasks.value
        .filter(t => t.planId === planId && !t.parentId)
        .sort((a, b) => a.order - b.order)
    }
  })

  // 获取子任务
  const subtasks = computed(() => {
    return (parentId: string) => {
      return tasks.value
        .filter(t => t.parentId === parentId)
        .sort((a, b) => a.order - b.order)
    }
  })

  function upsertTask(task: Task): void {
    const index = tasks.value.findIndex(item => item.id === task.id)
    if (index === -1) {
      tasks.value.push(task)
      sortTasks()
      return
    }

    tasks.value[index] = task
  }

  function replacePlanTasks(planId: string, nextTasks: Task[]): void {
    tasks.value = [
      ...tasks.value.filter(task => task.planId !== planId),
      ...nextTasks
    ]
    sortTasks()

    if (currentTaskId.value && !tasks.value.some(task => task.id === currentTaskId.value)) {
      currentTaskId.value = null
    }

    if (editingTask.value && !tasks.value.some(task => task.id === editingTask.value?.id)) {
      closeEditDialog()
    }
  }

  // Actions
  async function loadTasks(planId: string) {
    isLoading.value = true
    loadError.value = null
    const notificationStore = useNotificationStore()
    try {
      const rustTasks = await invoke<RustTask[]>('list_tasks', { planId })
      replacePlanTasks(planId, rustTasks.map(transformTask))
    } catch (error) {
      console.error('Failed to load tasks:', error)
      loadError.value = getErrorMessage(error)
      notificationStore.networkError(
        '加载任务列表',
        getErrorMessage(error),
        () => loadTasks(planId)
      )
    } finally {
      isLoading.value = false
    }
  }

  async function getTask(id: string): Promise<Task | null> {
    try {
      const rustTask = await invoke<RustTask>('get_task', { id })
      return transformTask(rustTask)
    } catch (error) {
      console.error('Failed to get task:', error)
      return null
    }
  }

  // 根据会话 ID 获取关联的任务
  async function getTaskBySessionId(sessionId: string): Promise<Task | null> {
    try {
      const rustTask = await invoke<RustTask | null>('get_task_by_session_id', { sessionId })
      if (rustTask) {
        const task = transformTask(rustTask)
        // 合并到本地状态
        upsertTask(task)
        return task
      }
      return null
    } catch (error) {
      console.error('Failed to get task by session id:', error)
      return null
    }
  }

  async function createTask(input: CreateTaskInput): Promise<Task> {
    const notificationStore = useNotificationStore()
    const rustInput = {
      plan_id: input.planId,
      parent_id: input.parentId ?? null,
      title: input.title,
      description: input.description ?? null,
      priority: input.priority ?? null,
      assignee: input.assignee ?? null,
      agent_id: input.agentId ?? null,
      model_id: input.modelId ?? null,
      dependencies: input.dependencies ?? null,
      order: input.order ?? null,
      max_retries: input.maxRetries ?? null,
      implementation_steps: input.implementationSteps ?? null,
      test_steps: input.testSteps ?? null,
      acceptance_criteria: input.acceptanceCriteria ?? null
    }

    try {
      const rustTask = await invoke<RustTask>('create_task', { input: rustInput })
      const newTask = transformTask(rustTask)
      upsertTask(newTask)
      return newTask
    } catch (error) {
      console.error('Failed to create task:', error)
      notificationStore.databaseError(
        '创建任务失败',
        getErrorMessage(error),
        async () => { await createTask(input) }
      )
      throw error
    }
  }

  async function updateTask(id: string, updates: UpdateTaskInput): Promise<Task> {
    const notificationStore = useNotificationStore()
    const input: Record<string, unknown> = {}

    if ('title' in updates) input.title = updates.title ?? null
    if ('description' in updates) input.description = updates.description ?? null
    if ('status' in updates) input.status = updates.status ?? null
    if ('priority' in updates) input.priority = updates.priority ?? null
    if ('assignee' in updates) input.assignee = updates.assignee ?? null
    if ('agentId' in updates) input.agent_id = updates.agentId ?? null
    if ('modelId' in updates) input.model_id = updates.modelId ?? null
    if ('sessionId' in updates) input.session_id = updates.sessionId ?? null
    if ('progressFile' in updates) input.progress_file = updates.progressFile ?? null
    if ('dependencies' in updates) input.dependencies = updates.dependencies ?? null
    if ('order' in updates) input.order = updates.order ?? null
    if ('retryCount' in updates) input.retry_count = updates.retryCount ?? null
    if ('maxRetries' in updates) input.max_retries = updates.maxRetries ?? null
    if ('errorMessage' in updates) input.error_message = updates.errorMessage ?? null
    if ('implementationSteps' in updates) input.implementation_steps = updates.implementationSteps ?? null
    if ('testSteps' in updates) input.test_steps = updates.testSteps ?? null
    if ('acceptanceCriteria' in updates) input.acceptance_criteria = updates.acceptanceCriteria ?? null
    if ('blockReason' in updates) input.block_reason = updates.blockReason ?? null
    if ('inputRequest' in updates) input.input_request = updates.inputRequest ?? null
    if ('inputResponse' in updates) input.input_response = updates.inputResponse ?? null

    try {
      const rustTask = await invoke<RustTask>('update_task', { id, input })
      const updatedTask = transformTask(rustTask)
      upsertTask(updatedTask)
      return updatedTask
    } catch (error) {
      console.error('Failed to update task:', error)
      notificationStore.databaseError(
        '更新任务失败',
        getErrorMessage(error),
        async () => { await updateTask(id, updates) }
      )
      throw error
    }
  }

  async function deleteTask(id: string): Promise<void> {
    const notificationStore = useNotificationStore()

    const previousTasks = [...tasks.value]
    const previousCurrentTaskId = currentTaskId.value
    const previousEditDialogVisible = editDialogVisible.value
    const previousEditingTask = editingTask.value ? { ...editingTask.value } : null

    const deletedIds = collectTaskAndDescendantIds(id, previousTasks)

    // 乐观更新：立即从本地移除任务及其子任务，并同步剔除其它任务上的已删除依赖
    tasks.value = tasks.value
      .filter(task => !deletedIds.has(task.id))
      .map(task => {
        if (!task.dependencies?.length) {
          return task
        }

        const nextDependencies = task.dependencies.filter(dependencyId => !deletedIds.has(dependencyId))
        if (nextDependencies.length === task.dependencies.length) {
          return task
        }

        return {
          ...task,
          dependencies: nextDependencies
        }
      })

    if (currentTaskId.value && deletedIds.has(currentTaskId.value)) {
      currentTaskId.value = null
    }

    if (editingTask.value && deletedIds.has(editingTask.value.id)) {
      closeEditDialog()
    }

    try {
      await invoke('delete_task', { id })
    } catch (error) {
      // 回滚本地状态
      tasks.value = previousTasks
      currentTaskId.value = previousCurrentTaskId
      editDialogVisible.value = previousEditDialogVisible
      editingTask.value = previousEditingTask

      console.error('Failed to delete task:', error)
      notificationStore.databaseError(
        '删除任务失败',
        getErrorMessage(error),
        async () => { await deleteTask(id) }
      )
      throw error
    }
  }

  // 批量更新任务顺序
  async function reorderTasks(taskOrders: TaskOrderItem[]): Promise<void> {
    const notificationStore = useNotificationStore()

    try {
      await invoke('reorder_tasks', {
        input: { task_orders: taskOrders }
      })

      // 更新本地状态
      taskOrders.forEach(item => {
        const task = tasks.value.find(t => t.id === item.id)
        if (task) {
          task.order = item.order
        }
      })

      // 重新排序
      sortTasks()
    } catch (error) {
      console.error('Failed to reorder tasks:', error)
      notificationStore.databaseError(
        '更新任务顺序失败',
        getErrorMessage(error),
        async () => { await reorderTasks(taskOrders) }
      )
      throw error
    }
  }

  // 获取子任务
  async function loadSubtasks(parentId: string): Promise<Task[]> {
    try {
      const rustTasks = await invoke<RustTask[]>('list_subtasks', { parentId })
      const subtasks = rustTasks.map(transformTask)

      // 合并到本地状态
      subtasks.forEach(st => {
        upsertTask(st)
      })

      return subtasks
    } catch (error) {
      console.error('Failed to load subtasks:', error)
      return []
    }
  }

  function setCurrentTask(id: string | null) {
    currentTaskId.value = id
  }

  // 编辑对话框状态
  const editDialogVisible = ref(false)
  const editingTask = ref<Task | null>(null)

  // 打开编辑对话框
  function openEditDialog(task: Task) {
    editingTask.value = { ...task }
    editDialogVisible.value = true
  }

  // 关闭编辑对话框
  function closeEditDialog() {
    editDialogVisible.value = false
    editingTask.value = null
  }

  // 内部排序函数
  function sortTasks() {
    tasks.value.sort((a, b) => a.order - b.order)
  }

  // 检查任务的依赖是否都已完成
  function areDependenciesMet(taskId: string): boolean {
    const task = tasks.value.find(t => t.id === taskId)
    if (!task?.dependencies || task.dependencies.length === 0) {
      return true
    }

    return task.dependencies.every(depId => {
      const depTask = tasks.value.find(t => t.id === depId)
      return depTask?.status === 'completed'
    })
  }

  // 获取未满足依赖的任务标题列表
  function getDependencyTitles(taskId: string): string[] {
    const task = tasks.value.find(t => t.id === taskId)
    if (!task?.dependencies || task.dependencies.length === 0) {
      return []
    }

    return task.dependencies
      .map(depId => {
        const depTask = tasks.value.find(t => t.id === depId)
        return depTask?.title ?? depId
      })
      .filter(Boolean) as string[]
  }

  // 获取未完成的依赖数量
  function getUnmetDependenciesCount(taskId: string): number {
    const task = tasks.value.find(t => t.id === taskId)
    if (!task?.dependencies || task.dependencies.length === 0) {
      return 0
    }

    return task.dependencies.filter(depId => {
      const depTask = tasks.value.find(t => t.id === depId)
      return depTask?.status !== 'completed'
    }).length
  }

  // 获取可以开始的任务（依赖已满足且状态为 pending）
  function getReadyTasks(planId: string): Task[] {
    return tasks.value
      .filter(t => t.planId === planId && t.status === 'pending')
      .filter(t => areDependenciesMet(t.id))
      .sort((a, b) => a.order - b.order)
  }

  // 批量启动待办任务
  async function batchStartTasks(planId: string): Promise<Task[]> {
    const notificationStore = useNotificationStore()

    try {
      const rustTasks = await invoke<RustTask[]>('batch_update_status', {
        planId,
        status: 'in_progress'
      })

      const updatedTasks = rustTasks.map(transformTask)

      // 更新本地状态
      updatedTasks.forEach(updatedTask => {
        upsertTask(updatedTask)
      })

      return updatedTasks
    } catch (error) {
      console.error('Failed to batch start tasks:', error)
      notificationStore.databaseError(
        '批量启动任务失败',
        getErrorMessage(error),
        async () => { await batchStartTasks(planId) }
      )
      throw error
    }
  }

  // 重试失败任务
  async function retryTask(taskId: string): Promise<Task> {
    const notificationStore = useNotificationStore()

    try {
      const rustTask = await invoke<RustTask>('retry_task', { id: taskId })
      const updatedTask = transformTask(rustTask)
      upsertTask(updatedTask)
      return updatedTask
    } catch (error) {
      console.error('Failed to retry task:', error)
      notificationStore.databaseError(
        '重试任务失败',
        getErrorMessage(error),
        async () => { await retryTask(taskId) }
      )
      throw error
    }
  }

  // 停止执行中任务
  async function stopTask(taskId: string): Promise<Task> {
    const notificationStore = useNotificationStore()

    try {
      const rustTask = await invoke<RustTask>('stop_task', { id: taskId })
      const updatedTask = transformTask(rustTask)
      upsertTask(updatedTask)
      return updatedTask
    } catch (error) {
      console.error('Failed to stop task:', error)
      notificationStore.databaseError(
        '停止任务失败',
        getErrorMessage(error),
        async () => { await stopTask(taskId) }
      )
      throw error
    }
  }

  // 从拆分结果创建任务
  async function createTasksFromSplit(planId: string, taskInputs: CreateTaskInput[]): Promise<Task[]> {
    const notificationStore = useNotificationStore()

    // 先创建所有任务（此时 dependencies 可能存储的是任务标题）
    // 转换输入格式 - Rust后端期望Vec<String>而非JSON字符串
    const rustInputs = taskInputs.map(input => ({
      plan_id: planId,
      parent_id: input.parentId ?? null,
      title: input.title,
      description: input.description ?? null,
      priority: input.priority ?? null,
      assignee: input.assignee ?? null,
      agent_id: input.agentId ?? null,
      model_id: input.modelId ?? null,
      dependencies: null, // 先不设置依赖，创建后再更新
      order: input.order ?? null,
      max_retries: input.maxRetries ?? null,
      implementation_steps: input.implementationSteps ?? null,
      test_steps: input.testSteps ?? null,
      acceptance_criteria: input.acceptanceCriteria ?? null
    }))

    try {
      const rustTasks = await invoke<RustTask[]>('batch_create_tasks', {
        planId,
        tasks: rustInputs
      })

      const newTasks = rustTasks.map(transformTask)

      // 建立标题到 ID 的映射
      const titleToId = new Map<string, string>()
      newTasks.forEach((task, index) => {
        titleToId.set(taskInputs[index].title, task.id)
      })

      // 转换依赖关系：将 dependsOn 中的标题转换为任务 ID
      const dependencyUpdates: Promise<void>[] = []
      for (let i = 0; i < taskInputs.length; i++) {
        const input = taskInputs[i]
        const task = newTasks[i]
        // 检查是否有 dependsOn 字段（来自 AI 拆分结果）
        const dependsOn = (input as any).dependsOn as string[] | undefined
        if (dependsOn?.length) {
          const dependencyIds = dependsOn
            .map(title => titleToId.get(title))
            .filter(Boolean) as string[]

          if (dependencyIds.length > 0) {
            dependencyUpdates.push(
              updateTask(task.id, { dependencies: dependencyIds }).then(() => {
                task.dependencies = dependencyIds
              })
            )
          }
        }
      }

      // 等待所有依赖更新完成
      await Promise.all(dependencyUpdates)

      // 添加到本地状态
      newTasks.forEach(task => {
        upsertTask(task)
      })

      return newTasks
    } catch (error) {
      console.error('Failed to create tasks from split:', error)
      notificationStore.databaseError(
        '创建任务失败',
        getErrorMessage(error),
        async () => { await createTasksFromSplit(planId, taskInputs) }
      )
      throw error
    }
  }

  /**
   * 清理指定计划的所有任务（本地缓存）
   * 在删除计划时调用
   */
  function clearPlanTasks(planId: string): void {
    // 过滤掉该计划的所有任务
    const taskIdsToKeep = tasks.value
      .filter(t => t.planId !== planId)
    tasks.value = taskIdsToKeep

    // 如果当前任务属于该计划，清除当前任务
    if (currentTaskId.value) {
      const currentTask = tasks.value.find(t => t.id === currentTaskId.value)
      if (!currentTask || currentTask.planId === planId) {
        currentTaskId.value = null
      }
    }
  }

  function clearPlansTasks(planIds: string[]): void {
    if (planIds.length === 0) {
      return
    }

    const planIdSet = new Set(planIds)
    tasks.value = tasks.value.filter(task => !planIdSet.has(task.planId))

    if (currentTaskId.value) {
      const currentTask = tasks.value.find(task => task.id === currentTaskId.value)
      if (!currentTask) {
        currentTaskId.value = null
      }
    }
  }

  return {
    // State
    tasks,
    currentTaskId,
    isLoading,
    loadError,
    draggingTaskId,
    // Getters
    currentTask,
    tasksByPlan,
    tasksByStatus,
    rootTasks,
    subtasks,
    // Actions
    loadTasks,
    getTask,
    getTaskBySessionId,
    createTask,
    updateTask,
    deleteTask,
    reorderTasks,
    loadSubtasks,
    setCurrentTask,
    areDependenciesMet,
    getDependencyTitles,
    getUnmetDependenciesCount,
    getReadyTasks,
    // 批量操作
    batchStartTasks,
    retryTask,
    stopTask,
    createTasksFromSplit,
    // 清理操作
    clearPlanTasks,
    clearPlansTasks,
    // 编辑对话框
    editDialogVisible,
    editingTask,
    openEditDialog,
    closeEditDialog
  }
})
