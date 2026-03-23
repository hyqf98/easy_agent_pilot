<script setup lang="ts">
import { EaButton, EaIcon } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'
import { useI18n } from 'vue-i18n'
import type { ExportOptionItem, ExportOptionKey, ImportStatItem } from './types'

interface Props {
  exportOptionItems: ExportOptionItem[]
  isExporting: boolean
  exportMessage: string
  exportSuccess: boolean
  isImporting: boolean
  importMessage: string
  importSuccess: boolean
  importStatItems: ImportStatItem[]
  allSelected: boolean
  hasAnySelected: boolean
}

defineProps<Props>()

const emit = defineEmits<{
  export: []
  import: []
  toggleAll: [select: boolean]
  updateOption: [key: ExportOptionKey, checked: boolean]
}>()

const { t } = useI18n()
</script>

<template>
  <SettingsSectionCard
    :title="t('settings.data.exportImport')"
    :description="t('settings.data.transferCardHint')"
  >
    <div class="transfer-layout">
      <section class="transfer-panel transfer-panel--export">
        <div class="transfer-panel__hero">
          <span class="transfer-panel__badge">{{ t('settings.data.exportPanelTag') }}</span>
          <div class="transfer-panel__hero-main">
            <span class="transfer-panel__icon-wrap">
              <EaIcon
                name="download"
                :size="18"
              />
            </span>
            <div class="transfer-panel__hero-copy">
              <div class="transfer-panel__title">
                {{ t('settings.data.exportData') }}
              </div>
              <div class="transfer-panel__desc">
                {{ t('settings.data.exportDataDesc') }}
              </div>
            </div>
          </div>
          <EaButton
            type="secondary"
            size="small"
            :disabled="isExporting"
            @click="emit('export')"
          >
            {{ isExporting ? t('settings.data.exporting') : t('settings.data.export') }}
          </EaButton>
        </div>

        <div class="option-panel">
          <div class="option-panel__header">
            <div>
              <div class="option-panel__title">
                {{ t('settings.data.exportOptions') }}
              </div>
              <div class="option-panel__subtitle">
                {{ t('settings.data.exportOptionsHint') }}
              </div>
            </div>
            <div class="option-panel__actions">
              <button
                class="option-panel__action"
                :disabled="allSelected"
                @click="emit('toggleAll', true)"
              >
                {{ t('settings.data.selectAll') }}
              </button>
              <button
                class="option-panel__action"
                :disabled="!hasAnySelected"
                @click="emit('toggleAll', false)"
              >
                {{ t('settings.data.deselectAll') }}
              </button>
            </div>
          </div>

          <div class="option-panel__grid">
            <label
              v-for="item in exportOptionItems"
              :key="item.key"
              class="option-chip"
            >
              <input
                :checked="item.checked"
                type="checkbox"
                class="option-chip__checkbox"
                @change="emit('updateOption', item.key, ($event.target as HTMLInputElement).checked)"
              >
              <span class="option-chip__label">{{ item.label }}</span>
            </label>
          </div>
        </div>

        <div
          v-if="exportMessage"
          class="status-message"
          :class="{ 'status-message--success': exportSuccess, 'status-message--error': !exportSuccess }"
        >
          <EaIcon
            :name="exportSuccess ? 'check-circle-2' : 'circle-alert'"
            :size="16"
          />
          <span>{{ exportMessage }}</span>
        </div>
      </section>

      <section class="transfer-panel transfer-panel--import">
        <div class="transfer-panel__hero transfer-panel__hero--compact">
          <span class="transfer-panel__badge transfer-panel__badge--import">{{ t('settings.data.importPanelTag') }}</span>
          <div class="transfer-panel__hero-main">
            <span class="transfer-panel__icon-wrap transfer-panel__icon-wrap--import">
              <EaIcon
                name="upload"
                :size="18"
              />
            </span>
            <div class="transfer-panel__hero-copy">
              <div class="transfer-panel__title">
                {{ t('settings.data.importData') }}
              </div>
              <div class="transfer-panel__desc">
                {{ t('settings.data.importDataDesc') }}
              </div>
            </div>
          </div>
          <EaButton
            type="secondary"
            size="small"
            :disabled="isImporting"
            @click="emit('import')"
          >
            {{ isImporting ? t('settings.data.importing') : t('settings.data.import') }}
          </EaButton>
        </div>

        <div class="import-panel__hint">
          <EaIcon
            name="shield-check"
            :size="15"
          />
          <span>{{ t('settings.data.importPanelHint') }}</span>
        </div>

        <div
          v-if="importMessage"
          class="status-message"
          :class="{ 'status-message--success': importSuccess, 'status-message--error': !importSuccess }"
        >
          <EaIcon
            :name="importSuccess ? 'check-circle-2' : 'circle-alert'"
            :size="16"
          />
          <span>{{ importMessage }}</span>
        </div>

        <div
          v-if="importSuccess && importStatItems.length > 0"
          class="import-stats"
        >
          <div class="import-stats__header">
            <div>
              <div class="import-stats__title">
                {{ t('settings.data.importStats') }}
              </div>
              <div class="import-stats__subtitle">
                {{ t('settings.data.importStatsHint') }}
              </div>
            </div>
            <EaIcon
              name="sparkles"
              :size="16"
              class="import-stats__icon"
            />
          </div>
          <div class="import-stats__grid">
            <div
              v-for="item in importStatItems"
              :key="item.key"
              class="import-stats__item"
            >
              <span class="import-stats__label">{{ item.label }}</span>
              <span class="import-stats__value">{{ item.value }}</span>
            </div>
          </div>
        </div>
      </section>
    </div>
  </SettingsSectionCard>
