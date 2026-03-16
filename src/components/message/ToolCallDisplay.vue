<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ToolCall } from '@/stores/message'

const props = defineProps<{ toolCall: ToolCall }>()
const { t } = useI18n()

// 折叠状态 - 默认展开工具调用，收起结果
const isExpanded = ref(true)
const isResultExpanded = ref(false)

// 切换展开状态
const toggleExpand = () => {
  isExpanded.value = !isExpanded.value
}

const toggleResultExpand = () => {
  isResultExpanded.value = !isResultExpanded.value
}

// 状态样式
const statusClass = computed(() => {
  switch (props.toolCall.status) {
    case 'running':
      return 'tool-call--running'
    case 'success':
      return 'tool-call--success'
    case 'error':
      return 'tool-call--error'
    default:
      return ''
  }
})

// 状态图标
const statusIcon = computed(() => {
  switch (props.toolCall.status) {
    case 'running':
      return '⏳'
    case 'success':
      return '✓'
    case 'error':
      return '✗'
    default:
      return '○'
  }
})

// 工具图标
const toolIcon = computed(() => {
  const name = props.toolCall.name.toLowerCase()
  if (name.includes('web') || name.includes('search')) return '🌐'
  if (name.includes('read') || name.includes('file')) return '📄'
  if (name.includes('write') || name.includes('edit')) return '✏️'
  if (name.includes('bash') || name.includes('shell')) return '💻'
  if (name.includes('grep') || name.includes('search')) return '🔍'
  return '🔧'
})

// 格式化参数
const formattedArguments = computed(() => {
  return JSON.stringify(props.toolCall.arguments, null, 2)
})

// 格式化结果（截取前500字符用于预览）
const resultPreview = computed(() => {
  if (!props.toolCall.result) return ''
  const result = props.toolCall.result
  if (result.length > 500) {
    return result.substring(0, 500) + '...'
  }
  return result
})
</script>

<template>
  <div
    class="tool-call"
    :class="statusClass"
  >
    <!-- 工具调用头部 -->
    <div
      class="tool-call__header"
      @click="toggleExpand"
    >
      <div class="tool-call__header-left">
        <span class="tool-call__icon">{{ toolIcon }}</span>
        <span class="tool-call__name">{{ toolCall.name }}</span>
        <span
          class="tool-call__status"
          :class="`tool-call__status--${toolCall.status}`"
        >
          {{ statusIcon }}
        </span>
      </div>
      <div class="tool-call__header-right">
        <span class="tool-call__toggle">
          {{ isExpanded ? t('message.collapse') : t('message.expand') }}
        </span>
        <span
          class="tool-call__chevron"
          :class="{ 'tool-call__chevron--expanded': isExpanded }"
        >▼</span>
      </div>
    </div>

    <!-- 工具调用内容 -->
    <div
      v-show="isExpanded"
      class="tool-call__content"
    >
      <!-- 参数 -->
      <div class="tool-call__section">
        <div class="tool-call__section-title">
          <span>📥</span>
          <span>{{ t('message.parameters') }}</span>
        </div>
        <pre class="tool-call__code">{{ formattedArguments }}</pre>
      </div>

      <!-- 结果 -->
      <div
        v-if="toolCall.result"
        class="tool-call__section"
      >
        <div
          class="tool-call__section-header"
          @click="toggleResultExpand"
        >
          <div class="tool-call__section-title">
            <span>📤</span>
            <span>{{ t('message.result') }}</span>
          </div>
          <div class="tool-call__section-toggle">
            <span>{{ isResultExpanded ? t('message.collapse') : t('message.expand') }}</span>
            <span
              class="tool-call__chevron"
              :class="{ 'tool-call__chevron--expanded': isResultExpanded }"
            >▼</span>
          </div>
        </div>
        <div
          v-show="isResultExpanded"
          class="tool-call__result"
        >
          <pre class="tool-call__code tool-call__result-content">{{ toolCall.result }}</pre>
        </div>
        <div
          v-show="!isResultExpanded"
          class="tool-call__result-preview"
        >
          <pre class="tool-call__code">{{ resultPreview }}</pre>
        </div>
      </div>

      <!-- 错误信息 -->
      <div
        v-if="toolCall.errorMessage"
        class="tool-call__section tool-call__error-section"
      >
        <div class="tool-call__section-title">
          <span>⚠️</span>
          <span>{{ t('message.error') }}</span>
        </div>
        <div class="tool-call__error">
          {{ toolCall.errorMessage }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tool-call {
  align-self: flex-start;
  width: var(--timeline-panel-width, min(100%, 29.5rem));
  max-width: 100%;
  border-radius: var(--radius-lg);
  background: linear-gradient(135deg, rgba(251, 146, 60, 0.1), rgba(251, 146, 60, 0.05));
  border: 1px solid rgba(251, 146, 60, 0.3);
  overflow: hidden;
  transition: all 0.3s ease;
}

.tool-call:hover {
  border-color: rgba(251, 146, 60, 0.5);
}

.tool-call--running {
  border-color: rgba(59, 130, 246, 0.5);
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.1), rgba(59, 130, 246, 0.05));
}

