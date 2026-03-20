<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import type { FormField } from '@/types/plan'
import { useSafeOutsideClick } from '@/composables/useSafeOutsideClick'
import { useThemeStore } from '@/stores/theme'

const props = defineProps<{
  field: FormField
  modelValue: unknown
  error?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const themeStore = useThemeStore()
const isDarkTheme = computed(() => themeStore.isDark)

const inputId = computed(() => `field-${props.field.name}`)
const OTHER_VALUE = '__other__'
const isOtherSelected = ref(false)
const otherValue = ref('')
const otherLabel = computed(() => props.field.otherLabel || '其他')
const rootRef = ref<HTMLElement | null>(null)
const triggerRef = ref<HTMLElement | null>(null)
const dropdownRef = ref<HTMLElement | null>(null)
const isOpen = ref(false)
const dropdownPosition = ref({
  top: 0,
  left: 0,
  width: 0,
  maxHeight: 220
})
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

function findMatchingOption(value: unknown) {
  return props.field.options?.find(option => String(option.value) === String(value)) ?? null
}

watch(() => props.modelValue, newVal => {
  if (newVal === '' || newVal === null || newVal === undefined) {
    isOtherSelected.value = false
    otherValue.value = ''
  } else if (props.field.allowOther && !findMatchingOption(newVal)) {
    isOtherSelected.value = true
    otherValue.value = String(newVal)
  } else if (newVal === OTHER_VALUE) {
    isOtherSelected.value = true
  } else {
    isOtherSelected.value = false
    otherValue.value = ''
  }
}, { immediate: true })

function onOtherInput(event: Event) {
  const target = event.target as HTMLInputElement
  otherValue.value = target.value
  emit('update:modelValue', target.value)
}

const selectedOption = computed(() =>
  findMatchingOption(props.modelValue)
)

const triggerLabel = computed(() => {
  if (isOtherSelected.value) {
    return otherValue.value || otherLabel.value
  }
  if (selectedOption.value) {
    return selectedOption.value.label
  }
  return props.field.placeholder || `请选择${props.field.label}`
})

const suggestedLabel = computed(() => {
  if (recommendedValues.value.length === 0) {
    return ''
  }

  return recommendedValues.value
    .map(value => props.field.options?.find(option => String(option.value) === value)?.label || value)
    .join('、')
})

const activeReason = computed(() => {
  if (isOtherSelected.value) {
    return props.field.suggestionReason || ''
  }

  if (props.modelValue === '' || props.modelValue === undefined || props.modelValue === null) {
    return ''
  }

  return optionReasons.value[String(props.modelValue)] || ''
})

function isSuggestedOption(value: string | number): boolean {
  return recommendedValues.value.includes(String(value))
}

function getOptionReason(value: string | number): string {
  return optionReasons.value[String(value)] || ''
}

function isSelectedOption(value: unknown): boolean {
  return String(props.modelValue) === String(value)
}

function updateDropdownPosition() {
  if (!triggerRef.value) {
    return
  }

  const rect = triggerRef.value.getBoundingClientRect()
  const safeGap = 12
  const estimatedHeight = dropdownRef.value?.offsetHeight ?? 220
  const spaceBelow = Math.max(110, window.innerHeight - rect.bottom - safeGap)
  const spaceAbove = Math.max(110, rect.top - safeGap)
  const shouldOpenUpward = spaceBelow < Math.min(estimatedHeight, 200) && spaceAbove > spaceBelow
  const maxHeight = Math.max(110, Math.floor(shouldOpenUpward ? spaceAbove : spaceBelow))
  const top = shouldOpenUpward
    ? Math.max(safeGap, rect.top - Math.min(estimatedHeight, maxHeight) - 6)
    : Math.min(window.innerHeight - safeGap - Math.min(estimatedHeight, maxHeight), rect.bottom + 6)
  const left = Math.min(rect.left, Math.max(safeGap, window.innerWidth - rect.width - safeGap))

  dropdownPosition.value = {
    top,
    left,
    width: rect.width,
    maxHeight
  }
}

async function openMenu() {
  updateDropdownPosition()
  isOpen.value = true
  await nextTick()
  updateDropdownPosition()
}

function toggleMenu() {
  if (props.disabled) return

  if (isOpen.value) {
    closeMenu()
    return
  }

  void openMenu()
}

function closeMenu() {
  isOpen.value = false
}

function selectOption(value: string | number) {
  if (props.disabled) return
  closeMenu()

  if (value === OTHER_VALUE) {
    isOtherSelected.value = true
    emit('update:modelValue', otherValue.value || OTHER_VALUE)
    return
  }

  isOtherSelected.value = false
  otherValue.value = ''
  emit('update:modelValue', value)
}

useSafeOutsideClick(
  () => [rootRef.value, dropdownRef.value],
  closeMenu
)

function handleViewportChange() {
  if (!isOpen.value) {
    return
  }

  updateDropdownPosition()
}

function handleEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    closeMenu()
  }
}

