<script setup lang="ts">
import { computed } from 'vue'
import type { FormField } from '@/types/plan'

const props = defineProps<{
  field: FormField
  modelValue: boolean
  error?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const inputId = computed(() => `field-${props.field.name}`)

function onChange(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', target.checked)
}
</script>

<template>
  <div class="form-field checkbox-field">
    <label
      :for="inputId"
      class="checkbox-label"
    >
      <input
        :id="inputId"
        type="checkbox"
        :checked="modelValue"
        :disabled="disabled"
        class="checkbox"
        @change="onChange"
      >
      <span class="label-text">
        {{ field.label }}
        <span
          v-if="field.required"
          class="required-mark"
        >*</span>
      </span>
    </label>
    <span
      v-if="error"
      class="error-message"
    >{{ error }}</span>
  </div>
</template>

<style scoped>
.form-field {
  margin-bottom: 1rem;
}

.checkbox-label {
  display: flex;
  align-items: center;
  cursor: pointer;
}

.checkbox {
  width: 1rem;
  height: 1rem;
  margin-right: 0.5rem;
  border: 1px solid var(--border-color, #d1d5db);
  border-radius: 0.25rem;
  cursor: pointer;
  accent-color: var(--primary-color, #3b82f6);
}

.label-text {
  color: var(--text-color);
  font-size: 0.875rem;
}

.required-mark {
  color: var(--error-color, #ef4444);
  margin-left: 0.25rem;
}

.error-message {
  display: block;
  margin-top: 0.25rem;
  font-size: 0.75rem;
  color: var(--error-color, #ef4444);
}
</style>
