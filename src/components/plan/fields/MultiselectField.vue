<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { FormField } from '@/types/plan'

const props = defineProps<{
  field: FormField
  modelValue: (string | number)[]
  error?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: (string | number)[]): void
}>()

const inputId = computed(() => `field-${props.field.name}`)
const field = computed(() => props.field)

// "其他"选项的值
const OTHER_VALUE = '__other__'

// 是否选择了"其他"
const isOtherSelected = ref(false)

// "其他"输入框的值
const otherValue = ref('')

// "其他"选项的标签
const otherLabel = computed(() => props.field.otherLabel || '其他')
const hasExplicitOtherOption = computed(() =>
  props.field.options?.some(option => String(option.value) === OTHER_VALUE) ?? false
)

// 获取预设选项的值集合
const presetValues = computed(() => {
  return new Set(props.field.options?.map(opt => opt.value) || [])
})


// 监听 modelValue 变化
watch(() => props.modelValue, (newVal) => {
  // 检查是否有非预设值
  const hasOtherValue = newVal.some(v => !presetValues.value.has(v) && v !== OTHER_VALUE)
  if (hasOtherValue) {
    isOtherSelected.value = true
    // 取第一个非预设值显示在输入框
    const others = newVal.filter(v => !presetValues.value.has(v) && v !== OTHER_VALUE)
    otherValue.value = others.length > 0 ? String(others[0]) : ''
  }
}, { immediate: true })

// 检查预设选项是否被选中
function isSelected(value: string | number): boolean {
  return props.modelValue.includes(value)
}

// 切换预设选项
function toggleOption(value: string | number) {
  const current = [...props.modelValue].filter(v => presetValues.value.has(v))
  const others = props.modelValue.filter(v => !presetValues.value.has(v) && v !== OTHER_VALUE)

  const index = current.indexOf(value)
  if (index === -1) {
    current.push(value)
  } else {
    current.splice(index, 1)
  }

  emit('update:modelValue', [...current, ...others])
}

// 切换"其他"选项
function toggleOther() {
  isOtherSelected.value = !isOtherSelected.value

  if (!isOtherSelected.value) {
    // 取消选择"其他"，移除所有非预设值
    const current = props.modelValue.filter(v => presetValues.value.has(v))
    otherValue.value = ''
    emit('update:modelValue', current)
  }
}

// 处理"其他"输入框变化
function onOtherInput(event: Event) {
  const target = event.target as HTMLInputElement
  otherValue.value = target.value

  // 更新值：保留预设值 + 新的"其他"值
  const current = props.modelValue.filter(v => presetValues.value.has(v))
  if (target.value.trim()) {
    emit('update:modelValue', [...current, target.value.trim()])
  } else {
    emit('update:modelValue', current)
  }
}
</script>

<template>
  <div class="form-field multiselect-field">
    <label class="field-label">
      {{ field.label }}
      <span
        v-if="field.required"
        class="required-mark"
      >*</span>
    </label>
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
        <span class="option-text">{{ option.label }}</span>
      </label>
      <!-- "其他"选项 -->
      <label
        v-if="field.allowOther && !hasExplicitOtherOption"
        class="option-label"
        :class="{ selected: isOtherSelected }"
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
        <span class="option-text">{{ otherLabel }}</span>
      </label>
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

.options-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.option-label {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.32rem 0.6rem;
  border: 1px solid color-mix(in srgb, var(--form-accent, #4f46e5) 28%, #cdd7e5);
  border-radius: 999px;
  background: linear-gradient(180deg, var(--color-surface, #ffffff), var(--color-bg-secondary, #f8fbff));
  color: var(--color-text-secondary, #475569);
  cursor: pointer;
  transition: all 0.16s ease;
  font-size: 0.8rem;
}

.option-label::before {
  content: '';
  width: 0.65rem;
  height: 0.65rem;
  border-radius: 3px;
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

.option-checkbox {
  display: none;
}

.option-text {
  font-size: 0.78rem;
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
</style>
