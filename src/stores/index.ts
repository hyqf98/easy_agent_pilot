import { createPinia } from 'pinia'

export const pinia = createPinia()

// 导出所有 store
export * from './project'
export * from './session'
export * from './agent'
export * from './agentConfig'
// 从 message 导出，但排除 CompressionStrategy（已在 token 中定义）
export {
  useMessageStore,
  type MessageRole,
  type MessageStatus,
  type ToolCallStatus,
  type ToolCall,
  type ToolCallSummary,
  type CompressionMetadata,
  type Message,
  type PaginationState
} from './message'
export type { FileEditTrace, FileEditRange, FileEditPreview, FileEditChangeType } from '@/types/fileTrace'
export * from './settings'
export * from './appUpdate'
export * from './theme'
export * from './layout'
export * from './ui'
export * from './windowState'
export * from './token'
export * from './windowManager'
export * from './appState'
export * from './aiEditTrace'
export * from './tracePreview'
