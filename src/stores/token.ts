import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAgentConfigStore } from './agentConfig'
import { useAgentStore } from './agent'
import { useMessageStore } from './message'
import { useSessionStore } from './session'
import { useSettingsStore } from './settings'
import {
  DEFAULT_CONTEXT_WINDOW,
  resolveConfiguredContextWindow
} from '@/utils/configuredModelContext'
import { resolveSessionAgent } from '@/utils/sessionAgent'

const SESSION_TOKEN_CACHE_KEY = 'ea-session-token-cache-v1'

export type TokenLevel = 'safe' | 'warning' | 'danger' | 'critical'

export interface TokenUsage {
  used: number
  limit: number
  percentage: number
  level: TokenLevel
}

export interface RealtimeTokenData {
  inputTokens: number
  outputTokens: number
  model?: string
  contextWindowOccupancy?: number
}

export type CompressionStrategy = 'simple' | 'smart' | 'summary'

export interface CompressionOptions {
  strategy: CompressionStrategy
  keepRecentCount: number
}

interface SessionTokenCache {
  sessionId: string
  totalTokens: number
  lastUpdated: number
}

function replaceMapEntry<K, V>(source: Map<K, V>, key: K, value: V): Map<K, V> {
  const next = new Map(source)
  next.set(key, value)
  return next
}

function deleteMapEntry<K, V>(source: Map<K, V>, key: K): Map<K, V> {
  if (!source.has(key)) {
    return source
  }

  const next = new Map(source)
  next.delete(key)
  return next
}

function accumulateTokenValue(
  currentValue: number,
  nextValue: number | undefined
): number {
  if (typeof nextValue !== 'number') {
    return currentValue
  }

  return currentValue + Math.max(0, nextValue)
}

function loadPersistedSessionTokenCaches(): Map<string, SessionTokenCache> {
  if (typeof window === 'undefined') {
    return new Map()
  }

  try {
    const raw = window.localStorage.getItem(SESSION_TOKEN_CACHE_KEY)
    if (!raw) {
      return new Map()
    }

    const parsed = JSON.parse(raw) as SessionTokenCache[]
    return new Map(parsed
      .filter(entry => entry?.sessionId)
      .map(entry => [entry.sessionId, entry]))
  } catch {
    return new Map()
  }
}

function getLevel(percentage: number): TokenLevel {
  if (percentage >= 95) return 'critical'
  if (percentage >= 80) return 'danger'
  if (percentage >= 60) return 'warning'
  return 'safe'
}

export function formatTokenCount(count: number): string {
  if (count >= 1000000) {
    return `${(count / 1000000).toFixed(1)}M`
  }
  if (count >= 1000) {
    return `${(count / 1000).toFixed(1)}K`
  }
  return count.toString()
}

