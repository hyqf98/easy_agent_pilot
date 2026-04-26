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

function normalizeFieldNameMapping(field: Record<string, unknown>): Record<string, unknown> {
  if (!field.name && field.field) {
    field = { ...field, name: field.field }
  }
  if (!field.label && field.question) {
    field = { ...field, label: field.question }
  }
  return field
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
  const fields = schema.fields
    .map((field) => {
      const mapped = normalizeFieldNameMapping(field as unknown as Record<string, unknown>) as unknown as FormField
      return normalizeFormFieldForRendering(mapped)
    })
    .filter((field): field is FormField => Boolean(field.name) && Boolean(field.label))

  return {
    ...schema,
    fields
  }
}

export function normalizeFormSchemasForRendering(schemas: DynamicFormSchema[]): DynamicFormSchema[] {
  return schemas.map(normalizeFormSchemaForRendering)
}
