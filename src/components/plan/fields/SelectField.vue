<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount, onMounted } from 'vue'
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

// "其他"选项的值
const OTHER_VALUE = '__other__'

// 是否选择了"其他"
const isOtherSelected = ref(false)

// "其他"输入框的值
const otherValue = ref('')

// "其他"选项的标签
const otherLabel = computed(() => props.field.otherLabel || '其他')
const rootRef = ref<HTMLElement | null>(null)
const isOpen = ref(false)
const hasExplicitOtherOption = computed(() =>
  props.field.options?.some(option => String(option.value) === OTHER_VALUE) ?? false
)

// 监听 modelValue 变化，同步 isOtherSelected 和 otherValue
watch(() => props.modelValue, (newVal) => {
  if (newVal === '' || newVal === null || newVal === undefined) {
    isOtherSelected.value = false
    otherValue.value = ''
  } else if (props.field.allowOther && !props.field.options?.some(opt => opt.value === newVal)) {
    // 如果当前值不在选项中，说明是"其他"值
    isOtherSelected.value = true
    otherValue.value = String(newVal)
  } else if (newVal === OTHER_VALUE) {
    isOtherSelected.value = true
  } else {
    isOtherSelected.value = false
    otherValue.value = ''
  }
}, { immediate: true })

// 处理"其他"输入框变化
function onOtherInput(event: Event) {
  const target = event.target as HTMLInputElement
  otherValue.value = target.value
  emit('update:modelValue', target.value)
}

const selectedOption = computed(() =>
  props.field.options?.find(option => option.value === props.modelValue) ?? null
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

function toggleMenu() {
  if (props.disabled) return
  isOpen.value = !isOpen.value
}

function closeMenu() {
  isOpen.value = false
}

function selectOption(value: string | number) {
  if (props.disabled) return
  closeMenu()

  if (value === OTHER_VALUE) {
    isOtherSelected.value = true
    return
  }

  isOtherSelected.value = false
  otherValue.value = ''
  const numValue = Number(value)
  emit('update:modelValue', isNaN(numValue) || value === '' ? value : numValue)
}

function handleDocumentClick(event: MouseEvent) {
  if (!rootRef.value) return
  if (rootRef.value.contains(event.target as Node)) return
  closeMenu()
}

onMounted(() => {
  document.addEventListener('click', handleDocumentClick)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleDocumentClick)
})
</script>

<template>
  <div
    ref="rootRef"
    class="form-field select-field"
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
    <button
      :id="inputId"
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
        class="select-trigger__chevron"
        :class="{ 'select-trigger__chevron--open': isOpen }"
      >⌄</span>
    </button>
    <div
      v-if="isOpen"
      class="select-menu"
    >
      <button
        v-if="field.placeholder"
        type="button"
        class="select-option"
        :class="{ 'select-option--active': props.modelValue === '' }"
        @click="selectOption('')"
      >
        {{ field.placeholder }}
      </button>
      <button
        v-for="option in field.options"
        :key="option.value"
        type="button"
        class="select-option"
        :class="{ 'select-option--active': option.value === props.modelValue && !isOtherSelected }"
        @click="selectOption(option.value)"
      >
        {{ option.label }}
      </button>
      <button
        v-if="field.allowOther && !hasExplicitOtherOption"
        type="button"
        class="select-option"
        :class="{ 'select-option--active': isOtherSelected }"
        @click="selectOption(OTHER_VALUE)"
      >
        {{ otherLabel }}
      </button>
    </div>
    <!-- "其他"输入框 -->
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

.select-field {
  position: relative;
}

.select-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
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

.select-trigger__label {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.select-trigger__chevron {
  flex-shrink: 0;
  color: #64748b;
  font-size: 0.95rem;
  line-height: 1;
  transition: transform 0.16s ease;
}

.select-trigger__chevron--open {
  transform: rotate(180deg);
}

.select-menu {
  position: absolute;
  top: calc(100% + 0.35rem);
  left: 0;
  right: 0;
  z-index: 40;
  display: grid;
  gap: 0.2rem;
  padding: 0.35rem;
  border-radius: 0.85rem;
  border: 1px solid color-mix(in srgb, var(--form-accent, #4f46e5) 20%, #d3dce8);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.98), rgba(248, 250, 252, 0.98));
  box-shadow: 0 18px 36px rgba(15, 23, 42, 0.12);
  backdrop-filter: blur(10px);
}

.select-option {
  width: 100%;
  padding: 0.5rem 0.65rem;
  border: none;
  border-radius: 0.65rem;
  background: transparent;
  color: var(--color-text-primary, #0f172a);
  font-size: 0.8rem;
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

:global([data-theme='dark']) .select-menu,
:global(.dark) .select-menu {
  background: linear-gradient(180deg, rgba(15, 23, 42, 0.98), rgba(30, 41, 59, 0.98));
  border-color: rgba(148, 163, 184, 0.22);
  box-shadow: 0 20px 40px rgba(2, 6, 23, 0.42);
}

:global([data-theme='dark']) .select-option,
:global(.dark) .select-option {
  color: #e2e8f0;
}

:global([data-theme='dark']) .select-option:hover,
:global(.dark) .select-option:hover {
  background: rgba(59, 130, 246, 0.14);
}

:global([data-theme='dark']) .select-option--active,
:global(.dark) .select-option--active {
  background: linear-gradient(135deg, rgba(30, 64, 175, 0.34), rgba(8, 145, 178, 0.28));
  color: #bfdbfe;
}
</style>
