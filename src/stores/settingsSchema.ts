import { DEFAULT_MINI_PANEL_SHORTCUT, migrateMiniPanelShortcut } from '@/utils/shortcut'

export interface AppSettings {
  language: string
  fontSize: number
  autoSave: boolean
  autoSaveInterval: number
  confirmBeforeDelete: boolean
  sendOnEnter: boolean
  miniPanelEnabled: boolean
  miniPanelShortcut: string
  miniPanelShortcutOverride: boolean
  editorFontSize: number
  editorTabSize: number
  editorWordWrap: boolean
  enableDebugMode: boolean
  logLevel: 'debug' | 'info' | 'warn' | 'error'
  autoCheckAppUpdate: boolean
  appUpdateLastCheckedAt: string | null
  compressionStrategy: 'simple' | 'smart' | 'summary'
  compressionThreshold: number
  autoCompressionEnabled: boolean
}

type SettingsValue = AppSettings[keyof AppSettings]

export const defaultSettings: AppSettings = {
  language: 'zh-CN',
  fontSize: 14,
  autoSave: true,
  autoSaveInterval: 30,
  confirmBeforeDelete: true,
  sendOnEnter: true,
  miniPanelEnabled: false,
  miniPanelShortcut: DEFAULT_MINI_PANEL_SHORTCUT,
  miniPanelShortcutOverride: false,
  editorFontSize: 14,
  editorTabSize: 2,
  editorWordWrap: true,
  enableDebugMode: false,
  logLevel: 'info',
  autoCheckAppUpdate: true,
  appUpdateLastCheckedAt: null,
  compressionStrategy: 'summary',
  compressionThreshold: 80,
  autoCompressionEnabled: true
}

export interface SettingsFieldCodec {
  key: keyof AppSettings
  parse: (raw: string) => SettingsValue
  stringify: (value: SettingsValue) => string
}

function parseBoolean(raw: string): boolean {
  return raw === 'true'
}

function parseInteger(raw: string, fallback: number): number {
  const parsed = parseInt(raw, 10)
  return Number.isFinite(parsed) ? parsed : fallback
}

function booleanCodec(key: keyof AppSettings): SettingsFieldCodec {
  return {
    key,
    parse: (raw) => parseBoolean(raw),
    stringify: (value) => String(value)
  }
}

function integerCodec(
  key: keyof AppSettings,
  fallback: number
): SettingsFieldCodec {
  return {
    key,
    parse: (raw) => parseInteger(raw, fallback),
    stringify: (value) => String(value)
  }
}

function stringCodec(
  key: keyof AppSettings,
  normalize?: (value: string) => string
): SettingsFieldCodec {
  return {
    key,
    parse: (raw) => normalize ? normalize(raw) : raw,
    stringify: (value) => String(value)
  }
}

function nullableStringCodec(
  key: keyof AppSettings
): SettingsFieldCodec {
  return {
    key,
    parse: (raw) => (raw && raw !== 'null' ? raw : null),
    stringify: (value) => (value == null ? 'null' : String(value))
  }
}

function enumCodec(
  key: keyof AppSettings,
  allowedValues: readonly SettingsValue[],
  fallback: SettingsValue
): SettingsFieldCodec {
  return {
    key,
    parse: (raw) => allowedValues.includes(raw) ? raw : fallback,
    stringify: (value) => String(value)
  }
}

export const settingsFieldCodecs: SettingsFieldCodec[] = [
  stringCodec('language'),
  integerCodec('fontSize', defaultSettings.fontSize),
  booleanCodec('autoSave'),
  integerCodec('autoSaveInterval', defaultSettings.autoSaveInterval),
  booleanCodec('confirmBeforeDelete'),
  booleanCodec('sendOnEnter'),
  booleanCodec('miniPanelEnabled'),
  stringCodec('miniPanelShortcut', (value) => migrateMiniPanelShortcut(value || defaultSettings.miniPanelShortcut)),
  booleanCodec('miniPanelShortcutOverride'),
  integerCodec('editorFontSize', defaultSettings.editorFontSize),
  integerCodec('editorTabSize', defaultSettings.editorTabSize),
  booleanCodec('editorWordWrap'),
  booleanCodec('enableDebugMode'),
  enumCodec('logLevel', ['debug', 'info', 'warn', 'error'] as const, defaultSettings.logLevel),
  booleanCodec('autoCheckAppUpdate'),
  nullableStringCodec('appUpdateLastCheckedAt'),
  enumCodec('compressionStrategy', ['simple', 'smart', 'summary'] as const, defaultSettings.compressionStrategy),
  integerCodec('compressionThreshold', defaultSettings.compressionThreshold),
  booleanCodec('autoCompressionEnabled')
]

const settingsFieldCodecMap = new Map(
  settingsFieldCodecs.map((codec) => [codec.key, codec])
)

export function parseStoredSettings(savedSettings: Record<string, string>): Partial<AppSettings> {
  const parsedSettings: Partial<AppSettings> = {}

  for (const [key, rawValue] of Object.entries(savedSettings)) {
    const codec = settingsFieldCodecMap.get(key as keyof AppSettings)
    if (!codec) {
      continue
    }

    ;(parsedSettings as Record<keyof AppSettings, SettingsValue>)[codec.key] = codec.parse(rawValue)
  }

  return parsedSettings
}

export function serializeSettings(settings: Partial<AppSettings>): Record<string, string> {
  const serialized: Record<string, string> = {}

  for (const codec of settingsFieldCodecs) {
    if (!Object.prototype.hasOwnProperty.call(settings, codec.key)) {
      continue
    }

    const value = settings[codec.key]
    if (value === undefined) {
      continue
    }

    serialized[codec.key] = codec.stringify(value)
  }

  return serialized
}
