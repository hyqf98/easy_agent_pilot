<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSettingsStore } from '@/stores/settings'
import { EaSelect } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'

const { t } = useI18n()
const settingsStore = useSettingsStore()

// 语言选项
const languageOptions = computed(() => [
  { value: 'zh-CN', label: t('languages.zhCN') },
  { value: 'en-US', label: t('languages.enUS') }
])

// Tab 宽度选项
const tabSizeOptions = computed(() => [
  { value: 2, label: '2' },
  { value: 4, label: '4' }
])

// 压缩策略选项
const compressionStrategyOptions = computed(() => [
  { value: 'simple', label: t('settings.general.compressionStrategySimple') },
  { value: 'smart', label: t('settings.general.compressionStrategySmart') },
  { value: 'summary', label: t('settings.general.compressionStrategySummary') }
])

// 压缩阈值选项
const compressionThresholdOptions = computed(() => [
  { value: 50, label: '50%' },
  { value: 60, label: '60%' },
  { value: 70, label: '70%' },
  { value: 80, label: '80%' },
  { value: 90, label: '90%' }
])
</script>

<template>
  <div class="settings-page">
    <h3 class="settings-page__title">
      {{ t('settings.general.title') }}
    </h3>

    <SettingsSectionCard :title="t('settings.general.appSettings')">
      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.language') }}</span>
          <span class="settings-item__desc">{{ t('settings.general.languageDesc') }}</span>
        </div>
        <EaSelect
          v-model="settingsStore.settings.language"
          :options="languageOptions"
        />
      </div>

      <div class="settings-item settings-item--column">
        <div class="settings-item__header">
          <div class="settings-item__info">
            <span class="settings-item__label">{{ t('settings.general.fontSize') }}</span>
            <span class="settings-item__desc">{{ t('settings.general.fontSizeDesc') }}</span>
          </div>
          <div class="font-size-value">
            {{ settingsStore.settings.fontSize }}px
          </div>
        </div>
        <div class="font-size-slider">
          <span class="font-size-slider__label">12px</span>
          <input
            v-model.number="settingsStore.settings.fontSize"
            type="range"
            min="12"
            max="24"
            step="1"
            class="font-size-slider__input"
          >
          <span class="font-size-slider__label">24px</span>
        </div>
      </div>
    </SettingsSectionCard>

    <SettingsSectionCard :title="t('settings.general.behaviorSettings')">
      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.autoSave') }}</span>
          <span class="settings-item__desc">{{ t('settings.general.autoSaveDesc') }}</span>
        </div>
        <label class="settings-toggle">
          <input
            v-model="settingsStore.settings.autoSave"
            type="checkbox"
          >
          <span class="settings-toggle__slider" />
        </label>
      </div>

      <div
        v-if="!settingsStore.settings.autoSave"
        class="settings-warning"
      >
        <span class="settings-warning__icon">⚠️</span>
        <span class="settings-warning__text">{{ t('settings.general.autoSaveWarning') }}</span>
      </div>

      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.confirmBeforeDelete') }}</span>
          <span class="settings-item__desc">{{ t('settings.general.confirmBeforeDeleteDesc') }}</span>
        </div>
        <label class="settings-toggle">
          <input
            v-model="settingsStore.settings.confirmBeforeDelete"
            type="checkbox"
          >
          <span class="settings-toggle__slider" />
        </label>
      </div>

      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.sendOnEnter') }}</span>
          <span class="settings-item__desc">{{ t('settings.general.sendOnEnterDesc') }}</span>
        </div>
        <label class="settings-toggle">
          <input
            v-model="settingsStore.settings.sendOnEnter"
            type="checkbox"
          >
          <span class="settings-toggle__slider" />
        </label>
      </div>
    </SettingsSectionCard>

    <SettingsSectionCard :title="t('settings.general.compressionSettings')">
      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.autoCompression') }}</span>
          <span class="settings-item__desc">{{ t('settings.general.autoCompressionDesc') }}</span>
        </div>
        <label class="settings-toggle">
          <input
            v-model="settingsStore.settings.autoCompressionEnabled"
            type="checkbox"
          >
          <span class="settings-toggle__slider" />
        </label>
      </div>

      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.compressionStrategy') }}</span>
          <span class="settings-item__desc">{{ t(`settings.general.compressionStrategy${settingsStore.settings.compressionStrategy.charAt(0).toUpperCase() + settingsStore.settings.compressionStrategy.slice(1)}Desc`) }}</span>
        </div>
        <EaSelect
          v-model="settingsStore.settings.compressionStrategy"
          :options="compressionStrategyOptions"
          :disabled="!settingsStore.settings.autoCompressionEnabled"
        />
      </div>

      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.compressionThreshold') }}</span>
          <span class="settings-item__desc">{{ t('settings.general.compressionThresholdDesc') }}</span>
        </div>
        <EaSelect
          v-model="settingsStore.settings.compressionThreshold"
          :options="compressionThresholdOptions"
          :disabled="!settingsStore.settings.autoCompressionEnabled"
        />
      </div>
    </SettingsSectionCard>

    <SettingsSectionCard :title="t('settings.general.editorSettings')">
      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.editorFontSize') }}</span>
          <span class="settings-item__desc">{{ t('settings.general.editorFontSizeDesc') }}</span>
        </div>
        <input
          v-model.number="settingsStore.settings.editorFontSize"
          type="number"
          class="settings-input settings-input--small"
          min="10"
          max="24"
        >
      </div>

      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.tabWidth') }}</span>
          <span class="settings-item__desc">{{ t('settings.general.tabWidthDesc') }}</span>
        </div>
        <EaSelect
          v-model="settingsStore.settings.editorTabSize"
          :options="tabSizeOptions"
        />
      </div>

      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.general.wordWrap') }}</span>
          <span class="settings-item__desc">{{ t('settings.general.wordWrapDesc') }}</span>
        </div>
        <label class="settings-toggle">
          <input
            v-model="settingsStore.settings.editorWordWrap"
            type="checkbox"
          >
          <span class="settings-toggle__slider" />
        </label>
      </div>
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

