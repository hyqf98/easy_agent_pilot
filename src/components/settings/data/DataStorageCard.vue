<script setup lang="ts">
import { computed } from 'vue'
import { EaButton, EaIcon, EaStateBlock } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import { useI18n } from 'vue-i18n'
import type { DataManagementStats } from './types'

interface Props {
  loading: boolean
  error: string | null
  stats: DataManagementStats | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  refresh: []
}>()

const { t } = useI18n()

function formatSize(bytes: number): string {
  const mb = bytes / (1024 * 1024)
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(2)} GB`
  }
  return `${mb.toFixed(2)} MB`
}

function formatReadableSize(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(2)} KB`
  }
  return `${bytes} B`
}

function formatCount(value: number): string {
  return new Intl.NumberFormat().format(value)
}

const sizeItems = computed(() => {
  if (!props.stats) {
    return []
  }

  return [
    {
      key: 'session',
      icon: 'folder-open',
      label: t('settings.data.sessionDataSize'),
      value: formatReadableSize(props.stats.session_data_size_bytes),
      rawValue: props.stats.session_data_size_bytes,
      accent: 'var(--color-primary)'
    },
    {
      key: 'message',
      icon: 'message-square',
      label: t('settings.data.messageDataSize'),
      value: formatReadableSize(props.stats.message_data_size_bytes),
      rawValue: props.stats.message_data_size_bytes,
      accent: 'var(--color-info)'
    },
    {
      key: 'log',
      icon: 'scroll-text',
      label: t('settings.data.logDataSize'),
      value: formatReadableSize(props.stats.log_data_size_bytes),
      rawValue: props.stats.log_data_size_bytes,
      accent: 'var(--color-warning)'
    },
    {
      key: 'config',
      icon: 'settings-2',
      label: t('settings.data.configDataSize'),
      value: formatReadableSize(props.stats.config_data_size_bytes),
      rawValue: props.stats.config_data_size_bytes,
      accent: 'var(--color-success)'
    }
  ]
})

const trackedSizeBytes = computed(() => sizeItems.value.reduce((sum, item) => sum + item.rawValue, 0))

const sizeScaleBase = computed(() => {
  const maxValue = Math.max(...sizeItems.value.map((item) => item.rawValue), 0)
  return maxValue > 0 ? maxValue : 1
})

const sizeBreakdownItems = computed(() =>
  sizeItems.value.map((item) => ({
    ...item,
    percent: trackedSizeBytes.value > 0 ? Math.round((item.rawValue / trackedSizeBytes.value) * 100) : 0,
    barWidth: `${Math.max(item.rawValue > 0 ? 14 : 0, Math.round((item.rawValue / sizeScaleBase.value) * 100))}%`
  }))
)

const countItems = computed(() => {
  if (!props.stats) {
    return []
  }

  return [
    {
      key: 'projects',
      icon: 'folder',
      label: t('settings.data.projectCount'),
      value: formatCount(props.stats.project_count),
      accent: 'var(--color-primary)'
    },
    {
      key: 'sessions',
      icon: 'database',
      label: t('settings.data.sessionCount'),
      value: formatCount(props.stats.session_count),
      accent: 'var(--color-info)'
    },
    {
      key: 'messages',
      icon: 'messages-square',
      label: t('settings.data.messageCount'),
      value: formatCount(props.stats.message_count),
      accent: 'var(--color-success)'
    },
    {
      key: 'logs',
      icon: 'activity',
      label: t('settings.data.logCount'),
      value: formatCount(props.stats.log_count),
      accent: 'var(--color-warning)'
    }
  ]
})
</script>

