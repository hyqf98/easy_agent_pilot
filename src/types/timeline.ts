import type { ToolCall } from '@/stores/message'
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
  content?: string
  timestamp?: string
  role?: 'user' | 'assistant' | 'system'
  toolCall?: ToolCall
  formSchema?: DynamicFormSchema
  formDisabled?: boolean
}
