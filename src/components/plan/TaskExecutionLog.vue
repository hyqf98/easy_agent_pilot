<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted } from 'vue'
import { useTaskExecutionStore } from '@/stores/taskExecution'
import { useTaskStore } from '@/stores/task'
import ExecutionTimeline from '@/components/message/ExecutionTimeline.vue'
import StructuredContentRenderer from '@/components/message/StructuredContentRenderer.vue'
import DynamicForm from '@/components/plan/DynamicForm.vue'
import type { TimelineEntry } from '@/types/timeline'
import type { TaskExecutionResultRecord } from '@/types/taskExecution'
import { buildToolCallFromLogs } from '@/utils/toolCallLog'
import { containsFormSchema } from '@/utils/structuredContent'
import { buildStructuredResultContentFromRecord } from '@/utils/taskExecutionResult'
import { getTaskExecutionStatusMeta, resolveTaskExecutionStatus } from '@/utils/taskExecutionStatus'

const props = defineProps<{
  taskId: string
}>()

const taskExecutionStore = useTaskExecutionStore()
const taskStore = useTaskStore()

// 日志容器引用
const logContainerRef = ref<HTMLElement | null>(null)

// 是否自动滚动
const autoScroll = ref(true)
const resultRecord = ref<TaskExecutionResultRecord | null>(null)

// 任务信息
const task = computed(() => {
  return taskStore.tasks.find(t => t.id === props.taskId)
})

// 执行状态
const executionState = computed(() => {
  return taskExecutionStore.getExecutionState(props.taskId)
})

// 日志列表
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

// 是否正在执行
const isRunning = computed(() => {
  return effectiveStatus.value === 'running'
})

const structuredResultContent = computed(() => {
  if (!resultRecord.value) return ''
  return buildStructuredResultContentFromRecord(resultRecord.value)
})

// 执行状态文本
const statusText = computed(() => {
  return getTaskExecutionStatusMeta(effectiveStatus.value).label
})

// 状态颜色
const statusColor = computed(() => {
  return getTaskExecutionStatusMeta(effectiveStatus.value).color
})

// 停止执行
async function handleStop() {
  await taskExecutionStore.stopTaskExecution(props.taskId)
}

// 清除日志
async function handleClearLogs() {
  await taskExecutionStore.clearTaskLogs(props.taskId)
}

// 提交表单输入
async function handleInputSubmit(values: Record<string, unknown>) {
  await taskExecutionStore.submitTaskInput(props.taskId, values)
}

// 跳过任务
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

// 滚动到底部
function scrollToBottom() {
  if (logContainerRef.value && autoScroll.value) {
    nextTick(() => {
      logContainerRef.value!.scrollTop = logContainerRef.value!.scrollHeight
    })
  }
}

// 监听日志变化，自动滚动
watch(logs, () => {
  scrollToBottom()
}, { deep: true })

// 监听滚动，判断是否自动滚动
function handleScroll() {
  if (!logContainerRef.value) return
  const { scrollTop, scrollHeight, clientHeight } = logContainerRef.value
  // 距离底部 50px 以内视为在底部
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
    <!-- 头部 -->
    <div class="log-header">
      <div class="header-left">
        <h4 class="log-title">
          {{ task?.title || '任务执行日志' }}
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
          停止
        </button>
        <button
          v-if="logs.length > 0"
          class="btn-clear"
          @click="handleClearLogs"
        >
          清除日志
        </button>
      </div>
    </div>

    <!-- 等待用户输入表单区域 -->
    <div
      v-if="isWaitingInput && task?.inputRequest"
      class="input-form-section"
    >
      <h5 class="section-title">
        {{ task.inputRequest.question || '需要您的输入' }}
      </h5>
      <DynamicForm
        :schema="task.inputRequest.formSchema"
        @submit="handleInputSubmit"
      />
      <button
        class="btn-skip"
        @click="handleSkip"
      >
        跳过此任务
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
        <span v-if="isRunning">正在启动...</span>
        <span v-else>暂无执行日志</span>
      </div>

      <div
        v-else
        class="log-entries"
      >
        <ExecutionTimeline :entries="timelineEntries" />
      </div>

      <!-- 运行指示器 -->
      <div
        v-if="isRunning"
        class="running-indicator"
      >
        <span class="indicator-dot" />
        <span class="indicator-text">AI 正在处理...</span>
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

.btn-stop,
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
