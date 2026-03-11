<script setup lang="ts">
/**
 * TokenProgressBar - Token 使用进度条组件
 * 显示当前会话 token 使用情况，支持压缩功能
 */
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTokenStore, type TokenLevel, formatTokenCount } from '@/stores/token'
import { useSessionStore } from '@/stores/session'
import EaButton from './EaButton.vue'
import EaIcon from './EaIcon.vue'

export interface TokenProgressBarProps {
  /** 是否显示压缩按钮 */
  showCompressButton?: boolean
}

const props = withDefaults(defineProps<TokenProgressBarProps>(), {
  showCompressButton: true
})

const emit = defineEmits<{
  (e: 'compress'): void
}>()

const tokenStore = useTokenStore()
const sessionStore = useSessionStore()
const { t } = useI18n()

// 是否显示 tooltip
const showTooltip = ref(false)

// 获取当前会话的 token 使用情况
const tokenUsage = computed(() => {
  if (!sessionStore.currentSessionId) {
    return { used: 0, limit: 128000, percentage: 0, level: 'safe' as TokenLevel }
  }
  return tokenStore.getTokenUsage(sessionStore.currentSessionId)
})

// 是否显示压缩按钮 - 始终显示，允许用户随时手动压缩
const shouldShowCompressButton = computed(() => {
  return props.showCompressButton
})

// 进度条样式
const progressStyle = computed(() => ({
  width: `${Math.min(100, tokenUsage.value.percentage)}%`
}))

// 进度条级别类
const levelClass = computed(() => `token-progress--${tokenUsage.value.level}`)

// tooltip 文本
const tooltipText = computed(() => {
  const { used, limit } = tokenUsage.value
  return t('token.usageTooltip', {
    used: formatTokenCount(used),
    limit: formatTokenCount(limit)
  })
})

// 处理压缩按钮点击
function handleCompress() {
  emit('compress')
}
</script>

<template>
  <div
    class="token-progress"
    :class="levelClass"
    @mouseenter="showTooltip = true"
    @mouseleave="showTooltip = false"
  >
    <!-- 图标 -->
    <EaIcon
      name="activity"
      :size="14"
      class="token-progress__icon"
    />

    <!-- 进度条 -->
    <div class="token-progress__bar">
      <div
        class="token-progress__fill"
        :style="progressStyle"
      >
        <div class="token-progress__shimmer" />
      </div>
    </div>

    <!-- 百分比显示 -->
    <span class="token-progress__text">
      {{ Math.round(tokenUsage.percentage) }}%
    </span>

    <!-- 压缩按钮 -->
    <EaButton
      v-if="shouldShowCompressButton"
      type="ghost"
      size="small"
      class="token-progress__compress"
      :title="t('token.compressTooltip')"
      @click="handleCompress"
    >
      <EaIcon name="minimize-2" />
    </EaButton>

    <!-- Tooltip -->
    <Transition name="fade">
      <div
        v-if="showTooltip"
        class="token-progress__tooltip"
      >
        {{ tooltipText }}
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.token-progress {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  position: relative;
  padding: var(--spacing-1) var(--spacing-3);
  border-radius: var(--radius-lg);
  background-color: var(--color-surface-hover);
  border: 1px solid var(--color-border);
  cursor: default;
  min-width: 200px;
  transition: all var(--transition-fast);
}

.token-progress:hover {
  border-color: var(--color-primary-light);
  box-shadow: 0 0 8px rgba(var(--color-primary-rgb, 59, 130, 246), 0.15);
}

.token-progress__icon {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

/* 不同级别的图标颜色 */
.token-progress--safe .token-progress__icon {
  color: var(--color-primary);
}

.token-progress--warning .token-progress__icon {
  color: var(--color-warning);
}

.token-progress--danger .token-progress__icon {
  color: var(--color-orange-500, #f97316);
}

.token-progress--critical .token-progress__icon {
  color: var(--color-error);
  animation: pulse-icon 1s ease-in-out infinite;
}

@keyframes pulse-icon {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.6;
    transform: scale(0.9);
  }
}

.token-progress__bar {
  flex: 1;
  height: 8px;
  background-color: var(--color-bg-tertiary);
  border-radius: var(--radius-full);
  overflow: hidden;
  min-width: 120px;
  position: relative;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.1);
}

.token-progress__fill {
  height: 100%;
  border-radius: var(--radius-full);
  transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  overflow: hidden;
}

/* 流光动画 */
.token-progress__shimmer {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.3) 50%,
    transparent 100%
  );
  animation: shimmer 2s infinite;
}

@keyframes shimmer {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(100%);
  }
}

/* 级别颜色 - 渐变效果 */
.token-progress--safe .token-progress__fill {
  background: linear-gradient(90deg, var(--color-primary), var(--color-primary-light, #60a5fa));
  box-shadow: 0 0 8px var(--color-primary);
}

.token-progress--warning .token-progress__fill {
  background: linear-gradient(90deg, var(--color-warning), #fbbf24);
  box-shadow: 0 0 8px var(--color-warning);
}

.token-progress--danger .token-progress__fill {
  background: linear-gradient(90deg, var(--color-orange-500, #f97316), #fb923c);
  box-shadow: 0 0 10px var(--color-orange-500, #f97316);
}

.token-progress--critical .token-progress__fill {
  background: linear-gradient(90deg, var(--color-error), #f87171);
  box-shadow: 0 0 12px var(--color-error);
  animation: pulse-fill 0.8s ease-in-out infinite;
}

@keyframes pulse-fill {
  0%, 100% {
    opacity: 1;
    filter: brightness(1);
  }
  50% {
    opacity: 0.85;
    filter: brightness(1.2);
  }
}

.token-progress__text {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-secondary);
  min-width: 36px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

/* 不同级别的文字颜色 */
.token-progress--safe .token-progress__text {
  color: var(--color-primary);
}

.token-progress--warning .token-progress__text {
  color: var(--color-warning);
}

.token-progress--danger .token-progress__text {
  color: var(--color-orange-500, #f97316);
}

.token-progress--critical .token-progress__text {
  color: var(--color-error);
  animation: pulse-text 0.8s ease-in-out infinite;
}

@keyframes pulse-text {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}

.token-progress__compress {
  padding: 2px !important;
  min-height: auto !important;
}

.token-progress__tooltip {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
  padding: var(--spacing-1) var(--spacing-2);
  background-color: var(--color-surface-active);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  white-space: nowrap;
  z-index: var(--z-tooltip);
  box-shadow: var(--shadow-md);
}

/* Fade transition */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
