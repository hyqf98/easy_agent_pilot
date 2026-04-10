import type { CompletionEntry, MonacoLanguageId } from '../types'

export interface LanguageStrategyContext {
  filePath: string
  fileName: string
  extension: string
}

export interface LanguageStrategy {
  id: string
  monacoLanguageId: MonacoLanguageId
  supportsCompletion: boolean
  match: (ctx: LanguageStrategyContext) => boolean
  getCompletions?: () => CompletionEntry[]
}
