import type { MessageAttachment } from '@/stores/message'

export type AttachmentKind = 'image' | 'video' | 'audio' | 'pdf' | 'file'

export function getAttachmentKind(attachment: Pick<MessageAttachment, 'mimeType'>): AttachmentKind {
  const mimeType = attachment.mimeType.toLowerCase()

  if (mimeType.startsWith('image/')) {
    return 'image'
  }

  if (mimeType.startsWith('video/')) {
    return 'video'
  }

  if (mimeType.startsWith('audio/')) {
    return 'audio'
  }

  if (mimeType === 'application/pdf') {
    return 'pdf'
  }

  return 'file'
}

export function isImageAttachment(attachment: Pick<MessageAttachment, 'mimeType'>): boolean {
  return getAttachmentKind(attachment) === 'image'
}

export function getAttachmentExtension(
  attachment: Pick<MessageAttachment, 'name' | 'path' | 'mimeType'>
): string {
  const fileName = attachment.name.trim() || attachment.path.trim()
  const extension = fileName.includes('.')
    ? fileName.split('.').pop()?.trim().toUpperCase() ?? ''
    : ''

  if (extension) {
    return extension.slice(0, 6)
  }

  const mimeTail = attachment.mimeType.split('/').pop()?.trim().toUpperCase() ?? ''
  return mimeTail.slice(0, 6) || 'FILE'
}

export function getAttachmentIconName(
  attachment: Pick<MessageAttachment, 'mimeType'>
): string {
  switch (getAttachmentKind(attachment)) {
    case 'image':
      return 'image'
    case 'video':
      return 'video'
    case 'audio':
      return 'audio-lines'
    case 'pdf':
      return 'file-text'
    default:
      return 'paperclip'
  }
}

export function buildNonImageAttachmentPrompt(
  attachments: MessageAttachment[]
): string {
  const nonImageAttachments = attachments.filter(attachment => !isImageAttachment(attachment))
  if (nonImageAttachments.length === 0) {
    return ''
  }

  const lines = nonImageAttachments.map((attachment, index) => {
    const kind = getAttachmentKind(attachment)
    return [
      `${index + 1}. Name: ${attachment.name}`,
      `   Kind: ${kind}`,
      `   MIME: ${attachment.mimeType}`,
      `   Path: ${attachment.path}`
    ].join('\n')
  })

  return [
    '[Attached local file references]',
    ...lines,
    'Use these local file paths when you need to inspect, summarize, transcribe, or process the attached files.'
  ].join('\n')
}
