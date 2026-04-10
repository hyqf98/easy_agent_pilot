import { readProjectFile } from '@/modules/fileEditor/services/fileEditorService'
import type { ToolCall } from '@/stores/message'
import type { FileEditChangeType, FileEditRange, FileEditTrace } from '@/types/fileTrace'

interface ToolFileTarget {
  filePath: string
  relativePath: string
  changeType: FileEditChangeType
  replacementSnippets: string[]
  reversionEdits: ReversionEdit[]
}

interface ReversionEdit {
  oldString: string
  newString: string
  replaceAll: boolean
}

interface PendingToolEdit {
  toolCallId: string
  toolName: string
  filePath: string
  relativePath: string
  changeType: FileEditChangeType
  requestedAt: string
  beforeContentPromise: Promise<string | null>
  replacementSnippets: string[]
  reversionEdits: ReversionEdit[]
}

interface CreateFileTraceCollectorOptions {
  sessionId: string
  messageId: string
  projectPath?: string
}

function normalizePath(value: string): string {
  return value.replace(/\\/g, '/').replace(/\/+/g, '/')
}

function trimTrailingSlash(value: string): string {
  return value.endsWith('/') ? value.slice(0, -1) : value
}

function resolveFilePath(projectPath: string, candidate: string): string {
  const normalizedProject = trimTrailingSlash(normalizePath(projectPath))
  const normalizedCandidate = normalizePath(candidate)

  if (normalizedCandidate.startsWith('/')) {
    return normalizedCandidate
  }

  return `${normalizedProject}/${normalizedCandidate}`.replace(/\/+/g, '/')
}

function toRelativePath(projectPath: string, absoluteFilePath: string): string {
  const normalizedProject = `${trimTrailingSlash(normalizePath(projectPath))}/`
  const normalizedFile = normalizePath(absoluteFilePath)

  if (normalizedFile.startsWith(normalizedProject)) {
    return normalizedFile.slice(normalizedProject.length)
  }

  return normalizedFile
}

function splitLines(content: string): string[] {
  return content.replace(/\r\n/g, '\n').split('\n')
}

function clampLine(line: number, maxLine: number): number {
  return Math.max(1, Math.min(line, maxLine))
}

function computeDiffRange(beforeContent: string | null, afterContent: string): FileEditRange {
  const beforeLines = splitLines(beforeContent ?? '')
  const afterLines = splitLines(afterContent)
  const maxAfterLine = Math.max(1, afterLines.length)
  const minLength = Math.min(beforeLines.length, afterLines.length)

  let start = 0
  while (start < minLength && beforeLines[start] === afterLines[start]) {
    start += 1
  }

  if (start === beforeLines.length && start === afterLines.length) {
    return {
      startLine: 1,
      endLine: maxAfterLine
    }
  }

  let beforeEnd = beforeLines.length - 1
  let afterEnd = afterLines.length - 1
  while (beforeEnd >= start && afterEnd >= start && beforeLines[beforeEnd] === afterLines[afterEnd]) {
    beforeEnd -= 1
    afterEnd -= 1
  }

  return {
    startLine: clampLine(start + 1, maxAfterLine),
    endLine: clampLine(afterEnd + 1, maxAfterLine)
  }
}

function findSnippetRange(content: string, snippets: string[]): FileEditRange | null {
  if (!content.trim()) {
    return null
  }

  const lines = splitLines(content)
  const normalizedContent = lines.join('\n')

  for (const snippet of snippets) {
    const normalizedSnippet = snippet.replace(/\r\n/g, '\n').trim()
    if (!normalizedSnippet) {
      continue
    }

    const index = normalizedContent.indexOf(normalizedSnippet)
    if (index === -1) {
      continue
    }

    const prefix = normalizedContent.slice(0, index)
    const startLine = prefix.length === 0 ? 1 : prefix.split('\n').length
    const endLine = startLine + normalizedSnippet.split('\n').length - 1

    return {
      startLine,
      endLine
    }
  }

  return null
}

function buildSnippet(lines: string[], startLine: number, endLine: number): string {
  const sliceStart = Math.max(0, startLine - 3)
  const sliceEnd = Math.min(lines.length, endLine + 2)
  return lines.slice(sliceStart, sliceEnd).join('\n')
}

function extractTrimmedString(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim().length > 0 ? value.trim() : undefined
}

function extractSnippetString(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined
}

