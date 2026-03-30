<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as echarts from 'echarts'
import { useI18n } from 'vue-i18n'
import { EaButton, EaSelect, EaStateBlock } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import { useAgentCliUsageStore } from '@/stores/agentCliUsage'
import type { SelectOption } from '@/components/common'

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
  { value: 'codex', label: 'Codex CLI' }
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

const providerBreakdown = computed(() => {
  const grouped = new Map<string, {
    provider: string
    label: string
    callCount: number
    totalTokens: number
    estimatedTotalCostUsd: number
  }>()

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
  const totalCalls = usageStore.stats.summary.totalCalls
  if (totalCalls <= 0) {
    return 0
  }

  return usageStore.stats.summary.totalTokens / totalCalls
})

const averageCostPerCall = computed(() => {
  const totalCalls = usageStore.stats.summary.totalCalls
  if (totalCalls <= 0) {
    return 0
  }

  return usageStore.stats.summary.estimatedTotalCostUsd / totalCalls
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

function formatInteger(value: number): string {
  return new Intl.NumberFormat().format(value)
}

function formatCurrency(value: number): string {
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: value < 1 ? 4 : 2,
    maximumFractionDigits: value < 1 ? 4 : 2
  }).format(value)
}

function formatPercentage(value: number): string {
  return `${(value * 100).toFixed(value >= 0.1 ? 0 : 1)}%`
}

