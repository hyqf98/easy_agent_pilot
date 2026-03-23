<script setup lang="ts">
import { computed } from 'vue'
import DynamicForm from '@/components/plan/DynamicForm.vue'
import type { TimelineEntry } from '@/types/timeline'
import StructuredContentRenderer from './StructuredContentRenderer.vue'
import ThinkingDisplay from './ThinkingDisplay.vue'
import ToolCallDisplay from './ToolCallDisplay.vue'
import MarkdownRenderer from './MarkdownRenderer.vue'
import RuntimeNoticeList from './RuntimeNoticeList.vue'
import { buildRuntimeNoticeFromSystemContent } from '@/utils/runtimeNotice'
import type { ToolCall } from '@/stores/message'

const props = withDefaults(defineProps<{
  entries: TimelineEntry[]
  groupToolCalls?: boolean
}>(), {
  groupToolCalls: false
})

const emit = defineEmits<{
  (e: 'form-submit', entryId: string, values: Record<string, unknown>): void
  (e: 'form-cancel', entryId: string): void
  (e: 'message-form-submit', formId: string, values: Record<string, unknown>): void
  (e: 'message-form-cancel', formId: string): void
}>()

function handleMessageFormSubmit(formId: string, values: Record<string, unknown>) {
  emit('message-form-submit', formId, values)
}

function handleMessageFormCancel(formId: string) {
  emit('message-form-cancel', formId)
}

function toRuntimeNotices(content?: string) {
  const notice = buildRuntimeNoticeFromSystemContent(content)
  return notice ? [notice] : []
}

function getToolCallRenderKey(toolCall: ToolCall) {
  const argumentsSignature = JSON.stringify(toolCall.arguments ?? {})
  return [
    toolCall.id,
    toolCall.status,
    argumentsSignature,
    toolCall.result?.length ?? 0,
    toolCall.errorMessage?.length ?? 0
  ].join(':')
}

interface TimelineRenderBlockEntry {
  kind: 'entry'
  key: string
  entry: TimelineEntry
}

interface TimelineRenderBlockToolGroup {
  kind: 'tool-group'
  key: string
  entries: TimelineEntry[]
}

type TimelineRenderBlock = TimelineRenderBlockEntry | TimelineRenderBlockToolGroup

function getToolGroupKey(entries: TimelineEntry[]) {
  const firstId = entries[0]?.id ?? 'start'
  const lastId = entries[entries.length - 1]?.id ?? 'end'
  return `tool-group:${firstId}:${lastId}:${entries.length}`
}

function shouldClampToolGroup(entries: TimelineEntry[]) {
  return entries.length > 10
}

const renderBlocks = computed<TimelineRenderBlock[]>(() => {
  if (!props.groupToolCalls) {
    return props.entries.map(entry => ({
      kind: 'entry',
      key: entry.id,
      entry
    }))
  }

  const blocks: TimelineRenderBlock[] = []
  let pendingToolEntries: TimelineEntry[] = []

  const flushPendingTools = () => {
    if (pendingToolEntries.length === 0) {
      return
    }

    blocks.push({
      kind: 'tool-group',
      key: getToolGroupKey(pendingToolEntries),
      entries: [...pendingToolEntries]
    })
    pendingToolEntries = []
  }

  for (const entry of props.entries) {
    if (entry.type === 'tool' && entry.toolCall) {
      pendingToolEntries.push(entry)
      continue
    }

    flushPendingTools()
    blocks.push({
      kind: 'entry',
      key: entry.id,
      entry
    })
  }

  flushPendingTools()
  return blocks
})
</script>

