const FENCE_MARKER = '```'

function isJsonLikeLine(line: string): boolean {
  const value = line.trim()
  if (!value) return false

  return (
    /^[\[{]/.test(value) ||
    /^[\]}],?$/.test(value) ||
    /^".*":/.test(value) ||
    /^'.*':/.test(value) ||
    /^(true|false|null)[,]?$/.test(value)
  )
}

function isCommandLikeLine(line: string): boolean {
  const value = line.trim()
  if (!value) return false

  return /^(#\s*)?(npm|npx|pnpm|yarn|bun|bunx|pnpx|uv|uvx|pip|python|python3|node|docker|git|cd|cp|mv|rm|curl|wget|export|set|claude|codex)\b/i.test(value)
}

function detectFenceLanguage(block: string[]): string {
  const jsonLikeCount = block.filter(isJsonLikeLine).length
  return jsonLikeCount >= Math.max(2, Math.ceil(block.length / 2)) ? 'json' : 'bash'
}

function flushCodeBlock(result: string[], block: string[]): void {
  if (block.length === 0) return
  const language = detectFenceLanguage(block)
  result.push(`${FENCE_MARKER}${language}`, ...block, FENCE_MARKER)
  block.length = 0
}

function normalizeStructuredLines(lines: string[]): string[] {
  const result: string[] = []

  for (let index = 0; index < lines.length; index += 1) {
    const current = lines[index] ?? ''
    const trimmed = current.trim()
    const next = lines[index + 1]?.trim() ?? ''

    if (
      trimmed &&
      trimmed.length <= 20 &&
      !/[.。:：,，;；]$/.test(trimmed) &&
      /^\d{1,2}[.)]?$/.test(next)
    ) {
      if (result.length > 0 && result[result.length - 1] !== '') {
        result.push('')
      }
      result.push(`## ${trimmed}`, '')
      continue
    }

    if (/^\d{1,2}[.)]?$/.test(trimmed) && next && !/^\d{1,2}[.)]?$/.test(next)) {
      result.push(`${Number.parseInt(trimmed, 10)}. ${next}`)
      index += 1
      continue
    }

    result.push(current)
  }

  return result
}

export function normalizeMarketplaceMarkdown(content: string | null | undefined): string {
  const normalized = String(content || '')
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    .trim()

  if (!normalized) {
    return ''
  }

  const lines = normalizeStructuredLines(normalized.split('\n'))
  const result: string[] = []
  const codeBlock: string[] = []
  let insideFence = false

  for (const line of lines) {
    const value = line.replace(/\s+$/g, '')
    const trimmed = value.trim()

    if (trimmed.startsWith(FENCE_MARKER)) {
      flushCodeBlock(result, codeBlock)
      insideFence = !insideFence
      result.push(value)
      continue
    }

    if (insideFence) {
      result.push(value)
      continue
    }

    if (trimmed) {
      if (isJsonLikeLine(trimmed) || isCommandLikeLine(trimmed)) {
        codeBlock.push(value)
        continue
      }

      if (codeBlock.length > 0) {
        flushCodeBlock(result, codeBlock)
      }
    } else if (codeBlock.length > 0) {
      flushCodeBlock(result, codeBlock)
      result.push('')
      continue
    }

    result.push(value)
  }

  flushCodeBlock(result, codeBlock)

  return result.join('\n').trim()
}
