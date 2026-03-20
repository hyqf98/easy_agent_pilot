import type { ToolCall } from '@/stores/message'
import type { AIFormRequest, DynamicFormSchema } from '@/types/plan'
import { normalizeFormSchemaForRendering, normalizeFormSchemasForRendering } from './formSchema'

interface ToolCallLogLike {
  id: string
  type: string
  content: string
  metadata?: unknown
}

interface ToolCallMetadata {
  toolName?: string
  toolCallId?: string
  toolInput?: string
  toolResult?: string
  isError?: boolean
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function toToolCallMetadata(metadata: unknown): ToolCallMetadata {
  if (!metadata) return {}

  if (typeof metadata === 'string') {
    try {
      return JSON.parse(metadata) as ToolCallMetadata
    } catch {
      return {}
    }
  }

  if (typeof metadata === 'object') {
    return metadata as ToolCallMetadata
  }

  return {}
}

function toToolCallArguments(raw: string | undefined, fallbackContent: string): Record<string, unknown> {
  const source = raw?.trim() || fallbackContent.trim()
  if (!source) return {}

  try {
    const parsed = JSON.parse(source) as unknown
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>
    }
    return { value: parsed }
  } catch {
    return { value: source }
  }
}

export function extractDynamicFormSchema(payload: unknown): DynamicFormSchema | null {
  if (!isRecord(payload)) {
    return null
  }

  const candidate = payload.formSchema ?? payload.form_schema ?? payload.schema
  if (!isRecord(candidate)) {
    return null
  }

  if (typeof candidate.formId !== 'string' || typeof candidate.title !== 'string' || !Array.isArray(candidate.fields)) {
    return null
  }

  return normalizeFormSchemaForRendering(candidate as unknown as DynamicFormSchema)
}

export function extractDynamicFormSchemas(payload: unknown): DynamicFormSchema[] {
  if (!isRecord(payload)) {
    return []
  }

  const formRequest = payload as Partial<AIFormRequest> & Record<string, unknown>
  const forms = Array.isArray(formRequest.forms)
    ? formRequest.forms
    : []

  const normalizedForms = forms.filter(isRecord).filter(form =>
    typeof form.formId === 'string'
    && typeof form.title === 'string'
    && Array.isArray(form.fields)
  ) as unknown as DynamicFormSchema[]

  if (normalizedForms.length > 0) {
    return normalizeFormSchemasForRendering(normalizedForms)
  }

  const singleSchema = extractDynamicFormSchema(payload)
  return singleSchema ? [singleSchema] : []
}

export function buildToolCallFromLogs<T extends ToolCallLogLike>(
  log: T,
  logs: T[],
  options: {
    toolUseType?: string
    toolInputDeltaType?: string
    toolResultType?: string
    fallbackStatus?: ToolCall['status']
  } = {}
): ToolCall | null {
  const toolUseType = options.toolUseType ?? 'tool_use'
  const toolInputDeltaType = options.toolInputDeltaType ?? 'tool_input_delta'
  const toolResultType = options.toolResultType ?? 'tool_result'
  const fallbackStatus = options.fallbackStatus ?? 'running'

  if (log.type !== toolUseType) {
    return null
  }

  const metadata = toToolCallMetadata(log.metadata)
  if (!metadata.toolName) {
    return null
  }

  const resultLog = logs.find(item => {
    if (item.type !== toolResultType) return false
    const resultMetadata = toToolCallMetadata(item.metadata)
    return resultMetadata.toolCallId === metadata.toolCallId
  })

  const resultMetadata = toToolCallMetadata(resultLog?.metadata)
  const isError = Boolean(resultMetadata.isError)
  const inputDeltaLogs = logs.filter(item => {
    if (item.type !== toolInputDeltaType) return false
    const inputMetadata = toToolCallMetadata(item.metadata)
    return inputMetadata.toolCallId === metadata.toolCallId
  })
  const mergedToolInput = [
    metadata.toolInput,
    ...inputDeltaLogs.map(item => {
      const inputMetadata = toToolCallMetadata(item.metadata)
      return inputMetadata.toolInput || item.content
    })
  ]
    .filter((value): value is string => Boolean(value))
    .join('')

  return {
    id: metadata.toolCallId || log.id,
    name: metadata.toolName,
    arguments: toToolCallArguments(mergedToolInput || metadata.toolInput, log.content),
    status: resultLog ? (isError ? 'error' : 'success') : fallbackStatus,
    result: resultLog?.content,
    errorMessage: isError ? resultLog?.content : undefined
  }
}