<template>
  <div class="execution-timeline">
    <template
      v-for="block in renderBlocks"
      :key="block.key"
    >
      <div
        v-if="block.kind === 'tool-group'"
        class="execution-timeline__tool-calls-shell"
        :class="{ 'execution-timeline__tool-calls-shell--scrollable': shouldClampToolGroup(block.entries) }"
      >
        <div class="execution-timeline__tool-calls-head">
          <span class="execution-timeline__tool-calls-title">工具调用</span>
          <span class="execution-timeline__tool-calls-count">{{ block.entries.length }}</span>
        </div>
        <div class="execution-timeline__tool-calls">
          <ToolCallDisplay
            v-for="toolEntry in block.entries"
            :key="getToolCallRenderKey(toolEntry.toolCall!)"
            :tool-call="toolEntry.toolCall!"
            :live="toolEntry.animate"
            :compact="toolEntry.toolCompact"
            :default-expanded="toolEntry.toolDefaultExpanded ?? false"
            :default-result-expanded="toolEntry.toolDefaultResultExpanded ?? false"
          />
        </div>
      </div>

      <template v-else>
        <div
          v-if="block.entry.type === 'message'"
          class="timeline-message"
          :class="`timeline-message--${block.entry.role || 'assistant'}`"
        >
          <div class="timeline-message__content">
            <StructuredContentRenderer
              v-if="block.entry.role !== 'user'"
              :content="block.entry.content || ''"
              :interactive-forms="block.entry.role === 'assistant'"
              :animate="block.entry.animate"
              @form-submit="handleMessageFormSubmit"
              @form-cancel="handleMessageFormCancel"
            />
            <p
              v-else
              class="timeline-message__text"
            >
              {{ block.entry.content || '' }}
            </p>
          </div>
        </div>

        <ThinkingDisplay
          v-else-if="block.entry.type === 'thinking'"
          :thinking="block.entry.content || ''"
          :live="block.entry.animate"
          :default-expanded="false"
        />

        <ToolCallDisplay
          v-else-if="block.entry.type === 'tool' && block.entry.toolCall"
          :key="getToolCallRenderKey(block.entry.toolCall)"
          :tool-call="block.entry.toolCall"
          :live="block.entry.animate"
          :compact="block.entry.toolCompact"
          :default-expanded="block.entry.toolDefaultExpanded ?? false"
          :default-result-expanded="block.entry.toolDefaultResultExpanded ?? false"
        />

        <div
          v-else-if="block.entry.type === 'content' && block.entry.content"
          class="timeline-message timeline-message--assistant"
        >
          <div class="timeline-message__content">
            <MarkdownRenderer
              :content="block.entry.content"
              :animate="block.entry.animate"
            />
          </div>
        </div>

        <div
          v-else-if="block.entry.type === 'form' && block.entry.formSchema"
          class="timeline-form"
          :class="`timeline-form--${block.entry.formVariant || 'active'}`"
        >
          <div
            class="timeline-form__content"
            :class="{
              'timeline-form__content--disabled': block.entry.formDisabled,
              'timeline-form__content--submitted': block.entry.formVariant === 'submitted',
              'timeline-form__content--active': (block.entry.formVariant || 'active') === 'active'
            }"
          >
            <DynamicForm
              :schema="block.entry.formSchema"
              :question="block.entry.formPrompt"
              :initial-values="block.entry.formInitialValues"
              :disabled="block.entry.formDisabled"
              :variant="block.entry.formVariant || 'active'"
              @submit="emit('form-submit', block.entry.id, $event)"
              @cancel="emit('form-cancel', block.entry.id)"
            />
          </div>
        </div>

        <div
          v-else-if="block.entry.type === 'system' && block.entry.content && toRuntimeNotices(block.entry.content).length > 0"
          class="timeline-runtime"
        >
          <RuntimeNoticeList :notices="toRuntimeNotices(block.entry.content)" />
        </div>

        <div
          v-else-if="block.entry.content"
          class="timeline-entry"
          :class="`timeline-entry--${block.entry.type}`"
        >
          <StructuredContentRenderer
            :content="block.entry.content"
            :animate="block.entry.animate"
          />
        </div>
      </template>
    </template>
  </div>
</template>

