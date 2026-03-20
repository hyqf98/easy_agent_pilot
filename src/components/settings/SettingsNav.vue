<script setup lang="ts">
import { watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useUIStore } from '@/stores/ui'
import { EaIcon } from '@/components/common'
import { SETTINGS_TAB_DESCRIPTORS } from './settingsTabs'

const { t } = useI18n()
const uiStore = useUIStore()

// 鼠标悬停事件处理
function handleMouseEnter() {
  uiStore.setSettingsNavCollapsed(false)
}

function handleMouseLeave() {
  uiStore.setSettingsNavCollapsed(true)
}

watch(
  () => uiStore.activeSettingsTab,
  (tab) => {
    if (tab === 'logs') {
      uiStore.setSettingsNavCollapsed(true)
    }
  },
  { immediate: true }
)
</script>

<template>
  <nav
    :class="[
      'settings-nav',
      {
        'settings-nav--collapsed': uiStore.settingsNavCollapsed,
        'settings-nav--logs': uiStore.settingsNavCollapsed && uiStore.activeSettingsTab === 'logs'
      }
    ]"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
  >
    <div class="settings-nav__list">
      <button
        v-for="item in SETTINGS_TAB_DESCRIPTORS"
        :key="item.id"
        :class="['settings-nav__item', { 'settings-nav__item--active': uiStore.activeSettingsTab === item.id }]"
        :title="uiStore.settingsNavCollapsed ? t(item.labelKey) : undefined"
        @click="uiStore.setActiveSettingsTab(item.id)"
      >
        <EaIcon
          :name="item.icon"
          :size="18"
        />
        <span
          v-if="!uiStore.settingsNavCollapsed"
          class="settings-nav__label"
        >
          {{ t(item.labelKey) }}
        </span>
      </button>
    </div>
  </nav>
</template>

<style scoped>
.settings-nav {
  display: flex;
  flex-direction: column;
  width: 240px;
  flex-shrink: 0;
  padding: var(--spacing-4);
  padding-bottom: var(--spacing-2);
  background-color: var(--color-bg-secondary);
  border-right: 1px solid var(--color-border);
  overflow-y: auto;
  overflow-x: hidden;
  transition: width var(--transition-normal) var(--easing-default);
}

.settings-nav--collapsed {
  width: 56px;
  padding: var(--spacing-2);
}

.settings-nav--logs {
  width: 44px;
  padding: 6px;
}

.settings-nav__list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
  flex: 1;
}

.settings-nav__item {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  width: 100%;
  padding: var(--spacing-3) var(--spacing-4);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  text-align: left;
  transition: all var(--transition-fast) var(--easing-default);
}

.settings-nav--collapsed .settings-nav__item {
  justify-content: center;
  padding: var(--spacing-3);
}

.settings-nav__item:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.settings-nav__item--active {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
  font-weight: var(--font-weight-medium);
}

.settings-nav__item--active:hover {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

[data-theme='dark'] .settings-nav__item--active {
  background-color: var(--color-active-bg);
  color: var(--color-active-text);
}

[data-theme='dark'] .settings-nav__item--active:hover {
  background-color: var(--color-active-bg-hover);
  color: var(--color-active-text);
}

.settings-nav__label {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
