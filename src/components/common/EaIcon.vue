<script setup lang="ts">
/**
 * EaIcon - 图标组件
 * 基于 lucide-vue-next 封装
 */
import { computed } from 'vue'
import * as LucideIcons from 'lucide-vue-next'

export interface EaIconProps {
  name: string
  size?: number | string
  color?: string
  strokeWidth?: number
  spin?: boolean
}

const props = withDefaults(defineProps<EaIconProps>(), {
  size: 20,
  strokeWidth: 2,
  spin: false
})

// 动态获取图标组件
const iconComponent = computed(() => {
  // 移除 lucide: 前缀（如果存在）
  let iconName = props.name
  if (iconName.startsWith('lucide:')) {
    iconName = iconName.slice(7)
  }

  // 转换图标名称：kebab-case -> PascalCase
  const pascalCase = iconName
    .split('-')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join('')

  const icons = LucideIcons as Record<string, unknown>
  return icons[pascalCase] || icons[`${pascalCase}Icon`] || null
})

const iconStyle = computed(() => ({
  width: typeof props.size === 'number' ? `${props.size}px` : props.size,
  height: typeof props.size === 'number' ? `${props.size}px` : props.size,
  color: props.color || 'currentColor'
}))
</script>

<template>
  <component
    :is="iconComponent"
    v-if="iconComponent"
    :size="typeof size === 'number' ? size : parseInt(size as string)"
    :stroke-width="strokeWidth"
    :style="iconStyle"
    :class="['ea-icon', { 'ea-icon--spin': spin }]"
  />
  <span
    v-else
    class="ea-icon ea-icon--missing"
  >
    ?
  </span>
</template>

<style scoped>
.ea-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.ea-icon--spin {
  animation: ea-icon-spin 1s linear infinite;
}

@keyframes ea-icon-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.ea-icon--missing {
  width: 1em;
  height: 1em;
  font-size: 12px;
  color: var(--color-error);
  background-color: var(--color-error-light);
  border-radius: var(--radius-sm);
}
</style>