<template>
  <SettingsSectionCard :title="t('settings.data.dataPath')">
    <template #actions>
      <EaButton
        type="ghost"
        size="small"
        :disabled="loading"
        @click="emit('refresh')"
      >
        <EaIcon
          name="refresh-cw"
          :size="14"
        />
        {{ t('settings.data.refreshStats') }}
      </EaButton>
    </template>

    <div
      v-if="loading"
      class="settings-state"
    >
      <EaStateBlock
        variant="loading"
        :description="t('settings.data.statsLoading')"
      />
    </div>

    <div
      v-else-if="error"
      class="settings-state"
    >
      <EaStateBlock
        variant="error"
        :title="t('settings.data.statsLoadFailed')"
        :description="error"
      />
    </div>

    <div
      v-else-if="stats"
      class="storage-panel"
    >
      <div class="storage-overview">
        <div class="storage-overview__content">
          <span class="storage-overview__eyebrow">{{ t('settings.data.storageOverviewTag') }}</span>
          <div class="storage-overview__headline">
            {{ formatSize(stats.total_size_bytes) }}
          </div>
          <div class="storage-overview__caption">
            {{ t('settings.data.databaseSize') }}
          </div>
        </div>
        <div class="storage-overview__meta">
          <div class="storage-overview__meta-item">
            <span class="storage-overview__meta-label">{{ t('settings.data.dataLocation') }}</span>
            <span class="storage-overview__meta-value">{{ stats.storage_path || '~/.easy-agent' }}</span>
          </div>
          <div class="storage-overview__meta-item">
            <span class="storage-overview__meta-label">{{ t('settings.data.databaseFile') }}</span>
            <span
              class="storage-overview__meta-value storage-overview__meta-value--break"
              :title="stats.database_path"
            >
              {{ stats.database_path }}
            </span>
          </div>
        </div>
      </div>

      <div class="storage-sections">
        <section class="storage-section">
          <div class="storage-section__header">
            <div>
              <div class="storage-section__title">
                {{ t('settings.data.sizeBreakdownTitle') }}
              </div>
              <div class="storage-section__subtitle">
                {{ t('settings.data.sizeBreakdownSubtitle', { size: formatReadableSize(trackedSizeBytes) }) }}
              </div>
            </div>
            <EaIcon
              name="pie-chart"
              :size="18"
              class="storage-section__icon"
            />
          </div>
          <div class="storage-breakdown">
            <div
              v-for="item in sizeBreakdownItems"
              :key="item.key"
              class="storage-breakdown__item"
              :style="{ '--storage-accent': item.accent }"
            >
              <div class="storage-breakdown__main">
                <div class="storage-breakdown__label-group">
                  <span class="storage-breakdown__icon-wrap">
                    <EaIcon
                      :name="item.icon"
                      :size="15"
                    />
                  </span>
                  <span class="storage-breakdown__label">{{ item.label }}</span>
                </div>
                <div class="storage-breakdown__value-group">
                  <span class="storage-breakdown__value">{{ item.value }}</span>
                  <span class="storage-breakdown__percent">{{ item.percent }}%</span>
                </div>
              </div>
              <div class="storage-breakdown__track">
                <span
                  class="storage-breakdown__bar"
                  :style="{ width: item.barWidth }"
                />
              </div>
            </div>
          </div>
        </section>

        <section class="storage-section">
          <div class="storage-section__header">
            <div>
              <div class="storage-section__title">
                {{ t('settings.data.recordOverviewTitle') }}
              </div>
              <div class="storage-section__subtitle">
                {{ t('settings.data.recordOverviewSubtitle') }}
              </div>
            </div>
            <EaIcon
              name="bar-chart-3"
              :size="18"
              class="storage-section__icon"
            />
          </div>
          <div class="storage-counts">
            <div
              v-for="item in countItems"
              :key="item.key"
              class="storage-counts__item"
              :style="{ '--storage-accent': item.accent }"
            >
              <span class="storage-counts__icon-wrap">
                <EaIcon
                  :name="item.icon"
                  :size="16"
                />
              </span>
              <span class="storage-counts__label">{{ item.label }}</span>
              <span class="storage-counts__value">{{ item.value }}</span>
            </div>
          </div>
        </section>
      </div>

      <div class="storage-summary-grid">
        <div
          class="storage-summary"
        >
          <span class="storage-summary__label">{{ t('settings.data.databaseSize') }}</span>
          <span class="storage-summary__value">{{ formatSize(stats.total_size_bytes) }}</span>
        </div>
        <div
          class="storage-summary"
        >
          <span class="storage-summary__label">{{ t('settings.data.sessionDataSize') }}</span>
          <span class="storage-summary__value">{{ formatReadableSize(stats.session_data_size_bytes) }}</span>
        </div>
        <div
          class="storage-summary"
        >
          <span class="storage-summary__label">{{ t('settings.data.messageDataSize') }}</span>
          <span class="storage-summary__value">{{ formatReadableSize(stats.message_data_size_bytes) }}</span>
        </div>
      </div>

      <div class="storage-note">
        {{ t('settings.data.statsHint') }}
      </div>
    </div>
  </SettingsSectionCard>
</template>

<style scoped>
.settings-state {
  display: flex;
}

.storage-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.storage-overview {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr);
  gap: var(--spacing-4);
  padding: var(--spacing-4);
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--color-primary) 16%, var(--color-border) 84%);
  border-radius: calc(var(--radius-lg) + 2px);
  background:
    radial-gradient(circle at top left, color-mix(in srgb, var(--color-primary-light) 70%, transparent) 0%, transparent 48%),
    linear-gradient(145deg, color-mix(in srgb, var(--color-primary-light) 40%, var(--color-bg-tertiary) 60%) 0%, var(--color-bg-secondary) 100%);
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 55%, transparent);
}

