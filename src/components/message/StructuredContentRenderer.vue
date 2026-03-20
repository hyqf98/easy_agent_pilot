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

function handleFormSubmit(formId: string, values: Record<string, unknown>) {
  emit('form-submit', formId, values)
}

function handleFormCancel(formId: string) {
  emit('form-cancel', formId)
}
</script>

<template>
  <div class="structured-content">
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
        :class="{ 'structured-content__form--disabled': !interactiveForms || formDisabled }"
      >
        <div
          v-if="block.question"
          class="structured-content__label"
        >
          {{ block.question }}
        </div>
        <DynamicForm
          :schema="block.formSchema"
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
