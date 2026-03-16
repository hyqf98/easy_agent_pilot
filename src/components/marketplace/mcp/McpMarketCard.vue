<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { EaIcon, EaButton, EaTag } from '@/components/common'
import type { McpMarketItem } from '@/types/marketplace'

interface Props {
  item: McpMarketItem
  isInstalled: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  install: [item: McpMarketItem]
  view: [item: McpMarketItem]
}>()

const { t } = useI18n()

function handleInstall() {
  emit('install', props.item)
}

function handleView() {
  emit('view', props.item)
}

function handleCardKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    handleView()
  }
}
</script>

<template>
  <div
    class="mcp-market-card"
    role="button"
    tabindex="0"
    @click="handleView"
    @keydown="handleCardKeydown"
  >
    <div class="mcp-market-card__header">
      <div class="mcp-market-card__icon">
        <EaIcon name="plug" :size="24" />
      </div>
      <div class="mcp-market-card__info">
        <h3 class="mcp-market-card__name">
          {{ item.name }}
          <EaTag
            v-if="isInstalled"
            variant="success"
            size="sm"
          >
            {{ t('marketplace.installed') }}
          </EaTag>
        </h3>
        <p class="mcp-market-card__author">
          {{ t('marketplace.by') }} {{ item.author || 'MCP Market' }}
        </p>
      </div>
    </div>

    <p class="mcp-market-card__description">
      {{ item.description }}
    </p>

    <div
      v-if="item.tags && item.tags.length > 0"
      class="mcp-market-card__tags"
    >
      <EaTag
        v-for="tag in item.tags.slice(0, 3)"
        :key="tag"
        variant="default"
        size="sm"
      >
        {{ tag }}
      </EaTag>
    </div>

    <div class="mcp-market-card__footer">
      <div class="mcp-market-card__stats">
        <span
          v-if="item.stars"
          class="mcp-market-card__stat"
        >
          <EaIcon name="star" :size="14" />
          {{ item.stars.toLocaleString() }}
        </span>
        <span
          v-if="item.category"
          class="mcp-market-card__stat"
        >
          <EaIcon name="tag" :size="14" />
          {{ item.category }}
        </span>
      </div>

      <EaButton
        :type="isInstalled ? 'ghost' : 'primary'"
        size="small"
        class="mcp-market-card__action"
        :class="{ 'mcp-market-card__action--installed': isInstalled }"
        @click.stop="handleInstall"
      >
        <EaIcon
          :name="isInstalled ? 'refresh-cw' : 'download'"
          :size="14"
        />
        {{ isInstalled ? t('marketplace.reinstall') : t('marketplace.install') }}
      </EaButton>
    </div>
  </div>
</template>

<style scoped>
.mcp-market-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  background-color: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.mcp-market-card:hover {
  border-color: var(--color-primary);
  box-shadow: var(--shadow-sm);
}

.mcp-market-card:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}

.mcp-market-card__header {
  display: flex;
  gap: var(--spacing-3);
  align-items: flex-start;
}

.mcp-market-card__icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background-color: var(--color-primary-light);
  border-radius: var(--radius-md);
  color: var(--color-primary);
}

.mcp-market-card__info {
  flex: 1;
  min-width: 0;
}

.mcp-market-card__name {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  margin: 0;
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.mcp-market-card__author {
  margin: var(--spacing-1) 0 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.mcp-market-card__description {
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.mcp-market-card__tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-1);
}

.mcp-market-card__footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: auto;
  padding-top: var(--spacing-3);
  border-top: 1px solid var(--color-border);
}

.mcp-market-card__stats {
  display: flex;
  gap: var(--spacing-3);
}

.mcp-market-card__stat {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

/* 安装按钮样式优化 */
.mcp-market-card__action {
  min-width: 80px;
}

.mcp-market-card__action--installed {
  color: var(--color-success);
}

.mcp-market-card__action--installed:hover:not(.ea-button--disabled) {
  background-color: var(--color-success-light);
}
</style>
