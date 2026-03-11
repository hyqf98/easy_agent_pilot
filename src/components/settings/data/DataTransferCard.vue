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
  <SettingsSectionCard :title="t('settings.data.exportImport')">
    <div class="data-actions">
      <div class="data-action">
        <EaIcon
          name="download"
          :size="24"
          class="data-action__icon"
        />
        <div class="data-action__info">
          <span class="data-action__title">{{ t('settings.data.exportData') }}</span>
          <span class="data-action__desc">{{ t('settings.data.exportDataDesc') }}</span>
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

      <div class="export-options">
        <div class="export-options__header">
          <span class="export-options__title">{{ t('settings.data.exportOptions') }}</span>
          <div class="export-options__actions">
            <button
              class="export-options__action"
              :disabled="allSelected"
              @click="emit('toggleAll', true)"
            >
              {{ t('settings.data.selectAll') }}
            </button>
            <span class="export-options__divider">|</span>
            <button
              class="export-options__action"
              :disabled="!hasAnySelected"
              @click="emit('toggleAll', false)"
            >
              {{ t('settings.data.deselectAll') }}
            </button>
          </div>
        </div>

        <div class="export-options__grid">
          <label
            v-for="item in exportOptionItems"
            :key="item.key"
            class="export-options__item"
          >
            <input
              :checked="item.checked"
              type="checkbox"
              class="export-options__checkbox"
              @change="emit('updateOption', item.key, ($event.target as HTMLInputElement).checked)"
            >
            <span>{{ item.label }}</span>
          </label>
        </div>
      </div>

      <div
        v-if="exportMessage"
        class="status-message"
        :class="{ 'status-message--success': exportSuccess, 'status-message--error': !exportSuccess }"
      >
        <EaIcon
          :name="exportSuccess ? 'check' : 'x'"
          :size="16"
        />
        <span>{{ exportMessage }}</span>
      </div>

      <div class="data-action">
        <EaIcon
          name="upload"
          :size="24"
          class="data-action__icon"
        />
        <div class="data-action__info">
          <span class="data-action__title">{{ t('settings.data.importData') }}</span>
          <span class="data-action__desc">{{ t('settings.data.importDataDesc') }}</span>
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

      <div
        v-if="importMessage"
        class="status-message"
        :class="{ 'status-message--success': importSuccess, 'status-message--error': !importSuccess }"
      >
        <EaIcon
          :name="importSuccess ? 'check' : 'x'"
          :size="16"
        />
        <span>{{ importMessage }}</span>
      </div>

      <div
        v-if="importSuccess && importStatItems.length > 0"
        class="import-stats"
      >
        <div class="import-stats__title">
          {{ t('settings.data.importStats') }}
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
    </div>
  </SettingsSectionCard>
</template>

<style scoped>
.data-actions {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
}

.data-action {
  display: flex;
  align-items: center;
  gap: var(--spacing-4);
  padding: var(--spacing-3);
  border-radius: var(--radius-md);
  background-color: var(--color-bg-tertiary);
}

.data-action__icon {
  color: var(--color-primary);
}

.data-action__info {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--spacing-1);
}

.data-action__title {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.data-action__desc {
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.status-message {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
}

.status-message--success {
  background-color: var(--color-success-light);
  color: var(--color-success);
}

.status-message--error {
  background-color: var(--color-error-light);
  color: var(--color-error);
}

.import-stats {
  padding: var(--spacing-3);
  border: 1px solid var(--color-success-light);
  border-radius: var(--radius-md);
  background-color: var(--color-bg-tertiary);
}

.import-stats__title {
  margin-bottom: var(--spacing-2);
  color: var(--color-success);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.import-stats__grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--spacing-2);
}

.import-stats__item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-1) var(--spacing-2);
  border-radius: var(--radius-sm);
  background-color: var(--color-bg-secondary);
}

.import-stats__label {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.import-stats__value {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.export-options {
  padding: var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-bg-tertiary);
}

.export-options__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--spacing-3);
}

.export-options__title {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.export-options__actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.export-options__action {
  padding: 0;
  border: none;
  background: none;
  color: var(--color-primary);
  cursor: pointer;
  font-size: var(--font-size-xs);
  transition: opacity var(--transition-fast) var(--easing-default);
}

.export-options__action:hover:not(:disabled) {
  opacity: 0.8;
}

.export-options__action:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.export-options__divider {
  color: var(--color-border);
  font-size: var(--font-size-xs);
}

.export-options__grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--spacing-2);
}

.export-options__item {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  border-radius: var(--radius-sm);
  background-color: var(--color-bg-secondary);
  cursor: pointer;
  transition: background-color var(--transition-fast) var(--easing-default);
}

.export-options__item:hover {
  background-color: var(--color-surface-hover);
}

.export-options__checkbox {
  width: 16px;
  height: 16px;
  cursor: pointer;
  accent-color: var(--color-primary);
}

.export-options__item span {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}
</style>
