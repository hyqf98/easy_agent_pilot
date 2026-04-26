import type {
  AIFormRequest,
  AITaskItem,
  AITaskSplitResult,
  DynamicFormSchema,
  FieldCondition,
  FieldType,
  FieldValidation,
  FormField,
  FormFieldOption,
  TaskPriority
} from '@/types/plan'
import { normalizeFormSchemaForRendering } from './formSchema'

export interface StructuredExecutionResult {
  summary: string
  generatedFiles: string[]
  modifiedFiles: string[]
  changedFiles: string[]
  deletedFiles: string[]
}

export interface StructuredFormResponse {
  formId: string
  values: Record<string, unknown>
}

export type StructuredContentBlock =
  | {
    type: 'markdown'
    content: string
  }
  | {
    type: 'form'
    question?: string
    formSchema: DynamicFormSchema
  }
  | {
    type: 'result'
    result: StructuredExecutionResult
  }

const STRUCTURED_CONTENT_CACHE_LIMIT = 200
const structuredContentCache = new Map<string, StructuredContentBlock[]>()
const VALID_FIELD_TYPES = new Set<FieldType>([
  'text',
  'textarea',
  'select',
  'multiselect',
  'number',
  'checkbox',
  'radio',
  'date',
  'file',
  'code',
  'slider'
])

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function normalizeStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return []

  return value
    .filter((item): item is string => typeof item === 'string')
    .map(item => item.trim())
    .filter(Boolean)
}

function normalizeTextListItem(value: unknown): string[] {
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    const text = String(value).trim()
    return text ? [text] : []
  }

  if (!isRecord(value)) {
    return []
  }

  const lines = [
    readString(value, 'title'),
    readString(value, 'name'),
    readString(value, 'step'),
    readString(value, 'content'),
    readString(value, 'description')
  ].filter((item): item is string => Boolean(item?.trim()))

  const steps = normalizeStringArray(value.steps ?? value.items)
  const expectedResults = normalizeStringArray(value.expectedResults ?? value.expected_results)
  const precondition = readString(value, 'precondition')
  const expectedResult = readString(value, 'expectedResult', 'expected_result')

  if (precondition) {
    lines.unshift(`前置条件：${precondition.trim()}`)
  }

  if (steps.length > 0) {
    lines.push(...steps)
  }

  if (expectedResults.length > 0) {
    lines.push(...expectedResults.map(item => `预期：${item}`))
  } else if (expectedResult) {
    lines.push(`预期：${expectedResult.trim()}`)
  }

  return uniqueStrings(lines.map(item => item.trim()).filter(Boolean))
}

function normalizeTextList(value: unknown): string[] {
  if (Array.isArray(value)) {
    return uniqueStrings(value.flatMap(normalizeTextListItem))
  }

  return normalizeTextListItem(value)
}

function normalizeStringMap(value: unknown): Record<string, string> | undefined {
  if (!isRecord(value)) {
    return undefined
  }

  const entries = Object.entries(value)
    .filter(([, item]) => typeof item === 'string')
    .map(([key, item]) => [String(key), String(item).trim()] as const)
    .filter(([, item]) => Boolean(item))

  if (entries.length === 0) {
    return undefined
  }

  return Object.fromEntries(entries)
}

function uniqueStrings(values: string[]): string[] {
  return Array.from(new Set(values.map(item => item.trim()).filter(Boolean)))
}

function readString(value: Record<string, unknown>, key: string, fallbackKey?: string): string | undefined {
  const target = value[key] ?? (fallbackKey ? value[fallbackKey] : undefined)
  return typeof target === 'string' ? target : undefined
}

function readBoolean(value: Record<string, unknown>, key: string, fallbackKey?: string): boolean | undefined {
  const target = value[key] ?? (fallbackKey ? value[fallbackKey] : undefined)
  return typeof target === 'boolean' ? target : undefined
}

