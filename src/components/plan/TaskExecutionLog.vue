<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted } from 'vue'
import { useTaskExecutionStore } from '@/stores/taskExecution'
import { useTaskStore } from '@/stores/task'
import { usePlanStore } from '@/stores/plan'
import { useAgentStore } from '@/stores/agent'
import { useAgentConfigStore } from '@/stores/agentConfig'
import ExecutionTimeline from '@/components/message/ExecutionTimeline.vue'
import StructuredContentRenderer from '@/components/message/StructuredContentRenderer.vue'
import DynamicForm from '@/components/plan/DynamicForm.vue'
import type { TimelineEntry } from '@/types/timeline'
import type { TaskExecutionResultRecord } from '@/types/taskExecution'
import { buildToolCallFromLogs } from '@/utils/toolCallLog'
import { containsFormSchema } from '@/utils/structuredContent'
import { buildStructuredResultContentFromRecord } from '@/utils/taskExecutionResult'
import { getTaskExecutionStatusMeta, resolveTaskExecutionStatus } from '@/utils/taskExecutionStatus'
import {
  DEFAULT_CONTEXT_WINDOW,
  resolveConfiguredContextWindow
} from '@/utils/configuredModelContext'
import { formatTokenCount } from '@/stores/token'

const props = defineProps<{
  taskId: string
}>()

const taskExecutionStore = useTaskExecutionStore()
const taskStore = useTaskStore()
const planStore = usePlanStore()
const agentStore = useAgentStore()
const agentConfigStore = useAgentConfigStore()

// 日志容器引用
const logContainerRef = ref<HTMLElement | null>(null)

const autoScroll = ref(true)
const resultRecord = ref<TaskExecutionResultRecord | null>(null)

const task = computed(() => {
  return taskStore.tasks.find(t => t.id === props.taskId)
})

const executionState = computed(() => {
  return taskExecutionStore.getExecutionState(props.taskId)
})

const tokenUsageWindow = computed(() => executionState.value?.tokenUsage ?? {
  inputTokens: 0,
  outputTokens: 0,
  resetCount: 0,
  lastUpdatedAt: null
})

const tokenContextLimit = computed(() => {
  const currentTask = task.value
  if (!currentTask) return DEFAULT_CONTEXT_WINDOW

  const plan = planStore.plans.find(item => item.id === currentTask.planId)
  const agentId = currentTask.agentId || plan?.splitAgentId
  const modelId = currentTask.modelId || plan?.splitModelId
  const runtimeModel = tokenUsageWindow.value.model?.trim()

  if (!agentId) {
    return DEFAULT_CONTEXT_WINDOW
  }

  const agent = agentStore.agents.find(item => item.id === agentId)
  return resolveConfiguredContextWindow(agentConfigStore.getModelsConfigs(agentId), {
    runtimeModelId: runtimeModel,
    selectedModelId: modelId,
    agentModelId: agent?.modelId
  })
})

const tokenUsageTotal = computed(() =>
  tokenUsageWindow.value.inputTokens + tokenUsageWindow.value.outputTokens
)

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

const logs = computed(() => {
  return executionState.value?.logs ?? []
})

// 是否等待用户输入
const isWaitingInput = computed(() => {
  return task.value?.status === 'blocked' && task.value?.blockReason === 'waiting_input'
})

const effectiveStatus = computed(() => {
  const memoryStatus = executionState.value?.status
  if (memoryStatus && memoryStatus !== 'idle') {
    return memoryStatus
  }
  return resolveTaskExecutionStatus(task.value, memoryStatus)
})

const isRunning = computed(() => {
  return effectiveStatus.value === 'running'
})

const structuredResultContent = computed(() => {
  if (!resultRecord.value) return ''
  return buildStructuredResultContentFromRecord(resultRecord.value)
})

const statusText = computed(() => {
  return getTaskExecutionStatusMeta(effectiveStatus.value).label
})