function formatDateTime(value?: string): string {
  if (!value) {
    return '-'
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return date.toLocaleString()
}

function getProviderColor(provider: string): string {
  switch (provider) {
    case 'claude':
      return '#0f766e'
    case 'codex':
      return '#ea580c'
    default:
      return '#64748b'
  }
}

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

function applyTrendChart() {
  if (!trendChart) {
    return
  }

  trendChart.setOption({
    animation: false,
    color: ['#2563eb', '#0f766e', '#ea580c'],
    tooltip: { trigger: 'axis' },
    legend: {
      top: 0,
      data: [
        t('settings.usageStats.summaryInputTokens'),
        t('settings.usageStats.summaryOutputTokens'),
        t('settings.usageStats.summaryEstimatedCost')
      ]
    },
    grid: { left: 48, right: 96, top: 48, bottom: 28 },
    xAxis: {
      type: 'category',
      data: usageStore.stats.timeline.map(item => item.label)
    },
    yAxis: [
      {
        type: 'value',
        name: t('settings.usageStats.summaryTotalTokens')
      },
      {
        type: 'value',
        name: 'USD',
        position: 'right',
        axisLabel: {
          formatter: (value: number) => `$${value.toFixed(value < 1 ? 3 : 2)}`
        }
      }
    ],
    series: [
      {
        name: t('settings.usageStats.summaryInputTokens'),
        type: 'bar',
        stack: 'tokens',
        barMaxWidth: 24,
        data: usageStore.stats.timeline.map(item => item.inputTokens)
      },
      {
        name: t('settings.usageStats.summaryOutputTokens'),
        type: 'bar',
        stack: 'tokens',
        barMaxWidth: 24,
        data: usageStore.stats.timeline.map(item => item.outputTokens)
      },
      {
        name: t('settings.usageStats.summaryEstimatedCost'),
        type: 'line',
        smooth: true,
        yAxisIndex: 1,
        data: usageStore.stats.timeline.map(item => Number(item.estimatedTotalCostUsd.toFixed(6)))
      }
    ]
  })
}

function applyProviderChart() {
  if (!providerChart) {
    return
  }

  providerChart.setOption({
    animation: false,
    color: providerBreakdown.value.map(item => getProviderColor(item.provider)),
    tooltip: {
      trigger: 'item',
      formatter: (params: { data?: { label: string, callCount: number, totalTokens: number, estimatedTotalCostUsd: number } }) => {
        const item = params.data
        if (!item) {
          return ''
        }

        return [
          item.label,
          `${t('settings.usageStats.summaryCalls')}：${formatInteger(item.callCount)}`,
          `${t('settings.usageStats.summaryTotalTokens')}：${formatInteger(item.totalTokens)}`,
          `${t('settings.usageStats.summaryEstimatedCost')}：${formatCurrency(item.estimatedTotalCostUsd)}`
        ].join('<br />')
      }
    },
    legend: {
      bottom: 0,
      icon: 'roundRect',
      itemWidth: 10,
      itemHeight: 10
    },
    series: [
      {
        name: t('settings.usageStats.providerShareTitle'),
        type: 'pie',
        radius: ['52%', '76%'],
        center: ['50%', '44%'],
        itemStyle: {
          borderRadius: 12,
          borderColor: '#fff',
          borderWidth: 4
        },
        label: {
          show: true,
          formatter: ({ data }: { data?: { label: string, estimatedTotalCostUsd: number } }) => {
            if (!data) {
              return ''
            }
            return `${data.label}\n${formatCurrency(data.estimatedTotalCostUsd)}`
          }
        },
        data: providerBreakdown.value.map(item => ({
          name: item.label,
          value: item.estimatedTotalCostUsd > 0 ? item.estimatedTotalCostUsd : item.totalTokens,
          ...item
        }))
      }
    ]
  })
}

function applyAgentChart() {
  if (!agentChart) {
    return
  }

  const rows = [...topAgentRows.value].reverse()
  const chartWidth = agentChartRef.value?.clientWidth ?? 0
  const axisLabelWidth = chartWidth > 0
    ? Math.max(112, Math.min(168, Math.floor(chartWidth * 0.24)))
    : 168
  const compact = chartWidth > 0 && chartWidth < 760
  agentChart.setOption({
    animation: false,
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      formatter: (params: Array<{ dataIndex: number }>) => {
        const row = rows[params[0]?.dataIndex ?? 0]
        if (!row) {
          return ''
        }

        return [
          row.label,
          `${t('settings.usageStats.summaryCalls')}：${formatInteger(row.callCount)}`,
          `${t('settings.usageStats.summaryTotalTokens')}：${formatInteger(row.totalTokens)}`,
          `${t('settings.usageStats.summaryEstimatedCost')}：${formatCurrency(row.estimatedTotalCostUsd)}`
        ].join('<br />')
      }
    },
    grid: {
      left: 8,
      right: compact ? 16 : 24,
      top: 16,
      bottom: compact ? 58 : 68,
      containLabel: true
    },
    xAxis: {
      type: 'value',
      name: t('settings.usageStats.summaryTotalTokens'),
      nameLocation: 'middle',
      nameGap: compact ? 34 : 42,
      axisLabel: {
        hideOverlap: true,
        margin: 12
      },
      nameTextStyle: {
        padding: [12, 0, 0, 0]
      }
    },
    yAxis: {
      type: 'category',
      data: rows.map(item => item.label),
      axisLabel: {
        width: axisLabelWidth,
        overflow: 'truncate'
      }
    },
    series: [
      {
        type: 'bar',
        barMaxWidth: 18,
        data: rows.map(item => item.totalTokens),
        itemStyle: { color: '#2563eb' }
      }
    ]
  })
}

