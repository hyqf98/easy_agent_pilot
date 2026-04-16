import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type { MessageAttachment } from '@/stores/message'
import { isImageAttachment } from '@/utils/attachmentMeta'

const previewCache = new Map<string, Promise<string>>()

function buildCacheKey(attachment: MessageAttachment): string {
  return `${attachment.path}::${attachment.mimeType}`
}

function getFileSrc(path: string): string {
  try {
    return convertFileSrc(path)
  } catch {
    return path
  }
}

/**
 * 统一解析图片附件预览地址。
 * 优先使用上传阶段返回的预览 data URL，缺失时再从本地持久化文件生成。
 */
export async function resolveAttachmentPreviewUrl(attachment: MessageAttachment): Promise<string> {
  if (!isImageAttachment(attachment)) {
    return getFileSrc(attachment.path)
  }

  if (attachment.previewUrl?.trim()) {
    return attachment.previewUrl
  }

  const cacheKey = buildCacheKey(attachment)
  const cached = previewCache.get(cacheKey)
  if (cached) {
    return cached
  }

  const fallbackUrl = getFileSrc(attachment.path)
  const task = invoke<string>('resolve_uploaded_image_preview', {
    path: attachment.path,
    mimeType: attachment.mimeType
  }).catch(() => fallbackUrl)

  previewCache.set(cacheKey, task)
  return task
}

export function getAttachmentPreviewUrl(attachment: MessageAttachment): string {
  if (attachment.previewUrl?.trim()) {
    return attachment.previewUrl
  }

  return getFileSrc(attachment.path)
}
