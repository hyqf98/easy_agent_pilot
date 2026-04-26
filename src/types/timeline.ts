import type { MessageAttachment, ToolCall } from '@/stores/message'
import type { DynamicFormSchema } from './plan'

export type TimelineEntryType =
  | 'message'
  | 'content'
  | 'thinking'
  | 'tool'
  | 'form'
  | 'error'
  | 'system'

export interface TimelineEntry {
  id: string
  type: TimelineEntryType
  sequence?: number
  content?: string
  attachments?: MessageAttachment[]
  timestamp?: string
  metaLabel?: string
  role?: 'user' | 'assistant' | 'system'
  animate?: boolean
  toolCall?: ToolCall
  toolCompact?: boolean
  toolDefaultExpanded?: boolean
  toolDefaultResultExpanded?: boolean
  formSchema?: DynamicFormSchema
  formPrompt?: string
  formInitialValues?: Record<string, unknown>
  formDisabled?: boolean
  formVariant?: 'active' | 'submitted' | 'archived'
  runtimeFallbackUsage?: {
    model?: string
    inputTokens?: number
    outputTokens?: number
  }
}