function normalizeFieldValidation(value: unknown): FieldValidation | undefined {
  if (!isRecord(value)) {
    return undefined
  }

  return {
    min: typeof value.min === 'number' ? value.min : undefined,
    max: typeof value.max === 'number' ? value.max : undefined,
    pattern: typeof value.pattern === 'string' ? value.pattern : undefined,
    message: typeof value.message === 'string' ? value.message : undefined
  }
}

function normalizeFieldCondition(value: unknown): FieldCondition | undefined {
  if (!isRecord(value) || typeof value.field !== 'string') {
    return undefined
  }

  return {
    field: value.field,
    value: value.value
  }
}

function normalizeFieldOptions(value: unknown): FormFieldOption[] | undefined {
  if (!Array.isArray(value)) {
    return undefined
  }

  const normalized = value.map((option): FormFieldOption | null => {
    if (typeof option === 'string' || typeof option === 'number' || typeof option === 'boolean') {
      return {
        label: String(option),
        value: option
      }
    }

    if (!isRecord(option)) {
      return null
    }

    const optionLabel = readString(option, 'label')
    const optionValue = option.value
    if (!optionLabel || optionValue === undefined) {
      return null
    }

    return {
      label: optionLabel,
      value: optionValue
    }
  }).filter((option): option is FormFieldOption => Boolean(option))

  return normalized.length > 0 ? normalized : undefined
}

function isTaskPriority(value: unknown): value is TaskPriority {
  return value === 'low' || value === 'medium' || value === 'high'
}

function normalizeTaskPriority(value: unknown): TaskPriority {
  if (isTaskPriority(value)) {
    return value
  }

  if (typeof value === 'number') {
    if (value === 1) return 'high'
    if (value === 2) return 'medium'
    if (value === 3) return 'low'
  }

  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === '1' || normalized === '高') return 'high'
  if (normalized === '2' || normalized === '中') return 'medium'
  if (normalized === '3' || normalized === '低') return 'low'
  if (normalized === 'high' || normalized === 'medium' || normalized === 'low') {
    return normalized as TaskPriority
  }

  return 'medium'
}

function normalizeTaskSplitItem(value: unknown): AITaskItem | null {
  if (!isRecord(value)) {
    return null
  }

  const title = readString(value, 'title')?.trim()
  if (!title) {
    return null
  }

  const description = readString(value, 'description')?.trim() ?? ''
  const priority = normalizeTaskPriority(value.priority)

  return {
    title,
    description,
    priority,
    expertId: readString(value, 'expertId', 'expert_id'),
    agentId: readString(value, 'agentId', 'agent_id'),
    modelId: readString(value, 'modelId', 'model_id'),
    memoryLibraryIds: normalizeStringArray(value.memoryLibraryIds ?? value.memory_library_ids),
    implementationSteps: normalizeTextList(value.implementationSteps ?? value.implementation_steps),
    testSteps: normalizeTextList(value.testSteps ?? value.test_steps),
    acceptanceCriteria: normalizeTextList(value.acceptanceCriteria ?? value.acceptance_criteria),
    dependsOn: normalizeStringArray(value.dependsOn ?? value.depends_on)
  }
}

function toTaskSplitResult(value: unknown): AITaskSplitResult | null {
  if (!isRecord(value)) {
    return null
  }

  const tasksValue = value.tasks
  if (!Array.isArray(tasksValue) || tasksValue.length === 0) {
    return null
  }

  const normalizedTasks = tasksValue
    .map(normalizeTaskSplitItem)
    .filter((item): item is AITaskItem => Boolean(item))

  if (normalizedTasks.length === 0) {
    return null
  }

  return {
    type: 'task_split',
    summary: readString(value, 'summary')?.trim(),
    tasks: normalizedTasks
  }
}

