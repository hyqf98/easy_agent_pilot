<script setup lang="ts">
import { computed } from 'vue'
import type { FormField } from '@/types/plan'

const props = defineProps<{
  field: FormField
  modelValue: number
  error?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
}>()

const inputId = computed(() => `field-${props.field.name}`)
const field = computed(() => props.field)
const min = computed(() => field.value.validation?.min ?? 0)
const max = computed(() => field.value.validation?.max ?? 100)

function onInput(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', parseInt(target.value, 10))
}
</script>

<template>
  <div class="form-field slider-field">
    <label
      :for="inputId"
      class="field-label"
    >
      {{ field.label }}
      <span
        v-if="field.required"
        class="required-mark"
      >*</span>
      <span class="slider-value">{{ modelValue }}</span>
    </label>
    <input
      :id="inputId"
      type="range"
      :value="modelValue"
      :min="min"
      :max="max"
      :disabled="disabled"
      class="slider"
      @input="onInput"
    >
    <div class="slider-labels">
      <span>{{ min }}</span>
      <span>{{ max }}</span>
    </div>
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

.field-label {
  display: flex;
  align-items: center;
  margin-bottom: 0.5rem;
  font-weight: 500;
  color: var(--text-color);
}

.required-mark {
  color: var(--error-color, #ef4444);
  margin-left: 0.25rem;
}

.slider-value {
  margin-left: auto;
  padding: 0.125rem 0.5rem;
  background-color: var(--primary-color, #3b82f6);
  color: white;
  border-radius: 0.25rem;
  font-size: 0.75rem;
}

.slider {
  width: 100%;
  height: 0.5rem;
  border-radius: 0.25rem;
  background: var(--border-color, #d1d5db);
  outline: none;
  cursor: pointer;
  -webkit-appearance: none;
  appearance: none;
}

.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 1.25rem;
  height: 1.25rem;
  border-radius: 50%;
  background: var(--primary-color, #3b82f6);
  cursor: pointer;
  transition: transform 0.15s;
}

.slider::-webkit-slider-thumb:hover {
  transform: scale(1.1);
}

.slider::-moz-range-thumb {
  width: 1.25rem;
  height: 1.25rem;
  border-radius: 50%;
  background: var(--primary-color, #3b82f6);
  cursor: pointer;
  border: none;
}

.slider-labels {
  display: flex;
  justify-content: space-between;
  margin-top: 0.25rem;
  font-size: 0.75rem;
  color: var(--text-secondary, #6b7280);
}

.error-message {
  display: block;
  margin-top: 0.25rem;
  font-size: 0.75rem;
  color: var(--error-color, #ef4444);
}
</style>
