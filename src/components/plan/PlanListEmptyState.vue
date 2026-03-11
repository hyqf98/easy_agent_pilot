<script setup lang="ts">
defineProps<{
  title: string
}>()
</script>

<template>
  <div class="empty-state">
    <div class="empty-icon">
      📝
    </div>
    <p class="empty-title">
      {{ title }}
    </p>
    <p class="hint">
      点击上方"新建"按钮创建计划
    </p>
  </div>
</template>

<style scoped>
.empty-state {
  padding: var(--spacing-8, 2rem) var(--spacing-4, 1rem);
  color: var(--color-text-secondary, #64748b);
  text-align: center;
}

.empty-icon {
  margin-bottom: var(--spacing-3, 0.75rem);
  opacity: 0.6;
  font-size: 2.5rem;
}

.empty-title {
  margin: 0 0 var(--spacing-2, 0.5rem);
  color: var(--color-text-primary, #1e293b);
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
}

.hint {
  margin: 0;
  color: var(--color-text-tertiary, #94a3b8);
  font-size: var(--font-size-xs, 12px);
}
</style>