function normalizeFormField(value: unknown): FormField | null {
  if (!isRecord(value)) {
    return null
  }

  const type = readString(value, 'type')
  const name = readString(value, 'name', 'field')
  const label = readString(value, 'label', 'question')

  if (!type || !VALID_FIELD_TYPES.has(type as FieldType) || !name || !label) {
    return null
  }

  return {
    name,
    label,
    type: type as FieldType,
    placeholder: readString(value, 'placeholder'),
    required: readBoolean(value, 'required') ?? false,
    default: value.default,
    suggestion: value.suggestion,
    suggestionReason: readString(value, 'suggestionReason', 'suggestion_reason'),
    optionReasons: normalizeStringMap(value.optionReasons ?? value.option_reasons),
    options: normalizeFieldOptions(value.options),
    validation: normalizeFieldValidation(value.validation),
    condition: normalizeFieldCondition(value.condition),
    allowOther: readBoolean(value, 'allowOther', 'allow_other'),
    otherLabel: readString(value, 'otherLabel', 'other_label')
  }
}

function normalizeDynamicFormSchema(value: unknown): DynamicFormSchema | null {
  if (!isRecord(value)) {
    return null
  }

  const fieldsValue = value.fields
  const fields = Array.isArray(fieldsValue)
    ? fieldsValue.map(normalizeFormField).filter((field): field is FormField => Boolean(field))
    : []
  const formId = readString(value, 'formId', 'form_id')
  const title = readString(value, 'title')

  if (!formId || !title || fields.length === 0) {
    return null
  }

  return normalizeFormSchemaForRendering({
    formId,
    title,
    description: readString(value, 'description'),
    fields,
    submitText: readString(value, 'submitText', 'submit_text')
  })
}

function toResultBlock(value: Record<string, unknown>): StructuredContentBlock | null {
  const generatedFiles = normalizeStringArray(value.generated_files)
  const modifiedFiles = normalizeStringArray(value.modified_files)
  const changedFiles = normalizeStringArray(value.changed_files)
  const deletedFiles = normalizeStringArray(value.deleted_files)
  const taskOverview = typeof value.task_overview === 'string' ? value.task_overview.trim() : ''
  const completedPoints = normalizeStringArray(value.completed_points)
  const summary = typeof value.result_summary === 'string' && value.result_summary.trim()
    ? value.result_summary.trim()
    : [taskOverview, completedPoints.length > 0 ? `完成项: ${completedPoints.join('；')}` : '']
        .filter(Boolean)
        .join('\n')

  if (!summary && !generatedFiles.length && !modifiedFiles.length && !changedFiles.length && !deletedFiles.length) {
    return null
  }

  return {
    type: 'result',
    result: {
      summary,
      generatedFiles,
      modifiedFiles,
      changedFiles,
      deletedFiles
    }
  }
}

function toFormBlocks(value: Record<string, unknown>): StructuredContentBlock[] | null {
  if (value.type !== 'form_request') {
    return null
  }

  const question = typeof value.question === 'string' ? value.question : undefined
  const forms = Array.isArray(value.forms) ? value.forms : []
  const normalizedForms = forms.map(normalizeDynamicFormSchema).filter((item): item is DynamicFormSchema => Boolean(item))
  const singleForm = normalizeDynamicFormSchema(value.formSchema)
  const formSchemas = normalizedForms.length > 0 ? normalizedForms : singleForm ? [singleForm] : []

  if (formSchemas.length === 0) {
    return null
  }

  return formSchemas.map(formSchema => ({
    type: 'form' as const,
    question,
    formSchema
  }))
}

function parseNestedStructuredJsonValue(value: Record<string, unknown>): StructuredContentBlock[] | null {
  const nestedKeys = ['payload', 'data', 'response', 'result', 'form_request', 'formRequest']

  for (const key of nestedKeys) {
    const nested = value[key]
    const parsed = parseStructuredJsonValue(nested)
    if (parsed && parsed.length > 0) {
      return parsed
    }
  }

  return null
}

