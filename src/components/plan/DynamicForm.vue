<script setup lang="ts">
import { ref, computed, watch, onMounted, type Component } from 'vue'
import type { DynamicFormSchema, FieldType } from '@/types/plan'
import { formEngine } from '@/services/plan'
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
  initialValues?: Record<string, any>
}>()

const emit = defineEmits<{
  (e: 'submit', values: Record<string, any>): void
  (e: 'cancel'): void
}>()

// 表单数据
const formValues = ref<Record<string, any>>({})

// 表单错误
const formErrors = ref<Record<string, string>>({})

// 是否已提交
const isSubmitted = ref(false)

// 初始化表单值
function initFormValues() {
  const values: Record<string, any> = {}

  props.schema.fields.forEach(field => {
    // 优先使用传入的初始值
    if (props.initialValues && props.initialValues[field.name] !== undefined) {
      values[field.name] = props.initialValues[field.name]
    } else if (field.default !== undefined) {
      values[field.name] = field.default
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
  () => props.schema,
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

// 获取字段组件
function getFieldComponent(type: FieldType) {
  const componentMap: Record<FieldType, Component | null> = {
    text: TextField,
    textarea: TextareaField,
    select: SelectField,
    multiselect: MultiselectField,
    number: NumberField,
    checkbox: CheckboxField,
    radio: RadioField,
    date: DateField,
    file: TextField, // 暂时用文本输入
    code: TextareaField, // 暂时用文本域
    slider: SliderField
  }

  return componentMap[type] ?? null
}

// 更新字段值
function updateFieldValue(fieldName: string, value: any) {
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

onMounted(() => {
  initFormValues()
})
</script>

<template>
  <div class="dynamic-form dynamic-form--compact">
    <div class="form-header">
      <h3 class="form-title">
        {{ schema.title }}
      </h3>
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
          @update:model-value="updateFieldValue(field.name, $event)"
        />
      </template>
    </form>

    <div class="form-footer">
      <button
        type="button"
        class="btn btn-secondary"
        @click="handleCancel"
      >
        取消
      </button>
      <button
        type="button"
        class="btn btn-primary"
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
  background:
    radial-gradient(circle at 100% 0%, rgba(8, 145, 178, 0.08), transparent 40%),
    radial-gradient(circle at 0% 100%, rgba(37, 99, 235, 0.06), transparent 36%),
    linear-gradient(160deg, var(--form-surface), #f8fbff 72%);
  border-radius: 0.95rem;
  border: 1px solid var(--form-border);
  overflow: hidden;
  box-shadow: 0 10px 26px rgba(15, 23, 42, 0.05);
}

.form-header {
  padding: 0.68rem 0.92rem 0.62rem;
  border-bottom: 1px solid color-mix(in srgb, var(--form-accent) 14%, #dbe3ee);
  background: linear-gradient(120deg, rgba(239, 246, 255, 0.92), rgba(236, 254, 255, 0.72));
}

.form-title {
  margin: 0 0 0.2rem;
  font-size: 0.84rem;
  font-weight: 600;
  color: #0f172a;
  letter-spacing: 0.01em;
}

.form-description {
  margin: 0;
  font-size: 0.7rem;
  color: var(--form-muted);
}

.form-body {
  padding: 0.72rem 0.92rem 0.82rem;
  max-height: 48vh;
  overflow-y: auto;
  display: grid;
  gap: 0.25rem;
}

.form-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.45rem;
  padding: 0.6rem 0.92rem;
  border-top: 1px solid color-mix(in srgb, var(--form-accent) 10%, #dbe3ee);
  background: linear-gradient(180deg, #fbfdff, #f3f8fd);
}

.btn {
  padding: 0.34rem 0.76rem;
  border-radius: 0.7rem;
  font-size: 0.76rem;
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
  margin-bottom: 0.52rem;
}

.dynamic-form :deep(.field-label) {
  margin-bottom: 0.26rem;
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.01em;
  color: var(--color-text-primary, #334155);
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
  border-radius: 0.7rem;
  padding: 0.42rem 0.62rem;
  font-size: 0.78rem;
  line-height: 1.35;
  transition: border-color 0.16s ease, box-shadow 0.16s ease, background-color 0.16s ease;
}

.dynamic-form :deep(.input::placeholder),
.dynamic-form :deep(.textarea::placeholder),
.dynamic-form :deep(.select:invalid) {
  color: #94a3b8;
}

.dynamic-form :deep(.textarea) {
  min-height: 4.4rem;
}

.dynamic-form :deep(.select) {
  appearance: none;
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
  gap: 0.32rem;
}

.dynamic-form :deep(.option-label) {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.3rem 0.56rem;
  border: 1px solid color-mix(in srgb, var(--form-accent) 24%, #cdd7e5);
  border-radius: 999px;
  background: linear-gradient(180deg, var(--color-surface, #ffffff), var(--color-bg-secondary, #f8fbff));
  color: var(--color-text-secondary, #475569);
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
  color: var(--color-text-primary, #1e293b);
  transform: translateY(-1px);
}

.dynamic-form :deep(.option-label.selected) {
  border-color: color-mix(in srgb, var(--form-accent) 72%, #4338ca);
  background: linear-gradient(135deg, rgba(219, 234, 254, 0.9), rgba(207, 250, 254, 0.76));
  color: #1d4ed8;
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
  padding: 0.32rem 0.56rem;
  border-radius: 0.7rem;
  border: 1px solid color-mix(in srgb, var(--form-accent) 18%, #d5deea);
  background: linear-gradient(180deg, #ffffff, #f8fbff);
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
  font-size: 0.7rem;
  color: #dc2626;
}
</style>