function applyModelChart() {
  if (!modelChart) {
    return
  }

  const rows = [...topModelRows.value].reverse()
  const chartWidth = modelChartRef.value?.clientWidth ?? 0
  const axisLabelWidth = chartWidth > 0
    ? Math.max(120, Math.min(184, Math.floor(chartWidth * 0.26)))
    : 184
  const compact = chartWidth > 0 && chartWidth < 760
  modelChart.setOption({
    animation: false,
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      formatter: (params: Array<{ dataIndex: number }>) => {
        const row = rows[params[0]?.dataIndex ?? 0]
        if (!row) {
          return ''
        }

        return [
          row.label,
          `${t('settings.usageStats.summaryCalls')}：${formatInteger(row.callCount)}`,
          `${t('settings.usageStats.summaryInputTokens')}：${formatInteger(row.inputTokens)}`,
          `${t('settings.usageStats.summaryOutputTokens')}：${formatInteger(row.outputTokens)}`,
          `${t('settings.usageStats.summaryEstimatedCost')}：${formatCurrency(row.estimatedTotalCostUsd)}`
        ].join('<br />')
      }
    },
    grid: {
      left: 8,
      right: compact ? 16 : 24,
      top: 16,
      bottom: compact ? 60 : 72,
      containLabel: true
    },
    xAxis: {
      type: 'value',
      name: t('settings.usageStats.summaryEstimatedCost'),
      nameLocation: 'middle',
      nameGap: compact ? 36 : 44,
      axisLabel: {
        formatter: (value: number) => formatCurrency(value),
        hideOverlap: true,
        margin: 12
      },
      nameTextStyle: {
        padding: [12, 0, 0, 0]
      }
    },
    yAxis: {
      type: 'category',
      data: rows.map(item => item.label),
      axisLabel: {
        width: axisLabelWidth,
        overflow: 'truncate'
      }
    },
    series: [
      {
        type: 'bar',
        barMaxWidth: 18,
        data: rows.map(item => Number(item.estimatedTotalCostUsd.toFixed(6))),
        itemStyle: { color: '#ea580c' }
      }
    ]
  })
}

async function refreshStats() {
  await usageStore.loadStats()
  await nextTick()
  initCharts()
  applyTrendChart()
  applyProviderChart()
  applyAgentChart()
  applyModelChart()
  resizeCharts()
}

function resetFilters() {
  usageStore.resetFilters()
  void refreshStats()
}

