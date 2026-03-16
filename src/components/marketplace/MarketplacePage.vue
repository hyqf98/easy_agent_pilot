<script setup lang="ts">
import { onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMarketplaceStore } from '@/stores/marketplace'
import { EaIcon, EaSelect } from '@/components/common'
import MarketplaceTabs from './MarketplaceTabs.vue'
import McpMarketList from './mcp/McpMarketList.vue'
import SkillMarketList from './skills/SkillMarketList.vue'

const { t } = useI18n()
const marketplaceStore = useMarketplaceStore()

// 初始化加载数据
onMounted(async () => {
  await marketplaceStore.loadMarketplaceSources()
  await marketplaceStore.loadAllInstalled()
  await marketplaceStore.refreshCurrentMarket()
})

watch(() => marketplaceStore.activeMarketTab, async (newTab) => {
  if (newTab === 'mcp' && marketplaceStore.mcpMarketItems.length === 0) {
    await marketplaceStore.fetchMcpMarket()
  } else if (newTab === 'skills' && marketplaceStore.skillsMarketItems.length === 0) {
    await marketplaceStore.fetchSkillsMarket()
  }
})

watch(() => marketplaceStore.activeMarketSource, async () => {
  await marketplaceStore.refreshCurrentMarket()
})
</script>

<template>
  <div class="marketplace-page">
    <div class="marketplace-page__header">
      <div>
        <h2 class="marketplace-page__title">
          <EaIcon name="store" :size="24" />
          {{ t('marketplace.title') }}
        </h2>
        <p class="marketplace-page__subtitle">
          {{ t('marketplace.subtitle') }}
        </p>
      </div>

      <div class="marketplace-page__source">
        <span class="marketplace-page__source-label">
          {{ t('marketplace.sourceMarket') }}
        </span>
        <EaSelect
          :model-value="marketplaceStore.activeMarketSource"
          :options="marketplaceStore.marketplaceSourceOptions"
          @update:model-value="value => marketplaceStore.setActiveMarketSource(String(value))"
        />
      </div>
    </div>

    <MarketplaceTabs />

    <div class="marketplace-page__content">
      <McpMarketList v-if="marketplaceStore.activeMarketTab === 'mcp'" />
      <SkillMarketList v-else-if="marketplaceStore.activeMarketTab === 'skills'" />
    </div>
  </div>
</template>

<style scoped>
.marketplace-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: var(--color-bg-primary);
}

.marketplace-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-4);
  padding: var(--spacing-6) var(--spacing-6) var(--spacing-4);
  border-bottom: 1px solid var(--color-border);
}

.marketplace-page__title {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.marketplace-page__subtitle {
  margin: var(--spacing-2) 0 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.marketplace-page__source {
  display: flex;
  min-width: 220px;
  flex-direction: column;
  gap: var(--spacing-2);
}

.marketplace-page__source-label {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.marketplace-page__content {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-4);
}

@media (max-width: 960px) {
  .marketplace-page__header {
    flex-direction: column;
  }

  .marketplace-page__source {
    width: 100%;
    min-width: 0;
  }
}
</style>
