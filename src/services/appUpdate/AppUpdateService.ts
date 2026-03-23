import type {
  AppUpdateCheckResult,
  AppUpdateDownloadEvent,
  AppUpdateHandle,
  AppUpdateInstallResult,
  AppUpdaterAdapter
} from '@/types/appUpdate'

/**
 * AppUpdateService 负责串联版本读取、更新检测与安装流程。
 */
export class AppUpdateService {
  constructor(private readonly adapter: AppUpdaterAdapter) {}

  /**
   * 返回当前运行环境是否支持应用内更新。
   */
  isSupported(): boolean {
    return this.adapter.isSupported()
  }

  /**
   * 读取当前应用版本。
   */
  async getCurrentVersion(): Promise<string> {
    return this.adapter.getCurrentVersion()
  }

  /**
   * 检查是否存在可安装更新。
   */
  async checkForUpdates(): Promise<AppUpdateCheckResult> {
    const [currentVersion, update] = await Promise.all([
      this.adapter.getCurrentVersion(),
      this.adapter.checkForUpdate()
    ])

    return {
      currentVersion,
      checkedAt: new Date().toISOString(),
      update
    }
  }

  /**
   * 下载并安装当前可用更新，完成后按平台策略决定是否重启。
   */
  async installUpdate(
    update: AppUpdateHandle,
    onEvent?: (event: AppUpdateDownloadEvent) => void
  ): Promise<AppUpdateInstallResult> {
    await update.downloadAndInstall(onEvent)
    const relaunchTriggered = await this.adapter.relaunchApp()

    return {
      relaunchTriggered
    }
  }
}