</template>

<style scoped>
.transfer-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.45fr) minmax(320px, 0.95fr);
  gap: var(--spacing-4);
}

.transfer-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  border: 1px solid color-mix(in srgb, var(--color-border) 84%, white 16%);
  border-radius: calc(var(--radius-lg) - 2px);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--color-bg-tertiary) 88%, white 12%) 0%, var(--color-bg-secondary) 100%);
}

.transfer-panel--export {
  background:
    radial-gradient(circle at top left, color-mix(in srgb, var(--color-primary-light) 36%, transparent) 0%, transparent 44%),
    linear-gradient(180deg, color-mix(in srgb, var(--color-bg-tertiary) 88%, white 12%) 0%, var(--color-bg-secondary) 100%);
}

.transfer-panel--import {
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--color-success-light) 40%, transparent) 0%, transparent 42%),
    linear-gradient(180deg, color-mix(in srgb, var(--color-bg-tertiary) 88%, white 12%) 0%, var(--color-bg-secondary) 100%);
}

.transfer-panel__hero {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-3);
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, white 18%);
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--color-bg-secondary) 86%, white 14%);
}

.transfer-panel__hero--compact {
  min-height: 0;
}

.transfer-panel__badge {
  display: inline-flex;
  width: fit-content;
  padding: 5px 10px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--color-primary) 16%, transparent);
  background: color-mix(in srgb, white 72%, var(--color-primary-light) 28%);
  color: color-mix(in srgb, var(--color-primary) 82%, #0f172a);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.transfer-panel__badge--import {
  border-color: color-mix(in srgb, var(--color-success) 16%, transparent);
  background: color-mix(in srgb, white 74%, var(--color-success-light) 26%);
  color: color-mix(in srgb, var(--color-success) 78%, #0f172a);
}

.transfer-panel__hero-main {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-3);
}

.transfer-panel__icon-wrap {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 38px;
  height: 38px;
  border-radius: 12px;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 12%, white 88%);
}

.transfer-panel__icon-wrap--import {
  color: var(--color-success);
  background: color-mix(in srgb, var(--color-success) 12%, white 88%);
}

