import type { DynamicFormSchema, FieldType, FormField, FormFieldOption } from '@/types/plan'

const OPTION_FIELD_TYPES = new Set<FieldType>(['select', 'radio', 'multiselect'])
const OTHER_VALUE = '__other__'
const DEFAULT_OTHER_LABEL = '其他'

function normalizeOption(option: FormFieldOption): FormFieldOption {
  const isOtherOption =
    String(option.value) === OTHER_VALUE
    || String(option.label).trim() === DEFAULT_OTHER_LABEL

  if (!isOtherOption) {
    return option
  }

  return {
    label: DEFAULT_OTHER_LABEL,
    value: OTHER_VALUE
  }
}

function dedupeOptions(options: FormFieldOption[]): FormFieldOption[] {
  return options.filter((option, index, array) =>
    array.findIndex(item => String(item.value) === String(option.value)) === index
  )
}

export function normalizeFormFieldForRendering(field: FormField): FormField {
  if (!OPTION_FIELD_TYPES.has(field.type)) {
    return field
  }

  const normalizedOptions = dedupeOptions((field.options ?? []).map(normalizeOption))
  const hasOtherOption = normalizedOptions.some(option => String(option.value) === OTHER_VALUE)
  const otherLabel = field.otherLabel || DEFAULT_OTHER_LABEL

  return {
    ...field,
    options: hasOtherOption
      ? normalizedOptions
      : [...normalizedOptions, { label: otherLabel, value: OTHER_VALUE }],
    allowOther: true,
    otherLabel
  }
}

export function normalizeFormSchemaForRendering(schema: DynamicFormSchema): DynamicFormSchema {
  return {
    ...schema,
    fields: schema.fields.map(normalizeFormFieldForRendering)
  }
}

export function normalizeFormSchemasForRendering(schemas: DynamicFormSchema[]): DynamicFormSchema[] {
  return schemas.map(normalizeFormSchemaForRendering)
}
