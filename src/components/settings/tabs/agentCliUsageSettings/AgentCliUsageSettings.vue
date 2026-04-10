<script setup lang="ts">
import { EaButton, EaSelect, EaStateBlock } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import { useAgentCliUsageSettings } from './useAgentCliUsageSettings'

const {
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
} = useAgentCliUsageSettings()
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
<style scoped src="./styles.css"></style>