<style scoped>
.execution-timeline {
  --timeline-entry-width: var(--timeline-panel-width, min(100%, clamp(18rem, 40%, 28rem)));
  --timeline-content-max-width: var(--timeline-panel-max-width, min(100%, clamp(18rem, 52%, 34rem)));
  --thinking-display-width: var(--timeline-entry-width);
  --timeline-bubble-bg: rgba(248, 250, 252, 0.92);
  --timeline-bubble-border: rgba(148, 163, 184, 0.22);
  --timeline-bubble-shadow: 0 10px 24px rgba(15, 23, 42, 0.05);
  --timeline-user-bubble-bg: linear-gradient(135deg, rgba(59, 130, 246, 0.12), rgba(14, 165, 233, 0.08));
  --timeline-user-bubble-border: rgba(59, 130, 246, 0.22);
  --timeline-entry-bg: rgba(248, 250, 252, 0.82);
  --timeline-entry-border: rgba(148, 163, 184, 0.18);
  --timeline-entry-error-bg: rgba(254, 242, 242, 0.88);
  --timeline-entry-error-border: rgba(239, 68, 68, 0.28);
  --timeline-entry-system-bg: rgba(239, 246, 255, 0.9);
  --timeline-entry-system-border: rgba(14, 165, 233, 0.2);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.execution-timeline__tool-calls-shell {
  --tool-call-shell-max-height: var(--timeline-tool-call-shell-max-height, min(40rem, calc(3.35rem * 10 + var(--spacing-2) * 9 + 3rem)));
  --tool-call-shell-border: rgba(249, 115, 22, 0.2);
  --tool-call-shell-bg: linear-gradient(180deg, rgba(255, 247, 237, 0.96), rgba(255, 255, 255, 0.96));
  --tool-call-shell-shadow: 0 10px 24px rgba(249, 115, 22, 0.08);
  --tool-call-shell-title: #c2410c;
  --tool-call-shell-count-bg: rgba(249, 115, 22, 0.12);
  --tool-call-shell-count-text: #9a3412;
  --tool-call-shell-scrollbar-thumb: rgba(249, 115, 22, 0.36);
  width: var(--timeline-entry-width);
  max-width: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: 0.75rem;
  border-radius: 1rem;
  border: 1px solid var(--tool-call-shell-border);
  background: var(--tool-call-shell-bg);
  box-shadow: var(--tool-call-shell-shadow);
  overflow: hidden;
}

.execution-timeline__tool-calls-shell--scrollable {
  max-height: var(--tool-call-shell-max-height);
}

.execution-timeline__tool-calls-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0 0.1rem;
}

.execution-timeline__tool-calls-title {
  font-size: 0.74rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--tool-call-shell-title);
}

.execution-timeline__tool-calls-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.5rem;
  height: 1.5rem;
  padding: 0 0.4rem;
  border-radius: 999px;
  background: var(--tool-call-shell-count-bg);
  color: var(--tool-call-shell-count-text);
  font-size: 0.72rem;
  font-weight: 700;
}

.execution-timeline__tool-calls {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  min-height: 0;
  overflow: visible;
}

.execution-timeline__tool-calls-shell--scrollable .execution-timeline__tool-calls {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  padding-right: 0.2rem;
  scrollbar-gutter: stable;
  overscroll-behavior: contain;
  scrollbar-width: thin;
  scrollbar-color: var(--tool-call-shell-scrollbar-thumb) transparent;
}

.execution-timeline__tool-calls-shell--scrollable .execution-timeline__tool-calls::-webkit-scrollbar {
  width: 6px;
}

.execution-timeline__tool-calls-shell--scrollable .execution-timeline__tool-calls::-webkit-scrollbar-track {
  background: transparent;
}

.execution-timeline__tool-calls-shell--scrollable .execution-timeline__tool-calls::-webkit-scrollbar-thumb {
  background: var(--tool-call-shell-scrollbar-thumb);
  border-radius: 999px;
}

