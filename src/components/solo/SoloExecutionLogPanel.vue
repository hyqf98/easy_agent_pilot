<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import DynamicForm from '@/components/plan/dynamicForm/DynamicForm.vue'
import ExecutionTimeline from '@/components/message/ExecutionTimeline.vue'
import { useSoloExecutionStore } from '@/stores/soloExecution'
import { useSoloRunStore } from '@/stores/soloRun'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import type { SoloLogEntry, SoloStep } from '@/types/solo'
import type { TimelineEntry } from '@/types/timeline'
import { buildToolCallMapFromLogs } from '@/utils/toolCallLog'
import { formatTokenCount } from '@/stores/token'

const props = defineProps<{
  runId: string
  stepId?: string | null
}>()

const soloExecutionStore = useSoloExecutionStore()
const soloRunStore = useSoloRunStore()
const agentTeamsStore = useAgentTeamsStore()

const scrollerRef = ref<HTMLElement | null>(null)
const autoScroll = ref(true)

const run = computed(() => soloRunStore.runs.find((item) => item.id === props.runId) || null)
const state = computed(() => soloExecutionStore.getExecutionState(props.runId))
const allLogs = computed(() => state.value?.logs ?? [])
const steps = computed(() => soloExecutionStore.getSteps(props.runId))
const selectedStep = computed<SoloStep | null>(() =>
  props.stepId ? steps.value.find((step) => step.id === props.stepId) || null : null
)
const selectedExpertLabel = computed(() => {
  const expertId = selectedStep.value?.selectedExpertId
  if (!expertId) return '未指定'
  return agentTeamsStore.experts.find((expert) => expert.id === expertId)?.name || expertId
})

const visibleLogs = computed<SoloLogEntry[]>(() => {
  if (props.stepId) {
    return allLogs.value.filter((log) => log.stepId === props.stepId)
  }

  return allLogs.value.filter((log) => !log.stepId && log.scope !== 'step')
})

const tokenUsage = computed(() => state.value?.tokenUsage ?? {
  inputTokens: 0,
  outputTokens: 0,
  model: undefined,
  resetCount: 0,
  lastUpdatedAt: null
})

const tokenTotal = computed(() => tokenUsage.value.inputTokens + tokenUsage.value.outputTokens)

const pendingInputVisible = computed(() => {
  if (!run.value?.inputRequest) return false
  if (!props.stepId) return !run.value.inputRequest.stepId
  return run.value.inputRequest.stepId === props.stepId
})

const panelTitle = computed(() => selectedStep.value ? '执行日志流程' : '协调日志流程')
const panelSubtitle = computed(() => {
  if (selectedStep.value) {
    return selectedStep.value.title
  }
  return '内置协调 AI 的调度决策与状态回写'
})

const timelineEntries = computed<TimelineEntry[]>(() => {
  const toolCallMap = buildToolCallMapFromLogs(visibleLogs.value, {
    fallbackStatus: run.value?.executionStatus === 'running' ? 'running' : 'success'
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
        animate: run.value?.executionStatus === 'running'
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
          animate: run.value?.executionStatus === 'running'
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
          animate: run.value?.executionStatus === 'running'
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
        timestamp: log.timestamp
      })
      lastThinkingEntry = null
      lastContentEntry = null
      return entries
    }

    entries.push({
      id: `system-${log.id}`,
      type: 'system',
      content: log.content,
      timestamp: log.timestamp
    })
    lastThinkingEntry = null
    lastContentEntry = null
    return entries
  }, [])
})

function handleScroll() {
  if (!scrollerRef.value) return
  const { scrollTop, scrollHeight, clientHeight } = scrollerRef.value
  autoScroll.value = scrollHeight - scrollTop - clientHeight < 36
}

async function handleSubmit(values: Record<string, unknown>) {
  await soloExecutionStore.submitRunInput(props.runId, values)
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
  await soloExecutionStore.loadLogs(props.runId)
})
</script>