onMounted(() => {
  window.addEventListener('resize', handleViewportChange)
  window.addEventListener('scroll', handleViewportChange, true)
  document.addEventListener('keydown', handleEscape)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleViewportChange)
  window.removeEventListener('scroll', handleViewportChange, true)
  document.removeEventListener('keydown', handleEscape)
})
</script>

<template>
  <div
    ref="rootRef"
    class="form-field select-field"
    :class="{ 'select-field--dark': isDarkTheme }"
  >
    <label
      :for="inputId"
      class="field-label"
    >
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
    <button
      :id="inputId"
      ref="triggerRef"
      type="button"
      class="select select-trigger"
      :disabled="disabled"
      :class="{
        'has-error': error,
        'select-trigger--open': isOpen,
        'select-trigger--placeholder': !selectedOption && !isOtherSelected && !props.modelValue
      }"
      @click="toggleMenu"
    >
      <span class="select-trigger__label">{{ triggerLabel }}</span>
      <span
        v-if="!disabled"
        class="select-trigger__chevron-wrap"
      >
        <svg
          class="select-trigger__chevron"
          :class="{ 'select-trigger__chevron--open': isOpen }"
          viewBox="0 0 16 16"
          aria-hidden="true"
        >
          <path
            d="M4 6.25 8 10l4-3.75"
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.8"
          />
        </svg>
      </span>
    </button>
    <Teleport to="body">
      <div
        v-if="isOpen"
        ref="dropdownRef"
        class="select-menu"
        :class="{ 'select-menu--dark': isDarkTheme }"
        :style="{
          top: `${dropdownPosition.top}px`,
          left: `${dropdownPosition.left}px`,
          minWidth: `${dropdownPosition.width}px`,
          maxHeight: `${dropdownPosition.maxHeight}px`
        }"
      >
        <button
          v-if="field.placeholder"
          type="button"
          class="select-option"
          :class="{ 'select-option--active': props.modelValue === '' }"
          @click="selectOption('')"
        >
          <span class="select-option__label">{{ field.placeholder }}</span>
        </button>
        <button
          v-for="option in field.options"
          :key="option.value"
          type="button"
          class="select-option"
          :class="{ 'select-option--active': isSelectedOption(option.value) && !isOtherSelected }"
          @click="selectOption(option.value)"
        >
          <span class="select-option__header">
            <span class="select-option__label">{{ option.label }}</span>
            <span
              v-if="isSuggestedOption(option.value)"
              class="select-option__badge"
            >推荐</span>
          </span>
          <span
            v-if="getOptionReason(option.value)"
            class="select-option__reason"
          >
            {{ getOptionReason(option.value) }}
          </span>
        </button>
        <button
          v-if="field.allowOther && !hasExplicitOtherOption"
          type="button"
          class="select-option"
          :class="{ 'select-option--active': isOtherSelected }"
          @click="selectOption(OTHER_VALUE)"
        >
          <span class="select-option__header">
            <span class="select-option__label">{{ otherLabel }}</span>
            <span
              v-if="isSuggestedOption(OTHER_VALUE)"
              class="select-option__badge"
            >推荐</span>
          </span>
        </button>
      </div>
    </Teleport>
    <input
      v-if="field.allowOther && isOtherSelected"
      type="text"
      class="other-input"
      :value="otherValue"
      :disabled="disabled"
      :placeholder="`请输入${field.label}`"
      @input="onOtherInput"
    >
    <p
      v-if="activeReason"
      class="active-reason"
    >
      {{ activeReason }}
    </p>
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

