<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useThemeStore, presetThemeColors } from '@/stores/theme'
import { EaSelect } from '@/components/common'
import SettingsSectionCard from '@/components/settings/common/SettingsSectionCard.vue'

const { t } = useI18n()
const themeStore = useThemeStore()

// 主题选项
const themeOptions = computed(() => [
  { value: 'light', label: t('settings.theme.light') },
  { value: 'dark', label: t('settings.theme.dark') },
  { value: 'system', label: t('settings.theme.system') }
])

// 处理主题模式变化
const handleThemeChange = async (newMode: string | number) => {
  await themeStore.setTheme(newMode as 'light' | 'dark' | 'system')
}

// 处理主题色变化
const handleThemeColorChange = async (themeColorId: string) => {
  await themeStore.setThemeColor(themeColorId)
}
</script>

<template>
  <div class="settings-page">
    <h3 class="settings-page__title">
      {{ t('settings.theme.title') }}
    </h3>

    <SettingsSectionCard :title="t('settings.theme.appearance')">
      <div class="settings-item">
        <div class="settings-item__info">
          <span class="settings-item__label">{{ t('settings.theme.themeMode') }}</span>
          <span class="settings-item__desc">{{ t('settings.theme.themeModeDesc') }}</span>
        </div>
        <EaSelect
          v-model="themeStore.mode"
          :options="themeOptions"
          @update:model-value="handleThemeChange"
        />
      </div>
    </SettingsSectionCard>

    <SettingsSectionCard
      :title="t('settings.theme.themeColor')"
      :description="t('settings.theme.themeColorDesc')"
    >
      <div class="theme-colors-grid">
        <button
          v-for="themeColor in presetThemeColors"
          :key="themeColor.id"
          class="theme-color-item"
          :class="{ 'theme-color-item--active': themeStore.currentThemeColorId === themeColor.id }"
          :title="themeColor.name"
          @click="handleThemeColorChange(themeColor.id)"
        >
          <span
            class="theme-color-preview"
            :style="{ backgroundColor: themeColor.primaryColor }"
          />
          <span class="theme-color-name">{{ themeColor.name }}</span>
          <span
            v-if="themeStore.currentThemeColorId === themeColor.id"
            class="theme-color-check"
          >
            ✓
          </span>
        </button>
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

.theme-colors-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: var(--spacing-3);
  margin-top: var(--spacing-2);
}

.theme-color-item {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  background-color: var(--color-surface);
  border: 2px solid var(--color-border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.theme-color-item:hover {
  border-color: var(--color-primary);
  background-color: var(--color-surface-hover);
}

.theme-color-item--active {
  border-color: var(--color-primary);
  background-color: var(--color-primary-light);
}

.theme-color-preview {
  width: 40px;
  height: 40px;
  border-radius: var(--radius-full);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.theme-color-name {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  text-align: center;
  line-height: 1.3;
}

.theme-color-item--active .theme-color-name {
  color: var(--color-primary);
  font-weight: var(--font-weight-medium);
}

.theme-color-check {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--color-primary);
  color: white;
  border-radius: 50%;
  font-size: 10px;
  font-weight: bold;
}
</style>
