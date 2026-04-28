import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { inferAgentProvider, useAgentStore } from '@/stores/agent'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { useNotificationStore } from '@/stores/notification'
import { usePlanStore } from '@/stores/plan'
import { useTaskStore } from '@/stores/task'
import { checkCircularDependency, getAvailableDependencies } from '@/composables'
import { useSafeOutsideClick } from '@/composables/useSafeOutsideClick'
import { resolveExpertById, resolveExpertRuntime } from '@/services/agentTeams/runtime'
import type { Task, TaskPriority } from '@/types/plan'
import { formatAgentModelLabel } from '@/utils/agentModelLabel'
import { getErrorMessage } from '@/utils/api'

export interface TaskEditModalProps {
  visible: boolean
  task: Task
  mode?: 'edit' | 'create'
}

export interface TaskEditModalEmits {
  (event: 'update:visible', value: boolean): void
  (event: 'saved', taskData: Partial<Task>): void
}

interface TaskEditFormState {
  title: string
  description: string
  priority: TaskPriority
  expertId?: string
  agentId?: string
  modelId: string
  memoryLibraryIds: string[]
  implementationSteps: string[]
  testSteps: string[]
  acceptanceCriteria: string[]
  dependencies: string[]
  maxRetries: number
}

type StepField = 'implementationSteps' | 'testSteps' | 'acceptanceCriteria'

const priorityOptions: Array<{ label: string; value: TaskPriority }> = [
  { label: '低', value: 'low' },
  { label: '中', value: 'medium' },
  { label: '高', value: 'high' }
]

/**
 * 任务编辑弹窗状态。
 * 负责表单初始化、专家/模型联动、依赖选择和创建/更新提交流程。
 */
