const LARGE_FILE_SIZE_BYTES = 512 * 1024
const LARGE_FILE_LINE_COUNT = 8000

export const UNSAVED_CHANGES_CONFIRM = '当前文件有未保存修改，确认放弃这些修改吗？'

export interface FileEditorSelectionRange {
  startLine: number
  endLine: number
}

export function resolveFileEditorName(filePath: string | null): string {
  if (!filePath) {
    return ''
  }

  const normalized = filePath.replace(/\\/g, '/')
  return normalized.split('/').pop() ?? normalized
}

export function isLargeEditorFile(fileSizeBytes: number, lineCount: number): boolean {
  return fileSizeBytes >= LARGE_FILE_SIZE_BYTES || lineCount >= LARGE_FILE_LINE_COUNT
}

/**
 * 将编辑器选区标准化为 store 需要的文本和行号范围。
 */
export function normalizeSelectionPayload(
  payload: { text: string; startLine: number; endLine: number } | null
): {
  selectedText: string
  selectionRange: FileEditorSelectionRange | null
} {
  if (!payload || !payload.text.trim()) {
    return {
      selectedText: '',
      selectionRange: null
    }
  }

  return {
    selectedText: payload.text,
    selectionRange: {
      startLine: Math.min(payload.startLine, payload.endLine),
      endLine: Math.max(payload.startLine, payload.endLine)
    }
  }
}
