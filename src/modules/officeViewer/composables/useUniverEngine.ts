import { LocaleType, createUniver, mergeLocales } from '@univerjs/presets'
import { ref, onBeforeUnmount } from 'vue'
import { useSettingsStore } from '@/stores/settings'

export type UniverPresetType = 'sheets' | 'docs'

interface UniverHandle {
  univer: { dispose: () => void }
  univerAPI: Record<string, any>
}

function resolveUniverLocale(appLang: string): LocaleType {
  return appLang === 'zh-CN' || appLang === 'zh-TW'
    ? LocaleType.ZH_CN
    : LocaleType.EN_US
}

export function useUniverEngine() {
  let handle: UniverHandle | null = null

  const isLoading = ref(false)

  const init = async (
    container: HTMLElement,
    presetType: UniverPresetType,
  ): Promise<UniverHandle['univerAPI']> => {
    if (handle) return handle.univerAPI

    isLoading.value = true
    try {
      const settingsStore = useSettingsStore()
      const locale = resolveUniverLocale(settingsStore.settings.language)

      if (presetType === 'sheets') {
        const { UniverSheetsCorePreset } = await import(
          '@univerjs/preset-sheets-core'
        )
        const sheetsEnUS = await import(
          '@univerjs/preset-sheets-core/locales/en-US'
        )
        const sheetsZhCN = await import(
          '@univerjs/preset-sheets-core/locales/zh-CN'
        )

        await import('@univerjs/preset-sheets-core/lib/index.css')

        const result = createUniver({
          locale,
          locales: {
            [LocaleType.EN_US]: mergeLocales(sheetsEnUS.default ?? sheetsEnUS),
            [LocaleType.ZH_CN]: mergeLocales(sheetsZhCN.default ?? sheetsZhCN),
          },
          presets: [
            UniverSheetsCorePreset({ container }),
          ],
        })

        handle = result as unknown as UniverHandle
      } else if (presetType === 'docs') {
        const { UniverDocsCorePreset } = await import(
          '@univerjs/preset-docs-core'
        )
        const docsEnUS = await import(
          '@univerjs/preset-docs-core/locales/en-US'
        )
        const docsZhCN = await import(
          '@univerjs/preset-docs-core/locales/zh-CN'
        )

        await import('@univerjs/preset-docs-core/lib/index.css')

        const presets: any[] = [
          UniverDocsCorePreset({ container }),
        ]

        try {
          const drawingMod = await import('@univerjs/preset-docs-drawing')
          presets.push(drawingMod.UniverDocsDrawingPreset())
          const drawingEnUS = await import(
            '@univerjs/preset-docs-drawing/locales/en-US'
          )
          const drawingZhCN = await import(
            '@univerjs/preset-docs-drawing/locales/zh-CN'
          )
          await import('@univerjs/preset-docs-drawing/lib/index.css')

          const result = createUniver({
            locale,
            locales: {
              [LocaleType.EN_US]: mergeLocales(
                docsEnUS.default ?? docsEnUS,
                drawingEnUS.default ?? drawingEnUS,
              ),
              [LocaleType.ZH_CN]: mergeLocales(
                docsZhCN.default ?? docsZhCN,
                drawingZhCN.default ?? drawingZhCN,
              ),
            },
            presets,
          })

          handle = result as unknown as UniverHandle
        } catch {
          const result = createUniver({
            locale,
            locales: {
              [LocaleType.EN_US]: mergeLocales(docsEnUS.default ?? docsEnUS),
              [LocaleType.ZH_CN]: mergeLocales(docsZhCN.default ?? docsZhCN),
            },
            presets,
          })

          handle = result as unknown as UniverHandle
        }
      }

      return handle!.univerAPI
    } finally {
      isLoading.value = false
    }
  }

  const dispose = (): void => {
    if (handle) {
      handle.univerAPI.dispose()
      handle.univer.dispose()
      handle = null
    }
  }

  onBeforeUnmount(dispose)

  return {
    get univerAPI() {
      return handle?.univerAPI ?? null
    },
    get isLoading() {
      return isLoading.value
    },
    init,
    dispose,
  }
}
