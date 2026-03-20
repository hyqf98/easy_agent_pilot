<script setup lang="ts">
import { computed, ref, toRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTypewriterText } from '@/composables/useTypewriterText'

const props = withDefaults(defineProps<{
  thinking: string
  live?: boolean
  defaultExpanded?: boolean
}>(), {
  live: false,
  defaultExpanded: false
})
const { t } = useI18n()

const { displayedText } = useTypewriterText(
  toRef(props, 'thinking'),
  toRef(props, 'live'),
  { charsPerSecond: 120, maxChunkSize: 16 }
)

const isExpanded = ref(props.defaultExpanded)
const placeholderText = computed(() => props.live ? '正在思考...' : '')

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
        <pre class="thinking-display__text">{{ displayedText || placeholderText }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.thinking-display {
  align-self: flex-start;
  width: min(100%, var(--thinking-display-width, var(--timeline-entry-width, clamp(18rem, 40%, 34rem))));
  min-width: 0;
  max-width: 100%;
  box-sizing: border-box;
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
  flex: 1;
}

.thinking-display__header-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
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
  min-width: 0;
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
  width: 100%;
  box-sizing: border-box;
  border-top: 1px solid rgba(14, 165, 233, 0.14);
  max-height: calc(var(--message-compact-max-height, 20rem) - 44px);
  overflow: hidden;
}

.thinking-display__scroll {
  width: 100%;
  max-height: calc(var(--message-compact-max-height, 20rem) - 44px);
  overflow: auto;
  scrollbar-gutter: stable;
  padding: var(--spacing-2) var(--spacing-3);
  box-sizing: border-box;
}

/* 自定义滚动条 */
.thinking-display__scroll::-webkit-scrollbar {
  width: var(--scrollbar-size-thin);
}

.thinking-display__scroll::-webkit-scrollbar-track {
  background: var(--scrollbar-track);
}

.thinking-display__scroll::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: var(--radius-full);
  border: 1px solid transparent;
  background-clip: padding-box;
}

.thinking-display__scroll::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}

.thinking-display__text {
  margin: 0;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  line-height: 1.5;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
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

:global([data-theme='dark']) .thinking-display__text,
:global(.dark) .thinking-display__text {
  color: #cbd5e1;
}
</style>
