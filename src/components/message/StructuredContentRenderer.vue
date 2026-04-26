<script setup lang="ts">
import { computed, toRef } from 'vue'
import DynamicForm from '@/components/plan/dynamicForm/DynamicForm.vue'
import { parseStructuredContent } from '@/utils/structuredContent'
import { useTypewriterText } from '@/composables/useTypewriterText'
import MarkdownRenderer from './MarkdownRenderer.vue'
import StructuredResultCard from './StructuredResultCard.vue'

const props = withDefaults(defineProps<{
  content: string
  interactiveForms?: boolean
  formDisabled?: boolean
  animate?: boolean
  resolvedFormValues?: Record<string, unknown> | null
  resolvedFormValuesByFormId?: Record<string, Record<string, unknown>> | null
}>(), {
  interactiveForms: false,
  formDisabled: false,
  animate: false,
  resolvedFormValues: null,
  resolvedFormValuesByFormId: null
})

const emit = defineEmits<{
  (e: 'form-submit', formId: string, values: Record<string, unknown>): void
  (e: 'form-cancel', formId: string): void
}>()

const { displayedText } = useTypewriterText(
  toRef(props, 'content'),
  toRef(props, 'animate'),
  { charsPerSecond: 140, maxChunkSize: 24 }
)

const blocks = computed(() => parseStructuredContent(displayedText.value))
const isFormOnly = computed(() =>
  blocks.value.length > 0 && blocks.value.every(block => block.type === 'form')
)

function getResolvedFormValues(formId: string): Record<string, unknown> | null {
  return props.resolvedFormValuesByFormId?.[formId] ?? props.resolvedFormValues ?? null
}

function isFormResolved(formId: string): boolean {
  return Boolean(getResolvedFormValues(formId))
}

function isFormDisabled(formId: string): boolean {
  return !props.interactiveForms || props.formDisabled || isFormResolved(formId)
}

function handleFormSubmit(formId: string, values: Record<string, unknown>) {
  emit('form-submit', formId, values)
}

function handleFormCancel(formId: string) {
  emit('form-cancel', formId)
}
</script>

<template>
  <div
    class="structured-content"
    :class="{ 'structured-content--form-only': isFormOnly }"
  >
    <template
      v-for="(block, index) in blocks"
      :key="`${block.type}-${index}`"
    >
      <MarkdownRenderer
        v-if="block.type === 'markdown'"
        :content="block.content"
        :animate="false"
      />

      <div
        v-else-if="block.type === 'result'"
        class="structured-content__result"
      >
        <StructuredResultCard :result="block.result" />
      </div>

      <div
        v-else-if="block.type === 'form'"
        class="structured-content__form"
        :class="{
          'structured-content__form--disabled': isFormDisabled(block.formSchema.formId),
          'structured-content__form--standalone': isFormOnly
        }"
      >
        <div
          v-if="block.question && !isFormOnly"
          class="structured-content__label"
        >
          {{ block.question }}
        </div>
        <DynamicForm
          :schema="block.formSchema"
          :question="isFormOnly ? block.question : undefined"
          :disabled="isFormDisabled(block.formSchema.formId)"
          :initial-values="getResolvedFormValues(block.formSchema.formId) ?? undefined"
          :variant="isFormResolved(block.formSchema.formId) ? 'submitted' : 'active'"
          :show-header="false"
          :show-submitted-state="false"
          @submit="handleFormSubmit(block.formSchema.formId, $event)"
          @cancel="handleFormCancel(block.formSchema.formId)"
        />
      </div>
    </template>
  </div>
</template>

<style scoped>
.structured-content {
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
  width: 100%;
}

.structured-content--form-only {
  gap: 0;
}

.structured-content__result,
.structured-content__form {
  border-radius: 1rem;
  border: 1px solid rgba(59, 130, 246, 0.16);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(248, 250, 252, 0.94));
  padding: 0.95rem 1rem;
}

.structured-content__form--disabled {
  opacity: 0.78;
}