.transfer-panel__hero-copy {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: 6px;
}

.transfer-panel__title {
  color: var(--color-text-primary);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
}

.transfer-panel__desc {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  line-height: 1.6;
}

.option-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-3);
  border: 1px solid color-mix(in srgb, var(--color-border) 84%, white 16%);
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--color-bg-tertiary) 90%, white 10%);
}

.option-panel__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-3);
}

.option-panel__title {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.option-panel__subtitle {
  margin-top: 4px;
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.option-panel__actions {
  display: inline-flex;
  gap: var(--spacing-2);
}

.option-panel__action {
  padding: 6px 10px;
  border: 1px solid color-mix(in srgb, var(--color-border) 80%, white 20%);
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-bg-secondary) 80%, white 20%);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  cursor: pointer;
  transition:
    border-color var(--transition-fast) var(--easing-default),
    color var(--transition-fast) var(--easing-default),
    background-color var(--transition-fast) var(--easing-default);
}

.option-panel__action:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--color-primary) 18%, var(--color-border) 82%);
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary-light) 40%, white 60%);
}

.option-panel__action:disabled {
  opacity: 0.48;
  cursor: not-allowed;
}

.option-panel__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--spacing-2);
}

.option-chip {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: 12px 14px;
  border: 1px solid color-mix(in srgb, var(--color-border) 84%, white 16%);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--color-bg-secondary) 84%, white 16%);
  cursor: pointer;
  transition:
    transform var(--transition-fast) var(--easing-default),
    border-color var(--transition-fast) var(--easing-default),
    background-color var(--transition-fast) var(--easing-default);
}

.option-chip:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--color-primary) 18%, var(--color-border) 82%);
  background: color-mix(in srgb, var(--color-primary-light) 22%, white 78%);
}

.option-chip__checkbox {
  width: 16px;
  height: 16px;
  cursor: pointer;
  accent-color: var(--color-primary);
}

.option-chip__label {
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.import-panel__hint {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: 10px 12px;
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--color-success) 8%, var(--color-bg-secondary) 92%);
  color: color-mix(in srgb, var(--color-success) 80%, #0f172a);
  font-size: var(--font-size-xs);
  line-height: 1.5;
}

.status-message {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: 12px 14px;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  line-height: 1.5;
}

.status-message--success {
  border-color: color-mix(in srgb, var(--color-success) 18%, transparent);
  background: color-mix(in srgb, var(--color-success-light) 55%, white 45%);
  color: var(--color-success);
}

.status-message--error {
  border-color: color-mix(in srgb, var(--color-error) 18%, transparent);
  background: color-mix(in srgb, var(--color-error-light) 58%, white 42%);
  color: var(--color-error);
}

.import-stats {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-3);
  border: 1px solid color-mix(in srgb, var(--color-success) 16%, var(--color-border) 84%);
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--color-success-light) 26%, var(--color-bg-secondary) 74%);
}

.import-stats__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-3);
}

.import-stats__title {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.import-stats__subtitle {
  margin-top: 4px;
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.import-stats__icon {
  color: var(--color-success);
}

.import-stats__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--spacing-2);
}

.import-stats__item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  padding: 12px 14px;
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--color-bg-secondary) 82%, white 18%);
}

.import-stats__label {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.import-stats__value {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  font-variant-numeric: tabular-nums;
}

@media (max-width: 980px) {
  .transfer-layout {
    grid-template-columns: minmax(0, 1fr);
  }
}

@media (max-width: 720px) {
  .option-panel__header,
  .transfer-panel__hero-main {
    flex-direction: column;
  }

  .option-panel__actions,
  .import-stats__grid,
  .option-panel__grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .option-panel__actions {
    display: flex;
    width: 100%;
  }

  .option-panel__action {
    flex: 1;
    justify-content: center;
  }
}
</style>