.storage-overview::after {
  content: '';
  position: absolute;
  inset: auto -48px -72px auto;
  width: 180px;
  height: 180px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 14%, transparent);
  filter: blur(6px);
}

.storage-overview__content,
.storage-overview__meta {
  position: relative;
  z-index: 1;
}

.storage-overview__content {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: var(--spacing-2);
  min-width: 0;
}

.storage-overview__eyebrow {
  display: inline-flex;
  width: fit-content;
  padding: 6px 10px;
  border: 1px solid color-mix(in srgb, var(--color-primary) 18%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, white 68%, var(--color-primary-light) 32%);
  color: color-mix(in srgb, var(--color-primary) 80%, #0f172a);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.storage-overview__headline {
  color: var(--color-text-primary);
  font-size: clamp(1.8rem, 3vw, 2.6rem);
  font-weight: 700;
  line-height: 1.05;
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.03em;
}

.storage-overview__caption {
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.storage-overview__meta {
  display: grid;
  gap: var(--spacing-3);
  min-width: 0;
}

.storage-overview__meta-item {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
  padding: var(--spacing-3);
  border: 1px solid color-mix(in srgb, var(--color-border) 76%, white 24%);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--color-bg-secondary) 80%, white 20%);
  backdrop-filter: blur(12px);
}

.storage-overview__meta-label {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.storage-overview__meta-value {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  line-height: 1.6;
  font-family: var(--font-family-mono);
}

.storage-overview__meta-value--break {
  word-break: break-all;
}

.storage-sections {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) minmax(0, 0.9fr);
  gap: var(--spacing-4);
}

.storage-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  border: 1px solid color-mix(in srgb, var(--color-border) 84%, white 16%);
  border-radius: calc(var(--radius-lg) - 2px);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--color-bg-tertiary) 90%, white 10%) 0%, var(--color-bg-secondary) 100%);
}

.storage-section__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
}

.storage-section__title {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.storage-section__subtitle {
  margin-top: 2px;
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
}

.storage-section__icon {
  color: var(--color-text-tertiary);
}

.storage-breakdown {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.storage-breakdown__item {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.storage-breakdown__main {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
}

.storage-breakdown__label-group,
.storage-breakdown__value-group {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
}

.storage-breakdown__label {
  min-width: 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.storage-breakdown__icon-wrap,
.storage-counts__icon-wrap {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 10px;
  color: var(--storage-accent);
  background: color-mix(in srgb, var(--storage-accent) 14%, white 86%);
}

.storage-breakdown__value {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  font-variant-numeric: tabular-nums;
}

.storage-breakdown__percent {
  min-width: 36px;
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.storage-breakdown__track {
  position: relative;
  height: 9px;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-border) 72%, transparent);
}

.storage-breakdown__bar {
  display: block;
  height: 100%;
  border-radius: inherit;
  background:
    linear-gradient(90deg, color-mix(in srgb, var(--storage-accent) 82%, white 18%) 0%, var(--storage-accent) 100%);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--storage-accent) 12%, transparent);
}

.storage-counts {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--spacing-3);
}

.storage-counts__item {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--spacing-2) var(--spacing-3);
  align-items: center;
  padding: var(--spacing-3);
  border: 1px solid color-mix(in srgb, var(--storage-accent) 18%, var(--color-border) 82%);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--storage-accent) 5%, var(--color-bg-secondary) 95%);
}

.storage-counts__label {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.storage-counts__value {
  grid-column: 2;
  color: var(--color-text-primary);
  font-size: 1.15rem;
  font-weight: 700;
  line-height: 1;
  font-variant-numeric: tabular-nums;
}

.storage-summary-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--spacing-3);
}

.storage-summary {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: var(--spacing-3);
  border: 1px solid color-mix(in srgb, var(--color-border) 86%, white 14%);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--color-bg-tertiary) 88%, white 12%);
}

.storage-summary__label {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.storage-summary__value {
  color: var(--color-text-primary);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
  font-variant-numeric: tabular-nums;
}

.storage-note {
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
  line-height: 1.6;
}

@media (max-width: 1100px) {
  .storage-overview,
  .storage-sections {
    grid-template-columns: minmax(0, 1fr);
  }
}

@media (max-width: 640px) {
  .storage-overview,
  .storage-section {
    padding: var(--spacing-3);
  }

  .storage-breakdown__main {
    align-items: flex-start;
    flex-direction: column;
  }

  .storage-breakdown__value-group {
    width: 100%;
    justify-content: space-between;
  }

  .storage-counts,
  .storage-summary-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
