<script setup lang="ts">
import DynamicForm from '@/components/plan/DynamicForm.vue'
import type { TimelineEntry } from '@/types/timeline'
import StructuredContentRenderer from './StructuredContentRenderer.vue'
import ThinkingDisplay from './ThinkingDisplay.vue'
import ToolCallDisplay from './ToolCallDisplay.vue'

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
        v-else-if="entry.type === 'thinking' && entry.content"
        :thinking="entry.content"
      />

      <ToolCallDisplay
        v-else-if="entry.type === 'tool' && entry.toolCall"
        :tool-call="entry.toolCall"
      />

      <div
        v-else-if="entry.type === 'form' && entry.formSchema"
        class="timeline-form"
      >
        <div
          class="timeline-form__content"
          :class="{ 'timeline-form__content--disabled': entry.formDisabled }"
        >
          <DynamicForm
            :schema="entry.formSchema"
            @submit="emit('form-submit', entry.id, $event)"
            @cancel="emit('form-cancel', entry.id)"
          />
        </div>
      </div>

      <div
        v-else-if="entry.content"
        class="timeline-entry"
        :class="`timeline-entry--${entry.type}`"
      >
        <StructuredContentRenderer :content="entry.content" />
      </div>
    </template>
  </div>
</template>

<style scoped>
.execution-timeline {
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
  max-width: min(100%, 44rem);
  border-radius: 1rem;
  padding: 0.875rem 1rem;
  background: rgba(248, 250, 252, 0.92);
  border: 1px solid rgba(148, 163, 184, 0.22);
  box-shadow: 0 10px 24px rgba(15, 23, 42, 0.05);
}

.timeline-message--user .timeline-message__content {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.12), rgba(14, 165, 233, 0.08));
  border-color: rgba(59, 130, 246, 0.22);
}

.timeline-message__text {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.65;
  color: var(--color-text-primary);
}

.timeline-entry {
  border-radius: 0.95rem;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background: rgba(248, 250, 252, 0.82);
  padding: 0.875rem 1rem;
}

.timeline-entry--error {
  border-color: rgba(239, 68, 68, 0.28);
  background: rgba(254, 242, 242, 0.88);
}

.timeline-entry--system {
  border-color: rgba(14, 165, 233, 0.2);
  background: rgba(239, 246, 255, 0.9);
}

.timeline-form {
  display: flex;
}

.timeline-form__content {
  width: min(100%, 44rem);
}

.timeline-form__content--disabled {
  opacity: 0.72;
  pointer-events: none;
}

[data-theme='dark'] .timeline-message__content {
  background: rgba(15, 23, 42, 0.8);
  border-color: rgba(148, 163, 184, 0.18);
  box-shadow: 0 12px 28px rgba(2, 6, 23, 0.28);
}

[data-theme='dark'] .timeline-message--user .timeline-message__content {
  background: linear-gradient(135deg, rgba(37, 99, 235, 0.28), rgba(14, 165, 233, 0.16));
  border-color: rgba(96, 165, 250, 0.24);
}

[data-theme='dark'] .timeline-message__text {
  color: #f8fafc;
}

[data-theme='dark'] .timeline-entry {
  border-color: rgba(148, 163, 184, 0.18);
  background: rgba(15, 23, 42, 0.76);
}

[data-theme='dark'] .timeline-entry--error {
  border-color: rgba(248, 113, 113, 0.26);
  background: rgba(69, 10, 10, 0.34);
}

[data-theme='dark'] .timeline-entry--system {
  border-color: rgba(56, 189, 248, 0.22);
  background: rgba(12, 74, 110, 0.28);
}
</style>