// 状��颜�?
const statusColor = computed(() => {
  return getTaskExecutionStatusMeta(effectiveStatus.value).color
})

async function handleStop() {
  await taskExecutionStore.stopTaskExecution(props.taskId)
}

async function handleResume() {
  await taskExecutionStore.resumeTaskExecution(props.taskId)
}

// 清除日志
async function handleClearLogs() {
  await taskExecutionStore.clearTaskLogs(props.taskId)
}

// 提交表单输入
async function handleInputSubmit(values: Record<string, unknown>) {
  await taskExecutionStore.submitTaskInput(props.taskId, values)
}

async function handleSkip() {
  await taskExecutionStore.skipBlockedTask(props.taskId)
}

async function loadResultRecord(taskId: string) {
  const currentTask = taskStore.tasks.find(item => item.id === taskId)
  if (!currentTask) {
    resultRecord.value = null
    return
  }

  const records = await taskExecutionStore.listRecentPlanResults(currentTask.planId, 200)
  resultRecord.value = records.find(record => record.task_id === taskId) ?? null
}

function scrollToBottom() {
  if (logContainerRef.value && autoScroll.value) {
    nextTick(() => {
      logContainerRef.value!.scrollTop = logContainerRef.value!.scrollHeight
    })
  }
}

watch(logs, () => {
  scrollToBottom()
}, { deep: true })

function handleScroll() {
  if (!logContainerRef.value) return
  const { scrollTop, scrollHeight, clientHeight } = logContainerRef.value
  autoScroll.value = scrollHeight - scrollTop - clientHeight < 50
}

const timelineEntries = computed<TimelineEntry[]>(() => {
  let lastThinkingEntry: TimelineEntry | null = null
  let lastContentEntry: TimelineEntry | null = null

  return logs.value.reduce<TimelineEntry[]>((entries, log) => {
    if (log.type === 'tool_result' || log.type === 'tool_input_delta') {
      return entries
    }

    const activeFormId = task.value?.inputRequest?.formSchema.formId
    if (
      log.type === 'content'
      && isWaitingInput.value
      && containsFormSchema(log.content, activeFormId)
    ) {
      lastThinkingEntry = null
      lastContentEntry = null
      return entries
    }

    if (log.type === 'thinking_start') {
      if (!lastThinkingEntry) {
        lastThinkingEntry = {
          id: `entry-${log.id}`,
          type: 'thinking',
          content: '',
          timestamp: log.timestamp,
          animate: isRunning.value
        }
        entries.push(lastThinkingEntry)
      }
      return entries
    }

    if (log.type === 'thinking') {
      if (lastThinkingEntry) {
        lastThinkingEntry.content = `${lastThinkingEntry.content || ''}${log.content}`
        lastThinkingEntry.timestamp = log.timestamp
        lastThinkingEntry.animate = isRunning.value
      } else {
        lastThinkingEntry = {
          id: `entry-${log.id}`,
          type: 'thinking',
          content: log.content,
          timestamp: log.timestamp,
          animate: isRunning.value
        }
        entries.push(lastThinkingEntry)
      }
      lastContentEntry = null
      return entries
    }

    if (log.type === 'content') {
      if (lastContentEntry) {
        lastContentEntry.content = `${lastContentEntry.content || ''}${log.content}`
        lastContentEntry.timestamp = log.timestamp
        lastContentEntry.animate = isRunning.value
      } else {
        lastContentEntry = {
          id: `entry-${log.id}`,
          type: 'content',
          content: log.content,
          timestamp: log.timestamp,
          animate: isRunning.value
        }
        entries.push(lastContentEntry)
      }
      lastThinkingEntry = null
      return entries
    }

    lastThinkingEntry = null
    lastContentEntry = null

    if (log.type === 'tool_use') {
      const toolCall = buildToolCallFromLogs(log, logs.value, {
        fallbackStatus: isRunning.value ? 'running' : 'success'
      })
      if (toolCall) {
        entries.push({
          id: `tool-${log.id}`,
          type: 'tool',
          toolCall,
          timestamp: log.timestamp,
          animate: isRunning.value
        })
      }
      return entries
    }

    entries.push({
      id: `entry-${log.id}`,
      type: log.type === 'error' ? 'error' : 'system',
      content: log.content,
      timestamp: log.timestamp,
      animate: isRunning.value
    })
    return entries
  }, [])
})