function parseStructuredJsonValue(value: unknown): StructuredContentBlock[] | null {
  if (Array.isArray(value)) {
    const blocks = value.flatMap(item => parseStructuredJsonValue(item) ?? [])
    return blocks.length > 0 ? blocks : null
  }

  if (!isRecord(value)) {
    return null
  }

  const formBlocks = toFormBlocks(value)
  if (formBlocks) {
    return formBlocks
  }

  const resultBlock = toResultBlock(value)
  if (resultBlock) {
    return [resultBlock]
  }

  const nestedBlocks = parseNestedStructuredJsonValue(value)
  if (nestedBlocks) {
    return nestedBlocks
  }

  return null
}

function toFormResponse(value: unknown): StructuredFormResponse | null {
  if (!isRecord(value) || value.type !== 'form_response' || typeof value.formId !== 'string') {
    return null
  }

  return {
    formId: value.formId,
    values: isRecord(value.values) ? value.values : {}
  }
}

function tryParseFormResponse(rawJson: string): StructuredFormResponse | null {
  try {
    return toFormResponse(JSON.parse(rawJson) as unknown)
  } catch {
    try {
      const repaired = repairJsonStructure(rawJson)
      if (repaired === rawJson) {
        return null
      }
      return toFormResponse(JSON.parse(repaired) as unknown)
    } catch {
      return null
    }
  }
}

function tryParseTaskSplitResult(rawJson: string): AITaskSplitResult | null {
  try {
    return toTaskSplitResult(JSON.parse(rawJson) as unknown)
  } catch {
    try {
      const repaired = repairJsonStructure(rawJson)
      if (repaired === rawJson) {
        return null
      }
      return toTaskSplitResult(JSON.parse(repaired) as unknown)
    } catch {
      return null
    }
  }
}

function extractTaskSplitSummaryFromValue(value: unknown): string | null {
  if (Array.isArray(value)) {
    for (const item of value) {
      const summary = extractTaskSplitSummaryFromValue(item)
      if (summary) {
        return summary
      }
    }
    return null
  }

  if (!isRecord(value)) {
    return null
  }

  const directSummary = readString(value, 'summary')?.trim()
  if (directSummary && toTaskSplitResult(value)) {
    return directSummary
  }

  const nestedKeys = ['structured_output', 'structuredOutput', 'payload', 'data', 'response']
  for (const key of nestedKeys) {
    const summary = extractTaskSplitSummaryFromValue(value[key])
    if (summary) {
      return summary
    }
  }

  const result = value.result
  if (typeof result === 'string') {
    const trimmed = result.trim()
    if (trimmed) {
      return trimmed
    }
  }

  if (isRecord(result) && Array.isArray(result.content)) {
    const textParts = result.content
      .filter((item): item is Record<string, unknown> => isRecord(item))
      .filter(item => item.type === 'text')
      .map(item => readString(item, 'text')?.trim() ?? '')
      .filter(Boolean)

    if (textParts.length > 0) {
      return textParts.join('\n\n')
    }
  }

  return null
}

function pushMarkdownBlock(blocks: StructuredContentBlock[], content: string): void {
  if (!content.trim()) {
    return
  }

  blocks.push({
    type: 'markdown',
    content
  })
}

function normalizeJsonLikeText(raw: string): string {
  return raw
    .replace(/[“”]/g, '"')
    .replace(/[‘’]/g, "'")
    .replace(/：/g, ':')
    .replace(/，/g, ',')
    .replace(/（/g, '(')
    .replace(/）/g, ')')
    .replace(/【/g, '[')
    .replace(/】/g, ']')
    .replace(/｛/g, '{')
    .replace(/｝/g, '}')
    .replace(/［/g, '[')
    .replace(/］/g, ']')
}

