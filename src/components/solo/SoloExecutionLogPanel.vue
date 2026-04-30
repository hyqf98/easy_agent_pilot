<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import DynamicForm from '@/components/plan/dynamicForm/DynamicForm.vue'
import ExecutionTimeline from '@/components/message/ExecutionTimeline.vue'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { useAgentStore } from '@/stores/agent'
import { formatTokenCount } from '@/stores/token'
import { useSoloExecutionStore } from '@/stores/soloExecution'
import { useSoloRunStore } from '@/stores/soloRun'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import type { SoloExecutionStatus, SoloLogEntry, SoloRunStatus, SoloStep, SoloStepStatus } from '@/types/solo'
import type { TimelineEntry } from '@/types/timeline'
import { buildToolCallMapFromLogs } from '@/utils/toolCallLog'
import { DEFAULT_CONTEXT_WINDOW, resolveConfiguredContextWindow } from '@/utils/configuredModelContext'
import { resolveExpertById, resolveExpertRuntime } from '@/services/agentTeams/runtime'

const props = defineProps<{
  runId: string
  stepId?: string | null
}>()

interface SoloCliRetryState {
  current: number
  max: number
}

const soloExecutionStore = useSoloExecutionStore()
const soloRunStore = useSoloRunStore()
const agentTeamsStore = useAgentTeamsStore()
const agentStore = useAgentStore()
const agentConfigStore = useAgentConfigStore()

const scrollerRef = ref<HTMLElement | null>(null)
const autoScroll = ref(true)

const run = computed(() => soloRunStore.runs.find((item) => item.id === props.runId) || null)
const state = computed(() => soloExecutionStore.getExecutionState(props.runId))
const allLogs = computed(() => state.value?.logs ?? [])
const steps = computed(() => soloExecutionStore.getSteps(props.runId))
const selectedStep = computed<SoloStep | null>(() =>
  props.stepId ? steps.value.find((step) => step.id === props.stepId) || null : null
)

const visibleLogs = computed<SoloLogEntry[]>(() => {
  if (props.stepId) {
    return allLogs.value.filter((log) => log.stepId === props.stepId)
  }

  return allLogs.value.filter((log) => !log.stepId && log.scope !== 'step')
})

const selectedExpert = computed(() =>
  resolveExpertById(selectedStep.value?.selectedExpertId, agentTeamsStore.experts)
)
const coordinatorExpert = computed(() =>
  resolveExpertById(run.value?.coordinatorExpertId, agentTeamsStore.experts)
)

const currentRuntime = computed(() => {
  if (selectedStep.value) {
    return resolveExpertRuntime(selectedExpert.value, agentStore.agents)
  }

  return resolveExpertRuntime(coordinatorExpert.value, agentStore.agents, run.value?.coordinatorModelId)
})

const selectedExpertLabel = computed(() => {
  if (selectedStep.value) {
    return selectedExpert.value?.name || selectedStep.value.selectedExpertId || '未指定'
  }

  return coordinatorExpert.value?.name || run.value?.coordinatorExpertId || '规划智能体'
})

const tokenUsage = computed(() => state.value?.tokenUsage ?? {
  inputTokens: 0,
  outputTokens: 0,
  model: undefined,
  resetCount: 0,
  lastUpdatedAt: null
})

const tokenUsageTotal = computed(() =>
  tokenUsage.value.contextWindowOccupancy
    ?? (tokenUsage.value.inputTokens + tokenUsage.value.outputTokens)
)

const resolvedModelId = computed(() =>
  tokenUsage.value.model
  || currentRuntime.value?.modelId
  || currentRuntime.value?.agent.modelId
  || run.value?.coordinatorModelId
  || ''
)

const tokenContextLimit = computed(() => {
  const runtimeAgent = currentRuntime.value?.agent
  if (!runtimeAgent) {
    return resolveConfiguredContextWindow([], {
      runtimeModelId: resolvedModelId.value,
      fallbackContextWindow: DEFAULT_CONTEXT_WINDOW
    })
  }

  return resolveConfiguredContextWindow(agentConfigStore.getModelsConfigs(runtimeAgent.id), {
    runtimeModelId: tokenUsage.value.model,
    selectedModelId: currentRuntime.value?.modelId,
    agentModelId: runtimeAgent.modelId,
    fallbackContextWindow: DEFAULT_CONTEXT_WINDOW
  })
})

