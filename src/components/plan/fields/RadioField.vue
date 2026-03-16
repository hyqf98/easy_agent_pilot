<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { FormField } from '@/types/plan'

const props = defineProps<{
  field: FormField
  modelValue: string | number
  error?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | number): void
}>()

const inputId = computed(() => `field-${props.field.name}`)
const field = computed(() => props.field)
const OTHER_VALUE = '__other__'
const isOtherSelected = ref(false)
const otherValue = ref('')
const hasExplicitOtherOption = computed(() =>
  props.field.options?.some(option => String(option.value) === OTHER_VALUE) ?? false
)

watch(() => props.modelValue, (value) => {
  const hasPresetValue = props.field.options?.some(option => option.value === value)
  if (props.field.allowOther && value && !hasPresetValue && value !== OTHER_VALUE) {
    isOtherSelected.value = true
    otherValue.value = String(value)
    return
  }
  if (value === OTHER_VALUE) {
    isOtherSelected.value = true
    return
  }
  isOtherSelected.value = false
  otherValue.value = ''
}, { immediate: true })

function onChange(value: string | number) {
  if (value === OTHER_VALUE) {
    isOtherSelected.value = true
    return
  }
  isOtherSelected.value = false
  otherValue.value = ''
  emit('update:modelValue', value)
}

function onOtherInput(event: Event) {
  const target = event.target as HTMLInputElement
  otherValue.value = target.value
  emit('update:modelValue', target.value)
}
</script>

<template>
  <div class="form-field radio-field">
    <label class="field-label">
      {{ field.label }}
      <span
        v-if="field.required"
        class="required-mark"
      >*</span>
    </label>
    <div class="radio-group">
      <label
        v-for="option in field.options"
        :key="option.value"
        class="radio-label"
      >
        <input
          type="radio"
          :name="inputId"
          :value="option.value"
          :checked="modelValue === option.value"
          :disabled="disabled"
          class="radio"
          @change="onChange(option.value)"
        >
        <span class="label-text">{{ option.label }}</span>
      </label>
      <label
        v-if="field.allowOther && !hasExplicitOtherOption"
        class="radio-label"
      >
        <input
          type="radio"
          :name="inputId"
          :value="OTHER_VALUE"
          :checked="isOtherSelected"
          :disabled="disabled"
          class="radio"
          @change="onChange(OTHER_VALUE)"
        >
        <span class="label-text">{{ field.otherLabel || '其他' }}</span>
      </label>
    </div>
    <input
      v-if="field.allowOther && isOtherSelected"
      type="text"
      class="other-input"
      :value="otherValue"
      :disabled="disabled"
      :placeholder="`请输入${field.label}`"
      @input="onOtherInput"
    >
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
  display: block;
  margin-bottom: 0.5rem;
  font-weight: 500;
  color: var(--text-color);
}

.required-mark {
  color: var(--error-color, #ef4444);
  margin-left: 0.25rem;
}

.radio-group {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
}

.radio-label {
  display: flex;
  align-items: center;
  cursor: pointer;
}

.radio {
  width: 1rem;
  height: 1rem;
  margin-right: 0.5rem;
  cursor: pointer;
  accent-color: var(--primary-color, #3b82f6);
}

.label-text {
  color: var(--text-color);
  font-size: 0.875rem;
}

.other-input {
  width: 100%;
  margin-top: 0.5rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--border-color, #d1d5db);
  border-radius: 0.375rem;
  background-color: var(--input-bg, #fff);
  color: var(--text-color);
}

.error-message {
  display: block;
  margin-top: 0.25rem;
  font-size: 0.75rem;
  color: var(--error-color, #ef4444);
}
</style>