function repairJsonStructure(raw: string): string {
  const normalized = normalizeJsonLikeText(raw)
  let inString = false
  let escaped = false
  const stack: string[] = []
  let repaired = ''

  for (let index = 0; index < normalized.length; index += 1) {
    const char = normalized[index]

    if (inString) {
      repaired += char
      if (escaped) {
        escaped = false
        continue
      }
      if (char === '\\') {
        escaped = true
        continue
      }
      if (char === '"') {
        inString = false
      }
      continue
    }

    if (char === '"') {
      inString = true
      repaired += char
      continue
    }

    if (char === '{' || char === '[') {
      stack.push(char)
      repaired += char
      continue
    }

    if (char === '}' || char === ']') {
      const expected = char === '}' ? '{' : '['
      if (stack[stack.length - 1] === expected) {
        stack.pop()
        repaired += char
      }
      continue
    }

    repaired += char
  }

  while (stack.length > 0) {
    repaired += stack.pop() === '{' ? '}' : ']'
  }

  return repaired
}

function tryParseStructuredBlocks(rawJson: string): StructuredContentBlock[] | null {
  try {
    const parsed = JSON.parse(rawJson) as unknown
    return parseStructuredJsonValue(parsed)
  } catch {
    try {
      const repaired = repairJsonStructure(rawJson)
      if (repaired === rawJson) {
        return null
      }
      const parsed = JSON.parse(repaired) as unknown
      return parseStructuredJsonValue(parsed)
    } catch {
      return null
    }
  }
}

function extractBalancedJsonRanges(content: string): Array<{ start: number, end: number }> {
  const ranges: Array<{ start: number, end: number }> = []
  const stack: string[] = []
  let start: number | null = null
  let inString = false
  let escaped = false

  for (let index = 0; index < content.length; index += 1) {
    const char = content[index]

    if (inString) {
      if (escaped) {
        escaped = false
        continue
      }
      if (char === '\\') {
        escaped = true
        continue
      }
      if (char === '"') {
        inString = false
      }
      continue
    }

    if (char === '"') {
      inString = true
      continue
    }

    if (char === '{' || char === '[') {
      if (stack.length === 0) {
        start = index
      }
      stack.push(char)
      continue
    }

    if (char === '}' || char === ']') {
      const expected = char === '}' ? '{' : '['
      if (stack[stack.length - 1] === expected) {
        stack.pop()
        if (stack.length === 0 && start !== null) {
          ranges.push({ start, end: index + 1 })
          start = null
        }
      }
    }
  }

  return ranges
}

function parseInlineStructuredBlocks(content: string): StructuredContentBlock[] | null {
  const blocks: StructuredContentBlock[] = []
  let lastIndex = 0
  let matched = false

  extractBalancedJsonRanges(content).forEach(({ start, end }) => {
    const rawJson = content.slice(start, end)
    const structuredBlocks = tryParseStructuredBlocks(rawJson)
    if (!structuredBlocks || structuredBlocks.length === 0) {
      return
    }

    matched = true
    pushMarkdownBlock(blocks, content.slice(lastIndex, start))
    blocks.push(...structuredBlocks)
    lastIndex = end
  })

  if (!matched) {
    return null
  }

  pushMarkdownBlock(blocks, content.slice(lastIndex))
  return blocks
}

function setStructuredContentCache(content: string, blocks: StructuredContentBlock[]): StructuredContentBlock[] {
  structuredContentCache.set(content, blocks)

  if (structuredContentCache.size > STRUCTURED_CONTENT_CACHE_LIMIT) {
    const oldestKey = structuredContentCache.keys().next().value
    if (oldestKey) {
      structuredContentCache.delete(oldestKey)
    }
  }

  return blocks
}

