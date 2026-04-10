import { getLanguageStrategy } from '../strategies/registry'
import { activateLspForFile } from '../services/lspService'
import type { CompletionEntry, MonacoLanguageId } from '../types'

export interface FileEditorLanguageState {
  strategyId: string
  languageId: MonacoLanguageId
  completionEntries: CompletionEntry[]
}

/**
 * 解析文件对应的 Monaco 语言和补全项，优先尝试 LSP，失败时静默回退到内置策略。
 */
export async function resolveFileEditorLanguageState(filePath: string): Promise<FileEditorLanguageState> {
  const strategy = getLanguageStrategy(filePath)
  let resolvedLanguageId: MonacoLanguageId = strategy.monacoLanguageId
  const resolvedCompletionEntries: CompletionEntry[] = strategy.getCompletions?.() ?? []

  try {
    const lspResult = await activateLspForFile(filePath)

    if (lspResult.activated) {
      resolvedLanguageId = lspResult.monacoLanguageId as MonacoLanguageId
    }
  } catch {
    // 静默回退到内置语言策略，不提示 LSP 激活信息。
  }

  return {
    strategyId: strategy.id,
    languageId: resolvedLanguageId,
    completionEntries: resolvedCompletionEntries
  }
}
