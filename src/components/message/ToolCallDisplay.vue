<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ToolCall } from '@/stores/message'
import { useTypewriterText } from '@/composables/useTypewriterText'

const props = withDefaults(defineProps<{
  toolCall: ToolCall
  live?: boolean
  compact?: boolean
  defaultExpanded?: boolean
  defaultResultExpanded?: boolean
}>(), {
  live: false,
  compact: false,
  defaultExpanded: undefined,
  defaultResultExpanded: undefined
})
const { t } = useI18n()

const isExpanded = ref(props.defaultExpanded ?? false)
const isResultExpanded = ref(props.defaultResultExpanded ?? false)

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

const isTerminalLikeTool = computed(() => {
  const name = props.toolCall.name.toLowerCase()
  return name.includes('bash')
    || name.includes('shell')
    || name.includes('terminal')
    || name.includes('command')
})

// 格式化参数
const formattedArguments = computed(() => {
  return JSON.stringify(props.toolCall.arguments, null, 2)
})

const { displayedText: animatedArguments } = useTypewriterText(
  formattedArguments,
  computed(() => props.live),
  { charsPerSecond: 120, maxChunkSize: 18 }
)

const { displayedText: animatedResult } = useTypewriterText(
  computed(() => props.toolCall.result || ''),
  computed(() => props.live),
  { charsPerSecond: 120, maxChunkSize: 18 }
)

const toolSummary = computed(() => {
  const command = props.toolCall.arguments?.command
  if (typeof command === 'string' && command.trim()) {
    const normalized = command.trim().replace(/\s+/g, ' ')
    return normalized.length > 56 ? `${normalized.slice(0, 56)}...` : normalized
  }

  const description = props.toolCall.arguments?.description
  if (typeof description === 'string' && description.trim()) {
    const normalized = description.trim().replace(/\s+/g, ' ')
    return normalized.length > 56 ? `${normalized.slice(0, 56)}...` : normalized
  }

  const firstArgument = Object.entries(props.toolCall.arguments ?? {})[0]
  if (firstArgument) {
    const [, value] = firstArgument
    const preview = (typeof value === 'string' ? value : JSON.stringify(value))
      .replace(/\s+/g, ' ')
      .trim()
    return preview.length > 56 ? `${preview.slice(0, 56)}...` : preview
  }

  return props.toolCall.status === 'running' ? '等待工具结果...' : '查看参数与结果'
})
</script>

<template>
  <div
    class="tool-call"
    :class="[statusClass, { 'tool-call--compact': compact }]"
  >
    <!-- 工具调用头部 -->
    <button
      type="button"
      class="tool-call__header"
      :aria-expanded="isExpanded"
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
        <span class="tool-call__summary">{{ toolSummary }}</span>
        <span class="tool-call__toggle">
          {{ isExpanded ? t('message.collapse') : t('message.expand') }}
        </span>
        <span
          class="tool-call__chevron"
          :class="{ 'tool-call__chevron--expanded': isExpanded }"
        >▼</span>
      </div>
    </button>

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
        <pre class="tool-call__code">{{ animatedArguments }}</pre>
      </div>

      <!-- 结果 -->
      <div
        v-if="toolCall.result"
        class="tool-call__section"
      >
        <button
          type="button"
          class="tool-call__section-header"
          @click.stop="toggleResultExpand"
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
        </button>
        <div
          v-show="isResultExpanded"
          class="tool-call__result"
        >
          <pre
            class="tool-call__code tool-call__result-content"
            :class="{ 'tool-call__code--terminal': isTerminalLikeTool }"
          >{{ animatedResult }}</pre>
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
  flex: 0 0 auto;
  width: min(100%, var(--thinking-display-width, var(--timeline-entry-width, clamp(18rem, 40%, 34rem))));
  min-width: 0;
  max-width: 100%;
  box-sizing: border-box;
  border-radius: var(--radius-lg);
  background: var(--tool-call-bg);
  border: 1px solid var(--tool-call-border);
  box-shadow: var(--tool-call-shadow);
  overflow: hidden;
  transition: border-color 0.25s ease, box-shadow 0.25s ease, background 0.25s ease;
}

.tool-call--compact {
  width: min(100%, var(--thinking-display-width, var(--timeline-entry-width, clamp(18rem, 40%, 34rem))));
}

.tool-call:hover {
  border-color: var(--tool-call-hover-border);
  box-shadow: var(--tool-call-hover-shadow);
}