<template>
  <div class="solo-log-panel">
    <div class="solo-log-panel__header">
      <div>
        <p class="solo-log-panel__eyebrow">
          {{ selectedStep ? 'Step Detail' : 'Coordinator Detail' }}
        </p>
        <h3>{{ panelTitle }}</h3>
        <p class="solo-log-panel__subtitle">
          {{ panelSubtitle }}
        </p>
      </div>
      <div class="solo-log-panel__chips">
        <span class="solo-log-panel__chip">{{ run?.status || 'idle' }}</span>
        <span
          v-if="selectedStep"
          class="solo-log-panel__chip solo-log-panel__chip--step"
        >
          depth {{ selectedStep.depth }}
        </span>
      </div>
    </div>

    <div
      v-if="selectedStep"
      class="solo-log-panel__summary"
    >
      <div class="solo-log-panel__summary-row">
        <span>专家视角</span>
        <strong>{{ selectedExpertLabel }}</strong>
      </div>
      <p class="solo-log-panel__summary-text">
        {{ selectedStep.resultSummary || selectedStep.summary || selectedStep.description || '等待本步输出内容。' }}
      </p>
      <div
        v-if="selectedStep.resultFiles.length > 0"
        class="solo-log-panel__files"
      >
        <span
          v-for="file in selectedStep.resultFiles"
          :key="file"
        >
          {{ file }}
        </span>
      </div>
    </div>

    <div class="solo-log-panel__usage">
      <div class="solo-log-panel__usage-main">
        <span class="solo-log-panel__usage-label">Model</span>
        <strong>{{ tokenUsage.model || run?.coordinatorModelId || 'default' }}</strong>
      </div>
      <div class="solo-log-panel__usage-stats">
        <span>输入 {{ formatTokenCount(tokenUsage.inputTokens) }}</span>
        <span>输出 {{ formatTokenCount(tokenUsage.outputTokens) }}</span>
        <span>总计 {{ formatTokenCount(tokenTotal) }}</span>
      </div>
    </div>

    <div
      v-if="pendingInputVisible && run?.inputRequest"
      class="solo-log-panel__form"
    >
      <div class="solo-log-panel__form-header">
        <span>等待补充输入</span>
        <small>{{ run.inputRequest.source === 'execution' ? '步骤执行' : '协调决策' }}</small>
      </div>
      <p
        v-if="run.inputRequest.question"
        class="solo-log-panel__form-question"
      >
        {{ run.inputRequest.question }}
      </p>
      <DynamicForm
        :schema="run.inputRequest.formSchema"
        @submit="handleSubmit"
      />
    </div>

    <div
      ref="scrollerRef"
      class="solo-log-panel__timeline"
      @scroll="handleScroll"
    >
      <div
        v-if="timelineEntries.length === 0"
        class="solo-log-panel__empty"
      >
        <p>当前还没有日志。</p>
        <span>{{ selectedStep ? '选中步骤开始执行后，这里会出现完整日志流程。' : '协调 AI 开始派发步骤后，这里会出现调度日志。' }}</span>
      </div>
      <ExecutionTimeline
        v-else
        :entries="timelineEntries"
        :group-tool-calls="true"
      />
    </div>
  </div>
</template>

<style scoped>
.solo-log-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
  width: 100%;
  background: linear-gradient(180deg, color-mix(in srgb, var(--color-surface) 96%, white 4%) 0%, var(--color-surface) 100%);
  overflow: hidden;
}

.solo-log-panel__header,
.solo-log-panel__usage,
.solo-log-panel__summary {
  padding: 12px 14px;
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
}

.solo-log-panel__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.solo-log-panel__eyebrow {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.24em;
  text-transform: uppercase;
  color: var(--color-primary);
}

.solo-log-panel__header h3 {
  margin: 6px 0 0;
  font-size: 17px;
  color: var(--color-text-primary);
}

.solo-log-panel__subtitle {
  margin: 4px 0 0;
  font-size: 11px;
  line-height: 1.45;
  color: var(--color-text-secondary);
}

