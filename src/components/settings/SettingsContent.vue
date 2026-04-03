<script setup lang="ts">
import { computed } from 'vue'
import { useUIStore } from '@/stores/ui'
import { getSettingsTabDescriptor } from './settingsTabs'

const uiStore = useUIStore()
const activeTabDescriptor = computed(() => getSettingsTabDescriptor(uiStore.activeSettingsTab))
</script>

<template>
  <div
    :class="[
      'settings-content',
      {
        'settings-content--wide': activeTabDescriptor.layout === 'wide',
        'settings-content--full': activeTabDescriptor.layout === 'full'
      }
    ]"
  >
    <component
      :is="activeTabDescriptor.component"
      v-if="activeTabDescriptor.layout === 'full'"
      class="settings-content__full"
    />
    <div
      v-else-if="activeTabDescriptor.layout === 'wide'"
      class="settings-content__wide"
    >
      <component :is="activeTabDescriptor.component" />
    </div>
    <div
      v-else
      class="settings-content__inner"
    >
      <component :is="activeTabDescriptor.component" />
    </div>
  </div>
</template>

<style scoped>
.settings-content {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-6);
  display: flex;
  justify-content: center;
  min-height: 0;
  min-width: 0;
  overscroll-behavior: contain;
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}

.settings-content--full {
  justify-content: stretch;
  padding: var(--spacing-4);
  overflow: hidden;
}

.settings-content--wide {
  justify-content: stretch;
}

.settings-content__inner {
  max-width: 640px;
  width: 100%;
}

.settings-content__wide {
  width: 100%;
  min-width: 0;
}

.settings-content__full {
  flex: 1;
  width: 100%;
  height: 100%;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}

/* 自定义滚动条 */
.settings-content::-webkit-scrollbar {
  width: var(--scrollbar-size, 6px);
}

.settings-content::-webkit-scrollbar-track {
  background: var(--scrollbar-track, transparent);
  border-radius: var(--radius-full);
}

.settings-content::-webkit-scrollbar-thumb {
  background-color: var(--scrollbar-thumb, var(--color-border));
  border-radius: var(--radius-full);
  border: 1px solid transparent;
  background-clip: padding-box;
}

.settings-content::-webkit-scrollbar-thumb:hover {
  background-color: var(--scrollbar-thumb-hover, var(--color-border-dark));
}

.settings-content::-webkit-scrollbar-thumb:active {
  background-color: var(--scrollbar-thumb-active, var(--color-text-secondary));
}
</style>