.settings-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-4);
}

.settings-item--column {
  flex-direction: column;
  align-items: stretch;
}

.settings-item__header {
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

.settings-select {
  min-width: 120px;
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  cursor: pointer;
}

.settings-select:focus {
  border-color: var(--color-primary);
  outline: none;
}

.settings-input {
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.settings-input--small {
  width: 80px;
}

.settings-input:focus {
  border-color: var(--color-primary);
  outline: none;
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

.font-size-value {
  min-width: 50px;
  padding: var(--spacing-1) var(--spacing-3);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  text-align: center;
}

.font-size-slider {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  margin-top: var(--spacing-3);
}

.font-size-slider__label {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  min-width: 32px;
}

.font-size-slider__input {
  flex: 1;
  height: 6px;
  background: var(--color-border);
  border-radius: var(--radius-full);
  outline: none;
  cursor: pointer;
  -webkit-appearance: none;
  appearance: none;
}

.font-size-slider__input::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 18px;
  height: 18px;
  background: var(--color-primary);
  border-radius: 50%;
  cursor: pointer;
  transition: transform var(--transition-fast) var(--easing-default);
}

.font-size-slider__input::-webkit-slider-thumb:hover {
  transform: scale(1.1);
}

.font-size-slider__input::-moz-range-thumb {
  width: 18px;
  height: 18px;
  background: var(--color-primary);
  border: none;
  border-radius: 50%;
  cursor: pointer;
  transition: transform var(--transition-fast) var(--easing-default);
}

.font-size-slider__input::-moz-range-thumb:hover {
  transform: scale(1.1);
}

.settings-warning {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  background-color: var(--color-warning-bg, rgba(234, 179, 8, 0.1));
  border: 1px solid var(--color-warning-border, rgba(234, 179, 8, 0.3));
  border-radius: var(--radius-md);
}

.settings-warning__icon {
  font-size: var(--font-size-base);
  flex-shrink: 0;
}

.settings-warning__text {
  font-size: var(--font-size-sm);
  color: var(--color-warning-text, #b45309);
  line-height: 1.4;
}
</style>