.field-recommendation__reason,
.active-reason {
  margin: 0;
  color: #475569;
  font-size: 0.68rem;
  line-height: 1.4;
}

.select {
  width: 100%;
  padding: 0.42rem 0.65rem;
  border: 1px solid color-mix(in srgb, var(--form-accent, #4f46e5) 22%, #ccd7e5);
  border-radius: 0.6rem;
  background-color: color-mix(in srgb, var(--form-accent, #4f46e5) 4%, #ffffff);
  color: var(--color-text-primary, #0f172a);
  font-size: 0.82rem;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.select.has-error {
  border-color: var(--error-color, #ef4444);
}

.select:disabled {
  cursor: default;
  color: var(--color-text-primary, #0f172a);
  opacity: 1;
}

.select-field {
  position: relative;
}

.select-trigger {
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  min-height: 2.15rem;
  padding-right: 0.65rem;
  background-image: none;
  text-align: left;
}

.select-trigger:focus {
  outline: none;
  border-color: color-mix(in srgb, var(--form-accent, #4f46e5) 72%, #3730a3);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--form-accent, #4f46e5) 15%, transparent);
}

.select-trigger--open {
  border-color: color-mix(in srgb, var(--form-accent, #4f46e5) 72%, #3730a3);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--form-accent, #4f46e5) 15%, transparent);
  background-color: var(--color-surface, #ffffff);
}

.select-trigger--placeholder {
  color: #94a3b8;
}

.select-trigger:disabled {
  background-color: color-mix(in srgb, var(--form-accent, #4f46e5) 3%, #f8fafc);
}

.select-trigger__label {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.select-trigger__chevron-wrap {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.35rem;
  height: 1.35rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--form-accent, #4f46e5) 10%, #ffffff);
  color: color-mix(in srgb, var(--form-accent, #4f46e5) 72%, #334155);
  transition: background-color 0.16s ease, color 0.16s ease;
}

.select-trigger--open .select-trigger__chevron-wrap {
  background: color-mix(in srgb, var(--form-accent, #4f46e5) 16%, #ffffff);
}

.select-trigger__chevron {
  width: 0.9rem;
  height: 0.9rem;
  flex-shrink: 0;
  color: #64748b;
  transition: transform 0.16s ease;
}

.select-trigger__chevron--open {
  transform: rotate(180deg);
}

.select-menu {
  position: fixed;
  z-index: var(--z-select-menu, 1200);
  display: grid;
  gap: 0.14rem;
  padding: 0.28rem;
  border-radius: 0.78rem;
  border: 1px solid color-mix(in srgb, var(--form-accent, #4f46e5) 20%, #d3dce8);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.98), rgba(248, 250, 252, 0.98));
  box-shadow: 0 18px 36px rgba(15, 23, 42, 0.12);
  backdrop-filter: blur(10px);
  overflow-y: auto;
}

.select-option {
  width: 100%;
  padding: 0.42rem 0.56rem;
  border: none;
  border-radius: 0.58rem;
  background: transparent;
  color: var(--color-text-primary, #0f172a);
  font-size: 0.76rem;
  text-align: left;
  cursor: pointer;
  transition: background-color 0.14s ease, color 0.14s ease, transform 0.14s ease;
}

.select-option:hover {
  background: color-mix(in srgb, var(--form-accent, #4f46e5) 10%, #ffffff);
  transform: translateX(1px);
}

.select-option--active {
  background: linear-gradient(135deg, rgba(219, 234, 254, 0.92), rgba(207, 250, 254, 0.78));
  color: color-mix(in srgb, var(--form-accent, #4f46e5) 78%, #1d4ed8);
  font-weight: 600;
}

.select-option__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.48rem;
}

.select-option__label {
  min-width: 0;
}

.select-option__badge {
  display: inline-flex;
  align-items: center;
  padding: 0.08rem 0.34rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--form-accent, #4f46e5) 18%, #ffffff);
  color: color-mix(in srgb, var(--form-accent, #4f46e5) 80%, #1d4ed8);
  font-size: 0.58rem;
  font-weight: 700;
  letter-spacing: 0.02em;
}

.select-option__reason {
  display: block;
  margin-top: 0.12rem;
  color: #64748b;
  font-size: 0.66rem;
  line-height: 1.35;
}

.other-input {
  width: 100%;
  margin-top: 0.4rem;
  padding: 0.42rem 0.65rem;
  border: 1px solid color-mix(in srgb, var(--form-accent, #4f46e5) 22%, #ccd7e5);
  border-radius: 0.6rem;
  background-color: var(--color-surface, #ffffff);
  color: var(--color-text-primary, #0f172a);
  font-size: 0.82rem;
  transition: border-color 0.15s, box-shadow 0.15s;
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

.select-field--dark .field-recommendation {
  border-color: rgba(71, 85, 105, 0.68) !important;
  background: linear-gradient(135deg, rgba(30, 64, 175, 0.18), rgba(8, 145, 178, 0.14)) !important;
}

.select-field--dark .field-recommendation__value {
  color: #e2e8f0 !important;
}

.select-field--dark .field-recommendation__reason,
.select-field--dark .active-reason,
.select-field--dark .field-label,
.select-field--dark .select-trigger__label {
  color: #cbd5e1 !important;
}

.select-field--dark .select,
.select-field--dark .other-input {
  background-color: rgba(15, 23, 42, 0.92) !important;
  border-color: rgba(71, 85, 105, 0.76) !important;
  color: #e2e8f0 !important;
}

.select-field--dark .select-trigger__chevron-wrap {
  background: rgba(51, 65, 85, 0.92) !important;
  color: #cbd5e1 !important;
}

.select-menu--dark {
  background: linear-gradient(180deg, rgba(15, 23, 42, 0.98), rgba(30, 41, 59, 0.98)) !important;
  border-color: rgba(148, 163, 184, 0.22) !important;
  box-shadow: 0 20px 40px rgba(2, 6, 23, 0.42) !important;
  color: #e2e8f0 !important;
}

.select-menu--dark .select-option {
  color: #e2e8f0 !important;
}

.select-menu--dark .select-option__label {
  color: #e2e8f0 !important;
}

.select-menu--dark .select-option__reason {
  color: #94a3b8 !important;
}

.select-menu--dark .select-option:hover {
  background: rgba(59, 130, 246, 0.14) !important;
}

.select-menu--dark .select-option--active {
  background: linear-gradient(135deg, rgba(30, 64, 175, 0.34), rgba(8, 145, 178, 0.28)) !important;
  color: #bfdbfe !important;
}

:global([data-theme='dark']) .select-menu,
:global(.dark) .select-menu {
  background: linear-gradient(180deg, rgba(15, 23, 42, 0.98), rgba(30, 41, 59, 0.98)) !important;
  border-color: rgba(148, 163, 184, 0.22) !important;
  box-shadow: 0 20px 40px rgba(2, 6, 23, 0.42) !important;
  color: #e2e8f0 !important;
}

:global([data-theme='dark']) .select-option,
:global(.dark) .select-option {
  color: #e2e8f0 !important;
}

:global([data-theme='dark']) .select-option__reason,
:global(.dark) .select-option__reason {
  color: #94a3b8 !important;
}

:global([data-theme='dark']) .select-option:hover,
:global(.dark) .select-option:hover {
  background: rgba(59, 130, 246, 0.14) !important;
}

:global([data-theme='dark']) .select-option--active,
:global(.dark) .select-option--active {
  background: linear-gradient(135deg, rgba(30, 64, 175, 0.34), rgba(8, 145, 178, 0.28)) !important;
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
:global([data-theme='dark']) .active-reason,
:global(.dark) .active-reason {
  color: #94a3b8;
}

:global([data-theme='dark']) .select,
:global(.dark) .select,
:global([data-theme='dark']) .other-input,
:global(.dark) .other-input {
  background-color: rgba(15, 23, 42, 0.92) !important;
  border-color: rgba(71, 85, 105, 0.76) !important;
  color: #e2e8f0 !important;
}

:global([data-theme='dark']) .select-trigger__chevron-wrap,
:global(.dark) .select-trigger__chevron-wrap {
  background: rgba(51, 65, 85, 0.92) !important;
  color: #cbd5e1 !important;
}

:global([data-theme='dark']) .select-trigger__label,
:global(.dark) .select-trigger__label,
:global([data-theme='dark']) .select-option__label,
:global(.dark) .select-option__label,
:global([data-theme='dark']) .field-label,
:global(.dark) .field-label {
  color: #e2e8f0 !important;
}
</style>