const tokenUsagePercentage = computed(() => {
  if (tokenContextLimit.value <= 0) return 0
  return Math.min(100, (tokenUsageTotal.value / tokenContextLimit.value) * 100)
})

const tokenUsageLevel = computed(() => {
  if (tokenUsagePercentage.value >= 95) return 'critical'
  if (tokenUsagePercentage.value >= 80) return 'danger'
  if (tokenUsagePercentage.value >= 60) return 'warning'
  return 'safe'
})

const tokenProgressStyle = computed(() => ({
  width: `${tokenUsagePercentage.value}%`
}))

const pendingInputVisible = computed(() => {
  if (!run.value?.inputRequest) return false
  if (!props.stepId) return !run.value.inputRequest.stepId
  return run.value.inputRequest.stepId === props.stepId
})

const isScopeRunning = computed(() => {
  if (selectedStep.value) {
    return selectedStep.value.status === 'running'
  }

  return run.value?.executionStatus === 'running'
})

const headerTitle = computed(() => selectedStep.value?.title || '协调日志流程')
const headerMetaLabel = computed(() => selectedStep.value ? '执行节点' : '调度节点')
const panelSubtitle = computed(() => {
  if (selectedStep.value) {
    return selectedStep.value.resultSummary
      || selectedStep.value.summary
      || selectedStep.value.description
      || '等待当前步骤的结构化结果与交付摘要。'
  }

  return '展示规划智能体的调度决策、异常与状态回写。'
})

const activeCliRetryState = computed<SoloCliRetryState | null>(() => {
  for (let index = visibleLogs.value.length - 1; index >= 0; index -= 1) {
    const log = visibleLogs.value[index]
    const metadata = log.metadata

    if (
      metadata?.retryGroup === 'cli_failure_retry'
      && typeof metadata.retryCount === 'number'
      && typeof metadata.maxRetries === 'number'
    ) {
      return {
        current: metadata.retryCount,
        max: metadata.maxRetries
      }
    }

    if (
      log.type === 'content'
      || log.type === 'thinking'
      || log.type === 'thinking_start'
      || log.type === 'tool_use'
      || log.type === 'tool_input_delta'
      || log.type === 'tool_result'
      || log.type === 'error'
      || (log.type === 'system' && metadata?.retryGroup !== 'cli_failure_retry')
    ) {
      return null
    }
  }

  return null
})

const statusText = computed(() => {
  if (selectedStep.value) {
    return getStepStatusLabel(selectedStep.value.status)
  }

  return getRunStatusLabel(run.value?.executionStatus, run.value?.status)
})

const statusColor = computed(() => {
  if (selectedStep.value) {
    return getStepStatusColor(selectedStep.value.status)
  }

  return getRunStatusColor(run.value?.executionStatus, run.value?.status)
})

