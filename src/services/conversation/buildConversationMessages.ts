import type { Message, MessageRole } from '@/stores/message'

export function buildConversationMessages(
  messages: Message[],
  fallbackUserContent?: string,
  sessionId?: string
): Message[] {
  const filtered = messages
    .filter(message =>
      message.role !== 'compression' &&
      (message.role === 'system' || message.role === 'user' || message.role === 'assistant')
    )

  if (filtered.length > 0) {
    return filtered
  }

  const trimmedFallback = fallbackUserContent?.trim()
  if (!trimmedFallback) {
    return []
  }

  return [{
    id: `fallback-${Date.now()}`,
    sessionId: sessionId || '',
    role: 'user' as MessageRole,
    content: trimmedFallback,
    status: 'completed',
    createdAt: new Date().toISOString()
  }]
}
