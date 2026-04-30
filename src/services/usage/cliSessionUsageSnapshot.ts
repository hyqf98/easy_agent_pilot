import { invoke } from '@tauri-apps/api/core'

export type CliSessionProvider = 'claude' | 'codex' | 'opencode'

export interface CliSessionUsageSnapshot {
  provider: CliSessionProvider
  model?: string
  inputTokens: number
  outputTokens: number
  contextWindowOccupancy?: number
}

export interface SessionCliSnapshotTarget {
  provider: CliSessionProvider
  cliSessionId: string
}

interface SessionBindingLike {
  id: string
  cliSessionProvider?: string | null
  cliSessionId?: string | null
}

interface RawCliSessionUsageSnapshot {
  provider: string
  model?: string | null
  inputTokens: number
  outputTokens: number
  contextWindowOccupancy?: number | null
}

interface RawRuntimeBindingRecord {
  external_session_id: string
}

function normalizeProvider(provider?: string | null): CliSessionProvider | null {
  const normalizedProvider = provider?.trim().toLowerCase()
  if (normalizedProvider === 'claude') return 'claude'
  if (normalizedProvider === 'codex') return 'codex'
  if (normalizedProvider === 'opencode') return 'opencode'
  return null
}

function resolveRuntimeKeyForProvider(provider: CliSessionProvider): string {
  if (provider === 'claude') return 'claude-cli'
  if (provider === 'codex') return 'codex-cli'
  return 'opencode-cli'
}

function normalizeCount(value?: number | null): number {
  return typeof value === 'number' && Number.isFinite(value) && value > 0
    ? Math.floor(value)
    : 0
}

function normalizeSnapshot(snapshot: RawCliSessionUsageSnapshot): CliSessionUsageSnapshot {
  const inputTokens = normalizeCount(snapshot.inputTokens)
  const outputTokens = normalizeCount(snapshot.outputTokens)
  const contextWindowOccupancy = normalizeCount(snapshot.contextWindowOccupancy)
  const derivedOccupancy = contextWindowOccupancy > 0
    ? contextWindowOccupancy
    : (inputTokens > 0 || outputTokens > 0 ? inputTokens + outputTokens : 0)

  return {
    provider: normalizeProvider(snapshot.provider) ?? 'claude',
    model: snapshot.model?.trim() || undefined,
    inputTokens,
    outputTokens,
    contextWindowOccupancy: derivedOccupancy > 0 ? derivedOccupancy : undefined
  }
}

export async function resolveSessionCliSnapshotTarget(
  session: SessionBindingLike
): Promise<SessionCliSnapshotTarget | null> {
  const provider = normalizeProvider(session.cliSessionProvider)
  if (!provider) {
    return null
  }

  const directCliSessionId = session.cliSessionId?.trim()
  if (directCliSessionId) {
    return {
      provider,
      cliSessionId: directCliSessionId
    }
  }

  const runtimeKey = resolveRuntimeKeyForProvider(provider)
  const binding = await invoke<RawRuntimeBindingRecord | null>('get_session_runtime_binding', {
    sessionId: session.id,
    runtimeKey
  })
  const boundCliSessionId = binding?.external_session_id?.trim()
  if (!boundCliSessionId) {
    return null
  }

  return {
    provider,
    cliSessionId: boundCliSessionId
  }
}

export async function readCliSessionUsageSnapshot(
  target: Partial<SessionCliSnapshotTarget> | null | undefined
): Promise<CliSessionUsageSnapshot | null> {
  const provider = normalizeProvider(target?.provider)
  const cliSessionId = target?.cliSessionId?.trim()
  if (!provider || !cliSessionId) {
    return null
  }

  const snapshot = await invoke<RawCliSessionUsageSnapshot | null>('read_cli_session_usage_snapshot', {
    provider,
    cliSessionId
  })

  return snapshot ? normalizeSnapshot(snapshot) : null
}

export async function readSessionCliUsageSnapshot(
  session: SessionBindingLike
): Promise<CliSessionUsageSnapshot | null> {
  const target = await resolveSessionCliSnapshotTarget(session)
  if (!target) {
    return null
  }

  return readCliSessionUsageSnapshot(target)
}
