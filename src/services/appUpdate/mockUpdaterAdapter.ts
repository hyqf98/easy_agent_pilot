import type {
  AppUpdateDownloadEvent,
  AppUpdateHandle,
  AppUpdateInfo,
  AppUpdaterAdapter
} from '@/types/appUpdate'

export interface MockUpdaterScenario {
  currentVersion?: string
  availableUpdate?: Partial<AppUpdateInfo> | null
  checkError?: string | Error | null
  installError?: string | Error | null
  relaunchAfterInstall?: boolean
  progressEvents?: AppUpdateDownloadEvent[]
}

class MockUpdateHandle implements AppUpdateHandle {
  metadata: AppUpdateInfo

  constructor(
    metadata: AppUpdateInfo,
    private readonly installError: string | Error | null | undefined,
    private readonly progressEvents: AppUpdateDownloadEvent[]
  ) {
    this.metadata = metadata
  }

  async downloadAndInstall(onEvent?: (event: AppUpdateDownloadEvent) => void): Promise<void> {
    for (const event of this.progressEvents) {
      onEvent?.(event)
    }

    if (this.installError) {
      throw this.installError
    }
  }

  async close(): Promise<void> {
    return Promise.resolve()
  }
}

/**
 * createMockUpdaterAdapter 用于单测和 Tauri MCP 回归中的场景注入。
 */
export function createMockUpdaterAdapter(
  scenario: MockUpdaterScenario = {}
): AppUpdaterAdapter {
  const currentVersion = scenario.currentVersion ?? '1.0.0'
  const progressEvents = scenario.progressEvents ?? [
    { event: 'Started', contentLength: 100 },
    { event: 'Progress', chunkLength: 25 },
    { event: 'Progress', chunkLength: 75 },
    { event: 'Finished' }
  ]

  return {
    isSupported: () => true,
    async getCurrentVersion() {
      return currentVersion
    },
    async checkForUpdate() {
      if (scenario.checkError) {
        throw scenario.checkError
      }

      if (!scenario.availableUpdate) {
        return null
      }

      return new MockUpdateHandle(
        {
          currentVersion,
          version: scenario.availableUpdate.version ?? '1.1.0',
          publishedAt: scenario.availableUpdate.publishedAt ?? '2026-03-22T00:00:00Z',
          notes: scenario.availableUpdate.notes ?? 'Mock release notes',
          rawJson: scenario.availableUpdate.rawJson ?? {}
        },
        scenario.installError,
        progressEvents
      )
    },
    async relaunchApp() {
      return scenario.relaunchAfterInstall ?? false
    }
  }
}
