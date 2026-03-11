import type { AIFormRequest, DynamicFormSchema } from '@/types/plan'

export interface StructuredExecutionResult {
  summary: string
  generatedFiles: string[]
  modifiedFiles: string[]
  changedFiles: string[]
  deletedFiles: string[]
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

function uniqueStrings(values: string[]): string[] {
  return Array.from(new Set(values.map(item => item.trim()).filter(Boolean)))
}

function isDynamicFormSchema(value: unknown): value is DynamicFormSchema {
  return isRecord(value)
    && typeof value.formId === 'string'
    && typeof value.title === 'string'
    && Array.isArray(value.fields)
}

function toResultBlock(value: Record<string, unknown>): StructuredContentBlock | null {
  const generatedFiles = normalizeStringArray(value.generated_files)
  const modifiedFiles = normalizeStringArray(value.modified_files)
  const changedFiles = normalizeStringArray(value.changed_files)
  const deletedFiles = normalizeStringArray(value.deleted_files)
  const summary = typeof value.result_summary === 'string' ? value.result_summary.trim() : ''

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
  const normalizedForms = forms.filter(isDynamicFormSchema)
  const singleForm = isDynamicFormSchema(value.formSchema) ? value.formSchema : null
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

function parseStructuredJsonValue(value: unknown): StructuredContentBlock[] | null {
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

function tryParseStructuredBlocks(rawJson: string): StructuredContentBlock[] | null {
  try {
    const parsed = JSON.parse(rawJson) as unknown
    return parseStructuredJsonValue(parsed)
  } catch {
    return null
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

export function parseStructuredContent(content: string): StructuredContentBlock[] {
  const blocks: StructuredContentBlock[] = []
  const codeBlockPattern = /```json\s*([\s\S]*?)```/g
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
    return blocks
  }

  const trimmed = content.trim()
  if (!trimmed) {
    return []
  }

  const fullStructuredBlocks = tryParseStructuredBlocks(trimmed)
  if (fullStructuredBlocks && fullStructuredBlocks.length > 0) {
    return fullStructuredBlocks
  }

  const inlineStructuredBlocks = parseInlineStructuredBlocks(content)
  if (inlineStructuredBlocks && inlineStructuredBlocks.length > 0) {
    return inlineStructuredBlocks
  }

  return [{ type: 'markdown', content }]
}

export function extractExecutionResult(content: string): StructuredExecutionResult | null {
  const block = parseStructuredContent(content).find(item => item.type === 'result')
  return block?.type === 'result' ? block.result : null
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
