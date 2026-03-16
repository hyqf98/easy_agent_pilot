<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMarketplaceStore } from '@/stores/marketplace'
import { EaButton, EaIcon, EaInput, EaLoading } from '@/components/common'
import McpMarketCard from './McpMarketCard.vue'
import McpDetailModal from './McpDetailModal.vue'
import McpInstallModal from './McpInstallModal.vue'
import type { McpMarketItem } from '@/types/marketplace'

const { t } = useI18n()
const marketplaceStore = useMarketplaceStore()

const listRef = ref<HTMLElement | null>(null)
const loadMoreTriggerRef = ref<HTMLElement | null>(null)
const scrollContainer = ref<HTMLElement | null>(null)
const searchQuery = ref('')
const selectedCategorySlug = ref('')
const showDetailModal = ref(false)
const showInstallModal = ref(false)
const selectedMcp = ref<McpMarketItem | null>(null)

let loadMoreScheduled = false
let loadMoreObserver: IntersectionObserver | null = null
let loadMorePollTimer: number | null = null

const categoryOptions = computed(() => [
  { value: '', label: t('marketplace.allCategories') },
  ...marketplaceStore.mcpMarketCategories.map(category => ({
    value: category.slug || category.value,
    label: category.label
  }))
])

function syncLocalFilters() {
  searchQuery.value = marketplaceStore.mcpMarketQuery.search || ''
  selectedCategorySlug.value = marketplaceStore.mcpMarketQuery.category_slug || ''
}

async function refreshMarket() {
  await marketplaceStore.fetchMcpMarket({
    ...marketplaceStore.mcpMarketQuery,
    page: 1
  })
}

async function submitSearch() {
  await marketplaceStore.fetchMcpMarket({
    page: 1,
    search: searchQuery.value || null,
    category: null,
    category_slug: null
  })
}

async function resetFilters() {
  searchQuery.value = ''
  selectedCategorySlug.value = ''
  await marketplaceStore.fetchMcpMarket({
    page: 1,
    search: null,
    category: null,
    category_slug: null
  })
}

async function handleCategoryChange(event: Event) {
  const value = (event.target as HTMLSelectElement).value
  selectedCategorySlug.value = value
  searchQuery.value = ''

  const selectedCategory = marketplaceStore.mcpMarketCategories.find(category => {
    const slug = category.slug || category.value
    return slug === value
  })

  await marketplaceStore.fetchMcpMarket({
    page: 1,
    search: null,
    category: selectedCategory?.label || null,
    category_slug: value || null
  })
}

async function loadMore() {
  if (!marketplaceStore.mcpMarketPagination.hasMore || marketplaceStore.isLoadingMcpMarket) {
    return
  }

  await marketplaceStore.fetchMcpMarket(
    {
      ...marketplaceStore.mcpMarketQuery,
      page: marketplaceStore.mcpMarketPagination.page + 1
    },
    { append: true }
  )
}

function openInstallModal(item: McpMarketItem) {
  selectedMcp.value = item
  showInstallModal.value = true
}

function openDetailModal(item: McpMarketItem) {
  selectedMcp.value = item
  showDetailModal.value = true
}

function closeDetailModal() {
  showDetailModal.value = false
  selectedMcp.value = null
}

function closeInstallModal() {
  showInstallModal.value = false
  selectedMcp.value = null
}

function openInstallFromDetail(item: McpMarketItem) {
  selectedMcp.value = item
  showDetailModal.value = false
  showInstallModal.value = true
}

function onInstallComplete() {
  closeInstallModal()
}

function isNearBottom() {
  const container = scrollContainer.value
  if (!container) {
    return false
  }

  const distanceToBottom = container.scrollHeight - container.scrollTop - container.clientHeight
  return distanceToBottom < 240
}

function scheduleLoadMoreCheck() {
  if (loadMoreScheduled) {
    return
  }

  loadMoreScheduled = true
  window.setTimeout(async () => {
    loadMoreScheduled = false
    if (isNearBottom()) {
      await loadMore()
    }
  }, 0)
}

function handleScroll() {
  scheduleLoadMoreCheck()
}

function cleanupLoadMoreObserver() {
  loadMoreObserver?.disconnect()
  loadMoreObserver = null
}

function stopLoadMorePolling() {
  if (loadMorePollTimer !== null) {
    window.clearInterval(loadMorePollTimer)
    loadMorePollTimer = null
  }
}

function startLoadMorePolling() {
  stopLoadMorePolling()

  loadMorePollTimer = window.setInterval(() => {
    scheduleLoadMoreCheck()
  }, 1000)
}

function initLoadMoreObserver() {
  cleanupLoadMoreObserver()

  if (!scrollContainer.value || !loadMoreTriggerRef.value || !marketplaceStore.mcpMarketPagination.hasMore) {
    return
  }

  loadMoreObserver = new IntersectionObserver((entries) => {
    if (entries.some(entry => entry.isIntersecting)) {
      scheduleLoadMoreCheck()
    }
  }, {
    root: scrollContainer.value,
    rootMargin: '0px 0px 320px 0px'
  })

  loadMoreObserver.observe(loadMoreTriggerRef.value)
}

watch(
  () => [marketplaceStore.mcpMarketQuery.search, marketplaceStore.mcpMarketQuery.category_slug],
  () => syncLocalFilters(),
  { immediate: true }
)

watch(
  () => marketplaceStore.mcpMarketItems.length,
  async () => {
    await nextTick()
    initLoadMoreObserver()
    scheduleLoadMoreCheck()
  }
)

