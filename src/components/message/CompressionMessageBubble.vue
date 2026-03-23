<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessageStore, type CompressionMetadata, type Message } from '@/stores/message'
import MarkdownRenderer from './MarkdownRenderer.vue'
import { EaIcon } from '@/components/common'
import { formatTokenCount } from '@/stores/token'

const { t, locale } = useI18n()
const props = defineProps<{ message: Message }>()
const messageStore = useMessageStore()

// 压缩消息元数据
const metadata = computed(() => props.message.compressionMetadata)
const isExpanded = computed(() => Boolean(metadata.value?.panelExpanded))

// 是否有工具调用摘要
const hasToolCalls = computed(() =>
  metadata.value?.toolCallsSummary && metadata.value.toolCallsSummary.length > 0
)

// 格式化时间
const formattedTime = computed(() => {
  if (!metadata.value?.compressedAt) return ''
  const date = new Date(metadata.value.compressedAt)
  return date.toLocaleString(locale.value === 'zh-CN' ? 'zh-CN' : 'en-US', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false
  })
})

// 压缩策略文本
const strategyText = computed(() => {
  if (!metadata.value?.strategy) return ''
  switch (metadata.value.strategy) {
    case 'simple':
      return t('compression.strategySimple')
    case 'summary':
      return t('compression.strategySummary')
    default:
      return ''
  }
})

// 切换展开状态
const toggleExpand = () => {
  const nextMetadata: CompressionMetadata = {
    compressedAt: metadata.value?.compressedAt || props.message.createdAt,
    originalMessageCount: metadata.value?.originalMessageCount || 0,
    originalTokenCount: metadata.value?.originalTokenCount || 0,
    strategy: metadata.value?.strategy || 'simple',
    toolCallsSummary: metadata.value?.toolCallsSummary,
    panelExpanded: !isExpanded.value
  }

  messageStore.updateMessageBuffered(props.message.id, {
    compressionMetadata: nextMetadata
  }, { immediate: true })
}
</script>

<template>
  <div class="compression-bubble">
    <!-- 压缩头部 -->
    <div class="compression-bubble__header">
      <div class="compression-bubble__icon">
        <EaIcon
          name="archive"
          :size="16"
        />
      </div>
      <div class="compression-bubble__title">
        {{ t('compression.summaryTitle') }}
      </div>
      <button
        class="compression-bubble__toggle"
        @click="toggleExpand"
      >
        <EaIcon
          :name="isExpanded ? 'chevron-up' : 'chevron-down'"
          :size="14"
        />
        {{ isExpanded ? t('compression.collapse') : t('compression.expand') }}
      </button>
    </div>

    <!-- 摘要内容 -->
    <div
      v-show="isExpanded"
      class="compression-bubble__content"
    >
      <MarkdownRenderer :content="message.content" />
    </div>

    <!-- 压缩信息 -->
    <div class="compression-bubble__meta">
      <div class="compression-bubble__info">
        <span class="compression-bubble__info-item">
          <EaIcon
            name="message-square"
            :size="12"
          />
          {{ t('compression.originalMessages') }}: {{ metadata?.originalMessageCount || 0 }}
        </span>
        <span class="compression-bubble__info-item">
          <EaIcon
            name="cpu"
            :size="12"
          />
          {{ t('compression.originalTokens') }}: {{ formatTokenCount(metadata?.originalTokenCount || 0) }}
        </span>
        <span class="compression-bubble__info-item">
          <EaIcon
            name="clock"
            :size="12"
          />
          {{ formattedTime }}
        </span>
      </div>
      <div
        v-if="strategyText"
        class="compression-bubble__strategy"
      >
        {{ strategyText }}
      </div>
    </div>

    <!-- 工具调用记录 -->
    <div
      v-if="hasToolCalls && isExpanded"
      class="compression-bubble__tools"
    >
      <div class="compression-bubble__tools-header">
        <EaIcon
          name="wrench"
          :size="14"
        />
        {{ t('compression.toolCallsSummary') }}
      </div>
      <div class="compression-bubble__tools-list">
        <div
          v-for="tool in metadata?.toolCallsSummary"
          :key="tool.name"
          class="compression-bubble__tool-item"
        >
          <span class="compression-bubble__tool-status">
            <EaIcon
              :name="tool.status === 'success' ? 'check-circle' : tool.status === 'error' ? 'x-circle' : 'alert-triangle'"
              :size="14"
            />
          </span>
          <span class="compression-bubble__tool-name">{{ tool.name }}</span>
          <span class="compression-bubble__tool-count">{{ t('compression.toolCount', { count: tool.count }) }}</span>
        </div>
      </div>
    </div>

    <!-- 无工具调用提示 -->
    <div
      v-if="!hasToolCalls && isExpanded"
      class="compression-bubble__no-tools"
    >
      {{ t('compression.noToolCalls') }}
    </div>
  </div>
</template>

<style scoped>
.compression-bubble {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: min(46rem, 68%);
  margin: 0 auto;
  background-color: var(--color-bg-tertiary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.compression-bubble__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-3);
  border-bottom: 1px solid var(--color-border);
}

.compression-bubble__icon {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--color-accent-light), var(--color-accent));
  border-radius: var(--radius-md);
  color: white;
}

.compression-bubble__title {
  flex: 1;
  font-size: var(--font-size-base);
  font-weight: 600;
  color: var(--color-text-primary);
}

.compression-bubble__toggle {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  padding: var(--spacing-1) var(--spacing-2);
  background: none;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
  margin-left: auto;
}

.compression-bubble__toggle:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.compression-bubble__content {
  padding: var(--spacing-3);
  overflow-y: auto;
  max-height: 300px;
}

.compression-bubble__meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-bg-secondary);
}

.compression-bubble__info {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-4);
}

.compression-bubble__info-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.compression-bubble__strategy {
  font-size: var(--font-size-xs);
  padding: var(--spacing-1) var(--spacing-2);
  background-color: var(--color-primary-light);
  color: var(--color-primary);
  border-radius: var(--radius-sm);
}

.compression-bubble__tools {
  padding: var(--spacing-3);
  border-top: 1px solid var(--color-border);
}

.compression-bubble__tools-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  font-size: var(--font-size-xs);
  font-weight: 500;
  color: var(--color-text-secondary);
}

.compression-bubble__tools-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
}

.compression-bubble__tool-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-1) var(--spacing-2);
  background-color: var(--color-surface);
  border-radius: var(--radius-sm);
}

.compression-bubble__tool-status {
  display: flex;
  align-items: center;
}

.compression-bubble__tool-name {
  flex: 1;
  font-size: var(--font-size-sm);
  font-weight: 500;
  color: var(--color-text-primary);
}

.compression-bubble__tool-count {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  background-color: var(--color-bg-tertiary);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}

@media (max-width: 768px) {
  .compression-bubble {
    max-width: 100%;
  }
}

.compression-bubble__no-tools {
  padding: var(--spacing-3);
  font-size: var(--font-size-sm);
  color: var(--color-text-tertiary);
  text-align: center;
}
</style>