const timelineEntries = computed<TimelineEntry[]>(() => {
  const toolCallMap = buildToolCallMapFromLogs(visibleLogs.value, {
    fallbackStatus: isScopeRunning.value ? 'running' : 'success'
  })
  let lastThinkingEntry: TimelineEntry | null = null
  let lastContentEntry: TimelineEntry | null = null

  return visibleLogs.value.reduce<TimelineEntry[]>((entries, log) => {
    if (log.type === 'tool_input_delta' || log.type === 'tool_result') {
      return entries
    }

    if (log.type === 'thinking_start') {
      lastThinkingEntry = {
        id: `thinking-${log.id}`,
        type: 'thinking',
        content: '',
        timestamp: log.timestamp,
        animate: isScopeRunning.value
      }
      entries.push(lastThinkingEntry)
      return entries
    }

    if (log.type === 'thinking') {
      if (!lastThinkingEntry) {
        lastThinkingEntry = {
          id: `thinking-${log.id}`,
          type: 'thinking',
          content: log.content,
          timestamp: log.timestamp,
          animate: isScopeRunning.value
        }
        entries.push(lastThinkingEntry)
      } else {
        lastThinkingEntry.content = `${lastThinkingEntry.content || ''}${log.content}`
        lastThinkingEntry.timestamp = log.timestamp
      }
      lastContentEntry = null
      return entries
    }

    if (log.type === 'content') {
      if (!lastContentEntry) {
        lastContentEntry = {
          id: `content-${log.id}`,
          type: 'content',
          content: log.content,
          timestamp: log.timestamp,
          role: 'assistant',
          animate: isScopeRunning.value
        }
        entries.push(lastContentEntry)
      } else {
        lastContentEntry.content = `${lastContentEntry.content || ''}${log.content}`
        lastContentEntry.timestamp = log.timestamp
      }
      lastThinkingEntry = null
      return entries
    }

    if (log.type === 'tool_use') {
      entries.push({
        id: `tool-${log.id}`,
        type: 'tool',
        timestamp: log.timestamp,
        toolCall: toolCallMap.get(log.metadata?.toolCallId || log.id)
      })
      lastThinkingEntry = null
      lastContentEntry = null
      return entries
    }

    if (log.type === 'error') {
      entries.push({
        id: `error-${log.id}`,
        type: 'error',
        content: log.content,
        timestamp: log.timestamp,
        runtimeFallbackUsage: {
          model: typeof log.metadata?.model === 'string' && log.metadata.model.trim()
            ? log.metadata.model.trim()
            : (resolvedModelId.value || undefined),
          inputTokens: typeof log.metadata?.inputTokens === 'number'
            ? log.metadata.inputTokens
            : undefined,
          outputTokens: typeof log.metadata?.outputTokens === 'number'
            ? log.metadata.outputTokens
            : undefined
        }
      })
      lastThinkingEntry = null
      lastContentEntry = null
      return entries
    }

    entries.push({
      id: `system-${log.id}`,
      type: 'system',
      content: log.content,
      timestamp: log.timestamp,
      runtimeFallbackUsage: {
        model: typeof log.metadata?.model === 'string' && log.metadata.model.trim()
          ? log.metadata.model.trim()
          : (resolvedModelId.value || undefined),
        inputTokens: typeof log.metadata?.inputTokens === 'number'
          ? log.metadata.inputTokens
          : undefined,
        outputTokens: typeof log.metadata?.outputTokens === 'number'
          ? log.metadata.outputTokens
          : undefined
      }
    })
    lastThinkingEntry = null
    lastContentEntry = null
    return entries
  }, [])
})

