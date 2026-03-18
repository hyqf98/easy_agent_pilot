import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useMessageStore } from './message'
import { useAgentConfigStore } from './agentConfig'
import { useAgentStore } from './agent'
import { useSessionStore } from './session'
import { useSettingsStore } from './settings'
import { resolveSessionAgent } from '@/utils/sessionAgent'

// 默认上下文窗口大小 (128K)
const DEFAULT_CONTEXT_WINDOW = 128000
const SESSION_TOKEN_CACHE_KEY = 'ea-session-token-cache-v1'

// Token 使用级别
export type TokenLevel = 'safe' | 'warning' | 'danger' | 'critical'

// Token 使用情况
export interface TokenUsage {
  used: number          // 已使用 token
  limit: number         // 模型上下文窗口
  percentage: number    // 使用百分比 (0-100)
  level: TokenLevel     // 使用级别
}

// 实时 token 数据
export interface RealtimeTokenData {
  inputTokens: number
  outputTokens: number
  model?: string
}

function hasMeaningfulRealtimeUsage(data: Pick<RealtimeTokenData, 'inputTokens' | 'outputTokens'> | null | undefined): boolean {
  if (!data) return false
  return data.inputTokens > 0 || data.outputTokens > 0
}

// 匋缩策略
export type CompressionStrategy = 'simple' | 'smart' | 'summary'

// 压缩选项
export interface CompressionOptions {
  strategy: CompressionStrategy
  keepRecentCount: number  // 保留最近 N 条消息
}

// 会话 token 缓存
interface SessionTokenCache {
  sessionId: string
  totalTokens: number
  lastUpdated: number
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

// 根据使用百分比获取级别
function getLevel(percentage: number): TokenLevel {
  if (percentage >= 95) return 'critical'
  if (percentage >= 80) return 'danger'
  if (percentage >= 60) return 'warning'
  return 'safe'
}

// 格式化 token 数量为可读字符串
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
  // 实时 token 存储（来自 CLI 返回）
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
   * 获取会话的 token 使用情况
   */
  const getTokenUsage = computed(() => {
    return (sessionId: string): TokenUsage => {
      const messageStore = useMessageStore()
      const agentConfigStore = useAgentConfigStore()
      const agentStore = useAgentStore()
      const sessionStore = useSessionStore()

      // 获取会话信息
      const session = sessionStore.sessions.find(s => s.id === sessionId)
      if (!session) {
        return { used: 0, limit: DEFAULT_CONTEXT_WINDOW, percentage: 0, level: 'safe' as TokenLevel }
      }

      // 获取会话的智能体
      const agent = resolveSessionAgent(session, agentStore.agents)

      // 获取智能体的模型配置
      let contextWindow = DEFAULT_CONTEXT_WINDOW
      if (agent) {
        const enabledModels = agentConfigStore.getModelsConfigs(agent.id).filter(m => m.enabled)
        const activeModel = enabledModels.find(m => m.isDefault) ?? enabledModels[0]
        if (activeModel?.contextWindow) {
          contextWindow = activeModel.contextWindow
        }
      }

      let estimatedTokens = 0
      const messages = messageStore.messagesBySession(sessionId)
      for (const message of messages) {
        if (message.tokens) {
          estimatedTokens += message.tokens
        } else {
          estimatedTokens += Math.ceil(message.content.length / 4)
        }
      }

      // 计算已使用的 token
      // 优先使用实时 token 数据，没有时回退到消息估算。
      const realtimeData = realtimeTokens.value.get(sessionId)
      const realtimeTotal = hasMeaningfulRealtimeUsage(realtimeData)
        ? (realtimeData?.inputTokens ?? 0) + (realtimeData?.outputTokens ?? 0)
        : 0
      const persistedTotal = sessionTokenCaches.value.get(sessionId)?.totalTokens ?? 0
      const usedTokens = realtimeTotal > 0
        ? realtimeTotal
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
   * 检查会话是否需要压缩
   */
  const needsCompression = computed(() => {
    return (sessionId: string): boolean => {
      const settingsStore = useSettingsStore()
      const usage = getTokenUsage.value(sessionId)
      // 使用设置中的压缩阈值
      const threshold = settingsStore.settings.compressionThreshold
      return usage.percentage >= threshold
    }
  })

  // Actions

  /**
   * 更新实时 token 数据（来自 CLI 返回）
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
   * 清除实时 token 数据
   */
  function clearRealtimeTokens(sessionId: string) {
    realtimeTokens.value.delete(sessionId)
  }

  /**
   * 更新会话的 token 缓存
   */
  function updateSessionTokenCache(sessionId: string) {
    const messageStore = useMessageStore()
    const messages = messageStore.messagesBySession(sessionId)

    let totalTokens = 0
    for (const message of messages) {
      if (message.tokens) {
        totalTokens += message.tokens
      } else {
        totalTokens += Math.ceil(message.content.length / 4)
      }
    }

    sessionTokenCaches.value.set(sessionId, {
      sessionId,
      totalTokens,
      lastUpdated: Date.now()
    })
    persistSessionTokenCaches()
  }

  /**
   * 清除会话的 token 缓存
   */
  function clearSessionTokenCache(sessionId: string) {
    sessionTokenCaches.value.delete(sessionId)
    // 同时清除实时 token
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
   * 清除所有 token 缓存
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