// 加载历史日志
onMounted(async () => {
  await taskExecutionStore.loadTaskLogs(props.taskId)
  await loadResultRecord(props.taskId)
  scrollToBottom()
})

watch(
  () => props.taskId,
  async (taskId) => {
    await taskExecutionStore.loadTaskLogs(taskId)
    await loadResultRecord(taskId)
    scrollToBottom()
  }
)

watch(
  () => `${task.value?.status || ''}:${task.value?.updatedAt || ''}`,
  async () => {
    await loadResultRecord(props.taskId)
  }
)
</script>

<template>
  <div class="task-execution-log">
    <div class="log-header">
      <div class="header-left">
        <h4 class="log-title">
          {{ task?.title || '??????' }}
        </h4>
        <span
          class="status-badge"
          :class="statusColor"
        >
          {{ statusText }}
        </span>
      </div>
      <div class="header-actions">
        <button
          v-if="isRunning"
          class="btn-stop"
          @click="handleStop"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <rect
              x="6"
              y="6"
              width="12"
              height="12"
            />
          </svg>
          ֹͣ
        </button>
        <button
          v-else-if="effectiveStatus === 'stopped'"
          class="btn-resume"
          @click="handleResume"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polygon points="5 3 19 12 5 21 5 3" />
          </svg>
          ??
        </button>
        <button
          v-if="logs.length > 0"
          class="btn-clear"
          @click="handleClearLogs"
        >
          ????
        </button>
      </div>
    </div>

    <div
      v-if="tokenUsageTotal > 0 || tokenUsageWindow.model"
      class="token-usage-panel"
    >
      <div class="token-usage-panel__meta">
        <div class="token-usage-panel__title">
          <span>Token ??</span>
          <span
            v-if="tokenUsageWindow.model"
            class="token-usage-panel__model"
          >
            {{ tokenUsageWindow.model }}
          </span>
        </div>
        <div class="token-usage-panel__stats">
          <span>{{ formatTokenCount(tokenUsageTotal) }} / {{ formatTokenCount(tokenContextLimit) }}</span>
          <span v-if="tokenUsageWindow.resetCount > 0">?? {{ tokenUsageWindow.resetCount }} ?</span>
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
        <span>?? {{ formatTokenCount(tokenUsageWindow.inputTokens) }}</span>
        <span>?? {{ formatTokenCount(tokenUsageWindow.outputTokens) }}</span>
        <span>{{ Math.round(tokenUsagePercentage) }}%</span>
      </div>
    </div>
    <!-- 等待用户输入表单区域 -->
    <div
      v-if="isWaitingInput && task?.inputRequest"
      class="input-form-section"
    >
      <h5 class="section-title">
        {{ task.inputRequest.question || '??????' }}
      </h5>
      <DynamicForm
        :schema="task.inputRequest.formSchema"
        @submit="handleInputSubmit"
      />
      <button
        class="btn-skip"
        @click="handleSkip"
      >
        ?????
      </button>
    </div>

    <!-- 日志内容 -->
    <div
      ref="logContainerRef"
      class="log-content"
      @scroll="handleScroll"
    >
      <div
        v-if="structuredResultContent"
        class="result-summary"
      >
        <StructuredContentRenderer :content="structuredResultContent" />
      </div>

      <div
        v-if="logs.length === 0"
        class="empty-state"
      >
        <span v-if="isRunning">????...</span>
        <span v-else>??????</span>
      </div>

      <div
        v-else
        class="log-entries"
      >
        <ExecutionTimeline :entries="timelineEntries" />
      </div>

      <!-- 运行指示�?-->
      <div
        v-if="isRunning"
        class="running-indicator"
      >
        <span class="indicator-dot" />
        <span class="indicator-text">AI ?????...</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.task-execution-log {
  --task-log-surface: var(--color-surface, #ffffff);
  --task-log-border: color-mix(in srgb, var(--color-border) 72%, transparent);
  --task-log-shadow: 0 16px 32px rgba(15, 23, 42, 0.08);
  --task-log-content-bg:
    linear-gradient(
      180deg,
      transparent 0%,
      color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 56%, transparent) 100%
    );
  --task-log-width: min(100%, calc(var(--detail-panel-width, 380px) - 1.5rem));
  --timeline-panel-width: var(--task-log-width);
  --timeline-panel-max-width: var(--task-log-width);
  display: flex;
  flex-direction: column;
  width: 100%;
  min-width: 0;
  height: 100%;
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-surface-elevated, #fff) 96%, var(--task-log-surface)) 0%,
      color-mix(in srgb, var(--task-log-surface) 92%, var(--color-bg-secondary, #f8fafc)) 100%
    );
  border-radius: var(--radius-lg, 12px);
  overflow: hidden;
  border: 1px solid var(--task-log-border);
  box-shadow: var(--task-log-shadow);
}

.log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3, 0.75rem) var(--spacing-4, 1rem);
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 94%, #ffffff),
      color-mix(in srgb, var(--color-surface, #ffffff) 90%, var(--color-bg-secondary, #f8fafc))
    );
}