export const useTokenStore = defineStore('token', () => {
  const sessionTokenCaches = ref<Map<string, SessionTokenCache>>(loadPersistedSessionTokenCaches())
  const realtimeTokens = ref<Map<string, RealtimeTokenData>>(new Map())
  const hydratedSessionIds = new Set<string>()

  function persistSessionTokenCaches() {
    if (typeof window === 'undefined') {
      return
    }

    try {
      window.localStorage.setItem(
        SESSION_TOKEN_CACHE_KEY,
        JSON.stringify(Array.from(sessionTokenCaches.value.values()))
      )
    } catch {
      // ignore persistence failures; runtime cache still works
    }
  }

  async function hydrateSessionTokenFromDb(sessionId: string) {
    if (hydratedSessionIds.has(sessionId)) {
      return
    }
    hydratedSessionIds.add(sessionId)

    if (sessionTokenCaches.value.has(sessionId) || realtimeTokens.value.has(sessionId)) {
      return
    }

    try {
      const result = await invoke<{ total_input_tokens: number; total_output_tokens: number }>(
        'get_session_usage_summary',
        { sessionId }
      )
      const total = result.total_input_tokens + result.total_output_tokens
      if (total > 0) {
        sessionTokenCaches.value = replaceMapEntry(sessionTokenCaches.value, sessionId, {
          sessionId,
          totalTokens: total,
          lastUpdated: Date.now()
        })
        persistSessionTokenCaches()
      }
    } catch {
      // DB query failed, skip hydration
    }
  }

  function getTokenUsage(sessionId: string): TokenUsage {
    const agentConfigStore = useAgentConfigStore()
    const agentStore = useAgentStore()
    const messageStore = useMessageStore()
    const sessionStore = useSessionStore()
    const session = sessionStore.sessions.find(s => s.id === sessionId)
    if (!session) {
      return { used: 0, limit: DEFAULT_CONTEXT_WINDOW, percentage: 0, level: 'safe' as TokenLevel }
    }

    const agent = resolveSessionAgent(session, agentStore.agents)

    const realtimeData = realtimeTokens.value.get(sessionId)
    const realtimeModel = realtimeData?.model?.trim()

    let contextWindow = DEFAULT_CONTEXT_WINDOW
    if (agent) {
      contextWindow = resolveConfiguredContextWindow(
        agentConfigStore.getModelsConfigs(agent.id),
        {
          runtimeModelId: realtimeModel,
          agentModelId: agent.modelId
        }
      )
    }

    const sessionMessages = messageStore.messagesBySession(sessionId)
    const hasCompressionPlaceholderOnly = sessionMessages.length > 0
      && sessionMessages.every(message => Boolean(message.compressionMetadata))
      && !session.cliSessionId?.trim()

    if (hasCompressionPlaceholderOnly) {
      if (sessionTokenCaches.value.has(sessionId) || realtimeTokens.value.has(sessionId)) {
        sessionTokenCaches.value = deleteMapEntry(sessionTokenCaches.value, sessionId)
        realtimeTokens.value = deleteMapEntry(realtimeTokens.value, sessionId)
        persistSessionTokenCaches()
      }

      return {
        used: 0,
        limit: contextWindow,
        percentage: 0,
        level: 'safe'
      }
    }

    const persistedOccupancy = sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0
    const realtimeOccupancy = realtimeData?.contextWindowOccupancy
    const usedTokens = realtimeOccupancy && realtimeOccupancy > 0
      ? realtimeOccupancy
      : persistedOccupancy

    if (usedTokens === 0 && session.cliSessionId?.trim()) {
      hydrateSessionTokenFromDb(sessionId)
    }
    const percentage = contextWindow > 0
      ? Math.min(100, (usedTokens / contextWindow) * 100)
      : 0

    return {
      used: usedTokens,
      limit: contextWindow,
      percentage,
      level: getLevel(percentage)
    }
  }

  function needsCompression(sessionId: string): boolean {
    const settingsStore = useSettingsStore()
    const usage = getTokenUsage(sessionId)
    const threshold = settingsStore.settings.compressionThreshold
    return usage.percentage >= threshold
  }

  function updateRealtimeTokens(
    sessionId: string,
    inputTokens: number | undefined,
    outputTokens: number | undefined,
    model?: string,
    contextWindowOccupancy?: number
  ) {
    if (inputTokens === undefined && outputTokens === undefined && !model && contextWindowOccupancy === undefined) return
    const existing = realtimeTokens.value.get(sessionId) || { inputTokens: 0, outputTokens: 0 }
    const incomingHasUsage = (inputTokens ?? 0) > 0 || (outputTokens ?? 0) > 0
    const nextInputTokens = incomingHasUsage
      ? accumulateTokenValue(existing.inputTokens, inputTokens)
      : existing.inputTokens
    const nextOutputTokens = incomingHasUsage
      ? accumulateTokenValue(existing.outputTokens, outputTokens)
      : existing.outputTokens
    const nextOccupancy = contextWindowOccupancy ?? existing.contextWindowOccupancy

    realtimeTokens.value = replaceMapEntry(realtimeTokens.value, sessionId, {
      inputTokens: nextInputTokens,
      outputTokens: nextOutputTokens,
      model: model ?? existing.model,
      contextWindowOccupancy: nextOccupancy
    })
  }

  function clearRealtimeTokens(sessionId: string) {
    const current = realtimeTokens.value.get(sessionId)
    if (current && (current.inputTokens > 0 || current.outputTokens > 0 || (current.contextWindowOccupancy ?? 0) > 0)) {
      const occupancy = current.contextWindowOccupancy
      sessionTokenCaches.value = replaceMapEntry(sessionTokenCaches.value, sessionId, {
        sessionId,
        totalTokens: occupancy && occupancy > 0 ? occupancy : (sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0),
        lastUpdated: Date.now()
      })
      persistSessionTokenCaches()
    }
    realtimeTokens.value = deleteMapEntry(realtimeTokens.value, sessionId)
  }

  function hardClearSessionTokens(sessionId: string) {
    sessionTokenCaches.value = deleteMapEntry(sessionTokenCaches.value, sessionId)
    realtimeTokens.value = deleteMapEntry(realtimeTokens.value, sessionId)
    persistSessionTokenCaches()
  }

  function updateSessionTokenCache(sessionId: string, totalTokens: number) {
    sessionTokenCaches.value = replaceMapEntry(sessionTokenCaches.value, sessionId, {
      sessionId,
      totalTokens: Math.max(0, totalTokens),
      lastUpdated: Date.now()
    })
    persistSessionTokenCaches()
  }

  function clearSessionTokenCache(sessionId: string) {
    sessionTokenCaches.value = deleteMapEntry(sessionTokenCaches.value, sessionId)
    clearRealtimeTokens(sessionId)
    persistSessionTokenCaches()
  }

  function clearProjectSessionTokenCaches(sessionIds: string[]) {
    if (sessionIds.length === 0) {
      return
    }

    const nextSessionTokenCaches = new Map(sessionTokenCaches.value)
    const nextRealtimeTokens = new Map(realtimeTokens.value)
    sessionIds.forEach((sessionId) => {
      nextSessionTokenCaches.delete(sessionId)
      nextRealtimeTokens.delete(sessionId)
    })
    sessionTokenCaches.value = nextSessionTokenCaches
    realtimeTokens.value = nextRealtimeTokens
    persistSessionTokenCaches()
  }

  function clearAllTokenCaches() {
    sessionTokenCaches.value = new Map()
    realtimeTokens.value = new Map()
    persistSessionTokenCaches()
  }

  return {
    sessionTokenCaches,
    realtimeTokens,
    getTokenUsage,
    needsCompression,
    updateRealtimeTokens,
    clearRealtimeTokens,
    hardClearSessionTokens,
    updateSessionTokenCache,
    clearSessionTokenCache,
    clearProjectSessionTokenCaches,
    clearAllTokenCaches,
    formatTokenCount
  }
})
