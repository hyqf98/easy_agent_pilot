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

interface ToolCallLogOptions {
  toolUseType?: string
  toolInputDeltaType?: string
  toolResultType?: string
  fallbackStatus?: ToolCall['status']
}

interface NormalizedToolCallLogOptions {
  toolUseType: string
  toolInputDeltaType: string
  toolResultType: string
  fallbackStatus: ToolCall['status']
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

function normalizeToolCallLogOptions(options: ToolCallLogOptions = {}): NormalizedToolCallLogOptions {
  return {
    toolUseType: options.toolUseType ?? 'tool_use',
    toolInputDeltaType: options.toolInputDeltaType ?? 'tool_input_delta',
    toolResultType: options.toolResultType ?? 'tool_result',
    fallbackStatus: options.fallbackStatus ?? 'running'
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
  options: ToolCallLogOptions = {}
): ToolCall | null {
  const {
    toolUseType,
    toolInputDeltaType,
    toolResultType,
    fallbackStatus
  } = normalizeToolCallLogOptions(options)

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

export function buildToolCallMapFromLogs<T extends ToolCallLogLike>(
  logs: T[],
  options: ToolCallLogOptions = {}
): Map<string, ToolCall> {
  const {
    toolUseType,
    toolInputDeltaType,
    toolResultType,
    fallbackStatus
  } = normalizeToolCallLogOptions(options)

  const resultLogByToolCallId = new Map<string, T>()
  const inputDeltaLogByToolCallId = new Map<string, string[]>()

  for (const log of logs) {
    const metadata = toToolCallMetadata(log.metadata)
    const toolCallId = metadata.toolCallId
    if (!toolCallId) {
      continue
    }

    if (log.type === toolResultType) {
      resultLogByToolCallId.set(toolCallId, log)
      continue
    }

    if (log.type === toolInputDeltaType) {
      const currentInputs = inputDeltaLogByToolCallId.get(toolCallId) ?? []
      currentInputs.push(metadata.toolInput || log.content)
      inputDeltaLogByToolCallId.set(toolCallId, currentInputs)
    }
  }

  const toolCallMap = new Map<string, ToolCall>()

  for (const log of logs) {
    if (log.type !== toolUseType) {
      continue
    }

    const metadata = toToolCallMetadata(log.metadata)
    if (!metadata.toolName) {
      continue
    }

    const toolCallId = metadata.toolCallId || log.id
    const resultLog = resultLogByToolCallId.get(toolCallId)
    const resultMetadata = toToolCallMetadata(resultLog?.metadata)
    const isError = Boolean(resultMetadata.isError)
    const mergedToolInput = [
      metadata.toolInput,
      ...(inputDeltaLogByToolCallId.get(toolCallId) ?? [])
    ]
      .filter((value): value is string => Boolean(value))
      .join('')

    toolCallMap.set(log.id, {
      id: toolCallId,
      name: metadata.toolName,
      arguments: toToolCallArguments(mergedToolInput || metadata.toolInput, log.content),
      status: resultLog ? (isError ? 'error' : 'success') : fallbackStatus,
      result: resultLog?.content,
      errorMessage: isError ? resultLog?.content : undefined
    })
  }

  return toolCallMap
}