.tool-call--success {
  border-color: rgba(34, 197, 94, 0.4);
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.1), rgba(34, 197, 94, 0.05));
}

.tool-call--error {
  border-color: rgba(239, 68, 68, 0.5);
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.1), rgba(239, 68, 68, 0.05));
}

.tool-call__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-2) var(--spacing-3);
  cursor: pointer;
  user-select: none;
  transition: background 0.2s ease;
}

.tool-call__header:hover {
  background: rgba(251, 146, 60, 0.15);
}

.tool-call--running .tool-call__header:hover {
  background: rgba(59, 130, 246, 0.15);
}

.tool-call--success .tool-call__header:hover {
  background: rgba(34, 197, 94, 0.15);
}

.tool-call--error .tool-call__header:hover {
  background: rgba(239, 68, 68, 0.15);
}

.tool-call__header-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
}

.tool-call__icon {
  font-size: 14px;
  line-height: 1;
}

.tool-call__name {
  font-size: var(--font-size-sm);
  font-weight: 500;
  color: var(--color-text-primary);
}

.tool-call__status {
  font-size: 12px;
  line-height: 1;
}

.tool-call__status--running {
  color: var(--color-primary);
  animation: spin 1s linear infinite;
}

.tool-call__status--success {
  color: var(--color-success);
}

.tool-call__status--error {
  color: var(--color-error);
}

.tool-call__header-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  flex-shrink: 0;
}

.tool-call__toggle {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.tool-call__chevron {
  font-size: 10px;
  color: var(--color-text-tertiary);
  transition: transform 0.2s ease;
}

.tool-call__chevron--expanded {
  transform: rotate(180deg);
}

.tool-call__content {
  border-top: 1px solid rgba(251, 146, 60, 0.15);
  padding: var(--spacing-2) var(--spacing-3);
}

.tool-call--running .tool-call__content {
  border-top-color: rgba(59, 130, 246, 0.15);
}

.tool-call--success .tool-call__content {
  border-top-color: rgba(34, 197, 94, 0.15);
}

.tool-call--error .tool-call__content {
  border-top-color: rgba(239, 68, 68, 0.15);
}

.tool-call__section {
  margin-bottom: var(--spacing-2);
}

.tool-call__section:last-child {
  margin-bottom: 0;
}

.tool-call__section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  user-select: none;
  padding: var(--spacing-1) 0;
  transition: background 0.2s ease;
  border-radius: var(--radius-sm);
  margin: calc(var(--spacing-1) * -1) calc(var(--spacing-2) * -1);
  padding-left: var(--spacing-2);
  padding-right: var(--spacing-2);
}

.tool-call__section-header:hover {
  background: rgba(251, 146, 60, 0.1);
}

.tool-call__section-title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-xs);
  font-weight: 500;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.tool-call__section-toggle {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.tool-call__code {
  margin: var(--spacing-1) 0 0 0;
  padding: var(--spacing-2);
  background: rgba(0, 0, 0, 0.05);
  border-radius: var(--radius-md);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-x: auto;
  color: var(--color-text-primary);
}

:global([data-theme='dark']) .tool-call__code,
:global(.dark) .tool-call__code {
  background: rgba(0, 0, 0, 0.2);
}

.tool-call__result {
  margin-top: var(--spacing-1);
}

.tool-call__result-content {
  max-height: 400px;
  overflow-y: auto;
}

.tool-call__result-preview {
  margin-top: var(--spacing-1);
}

.tool-call__error-section {
  margin-top: var(--spacing-2);
  padding: var(--spacing-2);
  background: rgba(239, 68, 68, 0.1);
  border-radius: var(--radius-md);
  border-left: 3px solid var(--color-error);
}

.tool-call__error {
  font-size: var(--font-size-xs);
  color: var(--color-error);
  line-height: 1.5;
}

/* 暗色模式适配 */
:global([data-theme='dark']) .tool-call,
:global(.dark) .tool-call {
  background: linear-gradient(135deg, rgba(251, 146, 60, 0.08), rgba(251, 146, 60, 0.03));
  border-color: rgba(251, 146, 60, 0.25);
}

:global([data-theme='dark']) .tool-call--running,
:global(.dark) .tool-call--running {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.08), rgba(59, 130, 246, 0.03));
  border-color: rgba(59, 130, 246, 0.4);
}

:global([data-theme='dark']) .tool-call--success,
:global(.dark) .tool-call--success {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.08), rgba(34, 197, 94, 0.03));
  border-color: rgba(34, 197, 94, 0.3);
}

:global([data-theme='dark']) .tool-call--error,
:global(.dark) .tool-call--error {
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.08), rgba(239, 68, 68, 0.03));
  border-color: rgba(239, 68, 68, 0.4);
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
