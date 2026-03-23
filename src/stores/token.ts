import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useMessageStore } from './message'
import { useAgentConfigStore } from './agentConfig'
import { useAgentStore } from './agent'
import { useSessionStore } from './session'
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
      const realtimeTotal = hasMeaningfulRealtimeUsage(realtimeData)
        ? (realtimeData?.inputTokens ?? 0) + (realtimeData?.outputTokens ?? 0)
        : 0
      const persistedTotal = sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0
      const hasLoadedMessages = messages.length > 0
      const usedTokens = realtimeTotal > 0
        ? realtimeTotal
        : hasLoadedMessages
          ? estimatedTokens
          : persistedTotal > 0
            ? persistedTotal
            : estimatedTokens

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
    const nextInputTokens = incomingHasUsage ? (inputTokens ?? 0) : existing.inputTokens
    const nextOutputTokens = incomingHasUsage ? (outputTokens ?? 0) : existing.outputTokens

    realtimeTokens.value.set(sessionId, {
      inputTokens: nextInputTokens,
      outputTokens: nextOutputTokens,
      model: model ?? existing.model
    })

    if (incomingHasUsage) {
      sessionTokenCaches.value.set(sessionId, {
        sessionId,
        totalTokens: nextInputTokens + nextOutputTokens,
        lastUpdated: Date.now()
      })
      persistSessionTokenCaches()
    }
  }

  /**
   * 濞撳懘娅庯拷锟界偞锟?token 閺佺増锟?
   */
  function clearRealtimeTokens(sessionId: string) {
    realtimeTokens.value.delete(sessionId)
  }

  /**
   */
  function updateSessionTokenCache(sessionId: string) {
    const messageStore = useMessageStore()
    const messages = messageStore.messagesBySession(sessionId)

    let estimatedTokens = 0
    for (const message of messages) {
      estimatedTokens += estimateMessageTokens(message.content)
    }

    const realtimeData = realtimeTokens.value.get(sessionId)
    const realtimeTotal = hasMeaningfulRealtimeUsage(realtimeData)
      ? (realtimeData?.inputTokens ?? 0) + (realtimeData?.outputTokens ?? 0)
      : 0
    const persistedTotal = sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0
    const hasLoadedMessages = messages.length > 0
    const totalTokens = realtimeTotal > 0
      ? realtimeTotal
      : hasLoadedMessages
        ? estimatedTokens
        : persistedTotal > 0
          ? persistedTotal
          : estimatedTokens

    sessionTokenCaches.value.set(sessionId, {
      sessionId,
      totalTokens,
      lastUpdated: Date.now()
    })
    persistSessionTokenCaches()
  }

  /**
   */
  function clearSessionTokenCache(sessionId: string) {
    sessionTokenCaches.value.delete(sessionId)
    // 閸氬本妞傚〒鍛存珟鐎圭偞锟?token
    clearRealtimeTokens(sessionId)
    persistSessionTokenCaches()
  }

  function clearProjectSessionTokenCaches(sessionIds: string[]) {
    if (sessionIds.length === 0) {
      return
    }

    sessionIds.forEach((sessionId) => {
      sessionTokenCaches.value.delete(sessionId)
      realtimeTokens.value.delete(sessionId)
    })
    persistSessionTokenCaches()
  }

  /**
   */
  function clearAllTokenCaches() {
    sessionTokenCaches.value.clear()
    realtimeTokens.value.clear()
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
    clearRealtimeTokens,
    updateSessionTokenCache,
    clearSessionTokenCache,
    clearProjectSessionTokenCaches,
    clearAllTokenCaches,
    // Utils
    formatTokenCount
  }
})
