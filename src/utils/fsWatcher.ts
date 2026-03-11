import { watch, type DebouncedWatchOptions, type UnwatchFn, type WatchEvent } from '@tauri-apps/plugin-fs'

let hasWarnedUnavailable = false

function isUnavailableError(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error)
  return message.includes('Command watch not found')
    || message.includes('plugin:fs|watch')
    || message.includes('unsupported')
}

export async function startFsWatcher(
  paths: string | string[],
  cb: (event: WatchEvent) => void,
  options?: DebouncedWatchOptions
): Promise<UnwatchFn | null> {
  try {
    return await watch(paths, cb, options)
  } catch (error) {
    if (isUnavailableError(error)) {
      if (!hasWarnedUnavailable) {
        hasWarnedUnavailable = true
        console.warn('[fsWatcher] 文件监听当前不可用，已降级为手动刷新:', error)
      }
      return null
    }

    throw error
  }
}