watch(
  () => [usageStore.stats, usageStore.modelStats],
  () => {
    applyTrendChart()
    applyProviderChart()
    applyAgentChart()
    applyModelChart()
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
</script>

<template>
  <div class="usage-stats-page">
    <section class="usage-stats-page__hero">
      <div>
        <p class="usage-stats-page__eyebrow">
          {{ t('settings.usageStats.eyebrow') }}
        </p>
        <h2 class="usage-stats-page__title">
          {{ t('settings.usageStats.title') }}
        </h2>
        <p class="usage-stats-page__subtitle">
          {{ t('settings.usageStats.subtitle') }}
        </p>
      </div>

      <div class="usage-stats-page__actions">
        <EaButton
          type="secondary"
          @click="resetFilters"
        >
          {{ t('settings.usageStats.resetFilters') }}
        </EaButton>
        <EaButton
          :loading="usageStore.isLoading"
          @click="refreshStats"
        >
          {{ t('common.refresh') }}
        </EaButton>
      </div>
    </section>

    <SettingsSectionCard
      :title="t('settings.usageStats.filtersTitle')"
      :description="t('settings.usageStats.filtersDescriptionSimplified')"
    >
      <div class="usage-filters">
        <label class="usage-field">
          <span class="usage-field__label">{{ t('settings.usageStats.startDate') }}</span>
          <input
            v-model="usageStore.filters.startDate"
            class="usage-field__input"
            type="date"
          >
        </label>

        <label class="usage-field">
          <span class="usage-field__label">{{ t('settings.usageStats.endDate') }}</span>
          <input
            v-model="usageStore.filters.endDate"
            class="usage-field__input"
            type="date"
          >
        </label>

        <label class="usage-field">
          <span class="usage-field__label">{{ t('settings.usageStats.cliType') }}</span>
          <EaSelect
            v-model="usageStore.filters.cliType"
            :options="cliTypeOptions"
          />
        </label>

        <label class="usage-field">
          <span class="usage-field__label">{{ t('settings.usageStats.modelName') }}</span>
          <input
            v-model.trim="usageStore.filters.modelKeyword"
            class="usage-field__input"
            type="text"
            :placeholder="t('settings.usageStats.modelNamePlaceholder')"
          >
        </label>
      </div>

      <div class="usage-presets">
        <span class="usage-presets__label">{{ t('settings.usageStats.rangePresetsTitle') }}</span>
        <div class="usage-presets__actions">
          <button
            v-for="preset in dateRangePresets"
            :key="preset.key"
            class="usage-preset-chip"
            type="button"
            @click="applyDatePreset(preset.days)"
          >
            {{ preset.label }}
          </button>
        </div>
      </div>
    </SettingsSectionCard>

    <div
      v-if="usageStore.errorMessage"
      class="usage-stats-page__feedback"
    >
      <EaStateBlock
        variant="error"
        :title="t('settings.usageStats.loadFailed')"
        :description="usageStore.errorMessage"
      />
    </div>

    <div
      v-if="usageStore.stats.meta.costPartial"
      class="usage-stats-page__warning"
    >
      <strong>{{ t('settings.usageStats.partialCostTitle') }}</strong>
      <span>{{ t('settings.usageStats.partialCostDescription', {
        count: usageStore.stats.summary.unpricedCalls,
        version: usageStore.stats.meta.pricingVersion
      }) }}</span>
    </div>

    <section class="usage-summary-grid">
      <article
        v-for="card in summaryCards"
        :key="card.key"
        class="usage-summary-card"
      >
        <span class="usage-summary-card__label">{{ card.label }}</span>
        <strong class="usage-summary-card__value">{{ card.value }}</strong>
      </article>
    </section>

    <section
      v-if="hasStats"
      class="usage-insight-grid"
    >
      <article
        v-for="card in insightCards"
        :key="card.key"
        class="usage-insight-card"
      >
        <span class="usage-insight-card__label">{{ card.label }}</span>
        <strong class="usage-insight-card__value">{{ card.value }}</strong>
        <span class="usage-insight-card__detail">{{ card.detail }}</span>
      </article>
    </section>

    <template v-if="usageStore.isLoading && !usageStore.hasLoaded">
      <EaStateBlock
        variant="loading"
        :title="t('common.loading')"
        :description="t('settings.usageStats.loadingDescription')"
      />
    </template>

    <template v-else-if="hasStats">
      <div class="usage-chart-grid">
        <SettingsSectionCard
          :title="t('settings.usageStats.trendTitle')"
          :description="t('settings.usageStats.trendDescriptionSimplified')"
        >
          <div
            ref="trendChartRef"
            class="usage-chart"
          />
        </SettingsSectionCard>

        <SettingsSectionCard
          :title="t('settings.usageStats.providerShareTitle')"
          :description="t('settings.usageStats.providerShareDescription')"
        >
          <div
            ref="providerChartRef"
            class="usage-chart usage-chart--provider"
          />
        </SettingsSectionCard>
      </div>

      <div class="usage-chart-grid usage-chart-grid--ranking">
        <SettingsSectionCard
          :title="t('settings.usageStats.agentRankingTitle')"
          :description="t('settings.usageStats.agentRankingDescription')"
        >
          <div
            ref="agentChartRef"
            class="usage-chart usage-chart--compact"
          />
        </SettingsSectionCard>

        <SettingsSectionCard
          :title="t('settings.usageStats.modelRankingTitle')"
          :description="t('settings.usageStats.modelRankingDescription')"
        >
          <div
            ref="modelChartRef"
            class="usage-chart usage-chart--compact"
          />
        </SettingsSectionCard>
      </div>

      <footer class="usage-meta">
        <span>{{ t('settings.usageStats.metaPricingVersion', { version: usageStore.stats.meta.pricingVersion || '-' }) }}</span>
        <span>{{ t('settings.usageStats.metaRange', {
          start: formatDateTime(usageStore.stats.meta.startAt),
          end: formatDateTime(usageStore.stats.meta.endAt)
        }) }}</span>
      </footer>
    </template>

    <template v-else>
      <EaStateBlock
        variant="empty"
        :title="t('settings.usageStats.emptyTitle')"
        :description="t('settings.usageStats.emptyDescription')"
      />
    </template>
  </div>
</template>

<style scoped>
.usage-stats-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-5);
  min-width: 0;
}

.usage-stats-page__hero {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: var(--spacing-4);
  padding: var(--spacing-5);
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, white 18%);
  border-radius: calc(var(--radius-lg) + 4px);
  background:
    radial-gradient(circle at top left, rgba(37, 99, 235, 0.16), transparent 42%),
    radial-gradient(circle at bottom right, rgba(14, 116, 144, 0.16), transparent 44%),
    linear-gradient(160deg, color-mix(in srgb, var(--color-bg-secondary) 92%, white 8%), var(--color-bg-secondary));
}

