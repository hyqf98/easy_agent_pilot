<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { FormField } from '@/types/plan'
import { useThemeStore } from '@/stores/theme'

const props = defineProps<{
  field: FormField
  modelValue: (string | number)[]
  error?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: (string | number)[]): void
}>()

const themeStore = useThemeStore()
const isDarkTheme = computed(() => themeStore.isDark)

const inputId = computed(() => `field-${props.field.name}`)
const field = computed(() => props.field)
const OTHER_VALUE = '__other__'
const isOtherSelected = ref(false)
const otherValues = ref<string[]>([])
const hasExplicitOtherOption = computed(() =>
  props.field.options?.some(option => String(option.value) === OTHER_VALUE) ?? false
)
const optionReasons = computed(() => props.field.optionReasons ?? {})
const recommendedValues = computed(() => {
  if (Array.isArray(props.field.suggestion)) {
    return props.field.suggestion.map(value => String(value))
  }

  if (props.field.suggestion === undefined || props.field.suggestion === null || props.field.suggestion === '') {
    return []
  }

  return [String(props.field.suggestion)]
})
const suggestedLabel = computed(() => {
  if (recommendedValues.value.length === 0) {
    return ''
  }

  return recommendedValues.value
    .map(value => props.field.options?.find(option => String(option.value) === value)?.label || value)
    .join('、')
})

const presetValues = computed(() => {
  return new Set(props.field.options?.map(opt => opt.value) || [])
})

function extractCustomValues(values: (string | number)[]): string[] {
  return values
    .filter(value => !presetValues.value.has(value) && value !== OTHER_VALUE)
    .map(value => String(value))
}

function normalizeCustomValues(values: string[]): string[] {
  const seen = new Set<string>()
  const normalized: string[] = []

  values.forEach(value => {
    const next = value.trim()
    if (!next || seen.has(next)) {
      return
    }
    seen.add(next)
    normalized.push(next)
  })

  return normalized
}

function isSameStringArray(left: string[], right: string[]): boolean {
  if (left.length !== right.length) {
    return false
  }

  return left.every((value, index) => value === right[index])
}

function emitCombinedValues(selectedPresetValues: (string | number)[], customValues: string[]) {
  emit('update:modelValue', [...selectedPresetValues, ...normalizeCustomValues(customValues)])
}

watch(() => props.modelValue, newVal => {
  const customValues = extractCustomValues(newVal)
  const normalizedIncoming = normalizeCustomValues(customValues)
  const normalizedLocal = normalizeCustomValues(otherValues.value)

  if (customValues.length > 0) {
    isOtherSelected.value = true
    if (!isSameStringArray(normalizedIncoming, normalizedLocal)) {
      otherValues.value = customValues
    }
    return
  }

  if (newVal.some(value => value === OTHER_VALUE)) {
    isOtherSelected.value = true
    otherValues.value = ['']
    return
  }

  if (isOtherSelected.value && otherValues.value.length > 0 && normalizedLocal.length === 0) {
    return
  }

  isOtherSelected.value = false
  otherValues.value = []
}, { immediate: true })

function isSelected(value: string | number): boolean {
  return props.modelValue.includes(value)
}

function toggleOption(value: string | number) {
  const current = [...props.modelValue].filter(v => presetValues.value.has(v))
  const customValues = extractCustomValues(props.modelValue)

  const index = current.indexOf(value)
  if (index === -1) {
    current.push(value)
  } else {
    current.splice(index, 1)
  }

  emitCombinedValues(current, customValues)
}

function toggleOther() {
  isOtherSelected.value = !isOtherSelected.value
  const current = props.modelValue.filter(v => presetValues.value.has(v))

  if (!isOtherSelected.value) {
    otherValues.value = []
    emitCombinedValues(current, [])
    return
  }

  otherValues.value = extractCustomValues(props.modelValue)
  if (otherValues.value.length === 0) {
    otherValues.value = ['']
  }
  emitCombinedValues(current, otherValues.value)
}

function addOtherInput() {
  otherValues.value = [...otherValues.value, '']
}