.structured-content__form--standalone {
  padding: 0;
  border: 0;
  background: transparent;
}

.structured-content__form--standalone :deep(.dynamic-form) {
  width: 100%;
  border-radius: 1.2rem;
  border: 1px solid color-mix(in srgb, var(--color-primary) 20%, rgba(148, 163, 184, 0.32));
  background:
    radial-gradient(circle at top right, rgba(59, 130, 246, 0.1), transparent 32%),
    linear-gradient(180deg, rgba(248, 250, 252, 0.98), rgba(239, 246, 255, 0.96));
  box-shadow: 0 14px 34px rgba(15, 23, 42, 0.08);
}

.structured-content__form--standalone :deep(.form-body) {
  padding: clamp(0.82rem, 4.2cqi, 1.1rem) clamp(0.84rem, 4.6cqi, 1.12rem) clamp(0.88rem, 4.6cqi, 1.08rem);
  max-height: min(52vh, 34rem);
  gap: 0.7rem;
}

.structured-content__form--standalone :deep(.form-footer) {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  align-items: stretch;
  gap: 0.55rem;
  padding: 0 clamp(0.84rem, 4.6cqi, 1.12rem) clamp(0.9rem, 4.6cqi, 1rem);
  border-top: 0;
  background: transparent;
}

.structured-content__form--standalone :deep(.form-field) {
  margin-bottom: 0.2rem;
}

.structured-content__form--standalone :deep(.field-label) {
  font-size: clamp(0.72rem, 2.1cqi, 0.82rem);
  margin-bottom: 0.38rem;
}

.structured-content__form--standalone :deep(.input),
.structured-content__form--standalone :deep(.textarea),
.structured-content__form--standalone :deep(.select) {
  padding: clamp(0.52rem, 2.5cqi, 0.72rem) clamp(0.62rem, 3.2cqi, 0.86rem);
  font-size: clamp(0.74rem, 2.2cqi, 0.86rem);
  border-radius: clamp(0.72rem, 2.2cqi, 0.88rem);
}

.structured-content__form--standalone :deep(.textarea) {
  min-height: clamp(4rem, 11cqi, 5rem);
}

.structured-content__form--standalone :deep(.checkbox-label),
.structured-content__form--standalone :deep(.radio-label),
.structured-content__form--standalone :deep(.option-label) {
  font-size: clamp(0.68rem, 2.1cqi, 0.8rem);
}

.structured-content__form--standalone :deep(.btn) {
  width: 100%;
  min-width: 0;
  padding: clamp(0.42rem, 2.2cqi, 0.56rem) clamp(0.78rem, 4.1cqi, 1rem);
  font-size: clamp(0.72rem, 2cqi, 0.82rem);
}

.structured-content__form--standalone :deep(.btn-secondary) {
  background: rgba(255, 255, 255, 0.72);
  backdrop-filter: blur(10px);
}

.structured-content__label {
  margin-bottom: 0.45rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-primary);
}

:global(.dark) .structured-content__result,
:global(.dark) .structured-content__form {
  border-color: rgba(96, 165, 250, 0.22);
  background: linear-gradient(180deg, rgba(17, 24, 39, 0.94), rgba(15, 23, 42, 0.96));
}

:global(.dark) .structured-content__form--standalone :deep(.dynamic-form),
:global([data-theme='dark']) .structured-content__form--standalone :deep(.dynamic-form) {
  border-color: rgba(96, 165, 250, 0.24);
  background:
    radial-gradient(circle at top right, rgba(56, 189, 248, 0.12), transparent 30%),
    linear-gradient(180deg, rgba(15, 23, 42, 0.96), rgba(17, 24, 39, 0.94));
  box-shadow: 0 16px 38px rgba(2, 6, 23, 0.28);
}

:global([data-theme='dark']) .structured-content__result,
:global([data-theme='dark']) .structured-content__form {
  border-color: rgba(96, 165, 250, 0.22);
  background: linear-gradient(180deg, rgba(17, 24, 39, 0.94), rgba(15, 23, 42, 0.96));
}
</style>