export function parseStructuredContent(content: string): StructuredContentBlock[] {
  const cached = structuredContentCache.get(content)
  if (cached) {
    return cached
  }

  const blocks: StructuredContentBlock[] = []
  const codeBlockPattern = /```(?:json|JSON)?\s*([\s\S]*?)```/g
  let lastIndex = 0
  let matchedJsonCodeBlock = false

  for (const match of content.matchAll(codeBlockPattern)) {
    matchedJsonCodeBlock = true
    const matchIndex = match.index ?? 0
    pushMarkdownBlock(blocks, content.slice(lastIndex, matchIndex))

    const rawJson = match[1]?.trim() ?? ''
    const structuredBlocks = tryParseStructuredBlocks(rawJson)
    if (structuredBlocks && structuredBlocks.length > 0) {
      blocks.push(...structuredBlocks)
    } else {
      pushMarkdownBlock(blocks, match[0])
    }

    lastIndex = matchIndex + match[0].length
  }

  if (matchedJsonCodeBlock) {
    pushMarkdownBlock(blocks, content.slice(lastIndex))
    return setStructuredContentCache(content, blocks)
  }

  const trimmed = content.trim()
  if (!trimmed) {
    return []
  }

  const fullStructuredBlocks = tryParseStructuredBlocks(trimmed)
  if (fullStructuredBlocks && fullStructuredBlocks.length > 0) {
    return setStructuredContentCache(content, fullStructuredBlocks)
  }

  const inlineStructuredBlocks = parseInlineStructuredBlocks(content)
  if (inlineStructuredBlocks && inlineStructuredBlocks.length > 0) {
    return setStructuredContentCache(content, inlineStructuredBlocks)
  }

  const fallbackBlocks = [{ type: 'markdown', content }] satisfies StructuredContentBlock[]
  return setStructuredContentCache(content, fallbackBlocks)
}

export function extractExecutionResult(content: string): StructuredExecutionResult | null {
  const block = parseStructuredContent(content).find(item => item.type === 'result')
  return block?.type === 'result' ? block.result : null
}

export function stripExecutionResultFromContent(content: string): string {
  const blocks = parseStructuredContent(content)
  const hasResultBlock = blocks.some(block => block.type === 'result')
  if (!hasResultBlock) {
    return content
  }

  return blocks
    .filter((block): block is Extract<StructuredContentBlock, { type: 'markdown' }> => block.type === 'markdown')
    .map(block => block.content)
    .join('')
    .trim()
}

export function normalizeStructuredExecutionResult(result: {
  summary?: string | null
  generatedFiles?: string[]
  modifiedFiles?: string[]
  changedFiles?: string[]
  deletedFiles?: string[]
}): StructuredExecutionResult {
  const rawSummary = result.summary?.trim() ?? ''
  const nestedResult = rawSummary ? extractExecutionResult(rawSummary) : null

  return {
    summary: nestedResult?.summary?.trim() || rawSummary,
    generatedFiles: uniqueStrings([
      ...(nestedResult?.generatedFiles ?? []),
      ...(result.generatedFiles ?? [])
    ]),
    modifiedFiles: uniqueStrings([
      ...(nestedResult?.modifiedFiles ?? []),
      ...(result.modifiedFiles ?? [])
    ]),
    changedFiles: uniqueStrings([
      ...(nestedResult?.changedFiles ?? []),
      ...(result.changedFiles ?? [])
    ]),
    deletedFiles: uniqueStrings([
      ...(nestedResult?.deletedFiles ?? []),
      ...(result.deletedFiles ?? [])
    ])
  }
}

export function buildStructuredResultContent(result: {
  summary?: string | null
  generatedFiles?: string[]
  modifiedFiles?: string[]
  changedFiles?: string[]
  deletedFiles?: string[]
}): string {
  const normalized = normalizeStructuredExecutionResult(result)

  return JSON.stringify({
    result_summary: normalized.summary,
    generated_files: normalized.generatedFiles,
    modified_files: normalized.modifiedFiles,
    changed_files: normalized.changedFiles,
    deleted_files: normalized.deletedFiles
  }, null, 2)
}

export function extractFirstFormRequest(content: string): AIFormRequest | null {
  const formBlocks = parseStructuredContent(content).filter(item => item.type === 'form')
  if (formBlocks.length === 0) {
    return null
  }

  return {
    type: 'form_request',
    question: formBlocks[0].question || '需要您的输入',
    forms: formBlocks.map(block => block.formSchema)
  }
}