function removeOtherInput(index: number) {
  const nextValues = otherValues.value.filter((_, itemIndex) => itemIndex !== index)
  otherValues.value = nextValues.length > 0 ? nextValues : ['']
  const current = props.modelValue.filter(v => presetValues.value.has(v))
  emitCombinedValues(current, otherValues.value)
}

function onOtherInput(index: number, event: Event) {
  const target = event.target as HTMLInputElement
  const nextValues = [...otherValues.value]
  nextValues[index] = target.value
  otherValues.value = nextValues

  const current = props.modelValue.filter(v => presetValues.value.has(v))
  emitCombinedValues(current, nextValues)
}

function isSuggestedOption(value: string | number): boolean {
  return recommendedValues.value.includes(String(value))
}

function getOptionReason(value: string | number): string {
  return optionReasons.value[String(value)] || ''
}
</script>

<template>
  <div
    class="form-field multiselect-field"
    :class="{ 'multiselect-field--dark': isDarkTheme }"
  >
    <label class="field-label">
      {{ field.label }}
      <span
        v-if="field.required"
        class="required-mark"
      >*</span>
    </label>
    <div
      v-if="suggestedLabel || field.suggestionReason"
      class="field-recommendation"
    >
      <span class="field-recommendation__eyebrow">AI 建议</span>
      <strong
        v-if="suggestedLabel"
        class="field-recommendation__value"
      >{{ suggestedLabel }}</strong>
      <span
        v-if="field.suggestionReason"
        class="field-recommendation__reason"
      >
        {{ field.suggestionReason }}
      </span>
    </div>
    <div class="options-grid">
      <label
        v-for="option in field.options"
        :key="option.value"
        class="option-label"
        :class="{ selected: isSelected(option.value) }"
      >
        <input
          type="checkbox"
          :name="inputId"
          :value="option.value"
          :checked="isSelected(option.value)"
          :disabled="disabled"
          class="option-checkbox"
          @change="toggleOption(option.value)"
        >
        <span class="option-content">
          <span class="option-header">
            <span class="option-text">{{ option.label }}</span>
            <span
              v-if="isSuggestedOption(option.value)"
              class="option-badge"
            >推荐</span>
          </span>
          <span
            v-if="getOptionReason(option.value)"
            class="option-reason"
          >
            {{ getOptionReason(option.value) }}
          </span>
        </span>
      </label>
      <label
        v-if="field.allowOther && !hasExplicitOtherOption"
        class="option-label"
        :class="{
          selected: isOtherSelected,
          'option-label--adder': true
        }"
        :title="`添加自定义${field.label}`"
      >
        <input
          type="checkbox"
          :name="inputId"
          :value="OTHER_VALUE"
          :checked="isOtherSelected"
          :disabled="disabled"
          class="option-checkbox"
          @change="toggleOther"
        >
        <span class="option-content">
          <span class="option-header option-header--adder">
            <span class="option-text option-text--adder">+</span>
          </span>
        </span>
      </label>
    </div>
    <div
      v-if="field.allowOther && isOtherSelected"
      class="other-inputs"
    >
      <div
        v-for="(otherValue, index) in otherValues"
        :key="`${field.name}-other-${index}`"
        class="other-input-row"
      >
        <input
          type="text"
          class="other-input"
          :value="otherValue"
          :disabled="disabled"
          :placeholder="`请输入${field.label}${otherValues.length > 1 ? ` ${index + 1}` : ''}`"
          @input="onOtherInput(index, $event)"
        >
        <button
          v-if="otherValues.length > 1"
          type="button"
          class="other-input-remove"
          :disabled="disabled"
          @click="removeOtherInput(index)"
        >
          -
        </button>
      </div>
      <button
        type="button"
        class="other-input-add"
        :disabled="disabled"
        @click="addOtherInput"
      >
        +
      </button>
    </div>
    <span
      v-if="error"
      class="error-message"
    >{{ error }}</span>
  </div>
</template>

<style scoped>
.form-field {
  margin-bottom: 0.75rem;
}

.field-label {
  display: block;
  margin-bottom: 0.35rem;
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--color-text-primary, #334155);
}