export function useTaskEditModal(props: TaskEditModalProps, emit: TaskEditModalEmits) {
  const taskStore = useTaskStore()
  const agentStore = useAgentStore()
  const agentConfigStore = useAgentConfigStore()
  const agentTeamsStore = useAgentTeamsStore()
  const planStore = usePlanStore()
  const notificationStore = useNotificationStore()
  const { t } = useI18n()

  const isCreateMode = computed(() => props.mode === 'create')
  const isSaving = ref(false)

  const expandedSections = ref({
    details: true
  })

  const isDepDropdownOpen = ref(false)
  const depDropdownRef = ref<HTMLElement | null>(null)
  const formScrollRef = ref<HTMLElement | null>(null)
  const depDropdownDirection = ref<'up' | 'down'>('down')
  const depDropdownMaxHeight = ref(220)

  const form = ref<TaskEditFormState>(buildFormState(props.task, planStore))

  const expertOptions = computed(() => agentTeamsStore.enabledExperts)

  const currentPlan = computed(() =>
    planStore.plans.find(plan => plan.id === props.task.planId) || null
  )

  const modelOptions = computed(() => {
    const runtime = resolveCurrentRuntime()
    if (!runtime?.agent.id) return []

    return agentConfigStore
      .getModelsConfigs(runtime.agent.id)
      .filter(config => config.enabled)
      .map(config => ({
        value: config.modelId,
        label: formatAgentModelLabel({
          provider: runtime.agent.provider || inferAgentProvider(runtime.agent),
          modelId: config.modelId,
          displayName: config.displayName
        })
      }))
  })

  const executionConfigHint = computed(() => {
    if (!currentPlan.value?.splitExpertId) {
      return '当前计划未配置默认执行专家，可直接为任务单独选择。'
    }

    const expertName = agentTeamsStore.getExpertById(currentPlan.value.splitExpertId)?.name || currentPlan.value.splitExpertId
    const modelLabel = currentPlan.value.splitModelId ? ` / ${currentPlan.value.splitModelId}` : ''
    return `默认来源于计划配置：${expertName}${modelLabel}`
  })

  const availableTasks = computed(() =>
    getAvailableDependencies(props.task.id, props.task.planId, taskStore.tasks)
  )

  const dependencyOptions = computed(() =>
    availableTasks.value.map(task => ({
      label: task.title,
      value: task.id
    }))
  )

  const dependencyLabelMap = computed(() =>
    new Map(dependencyOptions.value.map(option => [option.value, option.label]))
  )

  const selectedDependencies = computed(() =>
    form.value.dependencies
      .map(value => ({
        value,
        label: dependencyLabelMap.value.get(value) || ''
      }))
      .filter(option => option.label)
  )

  function resolveCurrentRuntime() {
    const expert = resolveExpertById(form.value.expertId, agentTeamsStore.experts)
    return resolveExpertRuntime(expert, agentStore.agents, form.value.modelId)
  }

  function toggleSection(section: 'details') {
    expandedSections.value[section] = !expandedSections.value[section]
  }

  function toggleDepDropdown() {
    isDepDropdownOpen.value = !isDepDropdownOpen.value
    if (!isDepDropdownOpen.value) return

    void nextTick(() => {
      updateDependencyDropdownLayout()
    })
  }

  function updateDependencyDropdownLayout() {
    const selectorRect = depDropdownRef.value?.getBoundingClientRect()
    const scrollRect = formScrollRef.value?.getBoundingClientRect()

    if (!selectorRect || !scrollRect) return

    const safeGap = 12
    const preferredHeight = 240
    const spaceBelow = Math.max(0, scrollRect.bottom - selectorRect.bottom - safeGap)
    const spaceAbove = Math.max(0, selectorRect.top - scrollRect.top - safeGap)
    const shouldOpenUp = spaceBelow < 180 && spaceAbove > spaceBelow
    const availableSpace = shouldOpenUp ? spaceAbove : spaceBelow

    depDropdownDirection.value = shouldOpenUp ? 'up' : 'down'
    depDropdownMaxHeight.value = Math.max(120, Math.min(preferredHeight, availableSpace))
  }

  function handleDependencyToggle(taskId: string) {
    const isSelected = form.value.dependencies.includes(taskId)

    if (isSelected) {
      removeDependency(taskId)
      return
    }

    if (checkCircularDependency(props.task.id, taskId, taskStore.tasks)) {
      notificationStore.error(t('task.circularDependencyError'), t('task.circularDependencyError'))
      return
    }

    form.value.dependencies.push(taskId)
  }

  function isDependencySelected(taskId: string) {
    return form.value.dependencies.includes(taskId)
  }

  function removeDependency(taskId: string) {
    const index = form.value.dependencies.indexOf(taskId)
    if (index > -1) {
      form.value.dependencies.splice(index, 1)
    }
  }

  function addStep(type: StepField) {
    form.value[type].push('')
  }

  function removeStep(type: StepField, index: number) {
    form.value[type].splice(index, 1)
  }

  function close() {
    emit('update:visible', false)
  }

  async function handleSave() {
    if (!form.value.title.trim()) {
      notificationStore.error('保存失败', '请输入任务标题')
      return
    }

    const runtime = resolveCurrentRuntime()
    const taskData: Partial<Task> = {
      title: sanitizeText(form.value.title),
      description: sanitizeText(form.value.description),
      priority: form.value.priority,
      expertId: form.value.expertId,
      agentId: runtime?.agent.id || form.value.agentId,
      modelId: runtime?.modelId || undefined,
      memoryLibraryIds: [...form.value.memoryLibraryIds],
      implementationSteps: form.value.implementationSteps.map(sanitizeText).filter(step => step.trim()),
      testSteps: form.value.testSteps.map(sanitizeText).filter(step => step.trim()),
      acceptanceCriteria: form.value.acceptanceCriteria.map(sanitizeText).filter(step => step.trim()),
      dependencies: form.value.dependencies,
      maxRetries: form.value.maxRetries
    }

    try {
      isSaving.value = true

      if (isCreateMode.value) {
        emit('saved', taskData)
        close()
        return
      }

      await taskStore.updateTask(props.task.id, taskData)
      notificationStore.success('保存成功', '任务已更新')
      emit('saved', taskData)
      close()
    } catch (error) {
      console.error('Failed to save task:', error)
      notificationStore.error('保存失败', getErrorMessage(error))
    } finally {
      isSaving.value = false
    }
  }

  watch(
    () => props.task,
    newTask => {
      form.value = buildFormState(newTask, planStore)
    },
    { immediate: true }
  )

  watch(
    () => currentPlan.value,
    plan => {
      if (!plan) return
      if (!form.value.expertId && plan.splitExpertId) {
        form.value.expertId = plan.splitExpertId
      }
      if ((form.value.modelId === undefined || form.value.modelId === null) && plan.splitModelId) {
        form.value.modelId = plan.splitModelId
      }
    },
    { immediate: true }
  )

  watch(
    () => [form.value.expertId, agentTeamsStore.experts.length] as const,
    async () => {
      const runtime = resolveCurrentRuntime()
      if (!runtime) {
        form.value.agentId = undefined
        form.value.modelId = ''
        return
      }

      form.value.agentId = runtime.agent.id
      const provider = inferAgentProvider(agentStore.agents.find(agent => agent.id === runtime.agent.id))
      const modelConfigs = await agentConfigStore.ensureModelsConfigs(runtime.agent.id, provider)
      const availableModels = modelConfigs.filter(config => config.enabled)

      if (availableModels.length === 0) {
        form.value.modelId = ''
        return
      }

      const preferredModelId = form.value.modelId ?? runtime.modelId ?? ''

      if (preferredModelId === '') {
        form.value.modelId = ''
        return
      }

      if (availableModels.some(model => model.modelId === preferredModelId)) {
        form.value.modelId = preferredModelId
        return
      }

      form.value.modelId = ''
    },
    { immediate: true }
  )

  useSafeOutsideClick(
    () => [depDropdownRef.value],
    () => {
      isDepDropdownOpen.value = false
    }
  )

  onMounted(() => {
    void Promise.all([
      agentStore.agents.length === 0 ? agentStore.loadAgents() : Promise.resolve(agentStore.agents),
      agentTeamsStore.experts.length === 0
        ? agentTeamsStore.loadExperts(true)
        : Promise.resolve(agentTeamsStore.experts)
    ])
    window.addEventListener('resize', updateDependencyDropdownLayout)
  })

  onUnmounted(() => {
    window.removeEventListener('resize', updateDependencyDropdownLayout)
  })

  return {
    t,
    form,
    isCreateMode,
    isSaving,
    expandedSections,
    isDepDropdownOpen,
    depDropdownRef,
    formScrollRef,
    depDropdownDirection,
    depDropdownMaxHeight,
    priorityOptions,
    expertOptions,
    modelOptions,
    executionConfigHint,
    dependencyOptions,
    selectedDependencies,
    toggleSection,
    toggleDepDropdown,
    handleDependencyToggle,
    isDependencySelected,
    removeDependency,
    addStep,
    removeStep,
    handleSave,
    close
  }
}

