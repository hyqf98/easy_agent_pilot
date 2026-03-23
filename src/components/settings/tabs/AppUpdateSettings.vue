<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton, EaProgressBar } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import { useAppUpdateStore } from '@/stores/appUpdate'
import { useSettingsStore } from '@/stores/settings'

const { t, locale } = useI18n()
const appUpdateStore = useAppUpdateStore()
const settingsStore = useSettingsStore()

const statusLabel = computed(() => {
  switch (appUpdateStore.status) {
    case 'checking':
      return t('settings.appUpdate.statusChecking')
    case 'up-to-date':
      return t('settings.appUpdate.statusUpToDate')
    case 'available':
      return t('settings.appUpdate.statusAvailable', {
        version: appUpdateStore.availableUpdate?.version ?? '-'
      })
    case 'downloading':
      return t('settings.appUpdate.statusDownloading')
    case 'installing':
      return t('settings.appUpdate.statusInstalling')
    case 'completed':
      return t('settings.appUpdate.statusCompleted')
    case 'error':
      return t('settings.appUpdate.statusError')
    case 'unsupported':
      return t('settings.appUpdate.statusUnsupported')
    default:
      return t('settings.appUpdate.statusIdle')
  }
})

const statusDescription = computed(() => {
  if (appUpdateStore.status === 'error') {
    return appUpdateStore.errorMessage || t('settings.appUpdate.checkFailed')
  }

  if (appUpdateStore.availableUpdate) {
    return t('settings.appUpdate.availableDescription', {
      current: appUpdateStore.currentVersion,
      latest: appUpdateStore.availableUpdate.version
    })
  }

  if (appUpdateStore.status === 'up-to-date') {
    return t('settings.appUpdate.upToDateDescription', {
      version: appUpdateStore.currentVersion
    })
  }

  return t('settings.appUpdate.description')
})

const formattedLastCheckedAt = computed(() => formatDateTime(appUpdateStore.lastCheckedAt))
const formattedPublishedAt = computed(() => formatDateTime(appUpdateStore.availableUpdate?.publishedAt ?? null))
const progressValue = computed(() => appUpdateStore.progress?.percent ?? -1)

function formatDateTime(value: string | null | undefined): string {
  if (!value) {
    return t('settings.appUpdate.neverChecked')
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return new Intl.DateTimeFormat(locale.value, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  }).format(date)
}

onMounted(async () => {
  await appUpdateStore.initialize()
})
</script>

<template>
  <div class="settings-page">
    <h3 class="settings-page__title">
      {{ t('settings.appUpdate.title') }}
    </h3>

    <SettingsSectionCard
      :title="t('settings.appUpdate.summaryTitle')"
      :description="t('settings.appUpdate.description')"
    >
      <div class="update-summary">
        <div class="update-summary__meta">
          <div class="summary-pill">
            <span class="summary-pill__label">{{ t('settings.appUpdate.currentVersion') }}</span>
            <strong class="summary-pill__value">v{{ appUpdateStore.currentVersion }}</strong>
          </div>
          <div class="summary-pill">
            <span class="summary-pill__label">{{ t('settings.appUpdate.latestStatus') }}</span>
            <strong class="summary-pill__value">{{ statusLabel }}</strong>
          </div>
          <div class="summary-pill">
            <span class="summary-pill__label">{{ t('settings.appUpdate.lastCheckedAt') }}</span>
            <strong class="summary-pill__value">{{ formattedLastCheckedAt }}</strong>
          </div>
        </div>

        <div class="update-summary__actions">
          <EaButton
            type="secondary"
            :loading="appUpdateStore.status === 'checking'"
            @click="appUpdateStore.checkForUpdates()"
          >
            {{ t('settings.appUpdate.checkNow') }}
          </EaButton>
          <EaButton
            v-if="appUpdateStore.availableUpdate"
            :loading="appUpdateStore.status === 'downloading' || appUpdateStore.status === 'installing'"
            @click="appUpdateStore.installUpdate()"
          >
            {{ t('settings.appUpdate.installNow') }}
          </EaButton>
        </div>
      </div>

      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.appUpdate.autoCheck') }}</span>
          <span class="settings-item__desc">{{ t('settings.appUpdate.autoCheckDesc') }}</span>
        </div>
        <label class="settings-toggle">
          <input
            v-model="settingsStore.settings.autoCheckAppUpdate"
            type="checkbox"
          >
          <span class="settings-toggle__slider" />
        </label>
      </div>
    </SettingsSectionCard>

    <SettingsSectionCard :title="t('settings.appUpdate.statusTitle')">
      <div class="status-card">
        <div class="status-card__header">
          <span class="status-card__title">{{ statusLabel }}</span>
          <span class="status-card__subtitle">{{ statusDescription }}</span>
        </div>

        <EaProgressBar
          v-if="appUpdateStore.status === 'downloading' || appUpdateStore.status === 'installing'"
          :value="progressValue"
          :show-text="progressValue >= 0"
          size="large"
          striped
          animated
        />

        <div
          v-if="appUpdateStore.progress"
          class="status-grid"
        >
          <div class="status-grid__item">
            <span class="status-grid__label">{{ t('settings.appUpdate.downloadedBytes') }}</span>
            <strong class="status-grid__value">{{ appUpdateStore.progress.downloadedBytes }}</strong>
          </div>
          <div class="status-grid__item">
            <span class="status-grid__label">{{ t('settings.appUpdate.totalBytes') }}</span>
            <strong class="status-grid__value">{{ appUpdateStore.progress.contentLength ?? '-' }}</strong>
          </div>
        </div>
      </div>
    </SettingsSectionCard>

    <SettingsSectionCard
      v-if="appUpdateStore.availableUpdate"
      :title="t('settings.appUpdate.releaseNotesTitle')"
    >
      <div class="status-grid">
        <div class="status-grid__item">
          <span class="status-grid__label">{{ t('settings.appUpdate.newVersion') }}</span>
          <strong class="status-grid__value">v{{ appUpdateStore.availableUpdate.version }}</strong>
        </div>
        <div class="status-grid__item">
          <span class="status-grid__label">{{ t('settings.appUpdate.publishedAt') }}</span>
          <strong class="status-grid__value">{{ formattedPublishedAt }}</strong>
        </div>
      </div>

      <pre class="release-notes">{{ appUpdateStore.availableUpdate.notes || t('settings.appUpdate.noReleaseNotes') }}</pre>
    </SettingsSectionCard>
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-6);
}