.header-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-3, 0.75rem);
}

.log-title {
  margin: 0;
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
}

.status-badge {
  padding: 0.125rem 0.5rem;
  border-radius: var(--radius-full, 9999px);
  font-size: 0.6875rem;
  font-weight: var(--font-weight-medium, 500);
}

.status-badge.primary {
  background-color: var(--color-primary-light, #dbeafe);
  color: var(--color-primary, #2563eb);
}

.status-badge.success {
  background-color: var(--color-success-light, #d1fae5);
  color: var(--color-success, #16a34a);
}

.status-badge.warning {
  background-color: var(--color-warning-light, #fef3c7);
  color: var(--color-warning, #d97706);
}

.status-badge.error {
  background-color: var(--color-error-light, #fee2e2);
  color: var(--color-error, #dc2626);
}

.status-badge.gray {
  background-color: var(--color-bg-tertiary, #f1f5f9);
  color: var(--color-text-tertiary, #94a3b8);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
}

.token-usage-panel {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-surface, #ffffff) 92%, #eff6ff),
      color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 84%, #ffffff)
    );
}

.token-usage-panel__meta,
.token-usage-panel__breakdown,
.token-usage-panel__title,
.token-usage-panel__stats {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.token-usage-panel__title {
  justify-content: flex-start;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-text-primary, #1e293b);
}

.token-usage-panel__model {
  padding: 0.125rem 0.5rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary-light, #dbeafe) 88%, white);
  color: var(--color-primary, #2563eb);
}

.token-usage-panel__stats,
.token-usage-panel__breakdown {
  font-size: 0.6875rem;
  color: var(--color-text-secondary, #64748b);
}

.token-usage-panel__bar {
  height: 0.5rem;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-bg-tertiary, #e2e8f0) 88%, white);
}

.token-usage-panel__fill {
  height: 100%;
  border-radius: inherit;
  transition: width 0.25s ease;
}

.token-usage-panel__bar--safe .token-usage-panel__fill {
  background: var(--color-primary, #2563eb);
}

.token-usage-panel__bar--warning .token-usage-panel__fill {
  background: var(--color-warning, #d97706);
}

.token-usage-panel__bar--danger .token-usage-panel__fill {
  background: var(--color-orange-500, #f97316);
}

.token-usage-panel__bar--critical .token-usage-panel__fill {
  background: var(--color-error, #dc2626);
}

.btn-stop,
.btn-resume,
.btn-clear {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  border: none;
  border-radius: var(--radius-sm, 4px);
  font-size: 0.6875rem;
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-stop {
  background-color: var(--color-error-light, #fee2e2);
  color: var(--color-error, #dc2626);
}

.btn-stop:hover {
  background-color: var(--color-error, #dc2626);
  color: white;
}

.btn-resume {
  background-color: #dcfce7;
  color: #15803d;
}

.btn-resume:hover {
  background-color: #16a34a;
  color: white;
}

.btn-clear {
  background-color: color-mix(in srgb, var(--color-bg-tertiary, #f1f5f9) 92%, #ffffff);
  color: var(--color-text-secondary, #64748b);
  border: 1px solid color-mix(in srgb, var(--color-border) 60%, transparent);
}

.btn-clear:hover {
  background-color: color-mix(in srgb, var(--color-bg-tertiary, #f1f5f9) 100%, var(--color-bg-secondary, #e2e8f0));
}

.log-content {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: var(--spacing-3, 0.75rem);
  background: var(--task-log-content-bg);
}

.result-summary {
  margin-bottom: var(--spacing-3, 0.75rem);
  width: 100%;
  min-width: 0;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--color-text-tertiary, #94a3b8);
  font-size: var(--font-size-sm, 13px);
}

.log-entries {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3, 0.75rem);
}

.log-entry {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-2, 0.5rem);
}

.log-entry--system {
  padding: var(--spacing-2, 0.5rem);
  background-color: var(--color-bg-tertiary, #f1f5f9);
  border-radius: var(--radius-sm, 4px);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-secondary, #64748b);
}

.log-entry--error {
  padding: var(--spacing-2, 0.5rem);
  background-color: var(--color-error-light, #fef2f2);
  border-radius: var(--radius-sm, 4px);
  border-left: 3px solid var(--color-error, #dc2626);
}

.log-entry--error .log-content-text {
  color: var(--color-error, #dc2626);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs, 12px);
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}

.log-entry--content {
  padding: var(--spacing-2, 0.5rem) 0;
}

.log-time {
  font-size: 0.625rem;
  color: var(--color-text-tertiary, #94a3b8);
  flex-shrink: 0;
}

.log-icon {
  font-size: 12px;
  flex-shrink: 0;
}

.log-content-text {
  flex: 1;
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-primary, #1e293b);
}

.running-indicator {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
  padding: var(--spacing-2, 0.5rem);
  margin-top: var(--spacing-3, 0.75rem);
  background:
    linear-gradient(
      90deg,
      color-mix(in srgb, var(--color-primary-light, #eff6ff) 90%, white),
      color-mix(in srgb, var(--color-primary-light, #eff6ff) 68%, transparent)
    );
  border-radius: var(--radius-sm, 4px);
  border: 1px solid color-mix(in srgb, var(--color-primary) 22%, transparent);
}

.indicator-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--color-primary, #3b82f6);
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}

.indicator-text {
  font-size: var(--font-size-xs, 12px);
  color: var(--color-primary, #3b82f6);
}

/* 等待输入表单区域 */
.input-form-section {
  padding: var(--spacing-4, 1rem);
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-warning-light, #fef3c7) 74%, #fff8eb),
      color-mix(in srgb, var(--color-surface, #fff) 90%, var(--color-warning-light, #fef3c7))
    );
}

.section-title {
  margin: 0 0 var(--spacing-3, 0.75rem);
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
}

.btn-skip {
  display: block;
  width: 100%;
  margin-top: var(--spacing-3, 0.75rem);
  padding: var(--spacing-2, 0.5rem);
  border: 1px dashed var(--color-border, #e2e8f0);
  border-radius: var(--radius-sm, 4px);
  background-color: transparent;
  color: var(--color-text-secondary, #64748b);
  font-size: var(--font-size-xs, 12px);
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-skip:hover {
  background-color: var(--color-bg-tertiary, #f1f5f9);
  border-color: var(--color-text-tertiary, #94a3b8);
}

.status-badge.warning {
  background-color: var(--color-warning-light, #fef3c7);
  color: var(--color-warning, #d97706);
}

[data-theme='dark'] .task-execution-log {
  --task-log-surface: rgba(15, 23, 42, 0.96);
  --task-log-border: rgba(96, 165, 250, 0.14);
  --task-log-shadow: 0 22px 38px rgba(2, 6, 23, 0.42);
  --task-log-content-bg:
    linear-gradient(
      180deg,
      rgba(15, 23, 42, 0.12),
      rgba(2, 6, 23, 0.22)
    );
  background:
    linear-gradient(
      180deg,
      rgba(15, 23, 42, 0.96) 0%,
      rgba(15, 23, 42, 0.9) 100%
    );
}

[data-theme='dark'] .log-header {
  background:
    linear-gradient(
      180deg,
      rgba(30, 41, 59, 0.92),
      rgba(15, 23, 42, 0.88)
    );
  border-bottom-color: rgba(148, 163, 184, 0.12);
}

[data-theme='dark'] .token-usage-panel {
  border-bottom-color: rgba(148, 163, 184, 0.14);
  background:
    linear-gradient(
      180deg,
      rgba(15, 23, 42, 0.88),
      rgba(30, 41, 59, 0.78)
    );
}

[data-theme='dark'] .token-usage-panel__title {
  color: #f8fafc;
}

[data-theme='dark'] .token-usage-panel__model {
  background: rgba(37, 99, 235, 0.2);
  color: #bfdbfe;
}

[data-theme='dark'] .token-usage-panel__stats,
[data-theme='dark'] .token-usage-panel__breakdown {
  color: rgba(226, 232, 240, 0.72);
}

[data-theme='dark'] .token-usage-panel__bar {
  background: rgba(51, 65, 85, 0.92);
}

[data-theme='dark'] .btn-clear {
  background-color: rgba(51, 65, 85, 0.9);
  color: rgba(226, 232, 240, 0.86);
  border-color: rgba(148, 163, 184, 0.18);
}

[data-theme='dark'] .btn-clear:hover {
  background-color: rgba(71, 85, 105, 0.95);
}

[data-theme='dark'] .running-indicator {
  background:
    linear-gradient(
      90deg,
      rgba(30, 64, 175, 0.22),
      rgba(15, 23, 42, 0.12)
    );
  border-color: rgba(96, 165, 250, 0.18);
}

[data-theme='dark'] .input-form-section {
  border-bottom-color: rgba(148, 163, 184, 0.14);
  background:
    linear-gradient(
      180deg,
      rgba(120, 53, 15, 0.32),
      rgba(15, 23, 42, 0.24)
    );
}

[data-theme='dark'] .section-title {
  color: #f8fafc;
}

[data-theme='dark'] .btn-skip {
  border-color: rgba(251, 191, 36, 0.28);
  color: rgba(251, 191, 36, 0.92);
}

[data-theme='dark'] .btn-skip:hover {
  background-color: rgba(120, 53, 15, 0.24);
  border-color: rgba(251, 191, 36, 0.42);
}

[data-theme='dark'] .log-title,
[data-theme='dark'] .empty-state,
[data-theme='dark'] .log-content-text {
  color: #e2e8f0;
}

[data-theme='dark'] .log-entry--system {
  background-color: rgba(30, 41, 59, 0.7);
  color: rgba(226, 232, 240, 0.78);
}

[data-theme='dark'] .result-summary,
[data-theme='dark'] .log-entries {
  color: #e2e8f0;
}

[data-theme='dark'] .task-execution-log .result-summary {
  padding: 0.25rem;
  border-radius: var(--radius-lg, 12px);
  background: rgba(15, 23, 42, 0.82);
  border: 1px solid rgba(148, 163, 184, 0.16);
  box-shadow: 0 14px 30px rgba(2, 6, 23, 0.24);
}

[data-theme='dark'] .task-execution-log .result-summary :deep(.timeline-message__content),
[data-theme='dark'] .task-execution-log .result-summary :deep(.timeline-entry),
[data-theme='dark'] .task-execution-log .result-summary :deep(.structured-content-renderer),
[data-theme='dark'] .task-execution-log .result-summary :deep(.structured-renderer),
[data-theme='dark'] .task-execution-log .result-summary :deep(.markdown-content) {
  background: rgba(15, 23, 42, 0.88);
  color: #e2e8f0;
  border-color: rgba(148, 163, 184, 0.18);
}

[data-theme='dark'] .task-execution-log .result-summary :deep(.structured-content__result),
[data-theme='dark'] .task-execution-log .result-summary :deep(.structured-result-card) {
  background: linear-gradient(180deg, rgba(15, 23, 42, 0.94), rgba(15, 23, 42, 0.88));
  border-color: rgba(148, 163, 184, 0.18);
  box-shadow: 0 12px 28px rgba(2, 6, 23, 0.26);
  color: #e2e8f0;
}

[data-theme='dark'] .task-execution-log .result-summary :deep(.structured-result-card__label) {
  color: rgba(96, 165, 250, 0.92);
  border-color: rgba(96, 165, 250, 0.32);
}

[data-theme='dark'] .task-execution-log .result-summary :deep(.structured-result-card__summary),
[data-theme='dark'] .task-execution-log .result-summary :deep(.structured-result-card__section),
[data-theme='dark'] .task-execution-log .result-summary :deep(.structured-result-card__section p) {
  color: #e2e8f0;
}

[data-theme='dark'] .task-execution-log :deep(.timeline-message__content),
[data-theme='dark'] .task-execution-log :deep(.timeline-entry),
[data-theme='dark'] .task-execution-log :deep(.tool-call),
[data-theme='dark'] .task-execution-log :deep(.thinking-display),
[data-theme='dark'] .task-execution-log :deep(.structured-renderer),
[data-theme='dark'] .task-execution-log :deep(.markdown-content) {
  color: #e2e8f0;
}

[data-theme='dark'] .task-execution-log :deep(.execution-timeline) {
  --timeline-bubble-bg: rgba(15, 23, 42, 0.88);
  --timeline-bubble-border: rgba(148, 163, 184, 0.18);
  --timeline-bubble-shadow: 0 14px 30px rgba(2, 6, 23, 0.3);
  --timeline-user-bubble-bg: linear-gradient(135deg, rgba(37, 99, 235, 0.28), rgba(14, 165, 233, 0.16));
  --timeline-user-bubble-border: rgba(96, 165, 250, 0.24);
  --timeline-entry-bg: rgba(15, 23, 42, 0.78);
  --timeline-entry-border: rgba(148, 163, 184, 0.18);
}

[data-theme='dark'] .task-execution-log :deep(.timeline-message__content) {
  background: rgba(15, 23, 42, 0.88);
  border-color: rgba(148, 163, 184, 0.18);
  box-shadow: 0 14px 30px rgba(2, 6, 23, 0.3);
  color: #e2e8f0;
}

[data-theme='dark'] .task-execution-log :deep(.timeline-message--user .timeline-message__content) {
  background: linear-gradient(135deg, rgba(37, 99, 235, 0.28), rgba(14, 165, 233, 0.16));
  border-color: rgba(96, 165, 250, 0.24);
  color: #f8fafc;
}

[data-theme='dark'] .task-execution-log :deep(.timeline-message__text),
[data-theme='dark'] .task-execution-log :deep(.markdown-content),
[data-theme='dark'] .task-execution-log :deep(.markdown-content p),
[data-theme='dark'] .task-execution-log :deep(.markdown-content li),
[data-theme='dark'] .task-execution-log :deep(.markdown-content strong),
[data-theme='dark'] .task-execution-log :deep(.structured-content-renderer) {
  color: #e2e8f0;
}
</style>
