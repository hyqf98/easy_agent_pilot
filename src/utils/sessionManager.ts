interface RelativeTimeLabels {
  justNow: string
  minutesAgo: (minutes: number) => string
  hoursAgo: (hours: number) => string
  daysAgo: (days: number) => string
}

interface CliSessionPreviewLike {
  first_message: string | null
}

interface CliSessionMessageLike {
  content: string | null
  message_type: string
}

interface SessionMessageFallbackLabels {
  noPreview: string
  fileSnapshot: string
  progress: string
  noParsedContent: string
}

export function formatCliRelativeTime(value: string, labels: RelativeTimeLabels): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 1) return labels.justNow
  if (diffMins < 60) return labels.minutesAgo(diffMins)
  if (diffHours < 24) return labels.hoursAgo(diffHours)
  if (diffDays < 7) return labels.daysAgo(diffDays)

  return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

export function formatCliTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleString()
}

export function displayCliSessionMessage(session: CliSessionPreviewLike, noPreview: string): string {
  return session.first_message || noPreview
}

export function formatCliMessageCount(value: number): string {
  return value >= 0 ? String(value) : '-'
}

export function shortenCliSessionId(sessionId: string): string {
  return sessionId.length > 8 ? `${sessionId.slice(0, 8)}...` : sessionId
}

export function getCliProjectName(path: string, noProjectLabel: string): string {
  if (path === noProjectLabel) {
    return path
  }
  return path.split('/').pop() || path.split('\\').pop() || path
}

export function buildCliDeleteErrorMessage(failedPaths: string[], partialDeleteLabel: (count: number) => string) {
  if (!failedPaths.length) return ''
  return `${partialDeleteLabel(failedPaths.length)}\n${failedPaths.join('\n')}`
}

export function getCliMessageIcon(type: string) {
  switch (type) {
    case 'user': return 'user'
    case 'assistant': return 'bot'
    case 'summary': return 'file-text'
    case 'tool_use': return 'wrench'
    case 'tool_result': return 'terminal'
    case 'reasoning': return 'brain'
    case 'system': return 'settings'
    case 'progress': return 'activity'
    default: return 'message-square'
  }
}

export function getCliMessageColor(type: string) {
  switch (type) {
    case 'user': return 'var(--color-primary)'
    case 'assistant': return 'var(--color-success)'
    case 'summary': return 'var(--color-warning)'
    case 'tool_use': return 'var(--color-primary)'
    case 'tool_result': return 'var(--color-success)'
    case 'reasoning': return 'var(--color-warning)'
    case 'system': return 'var(--color-warning)'
    case 'progress': return 'var(--color-primary)'
    default: return 'var(--color-text-secondary)'
  }
}

export function getCliMessageDisplayContent(
  message: CliSessionMessageLike,
  labels: SessionMessageFallbackLabels
) {
  const content = message.content?.trim()
  if (content) {
    return content
  }

  if (message.message_type === 'file-history-snapshot') {
    return labels.fileSnapshot
  }

  if (message.message_type === 'progress') {
    return labels.progress
  }

  return labels.noParsedContent
}
