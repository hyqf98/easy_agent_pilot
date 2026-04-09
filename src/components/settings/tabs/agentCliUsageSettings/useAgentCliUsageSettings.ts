import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as echarts from 'echarts'
import { useI18n } from 'vue-i18n'
import type { SelectOption } from '@/components/common'
import { useAgentCliUsageStore } from '@/stores/agentCliUsage'
import {
  applyAgentChart,
  applyModelChart,
  applyProviderChart,
  applyTrendChart,
  formatCurrency,
  formatDateTime,
  formatInteger,
  formatPercentage
} from './chartUtils'

interface ProviderBreakdownEntry {
  provider: string
  label: string
  callCount: number
  totalTokens: number
  estimatedTotalCostUsd: number
}

/**
 * Agent CLI 用量页面逻辑。
 * 负责筛选条件、统计摘要、ECharts 生命周期以及图表重绘。
 */
export function useAgentCliUsageSettings() {
  const { t } = useI18n()
  const usageStore = useAgentCliUsageStore()

  const trendChartRef = ref<HTMLDivElement | null>(null)
  const providerChartRef = ref<HTMLDivElement | null>(null)
  const agentChartRef = ref<HTMLDivElement | null>(null)
  const modelChartRef = ref<HTMLDivElement | null>(null)

  let trendChart: echarts.ECharts | null = null
  let providerChart: echarts.ECharts | null = null
  let agentChart: echarts.ECharts | null = null
  let modelChart: echarts.ECharts | null = null
  let resizeObserver: ResizeObserver | null = null

  const cliTypeOptions = computed<SelectOption[]>(() => [
    { value: 'all', label: t('settings.usageStats.providerAll') },
    { value: 'claude', label: 'Claude CLI' },
    { value: 'codex', label: 'Codex CLI' },
    { value: 'opencode', label: 'OpenCode CLI' }
  ])

  const dateRangePresets = computed(() => [
    { key: 'today', label: t('settings.usageStats.presetToday'), days: 0 as const },
    { key: 'last7', label: t('settings.usageStats.presetLast7Days'), days: 6 },
    { key: 'last30', label: t('settings.usageStats.presetLast30Days'), days: 29 },
    { key: 'last90', label: t('settings.usageStats.presetLast90Days'), days: 89 }
  ])

  const summaryCards = computed(() => [
    {
      key: 'calls',
      label: t('settings.usageStats.summaryCalls'),
      value: formatInteger(usageStore.stats.summary.totalCalls)
    },
    {
      key: 'input',
      label: t('settings.usageStats.summaryInputTokens'),
      value: formatInteger(usageStore.stats.summary.inputTokens)
    },
    {
      key: 'output',
      label: t('settings.usageStats.summaryOutputTokens'),
      value: formatInteger(usageStore.stats.summary.outputTokens)
    },
    {
      key: 'total',
      label: t('settings.usageStats.summaryTotalTokens'),
      value: formatInteger(usageStore.stats.summary.totalTokens)
    },
    {
      key: 'cost',
      label: t('settings.usageStats.summaryEstimatedCost'),
      value: formatCurrency(usageStore.stats.summary.estimatedTotalCostUsd)
    }
  ])

  const providerBreakdown = computed<ProviderBreakdownEntry[]>(() => {
    const grouped = new Map<string, ProviderBreakdownEntry>()

    for (const row of usageStore.stats.breakdown) {
      const provider = (row.provider || 'unknown').toLowerCase()
      const label = provider === 'claude'
        ? 'Claude CLI'
        : provider === 'codex'
          ? 'Codex CLI'
          : provider

      const existing = grouped.get(provider)
      if (existing) {
        existing.callCount += row.callCount
        existing.totalTokens += row.totalTokens
        existing.estimatedTotalCostUsd += row.estimatedTotalCostUsd
        continue
      }

      grouped.set(provider, {
        provider,
        label,
        callCount: row.callCount,
        totalTokens: row.totalTokens,
        estimatedTotalCostUsd: row.estimatedTotalCostUsd
      })
    }

    return [...grouped.values()].sort((left, right) => (
      right.estimatedTotalCostUsd - left.estimatedTotalCostUsd
      || right.totalTokens - left.totalTokens
    ))
  })

  const topAgentRows = computed(() => usageStore.stats.breakdown.slice(0, 8))
  const topModelRows = computed(() => usageStore.modelStats.breakdown.slice(0, 10))

  const averageTokensPerCall = computed(() => {
    if (usageStore.stats.summary.totalCalls <= 0) {
      return 0
    }

    return usageStore.stats.summary.totalTokens / usageStore.stats.summary.totalCalls
  })

  const averageCostPerCall = computed(() => {
    if (usageStore.stats.summary.totalCalls <= 0) {
      return 0
    }

    return usageStore.stats.summary.estimatedTotalCostUsd / usageStore.stats.summary.totalCalls
  })

  const topModel = computed(() => usageStore.modelStats.breakdown[0] ?? null)

  const dominantProvider = computed(() => {
    const firstProvider = providerBreakdown.value[0]
    if (!firstProvider || usageStore.stats.summary.totalCalls <= 0) {
      return null
    }

    return {
      ...firstProvider,
      callShare: firstProvider.callCount / usageStore.stats.summary.totalCalls
    }
  })

  const insightCards = computed(() => [
    {
      key: 'provider',
      label: t('settings.usageStats.insightDominantProvider'),
      value: dominantProvider.value?.label ?? '-',
      detail: dominantProvider.value
        ? `${formatPercentage(dominantProvider.value.callShare)} ${t('settings.usageStats.summaryCalls')}`
        : '-'
    },
    {
      key: 'avgTokens',
      label: t('settings.usageStats.insightAverageTokens'),
      value: formatInteger(Math.round(averageTokensPerCall.value)),
      detail: t('settings.usageStats.summaryTotalTokens')
    },
    {
      key: 'avgCost',
      label: t('settings.usageStats.insightAverageCost'),
      value: formatCurrency(averageCostPerCall.value),
      detail: t('settings.usageStats.summaryEstimatedCost')
    },
    {
      key: 'topModel',
      label: t('settings.usageStats.insightTopModel'),
      value: topModel.value?.label ?? '-',
      detail: topModel.value
        ? `${formatInteger(topModel.value.totalTokens)} ${t('settings.usageStats.summaryTotalTokens')}`
        : '-'
    }
  ])

  const hasStats = computed(() => (
    usageStore.stats.timeline.length > 0
    || usageStore.stats.breakdown.length > 0
    || usageStore.modelStats.breakdown.length > 0
  ))

  function applyDatePreset(days: number) {
    const endDate = new Date()
    const startDate = new Date(endDate)

    if (days > 0) {
      startDate.setDate(endDate.getDate() - days)
    }

    usageStore.filters.startDate = startDate.toISOString().slice(0, 10)
    usageStore.filters.endDate = endDate.toISOString().slice(0, 10)
    void refreshStats()
  }

  function initCharts() {
    if (trendChartRef.value && !trendChart) {
      trendChart = echarts.init(trendChartRef.value)
    }
    if (providerChartRef.value && !providerChart) {
      providerChart = echarts.init(providerChartRef.value)
    }
    if (agentChartRef.value && !agentChart) {
      agentChart = echarts.init(agentChartRef.value)
    }
    if (modelChartRef.value && !modelChart) {
      modelChart = echarts.init(modelChartRef.value)
    }
  }

  function disposeCharts() {
    trendChart?.dispose()
    providerChart?.dispose()
    agentChart?.dispose()
    modelChart?.dispose()
    trendChart = null
    providerChart = null
    agentChart = null
    modelChart = null
  }

  function resizeCharts() {
    trendChart?.resize()
    providerChart?.resize()
    agentChart?.resize()
    modelChart?.resize()
  }

  function applyCharts() {
    applyTrendChart({
      chart: trendChart,
      timeline: usageStore.stats.timeline,
      t
    })
    applyProviderChart({
      chart: providerChart,
      rows: providerBreakdown.value,
      t
    })
    applyAgentChart({
      chart: agentChart,
      rows: topAgentRows.value,
      chartWidth: agentChartRef.value?.clientWidth ?? 0,
      t
    })
    applyModelChart({
      chart: modelChart,
      rows: topModelRows.value,
      chartWidth: modelChartRef.value?.clientWidth ?? 0,
      t
    })
  }

  async function refreshStats() {
    await usageStore.loadStats()
    await nextTick()
    initCharts()
    applyCharts()
    resizeCharts()
  }

  function resetFilters() {
    usageStore.resetFilters()
    void refreshStats()
  }

  watch(
    () => [usageStore.stats, usageStore.modelStats],
    () => {
      applyCharts()
    },
    { deep: true }
  )

  onMounted(async () => {
    await nextTick()
    initCharts()

    if (typeof ResizeObserver !== 'undefined') {
      resizeObserver = new ResizeObserver(() => resizeCharts())
      if (trendChartRef.value) resizeObserver.observe(trendChartRef.value)
      if (providerChartRef.value) resizeObserver.observe(providerChartRef.value)
      if (agentChartRef.value) resizeObserver.observe(agentChartRef.value)
      if (modelChartRef.value) resizeObserver.observe(modelChartRef.value)
    } else {
      window.addEventListener('resize', resizeCharts)
    }

    await refreshStats()
  })

  onBeforeUnmount(() => {
    if (resizeObserver) {
      resizeObserver.disconnect()
      resizeObserver = null
    } else {
      window.removeEventListener('resize', resizeCharts)
    }

    disposeCharts()
  })

  return {
    t,
    usageStore,
    trendChartRef,
    providerChartRef,
    agentChartRef,
    modelChartRef,
    cliTypeOptions,
    dateRangePresets,
    summaryCards,
    insightCards,
    hasStats,
    applyDatePreset,
    refreshStats,
    resetFilters,
    formatDateTime
  }
}