export function containsFormSchema(content: string, formId?: string): boolean {
  if (!formId) return false

  return parseStructuredContent(content).some(block =>
    block.type === 'form' && block.formSchema.formId === formId
  )
}

export function extractFormResponse(content: string): StructuredFormResponse | null {
  const trimmed = content.trim()
  if (!trimmed) {
    return null
  }

  const fullResponse = tryParseFormResponse(trimmed)
  if (fullResponse) {
    return fullResponse
  }

  for (const { start, end } of extractBalancedJsonRanges(content)) {
    const nestedResponse = tryParseFormResponse(content.slice(start, end))
    if (nestedResponse) {
      return nestedResponse
    }
  }

  return null
}

export function extractTaskSplitResult(content: string): AITaskSplitResult | null {
  const trimmed = content.trim()
  if (!trimmed) {
    return null
  }

  const fullResult = tryParseTaskSplitResult(trimmed)
  if (fullResult) {
    return fullResult
  }

  for (const { start, end } of extractBalancedJsonRanges(content)) {
    const nestedResult = tryParseTaskSplitResult(content.slice(start, end))
    if (nestedResult) {
      return nestedResult
    }
  }

  return null
}

export function extractTaskSplitSummary(content: string): string | null {
  const trimmed = content.trim()
  if (!trimmed) {
    return null
  }

  try {
    const parsed = JSON.parse(trimmed) as unknown
    const summary = extractTaskSplitSummaryFromValue(parsed)
    if (summary) {
      return summary
    }
  } catch {
    try {
      const normalized = normalizeJsonLikeText(trimmed)
      if (normalized !== trimmed) {
        const parsed = JSON.parse(normalized) as unknown
        const summary = extractTaskSplitSummaryFromValue(parsed)
        if (summary) {
          return summary
        }
      }
    } catch {
      // ignore
    }
  }

  for (const { start, end } of extractBalancedJsonRanges(content)) {
    const candidate = content.slice(start, end)
    const summary = extractTaskSplitSummary(candidate)
    if (summary) {
      return summary
    }
  }

  return null
}

export function stripTaskSplitResultFromContent(content: string): string {
  const summaryFromJson = extractTaskSplitSummary(content)
  let stripped = content

  const codeBlockPattern = /```(?:json|JSON)?\s*([\s\S]*?)```/g
  stripped = stripped.replace(codeBlockPattern, (fullMatch, rawJson) => (
    extractTaskSplitResult(String(rawJson ?? '').trim()) ? '' : fullMatch
  ))

  const ranges = extractBalancedJsonRanges(stripped)
  if (ranges.length > 0) {
    let nextContent = ''
    let cursor = 0

    for (const { start, end } of ranges) {
      const candidate = stripped.slice(start, end)
      if (!extractTaskSplitResult(candidate)) {
        continue
      }

      nextContent += stripped.slice(cursor, start)
      cursor = end
    }

    nextContent += stripped.slice(cursor)
    stripped = nextContent
  }

  const normalized = stripped
    .replace(/\n{3,}/g, '\n\n')
    .trim()

  return normalized || summaryFromJson || ''
}

export function extractFirstFormRequestFromContents(contents: Array<string | null | undefined>): AIFormRequest | null {
  for (let index = contents.length - 1; index >= 0; index -= 1) {
    const content = contents[index]?.trim()
    if (!content) {
      continue
    }

    const formRequest = extractFirstFormRequest(content)
    if (formRequest) {
      return formRequest
    }
  }

  return null
}

export function extractTaskSplitResultFromContents(contents: Array<string | null | undefined>): AITaskSplitResult | null {
  for (let index = contents.length - 1; index >= 0; index -= 1) {
    const content = contents[index]?.trim()
    if (!content) {
      continue
    }

    const taskSplitResult = extractTaskSplitResult(content)
    if (taskSplitResult) {
      return taskSplitResult
    }
  }

  return null
}