.usage-stats-page__eyebrow {
  margin: 0 0 var(--spacing-2);
  font-size: var(--font-size-xs);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--color-primary);
}

.usage-stats-page__title {
  margin: 0;
  font-size: clamp(1.5rem, 2vw, 2rem);
  color: var(--color-text-primary);
}

.usage-stats-page__subtitle {
  margin: var(--spacing-2) 0 0;
  max-width: 760px;
  color: var(--color-text-secondary);
  line-height: 1.7;
}

.usage-stats-page__actions {
  display: flex;
  gap: var(--spacing-2);
  flex-wrap: wrap;
}

.usage-filters {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--spacing-4);
}

.usage-field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.usage-field__label {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.usage-field__input {
  width: 100%;
  min-height: 40px;
  padding: var(--spacing-2) var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  color: var(--color-text-primary);
}

.usage-presets {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  margin-top: var(--spacing-4);
  padding-top: var(--spacing-4);
  border-top: 1px dashed color-mix(in srgb, var(--color-border) 82%, white 18%);
}

.usage-presets__label {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.usage-presets__actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
}

.usage-preset-chip {
  min-height: 32px;
  padding: 0 var(--spacing-3);
  border: 1px solid color-mix(in srgb, var(--color-border) 85%, white 15%);
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-bg-secondary) 92%, white 8%);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: transform 0.16s ease, border-color 0.16s ease, color 0.16s ease;
}

.usage-preset-chip:hover {
  color: var(--color-primary);
  border-color: color-mix(in srgb, var(--color-primary) 40%, var(--color-border) 60%);
  transform: translateY(-1px);
}

.usage-stats-page__feedback,
.usage-stats-page__warning {
  padding: var(--spacing-4);
  border-radius: var(--radius-lg);
}

.usage-stats-page__warning {
  display: flex;
  gap: var(--spacing-3);
  align-items: center;
  border: 1px solid rgba(234, 88, 12, 0.2);
  background: rgba(255, 237, 213, 0.6);
  color: #9a3412;
}

[data-theme='dark'] .usage-stats-page__warning {
  background: rgba(124, 45, 18, 0.24);
  color: #fdba74;
}

.usage-summary-grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: var(--spacing-3);
}

.usage-summary-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-4);
  border-radius: var(--radius-lg);
  border: 1px solid color-mix(in srgb, var(--color-border) 84%, white 16%);
  background: var(--color-bg-secondary);
}

.usage-summary-card__label {
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.usage-summary-card__value {
  color: var(--color-text-primary);
  font-size: 1.25rem;
}

.usage-insight-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--spacing-3);
}

.usage-insight-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-4);
  border-radius: var(--radius-lg);
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, white 18%);
  background: linear-gradient(180deg, color-mix(in srgb, var(--color-bg-secondary) 96%, white 4%), var(--color-surface));
}

.usage-insight-card__label,
.usage-insight-card__detail {
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.usage-insight-card__value {
  color: var(--color-text-primary);
  font-size: 1.05rem;
}

.usage-chart-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--spacing-4);
}

.usage-chart-grid--ranking {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.usage-chart {
  width: 100%;
  min-height: 340px;
}

.usage-chart--compact {
  min-height: 450px;
}

.usage-chart--provider {
  min-height: 340px;
}

.usage-meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-4);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

@media (max-width: 1200px) {
  .usage-filters,
  .usage-summary-grid,
  .usage-insight-grid,
  .usage-chart-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 1480px) {
  .usage-chart-grid--ranking {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 768px) {
  .usage-stats-page__hero,
  .usage-summary-grid,
  .usage-insight-grid,
  .usage-filters,
  .usage-chart-grid {
    grid-template-columns: 1fr;
    display: grid;
  }

  .usage-stats-page__hero {
    align-items: flex-start;
  }

  .usage-stats-page__actions {
    width: 100%;
  }

  .usage-stats-page__warning,
  .usage-presets {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
