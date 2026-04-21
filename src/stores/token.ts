import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useMessageStore } from './message'
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

// Token 娴ｈ法鏁ょ痪褍锟?
export type TokenLevel = 'safe' | 'warning' | 'danger' | 'critical'

// Token 娴ｈ法鏁ら幆鍛枌
export interface TokenUsage {
  used: number          // 瀹歌弓濞囬悽?token
  limit: number         // 濡拷锟斤拷锟芥稉濠佺瑓閺傚洨鐛ラ崣?
  percentage: number    // 娴ｈ法鏁ら惂鎯у瀻锟?(0-100)
  level: TokenLevel     // 娴ｈ法鏁ょ痪褍锟?
}

// 鐎圭偞锟?token 閺佺増锟?
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
  keepRecentCount: number  // 压缩时保留最近 N 条消息
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

function resolvePreferredTokenTotal(options: {
  estimatedTokens: number
  persistedTotal: number
  realtimeTotal: number
}): number {
  const { estimatedTokens, persistedTotal, realtimeTotal } = options

  if (realtimeTotal > 0) {
    return Math.max(realtimeTotal, persistedTotal, estimatedTokens)
  }

  if (persistedTotal > 0) {
    return Math.max(persistedTotal, estimatedTokens)
  }

  return estimatedTokens
}

function resolveNonDecreasingTokenValue(
  currentValue: number,
  nextValue: number | undefined
): number {
  if (typeof nextValue !== 'number') {
    return currentValue
  }

  return Math.max(currentValue, nextValue)
}

