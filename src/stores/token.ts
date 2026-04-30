import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useAgentConfigStore } from './agentConfig'
import { inferAgentProvider, useAgentStore } from './agent'
import { useMessageStore } from './message'
import { useSessionStore } from './session'
import { readSessionCliUsageSnapshot } from '@/services/usage/cliSessionUsageSnapshot'
import {
  DEFAULT_CONTEXT_WINDOW,
  resolveConfiguredContextWindow
} from '@/utils/configuredModelContext'
import { formatContextWindowCount } from '@/utils/contextWindow'
import { resolveSessionAgent } from '@/utils/sessionAgent'
import { getUsageNoticeSummary, type RuntimeNotice } from '@/utils/runtimeNotice'

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

function parsePersistedTokenCount(value?: string | null): number | undefined {
  if (!value) {
    return undefined
  }

  const digitsOnly = value.replace(/[^\d]/g, '')
  if (!digitsOnly) {
    return undefined
  }

  const parsed = Number.parseInt(digitsOnly, 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : undefined
}

function extractPersistedUsageCounts(notice: RuntimeNotice): {
  inputTokens?: number
  outputTokens?: number
  contextWindowOccupancy?: number
} {
  const lines = notice.content
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)
    .map(line => line.replace(/^-\s*/, ''))

  let inputTokens: number | undefined
  let outputTokens: number | undefined
  let contextWindowOccupancy: number | undefined

  for (const line of lines) {
    const separatorIndex = line.indexOf(':')
    if (separatorIndex < 0) {
      continue
    }

    const label = line.slice(0, separatorIndex).trim().toLowerCase()
    const value = line.slice(separatorIndex + 1).trim()
    const parsed = parsePersistedTokenCount(value)
    if (parsed === undefined) {
      continue
    }

    if (label.includes('输入') || label.includes('input')) {
      inputTokens = parsed
      continue
    }

    if (label.includes('上下文占用') || label.includes('context occupancy') || label.includes('context window occupancy')) {
      contextWindowOccupancy = parsed
      continue
    }

    if (label.includes('输出') || label.includes('output')) {
      outputTokens = parsed
    }
  }

  return {
    inputTokens,
    outputTokens,
    contextWindowOccupancy
  }
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

function getLevel(percentage: number): TokenLevel {
  if (percentage >= 95) return 'critical'
  if (percentage >= 80) return 'danger'
  if (percentage >= 60) return 'warning'
  return 'safe'
}

export function formatTokenCount(count: number): string {
  if (count <= 0) {
    return '0'
  }
  return formatContextWindowCount(count)
}

export const useTokenStore = defineStore('token', () => {
  const realtimeTokens = ref<Map<string, RealtimeTokenData>>(new Map())

  async function restorePersistedSessionTokens(sessionId: string): Promise<RealtimeTokenData | null> {
    const messageStore = useMessageStore()
    const sessionStore = useSessionStore()
    const session = sessionStore.sessions.find(item => item.id === sessionId)
    const boundProvider = session?.cliSessionProvider?.trim().toLowerCase()
    const hasPersistedCliRuntime = Boolean(boundProvider)

    if (!hasPersistedCliRuntime) {
      return null
    }

    const latestUsageNotice = [...messageStore.messagesBySession(sessionId)]
      .reverse()
      .find(message => message.role === 'assistant' && message.runtimeNotices?.some(notice => notice.id === 'usage'))
      ?.runtimeNotices
      ?.find(notice => notice.id === 'usage')

    if (session) {
      try {
        const snapshot = await readSessionCliUsageSnapshot(session)
        if (snapshot) {
          return {
            inputTokens: snapshot.inputTokens ?? 0,
            outputTokens: snapshot.outputTokens ?? 0,
            model: snapshot.model,
            contextWindowOccupancy: snapshot.contextWindowOccupancy
          }
        }
      } catch (error) {
        console.warn(`[TokenStore] Failed to restore ${boundProvider} usage snapshot:`, error)
      }
    }

    if (!latestUsageNotice) {
      return null
    }

    const summary = getUsageNoticeSummary(latestUsageNotice)
    if (!summary) {
      return null
    }

    const rawCounts = extractPersistedUsageCounts(latestUsageNotice)
    const inputTokens = rawCounts.inputTokens ?? parsePersistedTokenCount(summary.input)
    const outputTokens = rawCounts.outputTokens ?? parsePersistedTokenCount(summary.output)
    const contextWindowOccupancy = rawCounts.contextWindowOccupancy
      ?? (
        inputTokens !== undefined || outputTokens !== undefined
          ? (inputTokens ?? 0) + (outputTokens ?? 0)
          : undefined
      )
    const model = summary.model?.trim() || undefined

    if (inputTokens === undefined && outputTokens === undefined && !model) {
      return null
    }

    return {
      inputTokens: inputTokens ?? 0,
      outputTokens: outputTokens ?? 0,
      model,
      contextWindowOccupancy
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
    if (!realtimeData) {
      void restorePersistedSessionTokens(sessionId).then(restored => {
        if (!restored) {
          return
        }
        realtimeTokens.value = replaceMapEntry(realtimeTokens.value, sessionId, restored)
      })
    }
    const sessionMessages = messageStore.messagesBySession(sessionId)
    const latestUsageNotice = [...sessionMessages]
      .reverse()
      .find(message => message.role === 'assistant' && message.runtimeNotices?.some(notice => notice.id === 'usage'))
      ?.runtimeNotices
      ?.find(notice => notice.id === 'usage')
    const persistedUsageCounts = latestUsageNotice
      ? extractPersistedUsageCounts(latestUsageNotice)
      : {}
    const persistedUsageSummary = latestUsageNotice
      ? getUsageNoticeSummary(latestUsageNotice)
      : null
    const persistedOccupancy = persistedUsageCounts.contextWindowOccupancy
      ?? (
        persistedUsageCounts.inputTokens !== undefined || persistedUsageCounts.outputTokens !== undefined
          ? (persistedUsageCounts.inputTokens ?? 0) + (persistedUsageCounts.outputTokens ?? 0)
          : undefined
      )
    const realtimeModel = realtimeData?.model?.trim() || persistedUsageSummary?.model?.trim() || undefined

    let contextWindow = DEFAULT_CONTEXT_WINDOW
    if (agent) {
      const configuredModels = agentConfigStore.getModelsConfigs(agent.id)
      if (configuredModels.length === 0) {
        void agentConfigStore.ensureModelsConfigs(agent.id, inferAgentProvider(agent)).catch((error) => {
          console.warn(`[TokenStore] Failed to load model configs for ${agent.id}:`, error)
        })
      }

      contextWindow = resolveConfiguredContextWindow(
        configuredModels,
        {
          runtimeModelId: realtimeModel,
          agentModelId: agent.modelId
        }
      )
    }

    const hasCompressionPlaceholderOnly = sessionMessages.length > 0
      && sessionMessages.every(message => Boolean(message.compressionMetadata))
      && !session.cliSessionId?.trim()

    if (hasCompressionPlaceholderOnly) {
      if (realtimeTokens.value.has(sessionId)) {
        realtimeTokens.value = deleteMapEntry(realtimeTokens.value, sessionId)
      }

      return {
        used: 0,
        limit: contextWindow,
        percentage: 0,
        level: 'safe'
      }
    }

    const usedTokens = realtimeData?.contextWindowOccupancy
      ?? persistedOccupancy
      ?? 0

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

  function updateRealtimeTokens(
    sessionId: string,
    inputTokens: number | undefined,
    outputTokens: number | undefined,
    model?: string,
    contextWindowOccupancy?: number
  ) {
    if (inputTokens === undefined && outputTokens === undefined && !model && contextWindowOccupancy === undefined) return
    const existing = realtimeTokens.value.get(sessionId) || { inputTokens: 0, outputTokens: 0 }

    realtimeTokens.value = replaceMapEntry(realtimeTokens.value, sessionId, {
      inputTokens: inputTokens ?? existing.inputTokens,
      outputTokens: outputTokens ?? existing.outputTokens,
      model: model ?? existing.model,
      contextWindowOccupancy: contextWindowOccupancy ?? existing.contextWindowOccupancy
    })
  }

  function clearRealtimeTokens(sessionId: string) {
    realtimeTokens.value = deleteMapEntry(realtimeTokens.value, sessionId)
  }

  function hardClearSessionTokens(sessionId: string) {
    realtimeTokens.value = deleteMapEntry(realtimeTokens.value, sessionId)
  }

  function clearProjectSessionTokenCaches(sessionIds: string[]) {
    if (sessionIds.length === 0) {
      return
    }

    const nextRealtimeTokens = new Map(realtimeTokens.value)
    sessionIds.forEach((sessionId) => {
      nextRealtimeTokens.delete(sessionId)
    })
    realtimeTokens.value = nextRealtimeTokens
  }

  return {
    realtimeTokens,
    getTokenUsage,
    updateRealtimeTokens,
    clearRealtimeTokens,
    hardClearSessionTokens,
    clearProjectSessionTokenCaches,
    restorePersistedSessionTokens,
    formatTokenCount
  }
})