.tool-call--running {
  border-color: var(--tool-call-running-border);
  background: var(--tool-call-running-bg);
}

.tool-call--success {
  border-color: var(--tool-call-success-border);
  background: var(--tool-call-success-bg);
}

.tool-call--error {
  border-color: var(--tool-call-error-border);
  background: var(--tool-call-error-bg);
}

.tool-call__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
  width: 100%;
  padding: var(--spacing-2) var(--spacing-3);
  border: 0;
  background: transparent;
  cursor: pointer;
  user-select: none;
  text-align: left;
  transition: background 0.2s ease;
}

.tool-call__header:hover {
  background: var(--tool-call-header-hover);
}

.tool-call--running .tool-call__header:hover {
  background: var(--tool-call-running-hover);
}

.tool-call--success .tool-call__header:hover {
  background: var(--tool-call-success-hover);
}

.tool-call--error .tool-call__header:hover {
  background: var(--tool-call-error-hover);
}

.tool-call__header-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
  flex: 1 1 auto;
}

.tool-call__icon {
  font-size: 14px;
  line-height: 1;
}

.tool-call__name {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--tool-call-name);
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tool-call__status {
  font-size: 12px;
  line-height: 1;
}

.tool-call__status--running {
  color: var(--tool-call-running-status);
  animation: spin 1s linear infinite;
}

.tool-call__status--success {
  color: var(--tool-call-success-status);
}

.tool-call__status--error {
  color: var(--tool-call-error-status);
}

.tool-call__header-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  min-width: 0;
  flex: 0 1 auto;
  max-width: min(70%, 24rem);
}

.tool-call__summary {
  flex: 1 1 auto;
  min-width: 0;
  max-width: min(18rem, 42vw);
  font-size: 0.72rem;
  color: var(--tool-call-summary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  text-align: right;
}

.tool-call__toggle {
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  color: var(--tool-call-meta);
  white-space: nowrap;
}

.tool-call__chevron {
  font-size: 10px;
  color: var(--tool-call-meta);
  transition: transform 0.2s ease;
}

.tool-call__chevron--expanded {
  transform: rotate(180deg);
}

.tool-call__content {
  width: 100%;
  box-sizing: border-box;
  border-top: 1px solid var(--tool-call-content-border);
  padding: var(--spacing-2) var(--spacing-3);
  overflow: visible;
}

.tool-call--running .tool-call__content {
  border-top-color: var(--tool-call-running-content-border);
}

.tool-call--success .tool-call__content {
  border-top-color: var(--tool-call-success-content-border);
}

.tool-call--error .tool-call__content {
  border-top-color: var(--tool-call-error-content-border);
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
  width: 100%;
  cursor: pointer;
  user-select: none;
  border: 0;
  background: transparent;
  text-align: left;
  padding: var(--spacing-1) 0;
  transition: background 0.2s ease;
  border-radius: var(--radius-sm);
  margin: calc(var(--spacing-1) * -1) calc(var(--spacing-2) * -1);
  padding-left: var(--spacing-2);
  padding-right: var(--spacing-2);
}

.tool-call__section-header:hover {
  background: var(--tool-call-section-hover);
}

.tool-call__section-title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-xs);
  font-weight: 500;
  color: var(--tool-call-meta);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.tool-call__section-toggle {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--tool-call-meta);
}

.tool-call__code {
  margin: var(--spacing-1) 0 0 0;
  padding: var(--spacing-2);
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  background: var(--tool-call-code-bg);
  border-radius: var(--radius-md);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  line-height: 1.5;
  white-space: pre-wrap;
  overflow-wrap: break-word;
  word-break: break-word;
  overflow-x: auto;
  color: var(--color-text-primary);
}

.tool-call__code--terminal {
  white-space: pre;
  overflow-wrap: normal;
  word-break: normal;
}

.tool-call__result {
  margin-top: var(--spacing-1);
}

.tool-call__result-content {
  max-height: min(18rem, 42vh);
  overflow: auto;
  scrollbar-gutter: stable;
}

.tool-call__error-section {
  margin-top: var(--spacing-2);
  padding: var(--spacing-2);
  background: var(--tool-call-error-panel-bg);
  border-radius: var(--radius-md);
  border-left: 3px solid var(--color-error);
}

.tool-call__error {
  font-size: var(--font-size-xs);
  color: var(--tool-call-error-text);
  line-height: 1.5;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