function getStepStatusLabel(status: SoloStepStatus): string {
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

function getRunStatusLabel(executionStatus?: SoloExecutionStatus | null, runStatus?: SoloRunStatus | null): string {
  switch (executionStatus || runStatus) {
    case 'running': return '执行中'
    case 'blocked': return '待输入'
    case 'paused': return '已暂停'
    case 'completed': return '已完成'
    case 'failed':
    case 'error':
      return '失败'
    case 'stopped': return '已停止'
    case 'draft': return '草稿'
    default: return '空闲'
  }
}

function getStepStatusColor(status: SoloStepStatus): string {
  switch (status) {
    case 'running': return 'primary'
    case 'completed': return 'success'
    case 'blocked': return 'warning'
    case 'failed': return 'error'
    default: return 'gray'
  }
}

function getRunStatusColor(executionStatus?: SoloExecutionStatus | null, runStatus?: SoloRunStatus | null): string {
  switch (executionStatus || runStatus) {
    case 'running': return 'primary'
    case 'completed': return 'success'
    case 'blocked': return 'warning'
    case 'failed':
    case 'error':
      return 'error'
    default: return 'gray'
  }
}

function handleScroll() {
  if (!scrollerRef.value) return
  const { scrollTop, scrollHeight, clientHeight } = scrollerRef.value
  autoScroll.value = scrollHeight - scrollTop - clientHeight < 36
}

async function handleSubmit(values: Record<string, unknown>) {
  await soloExecutionStore.submitRunInput(props.runId, values)
}

async function ensureRuntimeConfigsLoaded() {
  const runtimeAgent = currentRuntime.value?.agent
  if (!runtimeAgent) {
    return
  }

  await agentConfigStore.ensureModelsConfigs(runtimeAgent.id, runtimeAgent.provider)
}

watch(
  () => props.runId,
  async (runId) => {
    if (!runId) return
    await soloExecutionStore.loadLogs(runId)
  },
  { immediate: true }
)

watch(
  () => currentRuntime.value?.agent.id,
  async () => {
    await ensureRuntimeConfigsLoaded()
  },
  { immediate: true }
)

watch(
  () => `${props.stepId ?? 'coordinator'}:${visibleLogs.value.map((log) => log.id).join(':')}`,
  async () => {
    if (!autoScroll.value) return
    await nextTick()
    if (scrollerRef.value) {
      scrollerRef.value.scrollTop = scrollerRef.value.scrollHeight
    }
  }
)

onMounted(async () => {
  await Promise.all([
    soloExecutionStore.loadLogs(props.runId),
    agentStore.loadAgents(),
    agentTeamsStore.loadExperts()
  ])
})
</script>

<template>
  <div class="task-execution-log solo-execution-log">
    <div class="log-header">
      <div class="header-left">
        <h4
          class="log-title"
          :title="headerTitle"
        >
          {{ headerTitle }}
        </h4>
        <span
          class="status-badge"
          :class="statusColor"
        >
          {{ statusText }}
        </span>
      </div>
      <div class="header-actions">
        <span class="solo-execution-log__meta-chip">
          {{ headerMetaLabel }}
        </span>
        <span
          v-if="selectedStep"
          class="solo-execution-log__meta-chip"
        >
          Depth {{ selectedStep.depth }}
        </span>
      </div>
    </div>

    <section class="solo-execution-log__summary">
      <p class="solo-execution-log__summary-eyebrow">
        {{ selectedStep ? 'Execution Summary' : 'Coordinator Summary' }}
      </p>
      <p class="solo-execution-log__summary-text">
        {{ panelSubtitle }}
      </p>
      <div class="solo-execution-log__summary-meta">
        <span>专家：{{ selectedExpertLabel }}</span>
        <span>{{ visibleLogs.length }} 条日志</span>
        <span v-if="selectedStep">更新时间：{{ new Date(selectedStep.updatedAt).toLocaleString('zh-CN') }}</span>
      </div>
      <div
        v-if="selectedStep?.resultFiles.length"
        class="solo-execution-log__summary-files"
      >
        <span
          v-for="file in selectedStep.resultFiles"
          :key="file"
        >
          {{ file }}
        </span>
      </div>
    </section>

    <div
      v-if="tokenUsageTotal > 0 || resolvedModelId"
      class="token-usage-panel"
    >
      <div class="token-usage-panel__meta">
        <div class="token-usage-panel__title">
          <span>Token Usage</span>
          <span
            v-if="resolvedModelId"
            class="token-usage-panel__model"
          >
            {{ resolvedModelId }}
          </span>
        </div>
        <div class="token-usage-panel__stats">
          <span>{{ formatTokenCount(tokenUsageTotal) }} / {{ formatTokenCount(tokenContextLimit) }}</span>
          <span v-if="tokenUsage.resetCount > 0">重置 {{ tokenUsage.resetCount }} 次</span>
        </div>
      </div>
      <div
        class="token-usage-panel__bar"
        :class="`token-usage-panel__bar--${tokenUsageLevel}`"
      >
        <div
          class="token-usage-panel__fill"
          :style="tokenProgressStyle"
        />
      </div>
      <div class="token-usage-panel__breakdown">
        <span>输入 {{ formatTokenCount(tokenUsage.inputTokens) }}</span>
        <span>输出 {{ formatTokenCount(tokenUsage.outputTokens) }}</span>
        <span>{{ Math.round(tokenUsagePercentage) }}%</span>
      </div>
    </div>

    <div
      v-if="pendingInputVisible && run?.inputRequest"
      class="input-form-section"
    >
      <h5 class="section-title">
        {{ run.inputRequest.question || '等待补充输入' }}
      </h5>
      <DynamicForm
        :schema="run.inputRequest.formSchema"
        @submit="handleSubmit"
      />
    </div>

    <div
      ref="scrollerRef"
      class="log-content"
      @scroll="handleScroll"
    >
      <div
        v-if="timelineEntries.length === 0"
        class="empty-state"
      >
        <span v-if="isScopeRunning">{{ selectedStep ? '当前步骤执行中...' : '协调 AI 正在调度...' }}</span>
        <span v-else>{{ selectedStep ? '选中步骤开始执行后，这里会出现完整日志流程。' : '协调 AI 开始派发步骤后，这里会出现调度日志。' }}</span>
      </div>

      <div
        v-else
        class="log-entries"
      >
        <ExecutionTimeline
          :entries="timelineEntries"
          :group-tool-calls="true"
          :compact-context-notices="true"
        />
      </div>

      <div
        v-if="isScopeRunning"
        class="running-indicator"
      >
        <span class="indicator-dot" />
        <span class="indicator-text">
          {{ activeCliRetryState
            ? `底层自动重试中 ${activeCliRetryState.current}/${activeCliRetryState.max}`
            : 'AI 正在执行...' }}
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped src="../plan/taskExecutionLog/styles.css"></style>

<style scoped>
.solo-execution-log {
  --task-log-width: 100%;
  --timeline-panel-width: 100%;
  --timeline-panel-max-width: 100%;
  height: 100%;
  border: none;
  border-radius: 0;
  box-shadow: none;
}

.solo-execution-log__meta-chip {
  display: inline-flex;
  align-items: center;
  padding: 0.125rem 0.5rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-bg-tertiary, #f1f5f9) 92%, #ffffff);
  border: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent);
  color: var(--color-text-secondary, #64748b);
  font-size: 0.6875rem;
  font-weight: 500;
  white-space: nowrap;
}

.solo-execution-log__summary {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  padding: 0.875rem 1rem;
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-primary-light, #eff6ff) 44%, #ffffff),
      color-mix(in srgb, var(--color-surface, #ffffff) 92%, var(--color-bg-secondary, #f8fafc))
    );
}

.solo-execution-log__summary-eyebrow {
  margin: 0;
  font-size: 0.6875rem;
  font-weight: 600;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--color-primary, #2563eb);
}

.solo-execution-log__summary-text {
  margin: 0;
  color: var(--color-text-primary, #1e293b);
  font-size: 0.8125rem;
  line-height: 1.55;
  white-space: pre-wrap;
}

.solo-execution-log__summary-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem 0.75rem;
  font-size: 0.6875rem;
  color: var(--color-text-secondary, #64748b);
}

.solo-execution-log__summary-files {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.solo-execution-log__summary-files span {
  display: inline-flex;
  align-items: center;
  padding: 0.25rem 0.5rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-bg-tertiary, #f1f5f9) 92%, #ffffff);
  border: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent);
  color: var(--color-text-secondary, #64748b);
  font-size: 0.6875rem;
  line-height: 1;
}

[data-theme='dark'] .solo-execution-log__meta-chip {
  background: rgba(51, 65, 85, 0.9);
  border-color: rgba(148, 163, 184, 0.18);
  color: rgba(226, 232, 240, 0.78);
}

[data-theme='dark'] .solo-execution-log__summary {
  border-bottom-color: rgba(148, 163, 184, 0.14);
  background:
    linear-gradient(
      180deg,
      rgba(30, 41, 59, 0.92),
      rgba(15, 23, 42, 0.88)
    );
}

[data-theme='dark'] .solo-execution-log__summary-text {
  color: #e2e8f0;
}

[data-theme='dark'] .solo-execution-log__summary-meta,
[data-theme='dark'] .solo-execution-log__summary-files span {
  color: rgba(226, 232, 240, 0.72);
}

[data-theme='dark'] .solo-execution-log__summary-files span {
  background: rgba(30, 41, 59, 0.9);
  border-color: rgba(148, 163, 184, 0.18);
}
</style>