.solo-log-panel__chips {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}

.solo-log-panel__chip {
  display: inline-flex;
  align-items: center;
  padding: 5px 9px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-surface-hover) 86%, transparent);
  color: var(--color-text-secondary);
  font-size: 11px;
}

.solo-log-panel__chip--step {
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.solo-log-panel__summary {
  display: flex;
  flex-direction: column;
  gap: 6px;
  background: color-mix(in srgb, var(--color-primary) 5%, transparent);
}

.solo-log-panel__summary-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: var(--color-text-secondary);
  font-size: 12px;
}

.solo-log-panel__summary-text {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
  color: var(--color-text-primary);
  white-space: pre-wrap;
}

.solo-log-panel__files {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.solo-log-panel__files span {
  display: inline-flex;
  align-items: center;
  padding: 5px 9px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-surface-hover) 88%, transparent);
  color: var(--color-text-secondary);
  font-size: 11px;
}

.solo-log-panel__usage {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.solo-log-panel__usage-main {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: var(--color-text-primary);
}

.solo-log-panel__usage-label {
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.solo-log-panel__usage-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 11px;
  color: var(--color-text-secondary);
}

.solo-log-panel__form {
  padding: 12px 14px 8px;
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
  background: color-mix(in srgb, #f59e0b 8%, transparent);
}

.solo-log-panel__form-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.solo-log-panel__form-header small,
.solo-log-panel__form-question {
  color: var(--color-text-secondary);
}

.solo-log-panel__form-question {
  margin: 0 0 12px;
  font-size: 13px;
  line-height: 1.6;
}

.solo-log-panel__timeline {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 12px 14px 16px;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--color-primary) 28%, transparent) transparent;
}

.solo-log-panel__empty {
  padding: 18px;
  border-radius: 20px;
  background: color-mix(in srgb, var(--color-surface-hover) 88%, transparent);
  text-align: center;
}

.solo-log-panel__empty p {
  margin: 0 0 8px;
  color: var(--color-text-primary);
}

.solo-log-panel__empty span {
  font-size: 13px;
  line-height: 1.6;
  color: var(--color-text-secondary);
}

[data-theme='dark'] .solo-log-panel {
  background: linear-gradient(180deg, color-mix(in srgb, var(--color-surface) 88%, #020617 12%) 0%, color-mix(in srgb, var(--color-surface) 94%, #020617 6%) 100%);
}

[data-theme='dark'] .solo-log-panel__header,
[data-theme='dark'] .solo-log-panel__usage,
[data-theme='dark'] .solo-log-panel__summary,
[data-theme='dark'] .solo-log-panel__form {
  border-bottom-color: color-mix(in srgb, var(--color-border) 54%, rgba(148, 163, 184, 0.18) 46%);
}

[data-theme='dark'] .solo-log-panel__chip,
[data-theme='dark'] .solo-log-panel__files span {
  background: color-mix(in srgb, var(--color-surface-hover) 54%, rgba(15, 23, 42, 0.46) 46%);
  color: color-mix(in srgb, var(--color-text-secondary) 90%, white 10%);
}

[data-theme='dark'] .solo-log-panel__summary {
  background: color-mix(in srgb, var(--color-primary) 10%, rgba(15, 23, 42, 0.32) 90%);
}

[data-theme='dark'] .solo-log-panel__form {
  background: color-mix(in srgb, #f59e0b 12%, rgba(120, 53, 15, 0.16));
}

[data-theme='dark'] .solo-log-panel__empty {
  background: color-mix(in srgb, var(--color-surface-hover) 48%, rgba(15, 23, 42, 0.54) 52%);
}

.solo-log-panel__timeline::-webkit-scrollbar {
  width: 10px;
}

.solo-log-panel__timeline::-webkit-scrollbar-thumb {
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 28%, transparent);
}

.solo-log-panel__timeline::-webkit-scrollbar-track {
  background: transparent;
}
</style>