function pickFirstPathArgument(argumentsObject: Record<string, unknown>): string | undefined {
  return [
    argumentsObject.file_path,
    argumentsObject.filePath,
    argumentsObject.path,
    argumentsObject.file,
    argumentsObject.filename,
    argumentsObject.target_file,
    argumentsObject.targetFile
  ]
    .map(extractTrimmedString)
    .find(Boolean)
}

function collectReplacementSnippets(argumentsObject: Record<string, unknown>): string[] {
  const snippets = new Set<string>()
  const directValues = [
    argumentsObject.new_string,
    argumentsObject.newString,
    argumentsObject.content,
    argumentsObject.replacement,
    argumentsObject.replace
  ]

  for (const value of directValues) {
    const text = extractSnippetString(value)
    if (text) {
      snippets.add(text)
    }
  }

  const edits = Array.isArray(argumentsObject.edits) ? argumentsObject.edits : []
  for (const edit of edits) {
    if (!edit || typeof edit !== 'object') {
      continue
    }

    const entry = edit as Record<string, unknown>
    const replacement = extractSnippetString(entry.new_string)
      ?? extractSnippetString(entry.newString)
      ?? extractSnippetString(entry.replacement)
      ?? extractSnippetString(entry.replace)

    if (replacement) {
      snippets.add(replacement)
    }
  }

  return Array.from(snippets)
}

function collectReversionEdits(argumentsObject: Record<string, unknown>): ReversionEdit[] {
  const edits: ReversionEdit[] = []
  const directOld = extractSnippetString(argumentsObject.old_string)
    ?? extractSnippetString(argumentsObject.oldString)
  const directNew = extractSnippetString(argumentsObject.new_string)
    ?? extractSnippetString(argumentsObject.newString)
    ?? extractSnippetString(argumentsObject.replacement)
    ?? extractSnippetString(argumentsObject.replace)

  if (directOld !== undefined && directNew !== undefined) {
    edits.push({
      oldString: directOld,
      newString: directNew,
      replaceAll: argumentsObject.replace_all === true || argumentsObject.replaceAll === true
    })
  }

  const nestedEdits = Array.isArray(argumentsObject.edits) ? argumentsObject.edits : []
  for (const edit of nestedEdits) {
    if (!edit || typeof edit !== 'object') {
      continue
    }

    const entry = edit as Record<string, unknown>
    const oldString = extractSnippetString(entry.old_string)
      ?? extractSnippetString(entry.oldString)
    const newString = extractSnippetString(entry.new_string)
      ?? extractSnippetString(entry.newString)
      ?? extractSnippetString(entry.replacement)
      ?? extractSnippetString(entry.replace)

    if (oldString === undefined || newString === undefined) {
      continue
    }

    edits.push({
      oldString,
      newString,
      replaceAll: entry.replace_all === true || entry.replaceAll === true
    })
  }

  return edits
}

function reconstructBeforeContent(afterContent: string, edits: ReversionEdit[]): string | null {
  if (!afterContent || edits.length === 0) {
    return null
  }

  let reconstructed = afterContent

  for (let index = edits.length - 1; index >= 0; index -= 1) {
    const edit = edits[index]
    if (!edit.newString && !edit.oldString) {
      continue
    }

    if (edit.replaceAll) {
      if (!edit.newString) {
        continue
      }
      reconstructed = reconstructed.split(edit.newString).join(edit.oldString)
      continue
    }

    const replaceIndex = reconstructed.indexOf(edit.newString)
    if (replaceIndex === -1) {
      return null
    }

    reconstructed = `${reconstructed.slice(0, replaceIndex)}${edit.oldString}${reconstructed.slice(replaceIndex + edit.newString.length)}`
  }

  return reconstructed
}

function extractToolFileTarget(
  toolName: string,
  argumentsObject: Record<string, unknown>,
  projectPath?: string
): ToolFileTarget | null {
  if (!projectPath) {
    return null
  }

  const normalizedName = toolName.toLowerCase()
  const isWriteTool = normalizedName === 'write'
  const isEditTool = normalizedName === 'edit' || normalizedName === 'multiedit'

  if (!isWriteTool && !isEditTool) {
    return null
  }

  const rawPath = pickFirstPathArgument(argumentsObject)
  if (!rawPath) {
    return null
  }

  const filePath = resolveFilePath(projectPath, rawPath)
  const relativePath = toRelativePath(projectPath, filePath)
  const replacementSnippets = collectReplacementSnippets(argumentsObject)
  const reversionEdits = collectReversionEdits(argumentsObject)

  return {
    filePath,
    relativePath,
    changeType: isWriteTool ? 'create' : 'modify',
    replacementSnippets,
    reversionEdits
  }
}

