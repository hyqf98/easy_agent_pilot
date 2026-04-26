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
} from '../fields'

export interface DynamicFormProps {
  schema: DynamicFormSchema
  question?: string
  initialValues?: Record<string, any>
  disabled?: boolean
  variant?: 'active' | 'submitted' | 'archived'
  cancelText?: string
  showHeader?: boolean
  showSubmittedState?: boolean
}

export interface DynamicFormEmits {
  (e: 'submit', values: Record<string, any>): void
  (e: 'cancel'): void
}

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

/**
 * 动态表单渲染与校验逻辑。
 * 负责字段初始化、条件可见性、校验错误和提交/取消动作。
 */
export function useDynamicForm(props: DynamicFormProps, emit: DynamicFormEmits) {
  const themeStore = useThemeStore()
  const isDarkTheme = computed(() => themeStore.isDark)
  const formValues = ref<Record<string, any>>({})
  const formErrors = ref<Record<string, string>>({})
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

  function buildFieldInitialValue(field: DynamicFormSchema['fields'][number]): unknown {
    const hasSuggestion = field.suggestion !== undefined
      && field.suggestion !== null
      && (!Array.isArray(field.suggestion) || field.suggestion.length > 0)

    if (props.initialValues && props.initialValues[field.name] !== undefined) {
      return normalizeFieldValue(field, props.initialValues[field.name])
    }

    if (hasSuggestion) {
      return normalizeFieldValue(field, field.suggestion)
    }

    if (field.default !== undefined) {
      return normalizeFieldValue(field, field.default)
    }

    switch (field.type) {
      case 'checkbox':
        return false
      case 'multiselect':
        return []
      case 'number':
      case 'slider':
        return field.validation?.min ?? 0
      default:
        return ''
    }
  }

  function initFormValues() {
    const values: Record<string, any> = {}

    props.schema.fields.forEach(field => {
      values[field.name] = buildFieldInitialValue(field)
    })

    formValues.value = values
  }

  watch(
    () => [props.schema, props.initialValues],
    () => {
      initFormValues()
      formErrors.value = {}
      isSubmitted.value = false
    },
    { immediate: true }
  )

  const visibleFields = computed(() => props.schema.fields.filter(field => {
    if (!field.condition) return true
    return formValues.value[field.condition.field] === field.condition.value
  }))

  const visibleFieldNames = computed(() => new Set(visibleFields.value.map(field => field.name)))
  const showActions = computed(() => {
    const variant = props.variant || 'active'
    return variant !== 'submitted' && variant !== 'archived'
  })

  function getFieldComponent(type: FieldType) {
    return fieldComponentMap[type] ?? null
  }

  function updateFieldValue(fieldName: string, value: any) {
    if (props.disabled) {
      return
    }

    formValues.value[fieldName] = value
    if (formErrors.value[fieldName]) {
      delete formErrors.value[fieldName]
    }
  }

  function getFieldError(fieldName: string): string | undefined {
    return formErrors.value[fieldName]
  }

  function validateForm(): boolean {
    const { valid, errors } = formEngine.validateFormData(props.schema, formValues.value)
    formErrors.value = errors
    return valid
  }

  function handleSubmit() {
    if (props.disabled) {
      return
    }

    isSubmitted.value = true
    if (validateForm()) {
      emit('submit', { ...formValues.value })
    }
  }

  function handleCancel() {
    emit('cancel')
  }

  function resetForm() {
    initFormValues()
    formErrors.value = {}
    isSubmitted.value = false
  }

  function getValues() {
    return { ...formValues.value }
  }

  function setValues(values: Record<string, any>) {
    Object.assign(formValues.value, values)
  }

  watch(visibleFieldNames, (nextVisibleFieldNames, previousVisibleFieldNames) => {
    for (const key of Object.keys(formErrors.value)) {
      if (!nextVisibleFieldNames.has(key)) {
        delete formErrors.value[key]
      }
    }

    if (!previousVisibleFieldNames) {
      return
    }

    for (const field of props.schema.fields) {
      if (nextVisibleFieldNames.has(field.name) || !previousVisibleFieldNames.has(field.name)) {
        continue
      }

      formValues.value[field.name] = buildFieldInitialValue(field)
    }
  })

  return {
    isDarkTheme,
    formValues,
    visibleFields,
    showActions,
    getFieldComponent,
    updateFieldValue,
    getFieldError,
    handleSubmit,
    handleCancel,
    validateForm,
    resetForm,
    getValues,
    setValues
  }
}
