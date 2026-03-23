<script setup lang="ts">
import { ref, computed, watch, type Component } from 'vue'
import type { DynamicFormSchema, FieldType } from '@/types/plan'
import { formEngine } from '@/services/plan'
import { useThemeStore } from '@/stores/theme'
import {
  TextField,
  TextareaField,
  SelectField,
  NumberField,
  CheckboxField,
  RadioField,
  DateField,
  SliderField,
  MultiselectField
} from './fields'

const props = defineProps<{
  schema: DynamicFormSchema
  question?: string
  initialValues?: Record<string, any>
  disabled?: boolean
  variant?: 'active' | 'submitted' | 'archived'
}>()

const emit = defineEmits<{
  (e: 'submit', values: Record<string, any>): void
  (e: 'cancel'): void
}>()

const themeStore = useThemeStore()
const isDarkTheme = computed(() => themeStore.isDark)

const fieldComponentMap: Record<FieldType, Component | null> = {
  text: TextField,
  textarea: TextareaField,
  select: SelectField,
  multiselect: MultiselectField,
  number: NumberField,
  checkbox: CheckboxField,
  radio: RadioField,
  date: DateField,
  file: TextField,
  code: TextareaField,
  slider: SliderField
}

// 表单数据
const formValues = ref<Record<string, any>>({})

// 表单错误
const formErrors = ref<Record<string, string>>({})

// 是否已提交
const isSubmitted = ref(false)

function resolveOptionValue(
  field: DynamicFormSchema['fields'][number],
  value: unknown
): unknown {
  return field.options?.find(option => String(option.value) === String(value))?.value ?? value
}

function normalizeFieldValue(
  field: DynamicFormSchema['fields'][number],
  value: unknown
): unknown {
  if (value === undefined) {
    return undefined
  }

  switch (field.type) {
    case 'checkbox':
      return Boolean(value)
    case 'multiselect':
      if (Array.isArray(value)) {
        return value.map(item => resolveOptionValue(field, item))
      }
      return value === null || value === '' ? [] : [resolveOptionValue(field, value)]
    case 'number':
    case 'slider': {
      const numericValue = typeof value === 'number' ? value : Number(value)
      return Number.isFinite(numericValue) ? numericValue : (field.validation?.min ?? 0)
    }
    case 'select':
    case 'radio':
      if (Array.isArray(value)) {
        return resolveOptionValue(field, value[0] ?? '')
      }
      return resolveOptionValue(field, value ?? '')
    default:
      return value ?? ''
  }
}

// 初始化表单值
function initFormValues() {
  const values: Record<string, any> = {}

  props.schema.fields.forEach(field => {
    const hasSuggestion = field.suggestion !== undefined
      && field.suggestion !== null
      && (!Array.isArray(field.suggestion) || field.suggestion.length > 0)

    // 优先使用传入的初始值
    if (props.initialValues && props.initialValues[field.name] !== undefined) {
      values[field.name] = normalizeFieldValue(field, props.initialValues[field.name])
    } else if (hasSuggestion) {
      values[field.name] = normalizeFieldValue(field, field.suggestion)
    } else if (field.default !== undefined) {
      values[field.name] = normalizeFieldValue(field, field.default)
    } else {
      // 根据类型设置默认值
      switch (field.type) {
        case 'checkbox':
          values[field.name] = false
          break
        case 'multiselect':
          values[field.name] = []
          break
        case 'number':
        case 'slider':
          values[field.name] = field.validation?.min ?? 0
          break
        default:
          values[field.name] = ''
      }
    }
  })

  formValues.value = values
}

// 监听 schema 变化重新初始化
watch(
  () => [props.schema, props.initialValues],
  () => {
    initFormValues()
    formErrors.value = {}
    isSubmitted.value = false
  },
  { immediate: true }
)

// 过滤可见字段
const visibleFields = computed(() => {
  return props.schema.fields.filter(field => {
    if (!field.condition) return true

    const dependentValue = formValues.value[field.condition.field]
    return dependentValue === field.condition.value
  })
})

const visibleFieldNames = computed(() => new Set(visibleFields.value.map(field => field.name)))
const showActions = computed(() => {
  const variant = props.variant || 'active'
  return variant !== 'submitted' && variant !== 'archived'
})

// 获取字段组件
function getFieldComponent(type: FieldType) {
  return fieldComponentMap[type] ?? null
}