watch(
  () => marketplaceStore.mcpMarketPagination.hasMore,
  async () => {
    await nextTick()
    initLoadMoreObserver()
  },
  { immediate: true }
)

onMounted(async () => {
  scrollContainer.value = listRef.value?.closest('.marketplace-page__content') as HTMLElement | null
  scrollContainer.value?.addEventListener('scroll', handleScroll, { passive: true })
  startLoadMorePolling()

  if (marketplaceStore.mcpMarketItems.length === 0 && !marketplaceStore.isLoadingMcpMarket) {
    await marketplaceStore.fetchMcpMarket({ page: 1 })
  }

  await nextTick()
  initLoadMoreObserver()
  scheduleLoadMoreCheck()
})

onBeforeUnmount(() => {
  cleanupLoadMoreObserver()
  stopLoadMorePolling()
  scrollContainer.value?.removeEventListener('scroll', handleScroll)
})
</script>

<template>
  <div ref="listRef" class="mcp-market-list">
    <div class="mcp-market-list__toolbar">
      <div class="mcp-market-list__search">
        <EaInput
          v-model="searchQuery"
          :placeholder="t('marketplace.search')"
          @keydown.enter="submitSearch"
        />
      </div>

      <div class="mcp-market-list__filters">
        <select
          :value="selectedCategorySlug"
          class="mcp-market-list__select"
          @change="handleCategoryChange"
        >
          <option
            v-for="option in categoryOptions"
            :key="option.value"
            :value="option.value"
          >
            {{ option.label }}
          </option>
        </select>

        <EaButton
          type="primary"
          size="small"
          @click="submitSearch"
        >
          {{ t('common.search') }}
        </EaButton>

        <EaButton
          type="ghost"
          size="small"
          @click="resetFilters"
        >
          {{ t('common.clear') }}
        </EaButton>

        <EaButton
          type="ghost"
          size="small"
          @click="refreshMarket"
        >
          <EaIcon name="refresh-cw" :size="16" />
        </EaButton>
      </div>
    </div>

    <EaLoading
      v-if="marketplaceStore.isLoadingMcpMarket && marketplaceStore.mcpMarketItems.length === 0"
      :message="t('marketplace.loading')"
    />

    <div
      v-else-if="marketplaceStore.mcpMarketError"
      class="mcp-market-list__error"
    >
      <EaIcon name="alert-circle" :size="24" />
      <p>{{ marketplaceStore.mcpMarketError }}</p>
      <EaButton
        type="secondary"
        @click="refreshMarket"
      >
        {{ t('common.retry') }}
      </EaButton>
    </div>

    <div
      v-else-if="marketplaceStore.mcpMarketItems.length === 0"
      class="mcp-market-list__empty"
    >
      <EaIcon name="package" :size="48" />
      <p>{{ t('marketplace.noResults') }}</p>
    </div>

    <template v-else>
      <div class="mcp-market-list__grid">
        <McpMarketCard
          v-for="item in marketplaceStore.mcpMarketItems"
          :key="item.id"
          :item="item"
          :is-installed="marketplaceStore.installedMcpNames.has(item.slug.toLowerCase())"
          @view="openDetailModal(item)"
          @install="openInstallModal(item)"
        />
      </div>

      <div
        v-if="marketplaceStore.mcpMarketPagination.hasMore"
        class="mcp-market-list__load-more"
      >
        <div
          ref="loadMoreTriggerRef"
          class="mcp-market-list__load-more-sentinel"
          aria-hidden="true"
        />
        <EaLoading
          v-if="marketplaceStore.isLoadingMcpMarket"
          size="sm"
          :message="t('marketplace.loadMore')"
        />
        <EaButton
          v-else
          type="secondary"
          size="small"
          @click="loadMore"
        >
          {{ t('marketplace.loadMore') }}
        </EaButton>
      </div>
    </template>

    <McpInstallModal
      v-if="showInstallModal && selectedMcp"
      :mcp-item="selectedMcp"
      @close="closeInstallModal"
      @complete="onInstallComplete"
    />

    <McpDetailModal
      v-if="showDetailModal && selectedMcp"
      :mcp-item="selectedMcp"
      @close="closeDetailModal"
      @install="openInstallFromDetail"
    />
  </div>
</template>

<style scoped>
.mcp-market-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.mcp-market-list__toolbar {
  display: flex;
  gap: var(--spacing-4);
  align-items: center;
  flex-wrap: wrap;
}

.mcp-market-list__search {
  flex: 1;
  min-width: 220px;
}

.mcp-market-list__filters {
  display: flex;
  gap: var(--spacing-2);
  align-items: center;
  flex-wrap: wrap;
}

.mcp-market-list__select {
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  height: 36px;
  min-width: 140px;
  padding: var(--spacing-2) var(--spacing-8) var(--spacing-2) var(--spacing-3);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  background-color: var(--color-surface);
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%2364748b' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.mcp-market-list__select:hover {
  border-color: var(--color-primary);
}

.mcp-market-list__select:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px var(--color-primary-light);
}

.mcp-market-list__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--spacing-4);
}

.mcp-market-list__load-more {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) 0 var(--spacing-5);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.mcp-market-list__load-more-sentinel {
  width: 100%;
  height: 1px;
}

.mcp-market-list__error,
.mcp-market-list__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-3);
  padding: var(--spacing-8);
  color: var(--color-text-secondary);
  text-align: center;
}

.mcp-market-list__error p,
.mcp-market-list__empty p {
  margin: 0;
}
</style>