function sanitizeText(value: string | null | undefined) {
  return Array.from((value || '').replace(/\r\n?/g, '\n'))
    .filter(character => {
      const code = character.charCodeAt(0)
      return code === 0x09 || code === 0x0a || code >= 0x20
    })
    .join('')
}

function sanitizeTextList(values: string[] | null | undefined) {
  return (values || []).map(item => sanitizeText(item))
}

function buildFormState(task: Task, planStore: ReturnType<typeof usePlanStore>): TaskEditFormState {
  const relatedPlan = planStore.plans.find(plan => plan.id === task.planId) || null

  return {
    title: sanitizeText(task.title),
    description: sanitizeText(task.description),
    priority: task.priority,
    expertId: task.expertId || relatedPlan?.splitExpertId || undefined,
    agentId: task.agentId || relatedPlan?.splitAgentId || undefined,
    modelId: task.modelId ?? relatedPlan?.splitModelId ?? '',
    memoryLibraryIds: [...(task.memoryLibraryIds ?? relatedPlan?.memoryLibraryIds ?? [])],
    implementationSteps: sanitizeTextList(task.implementationSteps),
    testSteps: sanitizeTextList(task.testSteps),
    acceptanceCriteria: sanitizeTextList(task.acceptanceCriteria),
    dependencies: [...(task.dependencies || [])],
    maxRetries: task.maxRetries ?? 3
  }
}