// 更新字段值
function updateFieldValue(fieldName: string, value: any) {
  if (props.disabled) {
    return
  }

  formValues.value[fieldName] = value

  // 清除该字段的错误
  if (formErrors.value[fieldName]) {
    delete formErrors.value[fieldName]
  }
}

// 获取字段错误
function getFieldError(fieldName: string): string | undefined {
  return formErrors.value[fieldName]
}

// 验证表单
function validateForm(): boolean {
  const { valid, errors } = formEngine.validateFormData(props.schema, formValues.value)
  formErrors.value = errors
  return valid
}

// 提交表单
function handleSubmit() {
  if (props.disabled) {
    return
  }

  isSubmitted.value = true

  if (validateForm()) {
    emit('submit', { ...formValues.value })
  }
}

// 取消
function handleCancel() {
  emit('cancel')
}

// 重置表单
function resetForm() {
  initFormValues()
  formErrors.value = {}
  isSubmitted.value = false
}

// 暴露方法给父组件
defineExpose({
  validateForm,
  resetForm,
  getValues: () => ({ ...formValues.value }),
  setValues: (values: Record<string, any>) => {
    Object.assign(formValues.value, values)
  }
})

watch(visibleFieldNames, nextVisibleFieldNames => {
  for (const key of Object.keys(formErrors.value)) {
    if (!nextVisibleFieldNames.has(key)) {
      delete formErrors.value[key]
    }
  }
})
</script>

