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
  const sessionTokenCaches = ref<Map<string, SessionTokenCache>>(new Map())
  // 实时 token 存储（来自 CLI 返回）
  const realtimeTokens = ref<Map<string, RealtimeTokenData>>(new Map())

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

      // 计算已使用的 token
      // 优先使用实时 token 数据
      const realtimeData = realtimeTokens.value.get(sessionId)
      let usedTokens = 0

      if (realtimeData) {
        usedTokens = realtimeData.inputTokens + realtimeData.outputTokens
      } else {
        // 如果没有实时数据，使用消息估算
        const messages = messageStore.messagesBySession(sessionId)
        for (const message of messages) {
          if (message.tokens) {
            usedTokens += message.tokens
          } else {
            // 如果消息没有 token 信息，使用简单估算（每4个字符约等于1个token）
            usedTokens += Math.ceil(message.content.length / 4)
          }
        }
      }

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
  function updateRealtimeTokens(sessionId: string, inputTokens: number | undefined, outputTokens: number | undefined) {
    if (inputTokens === undefined && outputTokens === undefined) return
    const existing = realtimeTokens.value.get(sessionId) || { inputTokens: 0, outputTokens: 0 }
    realtimeTokens.value.set(sessionId, {
      inputTokens: inputTokens ?? existing.inputTokens,
      outputTokens: outputTokens ?? existing.outputTokens
    })
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
  }

  /**
   * 清除会话的 token 缓存
   */
  function clearSessionTokenCache(sessionId: string) {
    sessionTokenCaches.value.delete(sessionId)
    // 同时清除实时 token
    clearRealtimeTokens(sessionId)
  }

  /**
   * 清除所有 token 缓存
   */
  function clearAllTokenCaches() {
    sessionTokenCaches.value.clear()
    realtimeTokens.value.clear()
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
    clearAllTokenCaches,
    // Utils
    formatTokenCount
  }
})
