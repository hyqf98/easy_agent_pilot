<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted } from 'vue'
import { useTaskExecutionStore } from '@/stores/taskExecution'
import { useTaskStore } from '@/stores/task'
import ExecutionTimeline from '@/components/message/ExecutionTimeline.vue'
import DynamicForm from '@/components/plan/DynamicForm.vue'
import type { TimelineEntry } from '@/types/timeline'
import { buildToolCallFromLogs } from '@/utils/toolCallLog'

const props = defineProps<{
  taskId: string
}>()

const taskExecutionStore = useTaskExecutionStore()
const taskStore = useTaskStore()

// 日志容器引用
const logContainerRef = ref<HTMLElement | null>(null)

// 是否自动滚动
const autoScroll = ref(true)

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

// 是否正在执行
const isRunning = computed(() => {
  return executionState.value?.status === 'running'
})

// 是否等待用户输入
const isWaitingInput = computed(() => {
  return task.value?.status === 'blocked' && task.value?.blockReason === 'waiting_input'
})

// 执行状态文本
const statusText = computed(() => {
  const status = executionState.value?.status
  switch (status) {
    case 'idle': return '等待执行'
    case 'queued': return '排队中'
    case 'running': return '执行中'
    case 'waiting_input': return '等待输入'
    case 'completed': return '执行完成'
    case 'failed': return '执行失败'
    case 'stopped': return '已停止'
    default: return ''
  }
})

// 状态颜色
const statusColor = computed(() => {
  const status = executionState.value?.status
  switch (status) {
    case 'running': return 'primary'
    case 'queued': return 'warning'
    case 'waiting_input': return 'warning'
    case 'completed': return 'success'
    case 'failed': return 'error'
    case 'stopped': return 'gray'
    default: return 'gray'
  }
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
  return logs.value.reduce<TimelineEntry[]>((entries, log) => {
    if (log.type === 'tool_result') {
      return entries
    }

    if (log.type === 'tool_use') {
      const toolCall = buildToolCallFromLogs(log, logs.value)
      if (toolCall) {
        entries.push({
          id: `tool-${log.id}`,
          type: 'tool',
          toolCall,
          timestamp: log.timestamp
        })
      }
      return entries
    }

    entries.push({
      id: `entry-${log.id}`,
      type:
        log.type === 'content'
          ? 'content'
          : log.type === 'thinking'
            ? 'thinking'
            : log.type === 'error'
              ? 'error'
              : 'system',
      content: log.content,
      timestamp: log.timestamp
    })
    return entries
  }, [])
})

// 加载历史日志
onMounted(async () => {
  await taskExecutionStore.loadTaskLogs(props.taskId)
  scrollToBottom()
})

watch(
  () => props.taskId,
  async (taskId) => {
    await taskExecutionStore.loadTaskLogs(taskId)
    scrollToBottom()
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
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: var(--color-surface, #fff);
  border-radius: var(--radius-lg, 12px);
  overflow: hidden;
}

.log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3, 0.75rem) var(--spacing-4, 1rem);
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  background-color: var(--color-bg-secondary, #f8fafc);
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
  background-color: var(--color-bg-tertiary, #f1f5f9);
  color: var(--color-text-secondary, #64748b);
}

.btn-clear:hover {
  background-color: var(--color-bg-secondary, #e2e8f0);
}

.log-content {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-3, 0.75rem);
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
  background-color: var(--color-primary-light, #eff6ff);
  border-radius: var(--radius-sm, 4px);
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
  background-color: var(--color-warning-light, #fef3c7);
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
</style>