.required-mark {
  color: var(--error-color, #ef4444);
  margin-left: 0.15rem;
}

.field-recommendation {
  margin-bottom: 0.42rem;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.16rem 0.42rem;
  padding: 0.38rem 0.52rem;
  border-radius: 0.7rem;
  border: 1px solid color-mix(in srgb, var(--form-accent, #4f46e5) 16%, #cbd5e1);
  background: linear-gradient(135deg, rgba(239, 246, 255, 0.94), rgba(236, 254, 255, 0.72));
}

.field-recommendation__eyebrow {
  display: inline-flex;
  color: color-mix(in srgb, var(--form-accent, #4f46e5) 74%, #1d4ed8);
  font-size: 0.62rem;
  font-weight: 700;
  letter-spacing: 0.04em;
}

.field-recommendation__value {
  display: inline-flex;
  color: #0f172a;
  font-size: 0.74rem;
  font-weight: 600;
}

.field-recommendation__reason {
  margin: 0;
  color: #475569;
  font-size: 0.68rem;
  line-height: 1.4;
}

.options-grid {
  display: grid;
  gap: 0.28rem;
}

.option-label {
  display: inline-flex;
  align-items: flex-start;
  gap: 0.4rem;
  padding: 0.42rem 0.58rem;
  border: 1px solid color-mix(in srgb, var(--form-accent, #4f46e5) 28%, #cdd7e5);
  border-radius: 0.72rem;
  background: linear-gradient(180deg, var(--color-surface, #ffffff), var(--color-bg-secondary, #f8fbff));
  color: var(--color-text-secondary, #475569);
  cursor: pointer;
  transition: all 0.16s ease;
  font-size: 0.76rem;
}

.option-label::before {
  content: '';
  width: 0.66rem;
  height: 0.66rem;
  margin-top: 0.12rem;
  border-radius: 4px;
  border: 1.5px solid color-mix(in srgb, var(--form-accent, #4f46e5) 44%, #64748b);
  background: var(--color-surface, #ffffff);
  transition: inherit;
}

.option-label:hover {
  border-color: color-mix(in srgb, var(--form-accent, #4f46e5) 62%, #4338ca);
  color: var(--color-text-primary, #1e293b);
  transform: translateY(-1px);
}

.option-label.selected {
  border-color: color-mix(in srgb, var(--form-accent, #4f46e5) 76%, #4338ca);
  background: linear-gradient(135deg, rgba(224, 231, 255, 0.9), rgba(207, 250, 254, 0.78));
  color: #3730a3;
  font-weight: 600;
  box-shadow: 0 4px 10px rgba(79, 70, 229, 0.1);
}

.option-label.selected::before {
  border-color: var(--form-accent, #4f46e5);
  background: var(--form-accent, #4f46e5);
}

.option-label--adder {
  align-items: center;
  justify-content: center;
  min-height: 2.25rem;
}

.option-label--adder::before {
  display: none;
}

.option-checkbox {
  display: none;
}

.option-content {
  min-width: 0;
  flex: 1;
}

.option-header {
  display: flex;
  align-items: center;
  gap: 0.36rem;
}

.option-header--adder {
  justify-content: center;
}

.option-text {
  font-size: 0.74rem;
  font-weight: 600;
}

.option-text--adder {
  font-size: 1rem;
  line-height: 1;
}

.option-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.08rem 0.32rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--form-accent, #4f46e5) 14%, #ffffff);
  color: color-mix(in srgb, var(--form-accent, #4f46e5) 82%, #1d4ed8);
  font-size: 0.58rem;
  font-weight: 700;
}

.option-reason {
  display: block;
  margin-top: 0.12rem;
  color: #64748b;
  font-size: 0.66rem;
  line-height: 1.35;
}

.other-inputs {
  margin-top: 0.4rem;
  display: flex;
  flex-direction: column;
  gap: 0.38rem;
}

.other-input-row {
  display: flex;
  align-items: center;
  gap: 0.34rem;
}

.other-input {
  width: 100%;
  padding: 0.42rem 0.65rem;
  border: 1px solid color-mix(in srgb, var(--form-accent, #4f46e5) 22%, #ccd7e5);
  border-radius: 0.6rem;
  background-color: var(--color-surface, #ffffff);
  color: var(--color-text-primary, #0f172a);
  font-size: 0.82rem;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.other-input-add,
.other-input-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.9rem;
  height: 1.9rem;
  border: 1px solid color-mix(in srgb, var(--form-accent, #4f46e5) 20%, #ccd7e5);
  border-radius: 0.58rem;
  background: color-mix(in srgb, var(--form-accent, #4f46e5) 5%, #ffffff);
  color: color-mix(in srgb, var(--form-accent, #4f46e5) 76%, #334155);
  font-size: 1rem;
  font-weight: 700;
  line-height: 1;
  flex-shrink: 0;
  transition: border-color 0.15s, background-color 0.15s, color 0.15s;
}

.other-input-add:hover,
.other-input-remove:hover {
  border-color: color-mix(in srgb, var(--form-accent, #4f46e5) 48%, #6366f1);
  background: color-mix(in srgb, var(--form-accent, #4f46e5) 10%, #ffffff);
}

.other-input:focus {
  outline: none;
  border-color: color-mix(in srgb, var(--form-accent, #4f46e5) 72%, #3730a3);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--form-accent, #4f46e5) 15%, transparent);
}

.other-input::placeholder {
  color: #94a3b8;
}

.error-message {
  display: block;
  margin-top: 0.2rem;
  font-size: 0.72rem;
  color: var(--error-color, #ef4444);
}

.multiselect-field--dark .field-recommendation {
  border-color: rgba(71, 85, 105, 0.68) !important;
  background: linear-gradient(135deg, rgba(30, 64, 175, 0.18), rgba(8, 145, 178, 0.14)) !important;
}

.multiselect-field--dark .field-recommendation__value,
.multiselect-field--dark .field-label,
.multiselect-field--dark .option-text {
  color: #e2e8f0 !important;
}

.multiselect-field--dark .field-recommendation__reason,
.multiselect-field--dark .option-reason {
  color: #94a3b8 !important;
}

.multiselect-field--dark .option-label,
.multiselect-field--dark .other-input,
.multiselect-field--dark .other-input-add,
.multiselect-field--dark .other-input-remove {
  background: linear-gradient(180deg, rgba(17, 24, 39, 0.92), rgba(15, 23, 42, 0.92)) !important;
  border-color: rgba(71, 85, 105, 0.76) !important;
  color: #cbd5e1 !important;
}

.multiselect-field--dark .option-label:hover {
  color: #f8fafc !important;
}

.multiselect-field--dark .option-label.selected {
  background: linear-gradient(135deg, rgba(30, 64, 175, 0.34), rgba(8, 145, 178, 0.26)) !important;
  color: #bfdbfe !important;
}

:global([data-theme='dark']) .field-recommendation,
:global(.dark) .field-recommendation {
  border-color: rgba(71, 85, 105, 0.68);
  background: linear-gradient(135deg, rgba(30, 64, 175, 0.18), rgba(8, 145, 178, 0.14));
}

:global([data-theme='dark']) .field-recommendation__value,
:global(.dark) .field-recommendation__value {
  color: #e2e8f0;
}

:global([data-theme='dark']) .field-recommendation__reason,
:global(.dark) .field-recommendation__reason,
:global([data-theme='dark']) .option-reason,
:global(.dark) .option-reason {
  color: #94a3b8;
}

:global([data-theme='dark']) .option-label,
:global(.dark) .option-label,
:global([data-theme='dark']) .other-input,
:global(.dark) .other-input,
:global([data-theme='dark']) .other-input-add,
:global(.dark) .other-input-add,
:global([data-theme='dark']) .other-input-remove,
:global(.dark) .other-input-remove {
  background: linear-gradient(180deg, rgba(17, 24, 39, 0.92), rgba(15, 23, 42, 0.92));
  border-color: rgba(71, 85, 105, 0.76);
  color: #cbd5e1;
}

:global([data-theme='dark']) .option-label:hover,
:global(.dark) .option-label:hover {
  color: #f8fafc;
}

:global([data-theme='dark']) .option-label.selected,
:global(.dark) .option-label.selected {
  background: linear-gradient(135deg, rgba(30, 64, 175, 0.34), rgba(8, 145, 178, 0.26));
  color: #bfdbfe;
}
</style>
