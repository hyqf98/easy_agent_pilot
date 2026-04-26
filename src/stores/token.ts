import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useAgentConfigStore } from './agentConfig'
import { useAgentStore } from './agent'
import { useSessionStore } from './session'
import { useSessionExecutionStore } from './sessionExecution'
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
}

function hasMeaningfulRealtimeUsage(data: Pick<RealtimeTokenData, 'inputTokens' | 'outputTokens'> | null | undefined): boolean {
  if (!data) return false
  return data.inputTokens > 0 || data.outputTokens > 0
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

  const getTokenUsage = computed(() => {
    return (sessionId: string): TokenUsage => {
      const agentConfigStore = useAgentConfigStore()
      const agentStore = useAgentStore()
      const sessionStore = useSessionStore()
      const sessionExecutionStore = useSessionExecutionStore()

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

      const hasRealtimeCurrentRequest = sessionExecutionStore.getIsSending(sessionId)
        && hasMeaningfulRealtimeUsage(realtimeData)
      const realtimeTotal = hasRealtimeCurrentRequest
        ? (realtimeData?.inputTokens ?? 0) + (realtimeData?.outputTokens ?? 0)
        : 0
      const persistedTotal = sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0
      const usedTokens = hasRealtimeCurrentRequest ? realtimeTotal : persistedTotal

      const percentage = Math.min(100, (usedTokens / contextWindow) * 100)
      const level = getLevel(percentage)

      return {
        used: usedTokens,
        limit: contextWindow,
        percentage,
        level
      }
    }
  })

  const needsCompression = computed(() => {
    return (sessionId: string): boolean => {
      const settingsStore = useSettingsStore()
      const usage = getTokenUsage.value(sessionId)
      const threshold = settingsStore.settings.compressionThreshold
      return usage.percentage >= threshold
    }
  })

  function updateRealtimeTokens(
    sessionId: string,
    inputTokens: number | undefined,
    outputTokens: number | undefined,
    model?: string
  ) {
    if (inputTokens === undefined && outputTokens === undefined && !model) return
    const existing = realtimeTokens.value.get(sessionId) || { inputTokens: 0, outputTokens: 0 }
    const incomingHasUsage = (inputTokens ?? 0) > 0 || (outputTokens ?? 0) > 0
    const nextInputTokens = incomingHasUsage
      ? accumulateTokenValue(existing.inputTokens, inputTokens)
      : existing.inputTokens
    const nextOutputTokens = incomingHasUsage
      ? accumulateTokenValue(existing.outputTokens, outputTokens)
      : existing.outputTokens

    realtimeTokens.value = replaceMapEntry(realtimeTokens.value, sessionId, {
      inputTokens: nextInputTokens,
      outputTokens: nextOutputTokens,
      model: model ?? existing.model
    })

    if (!incomingHasUsage) {
      return
    }

    const currentRequestTotal = nextInputTokens + nextOutputTokens
    sessionTokenCaches.value = replaceMapEntry(sessionTokenCaches.value, sessionId, {
      sessionId,
      totalTokens: currentRequestTotal,
      lastUpdated: Date.now()
    })
    persistSessionTokenCaches()
  }

  function clearRealtimeTokens(sessionId: string) {
    realtimeTokens.value = deleteMapEntry(realtimeTokens.value, sessionId)
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
    updateSessionTokenCache,
    clearSessionTokenCache,
    clearProjectSessionTokenCaches,
    clearAllTokenCaches,
    formatTokenCount
  }
})
