import { getVersion } from '@tauri-apps/api/app'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater'
import type {
  AppUpdateDownloadEvent,
  AppUpdateHandle,
  AppUpdateInfo,
  AppUpdaterAdapter
} from '@/types/appUpdate'

function extractTaggedVersion(value: string | null | undefined): string | null {
  const normalized = value?.trim()
  if (!normalized) {
    return null
  }

  const withoutRefPrefix = normalized.replace(/^refs\/tags\//i, '')
  const withoutVersionPrefix = withoutRefPrefix.replace(/^[vV]/, '')

  return withoutVersionPrefix.trim() || null
}

export function resolveUpdateVersion(
  version: string,
  rawJson: Record<string, unknown> | null | undefined
): string {
  const tagName = rawJson?.tag_name
  if (typeof tagName === 'string') {
    return extractTaggedVersion(tagName) || version
  }

  return version
}

function detectPlatform(): string {
  if (typeof navigator === 'undefined') {
    return 'unknown'
  }

  const browserNavigator = navigator as Navigator & {
    userAgentData?: {
      platform?: string
    }
  }

  return browserNavigator.userAgentData?.platform || browserNavigator.platform || browserNavigator.userAgent || 'unknown'
}

function shouldRelaunchAfterInstall(): boolean {
  return !/win/i.test(detectPlatform())
}

function toMetadata(update: Update): AppUpdateInfo {
  return {
    currentVersion: update.currentVersion,
    version: resolveUpdateVersion(update.version, update.rawJson),
    publishedAt: update.date ?? null,
    notes: update.body ?? null,
    rawJson: update.rawJson
  }
}

function toDownloadEvent(event: DownloadEvent): AppUpdateDownloadEvent {
  switch (event.event) {
    case 'Started':
      return {
        event: 'Started',
        contentLength: event.data.contentLength ?? null
      }
    case 'Progress':
      return {
        event: 'Progress',
        chunkLength: event.data.chunkLength
      }
    case 'Finished':
      return {
        event: 'Finished'
      }
  }
}

interface AppUpdateProxyInfo {
  proxy: string | null
  source: string | null
}

async function resolveUpdaterProxy(): Promise<string | undefined> {
  try {
    const result = await invoke<AppUpdateProxyInfo>('resolve_app_update_proxy')
    return result.proxy?.trim() ? result.proxy.trim() : undefined
  } catch (error) {
    console.warn('Failed to resolve updater proxy:', error)
    return undefined
  }
}

class TauriUpdateHandle implements AppUpdateHandle {
  metadata: AppUpdateInfo

  constructor(private readonly update: Update) {
    this.metadata = toMetadata(update)
  }

  async downloadAndInstall(onEvent?: (event: AppUpdateDownloadEvent) => void): Promise<void> {
    await this.update.downloadAndInstall((event) => {
      onEvent?.(toDownloadEvent(event))
    })
  }

  async close(): Promise<void> {
    await this.update.close()
  }
}

/**
 * TauriUpdaterAdapter 封装官方 updater / process 插件。
 */
export class TauriUpdaterAdapter implements AppUpdaterAdapter {
  isSupported(): boolean {
    return isTauri()
  }

  async getCurrentVersion(): Promise<string> {
    if (!this.isSupported()) {
      throw new Error('当前环境不是 Tauri 运行时，无法读取应用版本')
    }

    return getVersion()
  }

  async checkForUpdate(): Promise<AppUpdateHandle | null> {
    if (!this.isSupported()) {
      throw new Error('当前环境不支持应用内更新')
    }

    const proxy = await resolveUpdaterProxy()
    const update = await check({
      timeout: 20_000,
      ...(proxy ? { proxy } : {})
    })
    return update ? new TauriUpdateHandle(update) : null
  }

  async relaunchApp(): Promise<boolean> {
    if (!this.isSupported() || !shouldRelaunchAfterInstall()) {
      return false
    }

    await relaunch()
    return true
  }
}