.timeline-message {
  display: flex;
}

.timeline-message--user {
  justify-content: flex-end;
}

.timeline-message--assistant,
.timeline-message--system {
  justify-content: flex-start;
}

.timeline-message__content {
  width: fit-content;
  min-width: var(--timeline-entry-width);
  max-width: var(--timeline-content-max-width);
  border-radius: 1rem;
  padding: 0.875rem 1rem;
  background: var(--timeline-bubble-bg);
  border: 1px solid var(--timeline-bubble-border);
  box-shadow: var(--timeline-bubble-shadow);
}

.timeline-message--user .timeline-message__content {
  background: var(--timeline-user-bubble-bg);
  border-color: var(--timeline-user-bubble-border);
}

.timeline-message__text {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.65;
  color: var(--color-text-primary);
}

.timeline-entry {
  align-self: flex-start;
  width: var(--timeline-entry-width);
  max-width: 100%;
  border-radius: 0.95rem;
  border: 1px solid var(--timeline-entry-border);
  background: var(--timeline-entry-bg);
  padding: 0.875rem 1rem;
}

.timeline-entry--error {
  border-color: var(--timeline-entry-error-border);
  background: var(--timeline-entry-error-bg);
}

.timeline-entry--system {
  border-color: var(--timeline-entry-system-border);
  background: var(--timeline-entry-system-bg);
}

.timeline-form {
  display: flex;
  justify-content: flex-start;
}

.timeline-runtime {
  width: var(--timeline-entry-width);
  max-width: 100%;
}

.timeline-form__content {
  width: var(--timeline-entry-width);
  max-width: 100%;
  transition: transform 0.18s ease, opacity 0.18s ease, box-shadow 0.18s ease;
}

.timeline-form__content--active {
  filter: saturate(1.02);
}

.timeline-form__content--submitted {
  opacity: 0.92;
  transform: none;
}

.timeline-form__content--disabled {
  opacity: 0.72;
}

:global([data-theme='dark']) .execution-timeline,
:global(.dark) .execution-timeline {
  --timeline-bubble-bg: rgba(15, 23, 42, 0.8);
  --timeline-bubble-border: rgba(148, 163, 184, 0.18);
  --timeline-bubble-shadow: 0 12px 28px rgba(2, 6, 23, 0.28);
  --timeline-user-bubble-bg: linear-gradient(135deg, rgba(37, 99, 235, 0.28), rgba(14, 165, 233, 0.16));
  --timeline-user-bubble-border: rgba(96, 165, 250, 0.24);
  --timeline-entry-bg: rgba(15, 23, 42, 0.76);
  --timeline-entry-border: rgba(148, 163, 184, 0.18);
  --timeline-entry-error-bg: rgba(69, 10, 10, 0.34);
  --timeline-entry-error-border: rgba(248, 113, 113, 0.26);
  --timeline-entry-system-bg: rgba(12, 74, 110, 0.28);
  --timeline-entry-system-border: rgba(56, 189, 248, 0.22);
}

:global([data-theme='dark']) .execution-timeline__tool-calls-shell,
:global(.dark) .execution-timeline__tool-calls-shell {
  --tool-call-shell-border: rgba(249, 115, 22, 0.26);
  --tool-call-shell-bg: linear-gradient(180deg, rgba(67, 20, 7, 0.7), rgba(15, 23, 42, 0.92));
  --tool-call-shell-shadow: 0 12px 28px rgba(2, 6, 23, 0.28);
  --tool-call-shell-title: #fdba74;
  --tool-call-shell-count-bg: rgba(249, 115, 22, 0.18);
  --tool-call-shell-count-text: #fed7aa;
  --tool-call-shell-scrollbar-thumb: rgba(251, 146, 60, 0.42);
}

:global([data-theme='dark']) .timeline-message__text,
:global(.dark) .timeline-message__text {
  color: #f8fafc;
}
</style>
