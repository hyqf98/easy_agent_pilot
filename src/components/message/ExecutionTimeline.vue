<script setup lang="ts">
import DynamicForm from '@/components/plan/DynamicForm.vue'
import type { TimelineEntry } from '@/types/timeline'
import StructuredContentRenderer from './StructuredContentRenderer.vue'
import ThinkingDisplay from './ThinkingDisplay.vue'
import ToolCallDisplay from './ToolCallDisplay.vue'
import MarkdownRenderer from './MarkdownRenderer.vue'
import RuntimeNoticeList from './RuntimeNoticeList.vue'
import { buildRuntimeNoticeFromSystemContent } from '@/utils/runtimeNotice'
import type { ToolCall } from '@/stores/message'

defineProps<{
  entries: TimelineEntry[]
}>()

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
</script>

<template>
  <div class="execution-timeline">
    <template
      v-for="entry in entries"
      :key="entry.id"
    >
      <div
        v-if="entry.type === 'message'"
        class="timeline-message"
        :class="`timeline-message--${entry.role || 'assistant'}`"
      >
        <div class="timeline-message__content">
          <StructuredContentRenderer
            v-if="entry.role !== 'user'"
            :content="entry.content || ''"
            :interactive-forms="entry.role === 'assistant'"
            :animate="entry.animate"
            @form-submit="handleMessageFormSubmit"
            @form-cancel="handleMessageFormCancel"
          />
          <p
            v-else
            class="timeline-message__text"
          >
            {{ entry.content || '' }}
          </p>
        </div>
      </div>

      <ThinkingDisplay
        v-else-if="entry.type === 'thinking'"
        :thinking="entry.content || ''"
        :live="entry.animate"
        :default-expanded="false"
      />

      <ToolCallDisplay
        v-else-if="entry.type === 'tool' && entry.toolCall"
        :key="getToolCallRenderKey(entry.toolCall)"
        :tool-call="entry.toolCall"
        :live="entry.animate"
        :compact="entry.toolCompact"
        :default-expanded="entry.toolDefaultExpanded ?? false"
        :default-result-expanded="entry.toolDefaultResultExpanded ?? false"
      />

      <div
        v-else-if="entry.type === 'content' && entry.content"
        class="timeline-message timeline-message--assistant"
      >
        <div class="timeline-message__content">
          <MarkdownRenderer
            :content="entry.content"
            :animate="entry.animate"
          />
        </div>
      </div>

      <div
        v-else-if="entry.type === 'form' && entry.formSchema"
        class="timeline-form"
        :class="`timeline-form--${entry.formVariant || 'active'}`"
      >
        <div
          class="timeline-form__content"
          :class="{
            'timeline-form__content--disabled': entry.formDisabled,
            'timeline-form__content--submitted': entry.formVariant === 'submitted',
            'timeline-form__content--active': (entry.formVariant || 'active') === 'active'
          }"
        >
          <DynamicForm
            :schema="entry.formSchema"
            :question="entry.formPrompt"
            :initial-values="entry.formInitialValues"
            :disabled="entry.formDisabled"
            :variant="entry.formVariant || 'active'"
            @submit="emit('form-submit', entry.id, $event)"
            @cancel="emit('form-cancel', entry.id)"
          />
        </div>
      </div>

      <div
        v-else-if="entry.type === 'system' && entry.content && toRuntimeNotices(entry.content).length > 0"
        class="timeline-runtime"
      >
        <RuntimeNoticeList :notices="toRuntimeNotices(entry.content)" />
      </div>

      <div
        v-else-if="entry.content"
        class="timeline-entry"
        :class="`timeline-entry--${entry.type}`"
      >
        <StructuredContentRenderer
          :content="entry.content"
          :animate="entry.animate"
        />
      </div>
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

:global([data-theme='dark']) .timeline-message__text,
:global(.dark) .timeline-message__text {
  color: #f8fafc;
}
</style>
