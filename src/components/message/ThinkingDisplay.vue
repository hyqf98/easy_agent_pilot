<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

defineProps<{ thinking: string }>()
const { t } = useI18n()

// 折叠状态 - 默认收起
const isExpanded = ref(false)

// 切换展开状态
const toggleExpand = () => {
  isExpanded.value = !isExpanded.value
}
</script>

<template>
  <div class="thinking-display">
    <!-- 思考头部 -->
    <div
      class="thinking-display__header"
      @click="toggleExpand"
    >
      <div class="thinking-display__header-left">
        <span class="thinking-display__icon">💭</span>
        <span class="thinking-display__title">{{ t('message.thinking') }}</span>
        <span class="thinking-display__badge">{{ t('message.thinking') }}</span>
      </div>
      <div class="thinking-display__header-right">
        <span class="thinking-display__toggle">
          {{ isExpanded ? t('message.collapse') : t('message.expand') }}
        </span>
        <span
          class="thinking-display__chevron"
          :class="{ 'thinking-display__chevron--expanded': isExpanded }"
        >▼</span>
      </div>
    </div>

    <!-- 思考内容 - 默认收起 -->
    <div
      v-show="isExpanded"
      class="thinking-display__content"
    >
      <div class="thinking-display__scroll">
        <pre class="thinking-display__text">{{ thinking }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.thinking-display {
  align-self: flex-start;
  width: var(--timeline-panel-width, min(100%, 29.5rem));
  max-width: 100%;
  border-radius: var(--radius-lg);
  background:
    linear-gradient(135deg, rgba(14, 165, 233, 0.12), rgba(34, 197, 94, 0.05)),
    color-mix(in srgb, var(--color-surface) 92%, white 8%);
  border: 1px solid rgba(14, 165, 233, 0.2);
  overflow: hidden;
  box-shadow: 0 10px 24px rgba(14, 165, 233, 0.08);
  transition: border-color 0.25s ease, box-shadow 0.25s ease, background 0.25s ease;
}

.thinking-display:hover {
  border-color: rgba(8, 145, 178, 0.3);
  box-shadow: 0 12px 30px rgba(8, 145, 178, 0.1);
}

.thinking-display__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-2) var(--spacing-3);
  cursor: pointer;
  user-select: none;
  transition: background 0.2s ease;
}

.thinking-display__header:hover {
  background: rgba(14, 165, 233, 0.08);
}

.thinking-display__header-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
}

.thinking-display__header-right {
  flex-shrink: 0;
}

.thinking-display__icon {
  font-size: 14px;
  line-height: 1;
}

.thinking-display__title {
  font-size: var(--font-size-sm);
  font-weight: 500;
  color: var(--color-text-primary);
}

.thinking-display__badge {
  font-size: 10px;
  padding: 1px 6px;
  background: linear-gradient(135deg, #0891b2, #0f766e);
  color: white;
  border-radius: var(--radius-sm);
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.thinking-display__toggle {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.thinking-display__chevron {
  font-size: 10px;
  color: var(--color-text-tertiary);
  transition: transform 0.2s ease;
}

.thinking-display__chevron--expanded {
  transform: rotate(180deg);
}

.thinking-display__content {
  border-top: 1px solid rgba(14, 165, 233, 0.14);
}

.thinking-display__scroll {
  max-height: calc(1.5em * 6 + var(--spacing-2) * 2); /* 6行高度 */
  overflow-y: auto;
  padding: var(--spacing-2) var(--spacing-3);
}

/* 自定义滚动条 */
.thinking-display__scroll::-webkit-scrollbar {
  width: 4px;
}

.thinking-display__scroll::-webkit-scrollbar-track {
  background: transparent;
}

.thinking-display__scroll::-webkit-scrollbar-thumb {
  background: rgba(8, 145, 178, 0.24);
  border-radius: 2px;
}

.thinking-display__scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(8, 145, 178, 0.36);
}

.thinking-display__text {
  margin: 0;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--color-text-secondary);
}

/* 暗色模式适配 */
:global([data-theme='dark']) .thinking-display,
:global(.dark) .thinking-display {
  background:
    linear-gradient(135deg, rgba(8, 145, 178, 0.18), rgba(6, 95, 70, 0.08)),
    rgba(15, 23, 42, 0.78);
  border-color: rgba(34, 211, 238, 0.18);
  box-shadow: 0 14px 30px rgba(2, 6, 23, 0.26);
}

:global([data-theme='dark']) .thinking-display__header:hover,
:global(.dark) .thinking-display__header:hover {
  background: rgba(8, 145, 178, 0.16);
}

:global([data-theme='dark']) .thinking-display__content,
:global(.dark) .thinking-display__content {
  border-top-color: rgba(34, 211, 238, 0.14);
}

:global([data-theme='dark']) .thinking-display__badge,
:global(.dark) .thinking-display__badge {
  background: linear-gradient(135deg, #06b6d4, #0f766e);
}

:global([data-theme='dark']) .thinking-display__text,
:global(.dark) .thinking-display__text {
  color: #cbd5e1;
}
</style>
