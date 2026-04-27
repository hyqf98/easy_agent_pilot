import type { Message, MessageRole } from '@/stores/message'
import i18n from '@/i18n'

interface BuildConversationMessagesOptions {
  fallbackUserContent?: string
  sessionId?: string
  injectedSystemMessages?: string[]
}

function normalizeContent(content: string): string {
  return content.replace(/\r\n/g, '\n').trim()
}

function createSyntheticMessage(
  role: Extract<MessageRole, 'system' | 'user'>,
  content: string,
  sessionId: string,
  suffix: string
): Message {
  return {
    id: `synthetic-${suffix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    sessionId,
    role,
    content,
    status: 'completed',
    createdAt: new Date().toISOString()
  }
}

function buildCompressionSummaryMessage(messages: Message[], sessionId: string): Message | null {
  const summaries = messages
    .filter((message) => message.compressionMetadata)
    .map((message) => normalizeContent(message.compressionMetadata?.summaryContent || message.content))
    .filter(Boolean)

  const uniqueSummaries = Array.from(new Set(summaries))
  if (uniqueSummaries.length === 0) {
    return null
  }

  return createSyntheticMessage(
    'system',
    [
      i18n.global.t('compression.historyContextHint') as string,
      ...uniqueSummaries
    ].join('\n\n'),
    sessionId,
    'compression'
  )
}

function dedupeSystemMessages(messages: Message[]): Message[] {
  const seen = new Set<string>()

  return messages.filter((message) => {
    if (message.role !== 'system') {
      return true
    }

    const normalized = normalizeContent(message.content)
    if (!normalized || seen.has(normalized)) {
      return false
    }

    seen.add(normalized)
    return true
  })
}

export function buildConversationMessages(
  messages: Message[],
  options: BuildConversationMessagesOptions = {}
): Message[] {
  const sessionId = options.sessionId || messages[0]?.sessionId || ''
  const assembled: Message[] = []

  for (const content of options.injectedSystemMessages ?? []) {
    const normalized = normalizeContent(content)
    if (!normalized) {
      continue
    }
    assembled.push(createSyntheticMessage('system', normalized, sessionId, 'injected'))
  }

  const compressionSummaryMessage = buildCompressionSummaryMessage(messages, sessionId)
  if (compressionSummaryMessage) {
    assembled.push(compressionSummaryMessage)
  }

  assembled.push(
    ...messages.filter((message) =>
      (message.role === 'system' || message.role === 'user' || message.role === 'assistant') &&
      !(message.role === 'assistant' && !normalizeContent(message.content) && message.status === 'streaming')
    )
  )

  const deduped = dedupeSystemMessages(assembled)
  if (deduped.length > 0) {
    return deduped
  }

  const trimmedFallback = options.fallbackUserContent?.trim()
  if (!trimmedFallback) {
    return []
  }

  return [createSyntheticMessage('user', trimmedFallback, sessionId, 'fallback')]
}
