import { getVersion } from '@tauri-apps/api/app'
import { isTauri } from '@tauri-apps/api/core'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater'
import type {
  AppUpdateDownloadEvent,
  AppUpdateHandle,
  AppUpdateInfo,
  AppUpdaterAdapter
} from '@/types/appUpdate'

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
    version: update.version,
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

    const update = await check()
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