function estimateMessageTokens(content: string): number {
  const normalized = content.trim()
  if (!normalized) {
    return 0
  }

  return Math.ceil(normalized.length / 4)
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

// 閺嶈宓佹担璺ㄦ暏閻ф儳鍨庡В鏃囧箯閸欐牜楠囬崚?
function getLevel(percentage: number): TokenLevel {
  if (percentage >= 95) return 'critical'
  if (percentage >= 80) return 'danger'
  if (percentage >= 60) return 'warning'
  return 'safe'
}

// 閺嶇厧绱￠崠?token 閺佷即鍣烘稉鍝勫讲鐠囪鐡х粭锔胯
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
  // State
  const sessionTokenCaches = ref<Map<string, SessionTokenCache>>(loadPersistedSessionTokenCaches())
  // 鐎圭偞锟?token 鐎涙ê鍋嶉敍鍫熸降锟?CLI 鏉╂柨娲栭敍?
  const realtimeTokens = ref<Map<string, RealtimeTokenData>>(new Map())

  function calculateSessionEstimatedTokens(sessionId: string): number {
    const messageStore = useMessageStore()
    const messages = messageStore.messagesBySession(sessionId)

    let estimatedTokens = 0
    for (const message of messages) {
      estimatedTokens += estimateMessageTokens(message.content)
    }

    return estimatedTokens
  }

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

  // Getters

  /**
   */
  const getTokenUsage = computed(() => {
    return (sessionId: string): TokenUsage => {
      const messageStore = useMessageStore()
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

      let estimatedTokens = 0
      const messages = messageStore.messagesBySession(sessionId)
      for (const message of messages) {
        estimatedTokens += estimateMessageTokens(message.content)
      }

      // 鐠侊紕鐣诲韫▏閻?锟?锟斤拷锟?token
      const realtimeTotal = sessionExecutionStore.getIsSending(sessionId) && hasMeaningfulRealtimeUsage(realtimeData)
        ? (realtimeData?.inputTokens ?? 0) + (realtimeData?.outputTokens ?? 0)
        : 0
      const persistedTotal = sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0
      const usedTokens = resolvePreferredTokenTotal({
        estimatedTokens,
        persistedTotal,
        realtimeTotal
      })

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

  /**
   */
  const needsCompression = computed(() => {
    return (sessionId: string): boolean => {
      const settingsStore = useSettingsStore()
      const usage = getTokenUsage.value(sessionId)
      // 娴ｈ法鏁ょ拋鍓х枂娑擃厾娈戦崢瀣級闂冨牆锟?
      const threshold = settingsStore.settings.compressionThreshold
      return usage.percentage >= threshold
    }
  })

  // Actions

  /**
   * 閺囧瓨鏌婏拷锟界偞锟?token 閺佺増宓侀敍鍫熸降锟?CLI 鏉╂柨娲栭敍?
   */
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
      ? resolveNonDecreasingTokenValue(existing.inputTokens, inputTokens)
      : existing.inputTokens
    const nextOutputTokens = incomingHasUsage
      ? resolveNonDecreasingTokenValue(existing.outputTokens, outputTokens)
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
    const persistedTotal = sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0
    sessionTokenCaches.value = replaceMapEntry(sessionTokenCaches.value, sessionId, {
      sessionId,
      totalTokens: Math.max(persistedTotal, currentRequestTotal),
      lastUpdated: Date.now()
    })
    persistSessionTokenCaches()
  }

  /**
   * 流式输出期间，基于累积内容长度估算 output tokens。
   * 仅当估算值大于当前已知的 outputTokens 时才更新（真实值优先）。
   */
  function updateRealtimeOutputEstimate(sessionId: string, estimatedOutputTokens: number) {
    const existing = realtimeTokens.value.get(sessionId)
    if (!existing) return
    if (estimatedOutputTokens <= existing.outputTokens) return

    realtimeTokens.value = replaceMapEntry(realtimeTokens.value, sessionId, {
      ...existing,
      outputTokens: estimatedOutputTokens
    })

    const currentRequestTotal = existing.inputTokens + estimatedOutputTokens
    const persistedTotal = sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0
    sessionTokenCaches.value = replaceMapEntry(sessionTokenCaches.value, sessionId, {
      sessionId,
      totalTokens: Math.max(persistedTotal, currentRequestTotal),
      lastUpdated: Date.now()
    })
    persistSessionTokenCaches()
  }

  /**
   * 濞撳懘娅庯拷锟界偞锟?token 閺佺増锟?
   */
  function clearRealtimeTokens(sessionId: string) {
    realtimeTokens.value = deleteMapEntry(realtimeTokens.value, sessionId)
  }

  /**
   */
  function updateSessionTokenCache(sessionId: string) {
    const estimatedTokens = calculateSessionEstimatedTokens(sessionId)
    const persistedTotal = sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0
    const totalTokens = Math.max(estimatedTokens, persistedTotal)

    sessionTokenCaches.value = replaceMapEntry(sessionTokenCaches.value, sessionId, {
      sessionId,
      totalTokens,
      lastUpdated: Date.now()
    })
    persistSessionTokenCaches()
  }

  function rebuildSessionTokenCacheFromMessages(
    sessionId: string,
    options?: { clearRealtime?: boolean }
  ) {
    const estimatedTokens = calculateSessionEstimatedTokens(sessionId)

    sessionTokenCaches.value = replaceMapEntry(sessionTokenCaches.value, sessionId, {
      sessionId,
      totalTokens: estimatedTokens,
      lastUpdated: Date.now()
    })

    if (options?.clearRealtime) {
      realtimeTokens.value = deleteMapEntry(realtimeTokens.value, sessionId)
    }

    persistSessionTokenCaches()
  }

  /**
   */
  function clearSessionTokenCache(sessionId: string) {
    sessionTokenCaches.value = deleteMapEntry(sessionTokenCaches.value, sessionId)
    // 閸氬本妞傚〒鍛存珟鐎圭偞锟?token
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

  /**
   */
  function clearAllTokenCaches() {
    sessionTokenCaches.value = new Map()
    realtimeTokens.value = new Map()
    persistSessionTokenCaches()
  }

  return {
    // State
    sessionTokenCaches,
    realtimeTokens,
    // Getters
    getTokenUsage,
    needsCompression,
    // Actions
    updateRealtimeTokens,
    updateRealtimeOutputEstimate,
    clearRealtimeTokens,
    updateSessionTokenCache,
    rebuildSessionTokenCacheFromMessages,
    clearSessionTokenCache,
    clearProjectSessionTokenCaches,
    clearAllTokenCaches,
    // Utils
    formatTokenCount
  }
})
