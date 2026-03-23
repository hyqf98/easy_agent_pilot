import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createMockUpdaterAdapter } from '@/services/appUpdate'

function createStorageMock(): Storage {
  const data = new Map<string, string>()

  return {
    get length() {
      return data.size
    },
    clear() {
      data.clear()
    },
    getItem(key: string) {
      return data.get(key) ?? null
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null
    },
    removeItem(key: string) {
      data.delete(key)
    },
    setItem(key: string, value: string) {
      data.set(key, value)
    }
  }
}

async function loadStores() {
  const [{ useAppUpdateStore }, { useNotificationStore }, { useSettingsStore }] = await Promise.all([
    import('@/stores/appUpdate'),
    import('@/stores/notification'),
    import('@/stores/settings')
  ])

  return {
    useAppUpdateStore,
    useNotificationStore,
    useSettingsStore
  }
}

describe('useAppUpdateStore', () => {
  beforeEach(async () => {
    vi.resetModules()

    Object.defineProperty(globalThis, 'localStorage', {
      value: createStorageMock(),
      configurable: true
    })

    Object.defineProperty(globalThis, 'navigator', {
      value: {
        language: 'zh-CN',
        platform: 'MacIntel',
        userAgent: 'Vitest'
      },
      configurable: true
    })

    Object.defineProperty(globalThis, 'document', {
      value: {
        documentElement: {
          lang: 'zh-CN',
          style: {
            setProperty: vi.fn()
          }
        }
      },
      configurable: true
    })

    Object.defineProperty(globalThis, 'window', {
      value: {
        __TAURI_INTERNALS__: {
          invoke: vi.fn().mockResolvedValue({}),
          transformCallback: vi.fn(),
          unregisterCallback: vi.fn()
        }
      },
      configurable: true
    })

    setActivePinia(createPinia())

    const { useNotificationStore, useAppUpdateStore } = await loadStores()

    const notificationStore = useNotificationStore()
    notificationStore.dismissAll()

    const appUpdateStore = useAppUpdateStore()
    await appUpdateStore.__restoreDefaultAdapterFactory()
  })

  it('marks app as up to date when no update is available', async () => {
    const { useAppUpdateStore, useSettingsStore } = await loadStores()
    const appUpdateStore = useAppUpdateStore()
    const settingsStore = useSettingsStore()

    await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
      currentVersion: '1.0.0',
      availableUpdate: null
    }))

    await appUpdateStore.checkForUpdates()

    expect(appUpdateStore.currentVersion).toBe('1.0.0')
    expect(appUpdateStore.status).toBe('up-to-date')
    expect(appUpdateStore.availableUpdate).toBeNull()
    expect(settingsStore.settings.appUpdateLastCheckedAt).toBeTruthy()
  })

  it('stores update metadata when a new version is available', async () => {
    const { useAppUpdateStore } = await loadStores()
    const appUpdateStore = useAppUpdateStore()

    await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
      currentVersion: '1.0.0',
      availableUpdate: {
        version: '1.1.0',
        publishedAt: '2026-03-22T12:00:00Z',
        notes: 'Regression update'
      }
    }))

    await appUpdateStore.checkForUpdates()

    expect(appUpdateStore.status).toBe('available')
    expect(appUpdateStore.availableUpdate?.version).toBe('1.1.0')
    expect(appUpdateStore.availableUpdate?.notes).toBe('Regression update')
  })

  it('tracks download progress and completes install flow', async () => {
    const { useAppUpdateStore } = await loadStores()
    const appUpdateStore = useAppUpdateStore()

    await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
      currentVersion: '1.0.0',
      availableUpdate: {
        version: '1.1.0'
      },
      relaunchAfterInstall: true
    }))

    await appUpdateStore.checkForUpdates()
    const success = await appUpdateStore.installUpdate()

    expect(success).toBe(true)
    expect(appUpdateStore.status).toBe('completed')
    expect(appUpdateStore.progress?.percent).toBe(100)
    expect(appUpdateStore.progress?.downloadedBytes).toBe(100)
  })

  it('surfaces check failures', async () => {
    const { useAppUpdateStore } = await loadStores()
    const appUpdateStore = useAppUpdateStore()

    await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
      currentVersion: '1.0.0',
      checkError: 'Mock network failure'
    }))

    await appUpdateStore.checkForUpdates()

    expect(appUpdateStore.status).toBe('error')
    expect(appUpdateStore.errorMessage).toContain('Mock network failure')
  })

  it('surfaces install failures after update is found', async () => {
    const { useAppUpdateStore } = await loadStores()
    const appUpdateStore = useAppUpdateStore()

    await appUpdateStore.__setAdapterFactoryForTesting(() => createMockUpdaterAdapter({
      currentVersion: '1.0.0',
      availableUpdate: {
        version: '1.1.0'
      },
      installError: 'Mock install failure'
    }))

    await appUpdateStore.checkForUpdates()
    const success = await appUpdateStore.installUpdate()

    expect(success).toBe(false)
    expect(appUpdateStore.status).toBe('error')
    expect(appUpdateStore.errorMessage).toContain('Mock install failure')
  })
})
