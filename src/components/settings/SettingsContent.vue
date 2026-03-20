<script setup lang="ts">
import { useUIStore } from '@/stores/ui'
import GeneralSettings from './tabs/GeneralSettings.vue'
import AgentStudioSettings from './tabs/AgentStudioSettings.vue'
import ModelManagement from './tabs/ModelManagement.vue'
import ThemeSettings from './tabs/ThemeSettings.vue'
import DataSettings from './tabs/DataSettings.vue'
import LspSettings from './tabs/LspSettings.vue'
import SessionManagementSettings from './tabs/SessionManagementSettings.vue'
import MarketplacePage from '@/components/marketplace/MarketplacePage.vue'

const uiStore = useUIStore()
</script>

<template>
  <div class="settings-content">
    <!-- MarketplacePage 需要更大的空间 -->
    <MarketplacePage
      v-if="uiStore.activeSettingsTab === 'marketplace'"
      class="settings-content__full"
    />
    <AgentStudioSettings
      v-else-if="uiStore.activeSettingsTab === 'agentStudio'"
      class="settings-content__full"
    />
    <ModelManagement
      v-else-if="uiStore.activeSettingsTab === 'modelManagement' || uiStore.activeSettingsTab === 'agents' || uiStore.activeSettingsTab === 'agentConfig' || uiStore.activeSettingsTab === 'providerSwitch'"
      class="settings-content__full"
    />
    <!-- 其他设置页面使用固定宽度 -->
    <div
      v-else
      class="settings-content__inner"
    >
      <GeneralSettings v-if="uiStore.activeSettingsTab === 'general'" />
      <SessionManagementSettings v-else-if="uiStore.activeSettingsTab === 'sessions'" />
      <ThemeSettings v-else-if="uiStore.activeSettingsTab === 'theme'" />
      <LspSettings v-else-if="uiStore.activeSettingsTab === 'lsp'" />
      <DataSettings v-else-if="uiStore.activeSettingsTab === 'data'" />
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
}

.settings-content__inner {
  max-width: 640px;
  width: 100%;
}

.settings-content__full {
  flex: 1;
  width: 100%;
  height: 100%;
  min-width: 0;
}

/* 自定义滚动条 */
.settings-content::-webkit-scrollbar {
  width: var(--scrollbar-size, 6px);
}

.settings-content::-webkit-scrollbar-track {
  background: var(--scrollbar-track, transparent);
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
</style>