async function safeReadProjectFile(projectPath: string, filePath: string): Promise<string | null> {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      return (await readProjectFile(projectPath, filePath)).content
    } catch {
      if (attempt === 2) {
        return null
      }

      await new Promise((resolve) => {
        setTimeout(resolve, 120 * (attempt + 1))
      })
    }
  }

  return null
}

function buildHunkHeader(relativePath: string, range: FileEditRange): string {
  return `${relativePath}:${range.startLine}-${range.endLine}`
}

export class FileTraceCollector {
  private readonly sessionId: string
  private readonly messageId: string
  private readonly projectPath?: string
  private readonly pendingEdits = new Map<string, PendingToolEdit>()

  constructor(options: CreateFileTraceCollectorOptions) {
    this.sessionId = options.sessionId
    this.messageId = options.messageId
    this.projectPath = options.projectPath
  }

  async captureToolUse(toolCall: ToolCall): Promise<void> {
    const target = extractToolFileTarget(toolCall.name, toolCall.arguments, this.projectPath)
    if (!target || !this.projectPath) {
      return
    }

    this.pendingEdits.set(toolCall.id, {
      toolCallId: toolCall.id,
      toolName: toolCall.name,
      filePath: target.filePath,
      relativePath: target.relativePath,
      changeType: target.changeType,
      requestedAt: new Date().toISOString(),
      beforeContentPromise: safeReadProjectFile(this.projectPath, target.filePath),
      replacementSnippets: target.replacementSnippets,
      reversionEdits: target.reversionEdits
    })
  }

  async resolveToolResult(toolCallId: string, toolResult?: string): Promise<FileEditTrace | null> {
    const pending = this.pendingEdits.get(toolCallId)
    if (!pending || !this.projectPath) {
      return null
    }

    this.pendingEdits.delete(toolCallId)
    let beforeContent = await pending.beforeContentPromise
    const afterContent = await safeReadProjectFile(this.projectPath, pending.filePath)
    if (
      pending.changeType === 'modify'
      && afterContent !== null
      && pending.reversionEdits.length > 0
      && (beforeContent === null || beforeContent === afterContent)
    ) {
      beforeContent = reconstructBeforeContent(afterContent, pending.reversionEdits) ?? beforeContent
    }
    const resultHint = toolResult?.toLowerCase() ?? ''
    const hintedCreate = resultHint.includes('created')
    const hintedDelete = resultHint.includes('deleted') || resultHint.includes('removed')
    const changeType: FileEditChangeType = hintedDelete
      ? 'delete'
      : pending.changeType === 'create' && (beforeContent === null || hintedCreate)
        ? 'create'
        : 'modify'

    if (changeType !== 'delete' && afterContent === null) {
      return null
    }

    if (changeType === 'delete') {
      return {
        id: `trace-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        messageId: this.messageId,
        sessionId: this.sessionId,
        toolCallId,
        filePath: pending.filePath,
        relativePath: pending.relativePath,
        changeType,
        range: {
          startLine: 1,
          endLine: 1
        },
        preview: {
          beforeContent: beforeContent ?? undefined,
          beforeSnippet: beforeContent
            ? buildSnippet(splitLines(beforeContent), 1, Math.min(6, splitLines(beforeContent).length))
            : undefined
        },
        hunkHeader: `${pending.relativePath}:deleted`,
        timestamp: new Date().toISOString()
      }
    }

    const finalAfterContent = afterContent ?? ''
    const range = findSnippetRange(finalAfterContent, pending.replacementSnippets)
      ?? computeDiffRange(beforeContent, finalAfterContent)
    const afterLines = splitLines(finalAfterContent)
    const beforeLines = splitLines(beforeContent ?? '')

    return {
      id: `trace-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      messageId: this.messageId,
      sessionId: this.sessionId,
      toolCallId,
      filePath: pending.filePath,
      relativePath: pending.relativePath,
      changeType,
      range,
      preview: {
        beforeContent: beforeContent ?? undefined,
        afterContent: finalAfterContent,
        beforeSnippet: changeType === 'create'
          ? undefined
          : beforeContent
          ? buildSnippet(beforeLines, range.startLine, Math.min(range.endLine, beforeLines.length || 1))
          : undefined,
        afterSnippet: buildSnippet(afterLines, range.startLine, range.endLine)
      },
      hunkHeader: buildHunkHeader(pending.relativePath, range),
      timestamp: new Date().toISOString()
    }
  }
}
