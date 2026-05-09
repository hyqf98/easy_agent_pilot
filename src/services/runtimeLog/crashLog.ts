import { invoke } from '@tauri-apps/api/core'
import { getErrorMessage } from '@/utils/api'

export interface CrashLogEntry {
  timestamp: string
  source: string
  message: string
  stackTrace?: string | null
}

export interface CrashLogStatus {
  hasCrashLog: boolean
  crashLogPath?: string | null
  entries: CrashLogEntry[]
}

export async function readCrashLog(): Promise<CrashLogStatus> {
  return invoke<CrashLogStatus>('read_crash_log_command')
}

export async function writeCrashLog(
  source: string,
  message: string,
  stackTrace?: string
): Promise<void> {
  try {
    await invoke('write_crash_log_command', {
      source,
      message,
      stackTrace: stackTrace ?? null
    })
  } catch (error) {
    console.warn('[crashLog] Failed to write crash log:', getErrorMessage(error))
  }
}

export async function clearCrashLog(): Promise<boolean> {
  return invoke<boolean>('clear_crash_log_command')
}
