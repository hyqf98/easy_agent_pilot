<script setup lang="ts">
/**
 * TokenProgressBar - Token 使用进度条组件
 * 显示当前会话 token 使用情况，点击可打开压缩弹框
 */
import { computed, ref, watch } from 'vue'
import { useTokenStore, type TokenLevel, formatTokenCount } from '@/stores/token'
import { useSessionStore } from '@/stores/session'

const props = withDefaults(defineProps<{
  sessionId?: string | null
  showCompressButton?: boolean
}>(), {
  sessionId: null,
  showCompressButton: true
})

const emit = defineEmits<{
  (e: 'compress'): void
}>()

const tokenStore = useTokenStore()
const sessionStore = useSessionStore()

// 是否显示 tooltip
const showTooltip = ref(false)
// tooltip 位置
const tooltipPosition = ref({ top: 0, left: 0 })
const targetSessionId = computed(() => props.sessionId ?? sessionStore.currentSessionId ?? null)

// 获取当前会话的 token 使用情况
const tokenUsage = computed(() => {
  if (!targetSessionId.value) {
    return { used: 0, limit: 128000, percentage: 0, level: 'safe' as TokenLevel }
  }
  return tokenStore.getTokenUsage(targetSessionId.value)
})

const displayPercentage = computed(() => `${Math.round(tokenUsage.value.percentage)}%`)

watch(
  targetSessionId,
  () => {
    // Token usage is now driven entirely by CLI response data.
    // No local estimation or cache refresh needed on mount.
  },
  { immediate: true }
)

// 进度条样式
const progressStyle = computed(() => ({
  width: tokenUsage.value.used > 0 && tokenUsage.value.percentage > 0 && tokenUsage.value.percentage < 1
    ? '1%'
    : `${Math.min(100, tokenUsage.value.percentage)}%`
}))

// 进度条级别类
const levelClass = computed(() => `token-progress--${tokenUsage.value.level}`)

// tooltip 样式
const tooltipStyle = computed(() => ({
  top: `${tooltipPosition.value.top}px`,
  left: `${tooltipPosition.value.left}px`,
  transform: 'translateX(-50%)'
}))

// 点击整个进度条触发压缩
function handleClick() {
  emit('compress')
}

// 鼠标进入时计算 tooltip 位置
function handleMouseEnter(event: MouseEvent) {
  const target = event.currentTarget as HTMLElement
  const rect = target.getBoundingClientRect()
  tooltipPosition.value = {
    top: rect.bottom + 8,
    left: rect.left + rect.width / 2
  }
  showTooltip.value = true
}

function handleMouseLeave() {
  showTooltip.value = false
}
</script>

<template>
  <div
    class="token-progress"
    :class="levelClass"
    @click="handleClick"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
  >
    <!-- 进度条 -->
    <div class="token-progress__bar">
      <div
        class="token-progress__fill"
        :style="progressStyle"
      />
    </div>

    <!-- 百分比显示 -->
    <span class="token-progress__text">
      {{ displayPercentage }}
    </span>

    <!-- Tooltip - 使用 Teleport 渲染到 body -->
    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="showTooltip"
          class="token-progress__tooltip"
          :style="tooltipStyle"
        >
          <div class="token-progress__tooltip-row">
            <span class="token-progress__tooltip-label">Token 用量</span>
            <span class="token-progress__tooltip-value">
              {{ formatTokenCount(tokenUsage.used) }} / {{ formatTokenCount(tokenUsage.limit) }}
            </span>
          </div>
          <div class="token-progress__tooltip-hint">
            点击压缩上下文
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.token-progress {
  display: flex;
  align-items: center;
  gap: 10px;
  position: relative;
  padding: 6px 14px;
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.85);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(0, 0, 0, 0.06);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  cursor: pointer;
  user-select: none;
  transition: all var(--transition-fast);
}

.token-progress:hover {
  background: rgba(255, 255, 255, 0.92);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  transform: translateY(-1px);
}

.token-progress:active {
  transform: translateY(0);
  opacity: 0.9;
}

[data-theme='dark'] .token-progress {
  background: rgba(30, 41, 59, 0.85);
  border-color: rgba(255, 255, 255, 0.08);
}

[data-theme='dark'] .token-progress:hover {
  background: rgba(30, 41, 59, 0.92);
}

.token-progress__bar {
  flex: 1;
  height: 6px;
  background-color: var(--color-bg-tertiary);
  border-radius: 3px;
  overflow: hidden;
  min-width: 120px;
}

.token-progress__fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.4s ease;
}

/* 级别颜色 */
.token-progress--safe .token-progress__fill {
  background: var(--color-primary);
}

.token-progress--warning .token-progress__fill {
  background: var(--color-warning);
}

.token-progress--danger .token-progress__fill {
  background: var(--color-orange-500, #f97316);
}

.token-progress--critical .token-progress__fill {
  background: var(--color-error);
  animation: pulse-fill 0.8s ease-in-out infinite;
}

@keyframes pulse-fill {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}

.token-progress__text {
  font-size: 12px;
  font-weight: 600;
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
}

.token-progress__tooltip {
  position: fixed;
  padding: 6px 10px;
  background-color: var(--color-surface-elevated);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 11px;
  color: var(--color-text-primary);
  white-space: nowrap;
  z-index: 9999;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
  pointer-events: none;
}

.token-progress__tooltip-row {
  display: flex;
  justify-content: space-between;
  gap: 10px;
}

.token-progress__tooltip-label {
  color: var(--color-text-secondary);
  font-size: 10px;
}

.token-progress__tooltip-value {
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  font-size: 11px;
}

.token-progress__tooltip-hint {
  margin-top: 4px;
  padding-top: 3px;
  border-top: 1px solid var(--color-border-light);
  color: var(--color-primary);
  font-size: 9px;
  text-align: center;
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