.settings-page__title {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.update-summary {
  display: flex;
  justify-content: space-between;
  gap: var(--spacing-4);
  align-items: flex-start;
}

.update-summary__meta {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: var(--spacing-3);
  flex: 1;
}

.update-summary__actions {
  display: flex;
  gap: var(--spacing-2);
  flex-wrap: wrap;
  justify-content: flex-end;
}

.summary-pill {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
  padding: var(--spacing-3);
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--color-surface) 84%, white 16%);
  border: 1px solid color-mix(in srgb, var(--color-border) 88%, white 12%);
}

.summary-pill__label,
.status-grid__label {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.summary-pill__value,
.status-grid__value {
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.settings-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-4);
}

.settings-item__info {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.settings-item__label {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.settings-item__desc {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.settings-toggle {
  position: relative;
  width: 44px;
  height: 24px;
  cursor: pointer;
}

.settings-toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.settings-toggle__slider {
  position: absolute;
  inset: 0;
  background-color: var(--color-border);
  border-radius: var(--radius-full);
  transition: background-color var(--transition-fast) var(--easing-default);
}

.settings-toggle__slider::before {
  content: '';
  position: absolute;
  left: 2px;
  top: 2px;
  width: 20px;
  height: 20px;
  background-color: var(--color-surface);
  border-radius: 50%;
  transition: transform var(--transition-fast) var(--easing-default);
}

.settings-toggle input:checked + .settings-toggle__slider {
  background-color: var(--color-primary);
}

.settings-toggle input:checked + .settings-toggle__slider::before {
  transform: translateX(20px);
}

.status-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.status-card__header {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.status-card__title {
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.status-card__subtitle {
  font-size: var(--font-size-sm);
  line-height: 1.6;
  color: var(--color-text-secondary);
}

.status-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: var(--spacing-3);
}

.status-grid__item {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
  padding: var(--spacing-3);
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--color-bg-secondary) 88%, white 12%);
  border: 1px solid color-mix(in srgb, var(--color-border) 88%, white 12%);
}

.release-notes {
  margin: 0;
  padding: var(--spacing-4);
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--color-surface) 82%, white 18%);
  border: 1px solid color-mix(in srgb, var(--color-border) 88%, white 12%);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-family-mono, 'SFMono-Regular', Consolas, monospace);
}

@media (max-width: 900px) {
  .update-summary {
    flex-direction: column;
  }

  .update-summary__actions {
    width: 100%;
    justify-content: flex-start;
  }
}
</style>
