/**
 * 应用更新状态。
 */
export type AppUpdateStatus =
  | 'idle'
  | 'checking'
  | 'up-to-date'
  | 'available'
  | 'downloading'
  | 'installing'
  | 'completed'
  | 'error'
  | 'unsupported'

/**
 * 应用更新下载事件。
 */
export type AppUpdateDownloadEvent =
  | {
    event: 'Started'
    contentLength: number | null
  }
  | {
    event: 'Progress'
    chunkLength: number
  }
  | {
    event: 'Finished'
  }

/**
 * 前端展示用的更新信息。
 */
export interface AppUpdateInfo {
  currentVersion: string
  version: string
  publishedAt: string | null
  notes: string | null
  rawJson: Record<string, unknown>
}

/**
 * 下载中的进度快照。
 */
export interface AppUpdateProgress {
  contentLength: number | null
  downloadedBytes: number
  percent: number | null
}

/**
 * 检测到的更新句柄。
 */
export interface AppUpdateHandle {
  metadata: AppUpdateInfo
  downloadAndInstall(onEvent?: (event: AppUpdateDownloadEvent) => void): Promise<void>
  close(): Promise<void>
}

/**
 * 更新检查结果。
 */
export interface AppUpdateCheckResult {
  currentVersion: string
  checkedAt: string
  update: AppUpdateHandle | null
}

/**
 * 安装完成后的结果。
 */
export interface AppUpdateInstallResult {
  relaunchTriggered: boolean
}

/**
 * 更新适配器接口，用于隔离 Tauri 运行时与测试模拟。
 */
export interface AppUpdaterAdapter {
  isSupported(): boolean
  getCurrentVersion(): Promise<string>
  checkForUpdate(): Promise<AppUpdateHandle | null>
  relaunchApp(): Promise<boolean>
}
