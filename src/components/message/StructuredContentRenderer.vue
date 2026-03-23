<script setup lang="ts">
import { computed, toRef } from 'vue'
import DynamicForm from '@/components/plan/DynamicForm.vue'
import { parseStructuredContent } from '@/utils/structuredContent'
import { useTypewriterText } from '@/composables/useTypewriterText'
import MarkdownRenderer from './MarkdownRenderer.vue'
import StructuredResultCard from './StructuredResultCard.vue'

const props = withDefaults(defineProps<{
  content: string
  interactiveForms?: boolean
  formDisabled?: boolean
  animate?: boolean
}>(), {
  interactiveForms: false,
  formDisabled: false,
  animate: false
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
          'structured-content__form--disabled': !interactiveForms || formDisabled,
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
          :disabled="!interactiveForms || formDisabled"
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
  border-radius: 1.1rem;
}

.structured-content__form--standalone :deep(.form-header) {
  padding: clamp(0.64rem, 3cqi, 0.85rem) clamp(0.72rem, 4.2cqi, 1rem) clamp(0.62rem, 3.4cqi, 0.75rem);
}

.structured-content__form--standalone :deep(.form-title) {
  font-size: clamp(0.78rem, 2.4cqi, 0.92rem);
}

.structured-content__form--standalone :deep(.form-question) {
  font-size: clamp(0.7rem, 2.15cqi, 0.82rem);
  line-height: 1.6;
}

.structured-content__form--standalone :deep(.form-description) {
  font-size: clamp(0.64rem, 1.9cqi, 0.74rem);
}

.structured-content__form--standalone :deep(.form-body) {
  padding: clamp(0.68rem, 3.6cqi, 0.95rem) clamp(0.72rem, 4.2cqi, 1rem) clamp(0.72rem, 4.2cqi, 1rem);
  max-height: min(52vh, 34rem);
  gap: 0.55rem;
}

.structured-content__form--standalone :deep(.form-footer) {
  padding: clamp(0.58rem, 3.2cqi, 0.8rem) clamp(0.72rem, 4.2cqi, 1rem) clamp(0.7rem, 4cqi, 0.95rem);
}

.structured-content__form--standalone :deep(.form-field) {
  margin-bottom: 0.2rem;
}

.structured-content__form--standalone :deep(.field-label) {
  font-size: clamp(0.68rem, 2.05cqi, 0.78rem);
  margin-bottom: 0.35rem;
}

.structured-content__form--standalone :deep(.input),
.structured-content__form--standalone :deep(.textarea),
.structured-content__form--standalone :deep(.select) {
  padding: clamp(0.4rem, 2.3cqi, 0.6rem) clamp(0.52rem, 2.9cqi, 0.75rem);
  font-size: clamp(0.72rem, 2.2cqi, 0.84rem);
  border-radius: clamp(0.62rem, 2.1cqi, 0.8rem);
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
  min-width: clamp(4.2rem, 22cqi, 5.25rem);
  padding: clamp(0.34rem, 2cqi, 0.48rem) clamp(0.6rem, 3.8cqi, 0.9rem);
  font-size: clamp(0.68rem, 2cqi, 0.8rem);
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

:global([data-theme='dark']) .structured-content__result,
:global([data-theme='dark']) .structured-content__form {
  border-color: rgba(96, 165, 250, 0.22);
  background: linear-gradient(180deg, rgba(17, 24, 39, 0.94), rgba(15, 23, 42, 0.96));
}
</style>