<template>
  <div
    class="dynamic-form dynamic-form--compact"
    :class="[
      `dynamic-form--${props.variant || 'active'}`,
      { 'dynamic-form--dark': isDarkTheme }
    ]"
  >
    <div class="form-header">
      <h3 class="form-title">
        {{ schema.title }}
      </h3>
      <p
        v-if="question"
        class="form-question"
      >
        {{ question }}
      </p>
      <span
        v-if="props.variant === 'submitted'"
        class="form-state-badge"
      >
        已提交
      </span>
      <p
        v-if="schema.description"
        class="form-description"
      >
        {{ schema.description }}
      </p>
    </div>

    <form
      class="form-body"
      @submit.prevent="handleSubmit"
    >
      <template
        v-for="field in visibleFields"
        :key="field.name"
      >
        <component
          :is="getFieldComponent(field.type)"
          :field="field"
          :model-value="formValues[field.name]"
          :error="getFieldError(field.name)"
          :disabled="disabled"
          @update:model-value="updateFieldValue(field.name, $event)"
        />
      </template>
    </form>

    <div
      v-if="showActions"
      class="form-footer"
    >
      <button
        type="button"
        class="btn btn-secondary"
        :disabled="disabled"
        @click="handleCancel"
      >
        取消
      </button>
      <button
        type="button"
        class="btn btn-primary"
        :disabled="disabled"
        @click="handleSubmit"
      >
        {{ schema.submitText || '提交' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.dynamic-form {
  --form-accent: #2563eb;
  --form-accent-alt: #0891b2;
  --form-accent-soft: color-mix(in srgb, var(--form-accent) 14%, #ffffff);
  --form-border: color-mix(in srgb, var(--form-accent) 22%, #dbe3ee);
  --form-surface: var(--color-surface, #ffffff);
  --form-muted: var(--color-text-secondary, #64748b);
  --form-input-bg: color-mix(in srgb, var(--form-accent) 4%, #ffffff);
  --text-color: var(--color-text-primary, #0f172a);
  --text-secondary: var(--color-text-secondary, #64748b);
  --border-color: color-mix(in srgb, var(--form-accent) 20%, #ccd7e5);
  --input-bg: var(--form-input-bg);
  --primary-color: var(--form-accent);
  --form-card-bg:
    radial-gradient(circle at 100% 0%, rgba(8, 145, 178, 0.08), transparent 40%),
    radial-gradient(circle at 0% 100%, rgba(37, 99, 235, 0.06), transparent 36%),
    linear-gradient(160deg, var(--form-surface), #f8fbff 72%);
  --form-card-shadow: 0 10px 26px rgba(15, 23, 42, 0.05);
  --form-header-bg: linear-gradient(120deg, rgba(239, 246, 255, 0.92), rgba(236, 254, 255, 0.72));
  --form-submit-header-bg: linear-gradient(120deg, rgba(248, 250, 252, 0.96), rgba(241, 245, 249, 0.92));
  --form-archived-header-bg: linear-gradient(120deg, rgba(248, 250, 252, 0.94), rgba(241, 245, 249, 0.9));
  --form-title-color: #0f172a;
  --form-question-color: #1e3a8a;
  --form-state-bg: rgba(15, 23, 42, 0.06);
  --form-state-text: #475569;
  --form-footer-bg: linear-gradient(180deg, #fbfdff, #f3f8fd);
  --form-submit-footer-bg: linear-gradient(180deg, #fbfcfe, #f7f9fc);
  --form-submitted-bg: linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(248, 250, 252, 0.96));
  --form-submitted-shadow: 0 8px 20px rgba(15, 23, 42, 0.04);
  --form-archived-bg: linear-gradient(180deg, rgba(255, 255, 255, 0.95), rgba(248, 250, 252, 0.94));
  --form-archived-shadow: 0 8px 20px rgba(15, 23, 42, 0.035);
  --form-option-bg: linear-gradient(180deg, var(--color-surface, #ffffff), var(--color-bg-secondary, #f8fbff));
  --form-option-hover-text: var(--color-text-primary, #1e293b);
  --form-option-selected-bg: linear-gradient(135deg, rgba(219, 234, 254, 0.9), rgba(207, 250, 254, 0.76));
  --form-option-selected-text: #1d4ed8;
  --form-check-bg: linear-gradient(180deg, #ffffff, #f8fbff);
  --form-header-padding-x: 0.78rem;
  --form-header-padding-top: 0.56rem;
  --form-header-padding-bottom: 0.52rem;
  --form-body-padding-x: 0.78rem;
  --form-body-padding-top: 0.62rem;
  --form-body-padding-bottom: 0.72rem;
  --form-footer-padding-y: 0.5rem;
  --form-footer-padding-x: 0.78rem;
  --form-title-size: 0.78rem;
  --form-question-size: 0.72rem;
  --form-description-size: 0.66rem;
  --form-state-size: 0.62rem;
  --form-label-size: 0.68rem;
  --form-control-size: 0.74rem;
  --form-meta-size: 0.66rem;
  --form-button-size: 0.72rem;
  --form-button-min-width: 4.8rem;
  --form-control-padding-y: 0.36rem;
  --form-control-padding-x: 0.56rem;
  --form-control-radius: 0.64rem;
  --form-chip-size: 0.76rem;
  --form-chip-padding-y: 0.24rem;
  --form-chip-padding-x: 0.48rem;
  background: var(--form-card-bg);
  border-radius: 0.95rem;
  border: 1px solid var(--form-border);
  overflow: hidden;
  width: 100%;
  max-width: none;
  min-width: 0;
  box-shadow: var(--form-card-shadow);
  container-type: inline-size;
}

.dynamic-form--submitted {
  --form-accent: #64748b;
  --form-accent-alt: #94a3b8;
  --form-border: rgba(148, 163, 184, 0.22);
  --form-input-bg: rgba(248, 250, 252, 0.92);
  background: var(--form-submitted-bg);
  box-shadow: var(--form-submitted-shadow);
}

.dynamic-form--archived {
  --form-accent: #64748b;
  --form-accent-alt: #94a3b8;
  --form-border: rgba(148, 163, 184, 0.2);
  --form-input-bg: rgba(248, 250, 252, 0.9);
  background: var(--form-archived-bg);
  box-shadow: var(--form-archived-shadow);
}

.form-header {
  padding:
    var(--form-header-padding-top)
    var(--form-header-padding-x)
    var(--form-header-padding-bottom);
  border-bottom: 1px solid color-mix(in srgb, var(--form-accent) 14%, #dbe3ee);
  background: var(--form-header-bg);
}

.dynamic-form--submitted .form-header {
  background: var(--form-submit-header-bg);
}

.dynamic-form--archived .form-header {
  background: var(--form-archived-header-bg);
}

.form-title {
  margin: 0 0 0.2rem;
  font-size: var(--form-title-size);
  font-weight: 600;
  color: var(--form-title-color);
  letter-spacing: 0.01em;
  line-height: 1.35;
}

.form-question {
  margin: 0 0 0.34rem;
  font-size: var(--form-question-size);
  line-height: 1.55;
  color: var(--form-question-color);
  font-weight: 500;
}

.form-state-badge {
  display: inline-flex;
  align-items: center;
  margin-bottom: 0.24rem;
  padding: 0.14rem 0.42rem;
  border-radius: 999px;
  background: var(--form-state-bg);
  color: var(--form-state-text);
  font-size: var(--form-state-size);
  font-weight: 600;
  letter-spacing: 0.04em;
}

.form-description {
  margin: 0;
  font-size: var(--form-description-size);
  color: var(--form-muted);
  line-height: 1.45;
}

.form-body {
  padding:
    var(--form-body-padding-top)
    var(--form-body-padding-x)
    var(--form-body-padding-bottom);
  max-height: 42vh;
  overflow-y: auto;
  display: grid;
  gap: 0.25rem;
  min-width: 0;
}

.dynamic-form--submitted .form-body {
  max-height: 34vh;
}

.dynamic-form--archived .form-body {
  max-height: 34vh;
}

.form-footer {
  display: flex;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 0.45rem;
  padding: var(--form-footer-padding-y) var(--form-footer-padding-x);
  border-top: 1px solid color-mix(in srgb, var(--form-accent) 10%, #dbe3ee);
  background: var(--form-footer-bg);
}

.dynamic-form--submitted .form-footer {
  background: var(--form-submit-footer-bg);
}

.btn {
  padding: 0.3rem 0.66rem;
  border-radius: 0.7rem;
  min-width: var(--form-button-min-width);
  font-size: var(--form-button-size);
  font-weight: 500;
  cursor: pointer;
  transition: all 0.16s ease;
}

.btn-primary {
  background: linear-gradient(135deg, var(--form-accent), var(--form-accent-alt));
  color: white;
  border: 1px solid color-mix(in srgb, var(--form-accent) 78%, #1d4ed8);
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 8px 18px rgba(37, 99, 235, 0.18);
}

.btn-secondary {
  background-color: var(--color-surface, #ffffff);
  color: var(--color-text-primary, #1e293b);
  border: 1px solid color-mix(in srgb, var(--form-accent) 20%, #cfd8e3);
}

.btn-secondary:hover {
  border-color: color-mix(in srgb, var(--form-accent) 40%, #b6c4d5);
  background-color: color-mix(in srgb, var(--form-accent) 4%, var(--color-surface, #ffffff));
}

.dynamic-form :deep(.form-field) {
  margin-bottom: 0.42rem;
  min-width: 0;
}

.dynamic-form :deep(.field-label) {
  margin-bottom: 0.22rem;
  font-size: var(--form-label-size);
  font-weight: 600;
  letter-spacing: 0.01em;
  color: var(--color-text-primary, #334155);
  line-height: 1.4;
}

.dynamic-form :deep(.required-mark) {
  color: #ef4444;
}

.dynamic-form :deep(.input),
.dynamic-form :deep(.textarea),
.dynamic-form :deep(.select) {
  width: 100%;
  border: 1px solid color-mix(in srgb, var(--form-accent) 20%, #ccd7e5);
  background-color: var(--form-input-bg);
  color: var(--color-text-primary, #0f172a);
  border-radius: var(--form-control-radius);
  padding: var(--form-control-padding-y) var(--form-control-padding-x);
  font-size: var(--form-control-size);
  line-height: 1.35;
  transition: border-color 0.16s ease, box-shadow 0.16s ease, background-color 0.16s ease;
  min-width: 0;
}

.dynamic-form :deep(.input::placeholder),
.dynamic-form :deep(.textarea::placeholder),
.dynamic-form :deep(select.select:invalid) {
  color: #94a3b8;
}

.dynamic-form :deep(.textarea) {
  min-height: 3.8rem;
}

.dynamic-form :deep(select.select) {
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  padding-right: 1.8rem;
  background-image:
    linear-gradient(45deg, transparent 50%, #64748b 50%),
    linear-gradient(135deg, #64748b 50%, transparent 50%);
  background-position:
    calc(100% - 14px) calc(50% - 2px),
    calc(100% - 9px) calc(50% - 2px);
  background-size: 4px 4px, 4px 4px;
  background-repeat: no-repeat;
}

.dynamic-form :deep(.input:focus),
.dynamic-form :deep(.textarea:focus),
.dynamic-form :deep(.select:focus) {
  outline: none;
  border-color: color-mix(in srgb, var(--form-accent) 68%, #3730a3);
  box-shadow:
    0 0 0 3px color-mix(in srgb, var(--form-accent) 16%, transparent),
    0 5px 12px rgba(37, 99, 235, 0.08);
  background-color: var(--color-surface, #ffffff);
}

.dynamic-form :deep(.input.has-error),
.dynamic-form :deep(.textarea.has-error),
.dynamic-form :deep(.select.has-error) {
  border-color: #ef4444;
  box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.12);
}

.dynamic-form :deep(.options-grid) {
  display: flex;
  flex-wrap: wrap;
  gap: 0.26rem;
  min-width: 0;
}

.dynamic-form :deep(.option-label) {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  min-width: 0;
  max-width: 100%;
  padding: var(--form-chip-padding-y) var(--form-chip-padding-x);
  border: 1px solid color-mix(in srgb, var(--form-accent) 24%, #cdd7e5);
  border-radius: 999px;
  background: var(--form-option-bg);
  color: var(--color-text-secondary, #475569);
  font-size: var(--form-chip-size);
  transition: all 0.14s ease;
}

.dynamic-form :deep(.option-label::before) {
  content: '';
  width: 0.6rem;
  height: 0.6rem;
  border-radius: 50%;
  border: 1.5px solid color-mix(in srgb, var(--form-accent) 40%, #64748b);
  background: var(--color-surface, #ffffff);
  transition: inherit;
}

.dynamic-form :deep(.option-label:hover) {
  border-color: color-mix(in srgb, var(--form-accent) 56%, #4338ca);
  color: var(--form-option-hover-text);
  transform: translateY(-1px);
}

.dynamic-form :deep(.option-label.selected) {
  border-color: color-mix(in srgb, var(--form-accent) 72%, #4338ca);
  background: var(--form-option-selected-bg);
  color: var(--form-option-selected-text);
  font-weight: 500;
  box-shadow: 0 4px 10px rgba(37, 99, 235, 0.08);
}

.dynamic-form :deep(.option-label.selected::before) {
  border-color: var(--form-accent);
  background: radial-gradient(circle, var(--form-accent) 50%, transparent 52%);
}

.dynamic-form :deep(.option-checkbox) {
  display: none;
}

.dynamic-form :deep(.checkbox-label),
.dynamic-form :deep(.radio-label) {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  min-width: 0;
  max-width: 100%;
  padding: var(--form-chip-padding-y) var(--form-chip-padding-x);
  border-radius: 0.7rem;
  border: 1px solid color-mix(in srgb, var(--form-accent) 18%, #d5deea);
  background: var(--form-check-bg);
  font-size: var(--form-chip-size);
  transition: all 0.14s ease;
}

.dynamic-form :deep(.checkbox-label:hover),
.dynamic-form :deep(.radio-label:hover) {
  border-color: color-mix(in srgb, var(--form-accent) 40%, #6366f1);
  transform: translateY(-1px);
}

.dynamic-form :deep(.checkbox),
.dynamic-form :deep(.radio) {
  margin: 0;
  accent-color: var(--form-accent);
}

.dynamic-form :deep(.error-message) {
  margin-top: 0.2rem;
  font-size: var(--form-meta-size);
  color: #dc2626;
}

.dynamic-form :deep(.field-recommendation) {
  min-width: 0;
  padding: calc(var(--form-chip-padding-y) + 0.08rem) calc(var(--form-chip-padding-x) + 0.04rem);
}

.dynamic-form :deep(.field-recommendation__eyebrow),
.dynamic-form :deep(.option-badge) {
  font-size: calc(var(--form-meta-size) - 0.08rem);
}

.dynamic-form :deep(.field-recommendation__value),
.dynamic-form :deep(.label-text),
.dynamic-form :deep(.option-text),
.dynamic-form :deep(.select-trigger__label),
.dynamic-form :deep(.select-option__label) {
  min-width: 0;
  font-size: var(--form-control-size);
  overflow-wrap: anywhere;
}

.dynamic-form :deep(.field-recommendation__reason),
.dynamic-form :deep(.option-reason),
.dynamic-form :deep(.active-reason),
.dynamic-form :deep(.select-option__reason),
.dynamic-form :deep(.slider-labels),
.dynamic-form :deep(.slider-value) {
  font-size: var(--form-meta-size);
}

.dynamic-form :deep(.radio-group),
.dynamic-form :deep(.option-content),
.dynamic-form :deep(.radio-label__content),
.dynamic-form :deep(.select-option__header),
.dynamic-form :deep(.option-header),
.dynamic-form :deep(.radio-label__header) {
  min-width: 0;
}

.dynamic-form :deep(.radio-label),
.dynamic-form :deep(.option-label),
.dynamic-form :deep(.select-option) {
  width: 100%;
}

.dynamic-form :deep(.other-input),
.dynamic-form :deep(.select-trigger),
.dynamic-form :deep(.select-option),
.dynamic-form :deep(.other-input-add),
.dynamic-form :deep(.other-input-remove) {
  font-size: var(--form-control-size);
}

@container (max-width: 42rem) {
  .dynamic-form {
    --form-header-padding-x: 0.92rem;
    --form-header-padding-top: 0.74rem;
    --form-header-padding-bottom: 0.68rem;
    --form-body-padding-x: 0.92rem;
    --form-body-padding-top: 0.8rem;
    --form-body-padding-bottom: 0.88rem;
    --form-footer-padding-y: 0.68rem;
    --form-footer-padding-x: 0.92rem;
    --form-title-size: 0.86rem;
    --form-question-size: 0.78rem;
    --form-description-size: 0.72rem;
    --form-state-size: 0.64rem;
    --form-label-size: 0.76rem;
    --form-control-size: 0.8rem;
    --form-meta-size: 0.7rem;
    --form-button-size: 0.76rem;
    --form-button-min-width: 5rem;
    --form-control-padding-y: 0.5rem;
    --form-control-padding-x: 0.72rem;
    --form-control-radius: 0.76rem;
    --form-chip-size: 0.78rem;
    --form-chip-padding-y: 0.34rem;
    --form-chip-padding-x: 0.56rem;
  }
}

@container (max-width: 34rem) {
  .dynamic-form {
    --form-header-padding-x: 0.78rem;
    --form-header-padding-top: 0.64rem;
    --form-header-padding-bottom: 0.6rem;
    --form-body-padding-x: 0.78rem;
    --form-body-padding-top: 0.72rem;
    --form-body-padding-bottom: 0.8rem;
    --form-footer-padding-y: 0.62rem;
    --form-footer-padding-x: 0.78rem;
    --form-title-size: 0.8rem;
    --form-question-size: 0.74rem;
    --form-description-size: 0.68rem;
    --form-state-size: 0.6rem;
    --form-label-size: 0.7rem;
    --form-control-size: 0.75rem;
    --form-meta-size: 0.64rem;
    --form-button-size: 0.72rem;
    --form-button-min-width: 4.4rem;
    --form-control-padding-y: 0.44rem;
    --form-control-padding-x: 0.62rem;
    --form-control-radius: 0.68rem;
    --form-chip-size: 0.72rem;
    --form-chip-padding-y: 0.28rem;
    --form-chip-padding-x: 0.46rem;
  }

  .dynamic-form :deep(.select-trigger) {
    gap: 0.5rem;
    min-height: 1.95rem;
  }

  .dynamic-form :deep(.radio-label),
  .dynamic-form :deep(.option-label),
  .dynamic-form :deep(.checkbox-label) {
    gap: 0.32rem;
  }
}

@container (max-width: 28rem) {
  .dynamic-form {
    --form-header-padding-x: 0.66rem;
    --form-header-padding-top: 0.56rem;
    --form-header-padding-bottom: 0.54rem;
    --form-body-padding-x: 0.66rem;
    --form-body-padding-top: 0.64rem;
    --form-body-padding-bottom: 0.72rem;
    --form-footer-padding-y: 0.56rem;
    --form-footer-padding-x: 0.66rem;
    --form-title-size: 0.76rem;
    --form-question-size: 0.7rem;
    --form-description-size: 0.64rem;
    --form-state-size: 0.58rem;
    --form-label-size: 0.66rem;
    --form-control-size: 0.72rem;
    --form-meta-size: 0.62rem;
    --form-button-size: 0.7rem;
    --form-button-min-width: 100%;
    --form-control-padding-y: 0.38rem;
    --form-control-padding-x: 0.54rem;
    --form-control-radius: 0.6rem;
    --form-chip-size: 0.68rem;
    --form-chip-padding-y: 0.22rem;
    --form-chip-padding-x: 0.38rem;
  }

  .form-footer > .btn {
    flex: 1 1 100%;
  }

  .dynamic-form :deep(.select-trigger__chevron-wrap) {
    width: 1.1rem;
    height: 1.1rem;
  }

  .dynamic-form :deep(.other-input-row) {
    gap: 0.24rem;
  }

  .dynamic-form :deep(.other-input-add),
  .dynamic-form :deep(.other-input-remove) {
    width: 1.6rem;
    height: 1.6rem;
  }
}

:global([data-theme='dark']) .dynamic-form,
:global(.dark) .dynamic-form {
  --form-accent-soft: color-mix(in srgb, var(--form-accent) 18%, #0f172a);
  --form-border: color-mix(in srgb, var(--form-accent) 24%, #334155);
  --form-surface: #111827;
  --form-muted: #94a3b8;
  --form-input-bg: color-mix(in srgb, var(--form-accent) 10%, #0f172a);
  --text-color: #e2e8f0;
  --text-secondary: #94a3b8;
  --border-color: rgba(71, 85, 105, 0.76);
  --input-bg: rgba(15, 23, 42, 0.92);
  --primary-color: var(--form-accent);
  --form-card-bg:
    radial-gradient(circle at 100% 0%, rgba(8, 145, 178, 0.16), transparent 40%),
    radial-gradient(circle at 0% 100%, rgba(37, 99, 235, 0.12), transparent 36%),
    linear-gradient(160deg, rgba(15, 23, 42, 0.96), rgba(17, 24, 39, 0.98) 72%);
  --form-card-shadow: 0 16px 36px rgba(2, 6, 23, 0.32);
  --form-header-bg: linear-gradient(120deg, rgba(30, 64, 175, 0.2), rgba(8, 145, 178, 0.16));
  --form-submit-header-bg: linear-gradient(120deg, rgba(30, 41, 59, 0.96), rgba(51, 65, 85, 0.78));
  --form-archived-header-bg: linear-gradient(120deg, rgba(30, 41, 59, 0.94), rgba(51, 65, 85, 0.72));
  --form-title-color: #f8fafc;
  --form-question-color: #bfdbfe;
  --form-state-bg: rgba(148, 163, 184, 0.12);
  --form-state-text: #cbd5e1;
  --form-footer-bg: linear-gradient(180deg, rgba(17, 24, 39, 0.98), rgba(15, 23, 42, 0.96));
  --form-submit-footer-bg: linear-gradient(180deg, rgba(15, 23, 42, 0.94), rgba(17, 24, 39, 0.92));
  --form-submitted-bg: linear-gradient(180deg, rgba(15, 23, 42, 0.94), rgba(17, 24, 39, 0.94));
  --form-submitted-shadow: 0 14px 30px rgba(2, 6, 23, 0.24);
  --form-archived-bg: linear-gradient(180deg, rgba(15, 23, 42, 0.92), rgba(17, 24, 39, 0.9));
  --form-archived-shadow: 0 14px 30px rgba(2, 6, 23, 0.2);
  --form-option-bg: linear-gradient(180deg, rgba(17, 24, 39, 0.92), rgba(15, 23, 42, 0.92));
  --form-option-hover-text: #f8fafc;
  --form-option-selected-bg: linear-gradient(135deg, rgba(30, 64, 175, 0.34), rgba(8, 145, 178, 0.26));
  --form-option-selected-text: #bfdbfe;
  --form-check-bg: linear-gradient(180deg, rgba(17, 24, 39, 0.94), rgba(15, 23, 42, 0.92));
}

:global([data-theme='dark']) .dynamic-form--dark,
:global(.dark) .dynamic-form--dark,
.dynamic-form--dark {
  --form-accent-soft: color-mix(in srgb, var(--form-accent) 18%, #0f172a);
  --form-border: color-mix(in srgb, var(--form-accent) 24%, #334155);
  --form-surface: #111827;
  --form-muted: #94a3b8;
  --form-input-bg: color-mix(in srgb, var(--form-accent) 10%, #0f172a);
  --text-color: #e2e8f0;
  --text-secondary: #94a3b8;
  --border-color: rgba(71, 85, 105, 0.76);
  --input-bg: rgba(15, 23, 42, 0.92);
  --primary-color: var(--form-accent);
  --form-card-bg:
    radial-gradient(circle at 100% 0%, rgba(8, 145, 178, 0.16), transparent 40%),
    radial-gradient(circle at 0% 100%, rgba(37, 99, 235, 0.12), transparent 36%),
    linear-gradient(160deg, rgba(15, 23, 42, 0.96), rgba(17, 24, 39, 0.98) 72%);
  --form-card-shadow: 0 16px 36px rgba(2, 6, 23, 0.32);
  --form-header-bg: linear-gradient(120deg, rgba(30, 64, 175, 0.2), rgba(8, 145, 178, 0.16));
  --form-submit-header-bg: linear-gradient(120deg, rgba(30, 41, 59, 0.96), rgba(51, 65, 85, 0.78));
  --form-archived-header-bg: linear-gradient(120deg, rgba(30, 41, 59, 0.94), rgba(51, 65, 85, 0.72));
  --form-title-color: #f8fafc;
  --form-question-color: #bfdbfe;
  --form-state-bg: rgba(148, 163, 184, 0.12);
  --form-state-text: #cbd5e1;
  --form-footer-bg: linear-gradient(180deg, rgba(17, 24, 39, 0.98), rgba(15, 23, 42, 0.96));
  --form-submit-footer-bg: linear-gradient(180deg, rgba(15, 23, 42, 0.94), rgba(17, 24, 39, 0.92));
  --form-submitted-bg: linear-gradient(180deg, rgba(15, 23, 42, 0.94), rgba(17, 24, 39, 0.94));
  --form-submitted-shadow: 0 14px 30px rgba(2, 6, 23, 0.24);
  --form-archived-bg: linear-gradient(180deg, rgba(15, 23, 42, 0.92), rgba(17, 24, 39, 0.9));
  --form-archived-shadow: 0 14px 30px rgba(2, 6, 23, 0.2);
  --form-option-bg: linear-gradient(180deg, rgba(17, 24, 39, 0.92), rgba(15, 23, 42, 0.92));
  --form-option-hover-text: #f8fafc;
  --form-option-selected-bg: linear-gradient(135deg, rgba(30, 64, 175, 0.34), rgba(8, 145, 178, 0.26));
  --form-option-selected-text: #bfdbfe;
  --form-check-bg: linear-gradient(180deg, rgba(17, 24, 39, 0.94), rgba(15, 23, 42, 0.92));
}

:global([data-theme='dark']) .dynamic-form :deep(.input:focus),
:global(.dark) .dynamic-form :deep(.input:focus),
:global([data-theme='dark']) .dynamic-form :deep(.textarea:focus),
:global(.dark) .dynamic-form :deep(.textarea:focus),
:global([data-theme='dark']) .dynamic-form :deep(.select:focus),
:global(.dark) .dynamic-form :deep(.select:focus) {
  background-color: rgba(15, 23, 42, 0.98);
}

:global([data-theme='dark']) .dynamic-form--dark :deep(.field-recommendation),
:global(.dark) .dynamic-form--dark :deep(.field-recommendation),
.dynamic-form--dark :deep(.field-recommendation) {
  border-color: rgba(71, 85, 105, 0.68);
  background: linear-gradient(135deg, rgba(30, 64, 175, 0.18), rgba(8, 145, 178, 0.14));
}

:global([data-theme='dark']) .dynamic-form--dark :deep(.field-recommendation__value),
:global(.dark) .dynamic-form--dark :deep(.field-recommendation__value),
.dynamic-form--dark :deep(.field-recommendation__value) {
  color: #e2e8f0;
}

:global([data-theme='dark']) .dynamic-form--dark :deep(.field-recommendation__reason),
:global(.dark) .dynamic-form--dark :deep(.field-recommendation__reason),
.dynamic-form--dark :deep(.field-recommendation__reason) {
  color: #94a3b8;
}

:global([data-theme='dark']) .dynamic-form :deep(.input),
:global(.dark) .dynamic-form :deep(.input),
:global([data-theme='dark']) .dynamic-form :deep(.textarea),
:global(.dark) .dynamic-form :deep(.textarea),
:global([data-theme='dark']) .dynamic-form :deep(.other-input),
:global(.dark) .dynamic-form :deep(.other-input),
:global([data-theme='dark']) .dynamic-form :deep(input[type='date']),
:global(.dark) .dynamic-form :deep(input[type='date']),
:global([data-theme='dark']) .dynamic-form :deep(input[type='number']),
:global(.dark) .dynamic-form :deep(input[type='number']),
:global([data-theme='dark']) .dynamic-form :deep(input[type='text']),
:global(.dark) .dynamic-form :deep(input[type='text']) {
  background-color: rgba(15, 23, 42, 0.92);
  border-color: rgba(71, 85, 105, 0.76);
  color: #e2e8f0;
}

:global([data-theme='dark']) .dynamic-form :deep(.field-label),
:global(.dark) .dynamic-form :deep(.field-label),
:global([data-theme='dark']) .dynamic-form :deep(.label-text),
:global(.dark) .dynamic-form :deep(.label-text),
:global([data-theme='dark']) .dynamic-form :deep(.slider-labels),
:global(.dark) .dynamic-form :deep(.slider-labels) {
  color: #cbd5e1;
}
</style>
