<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMarketplaceStore } from '@/stores/marketplace'
import { EaIcon } from '@/components/common'

const { t } = useI18n()
const marketplaceStore = useMarketplaceStore()

interface TabItem {
  id: 'mcp' | 'skills'
  labelKey: string
  icon: string
}

const tabs: TabItem[] = [
  { id: 'mcp', labelKey: 'marketplace.tabs.mcp', icon: 'plug' },
  { id: 'skills', labelKey: 'marketplace.tabs.skills', icon: 'sparkles' }
]

const visibleTabs = computed(() =>
  tabs.filter(tab => marketplaceStore.activeMarketSupportedResources.includes(tab.id))
)
</script>

<template>
  <div class="marketplace-tabs">
    <button
      v-for="tab in visibleTabs"
      :key="tab.id"
      :class="['marketplace-tabs__tab', { 'marketplace-tabs__tab--active': marketplaceStore.activeMarketTab === tab.id }]"
      @click="marketplaceStore.setActiveMarketTab(tab.id)"
    >
      <EaIcon :name="tab.icon" :size="16" />
      <span>{{ t(tab.labelKey) }}</span>
    </button>
  </div>
</template>

<style scoped>
.marketplace-tabs {
  display: flex;
  gap: var(--spacing-2);
  padding: 0 var(--spacing-6);
  background-color: var(--color-bg-primary);
  border-bottom: 1px solid var(--color-border);
}

.marketplace-tabs__tab {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-3) var(--spacing-4);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.marketplace-tabs__tab:hover {
  color: var(--color-text-primary);
}

.marketplace-tabs__tab--active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
}
</style>
