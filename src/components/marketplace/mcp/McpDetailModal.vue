<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon, EaLoading, EaModal, EaTag } from '@/components/common'
import MarkdownRenderer from '@/components/message/MarkdownRenderer.vue'
import { useMarketplaceStore } from '@/stores/marketplace'
import type { McpMarketItem } from '@/types/marketplace'
import { normalizeMarketplaceMarkdown } from '@/utils/marketplaceMarkdown'

interface Props {
  mcpItem: McpMarketItem
}

const props = defineProps<Props>()

const emit = defineEmits<{
  close: []
  install: [item: McpMarketItem]
}>()

const { t } = useI18n()
const marketplaceStore = useMarketplaceStore()

const detail = computed(() => {
  const selected = marketplaceStore.selectedMcpDetail
  return selected?.slug === props.mcpItem.slug ? selected : null
})

const sourceLabel = computed(() => {
  const source = marketplaceStore.marketplaceSources.find(item => item.id === props.mcpItem.source_market)
  return source?.label || props.mcpItem.source_market
})

const displayTags = computed(() => {
  const values = detail.value?.tags?.length ? detail.value.tags : props.mcpItem.tags
  return values.filter(Boolean).slice(0, 6)
})

const description = computed(() => detail.value?.full_description || props.mcpItem.description)
const renderedDescription = computed(() => normalizeMarketplaceMarkdown(description.value))

async function loadDetail() {
  await marketplaceStore.fetchMcpDetail(props.mcpItem.slug, props.mcpItem.source_market)
}

function closeModal() {
  marketplaceStore.clearMcpDetail()
  emit('close')
}

function openInstall() {
  emit('install', props.mcpItem)
}

watch(() => props.mcpItem.slug, loadDetail)
onMounted(loadDetail)
</script>

<template>
  <EaModal
    :visible="true"
    content-class="mcp-detail-modal"
    @update:visible="value => !value && closeModal()"
  >
    <template #header>
      <div class="marketplace-detail__header">
        <div>
          <div class="marketplace-detail__eyebrow">
            <EaIcon name="plug" :size="14" />
            <span>{{ t('marketplace.viewDetails') }}</span>
          </div>
          <h3 class="marketplace-detail__title">{{ mcpItem.name }}</h3>
          <p class="marketplace-detail__subtitle">{{ mcpItem.description }}</p>
        </div>
        <button class="marketplace-detail__close" @click="closeModal">
          <EaIcon name="x" :size="16" />
        </button>
      </div>
    </template>

    <EaLoading
      v-if="marketplaceStore.isLoadingMcpDetail && !detail"
      :message="t('marketplace.loading')"
    />

    <div v-else-if="marketplaceStore.mcpDetailError" class="marketplace-detail__state">
      <EaIcon name="alert-circle" :size="18" />
      <span>{{ marketplaceStore.mcpDetailError }}</span>
      <EaButton type="secondary" size="small" @click="loadDetail">
        {{ t('common.retry') }}
      </EaButton>
    </div>

    <div v-else class="marketplace-detail">
      <div class="marketplace-detail__meta">
        <div class="marketplace-detail__meta-item">
          <span class="marketplace-detail__label">{{ t('marketplace.sourceLabel') }}</span>
          <strong>{{ sourceLabel }}</strong>
        </div>
        <div class="marketplace-detail__meta-item" v-if="detail?.author || mcpItem.author">
          <span class="marketplace-detail__label">{{ t('marketplace.authorLabel') }}</span>
          <strong>{{ detail?.author || mcpItem.author }}</strong>
        </div>
        <div class="marketplace-detail__meta-item" v-if="detail?.transport_type || mcpItem.transport_type">
          <span class="marketplace-detail__label">{{ t('marketplace.transportLabel') }}</span>
          <strong>{{ detail?.transport_type || mcpItem.transport_type }}</strong>
        </div>
      </div>

      <div v-if="displayTags.length > 0" class="marketplace-detail__tags">
        <EaTag v-for="tag in displayTags" :key="tag" variant="info" size="sm">
          {{ tag }}
        </EaTag>
      </div>

      <div v-if="detail?.repository_url || mcpItem.repository_url" class="marketplace-detail__links">
        <a
          :href="detail?.repository_url || mcpItem.repository_url || '#'"
          target="_blank"
          rel="noreferrer"
          class="marketplace-detail__link"
        >
          <EaIcon name="external-link" :size="14" />
          {{ t('marketplace.openRepository') }}
        </a>
      </div>

      <section class="marketplace-detail__section">
        <h4>{{ t('marketplace.descriptionLabel') }}</h4>
        <div class="marketplace-detail__markdown">
          <MarkdownRenderer :content="renderedDescription" />
        </div>
      </section>
    </div>

    <template #footer>
      <div class="marketplace-detail__footer">
        <EaButton type="ghost" @click="closeModal">
          {{ t('common.close') }}
        </EaButton>
        <EaButton type="primary" @click="openInstall">
          {{ t('marketplace.install') }}
        </EaButton>
      </div>
    </template>
  </EaModal>
</template>

<style scoped>
.marketplace-detail__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-4);
}

.marketplace-detail__eyebrow {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-2);
  margin-bottom: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.marketplace-detail__title {
  margin: 0;
  font-size: var(--font-size-xl);
  color: var(--color-text-primary);
}

.marketplace-detail__subtitle {
  margin: var(--spacing-2) 0 0;
  color: var(--color-text-secondary);
  line-height: 1.6;
}

.marketplace-detail__close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
}

.marketplace-detail {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.marketplace-detail__meta {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: var(--spacing-3);
}

.marketplace-detail__meta-item {
  padding: var(--spacing-3);
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
}

.marketplace-detail__label {
  display: block;
  margin-bottom: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.marketplace-detail__tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
}

.marketplace-detail__links {
  display: flex;
  gap: var(--spacing-3);
}

.marketplace-detail__link {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-2);
  color: var(--color-primary);
  text-decoration: none;
}

.marketplace-detail__section h4 {
  margin: 0 0 var(--spacing-3);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.marketplace-detail__markdown {
  min-width: 0;
}

.marketplace-detail__markdown :deep(.markdown-content) {
  color: var(--color-text-secondary);
}

.marketplace-detail__markdown :deep(.markdown-content > :first-child) {
  margin-top: 0;
}

.marketplace-detail__markdown :deep(.markdown-content > :last-child) {
  margin-bottom: 0;
}

.marketplace-detail__state {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  color: var(--color-error);
}

.marketplace-detail__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-2);
}

:deep(.mcp-detail-modal) {
  width: min(860px, calc(100vw - 32px));
  max-width: min(860px, calc(100vw - 32px));
}
</style>
