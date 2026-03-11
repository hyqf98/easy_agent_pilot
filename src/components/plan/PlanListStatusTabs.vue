<script setup lang="ts">
import type { PlanTabKey } from './planListShared'

defineProps<{
  tabs: Array<{ key: PlanTabKey, label: string }>
  activeTab: PlanTabKey
  counts: Record<PlanTabKey, number>
}>()

const emit = defineEmits<{
  'update:activeTab': [value: PlanTabKey]
}>()
</script>

<template>
  <div class="status-tabs">
    <button
      v-for="tab in tabs"
      :key="tab.key"
      class="status-tab"
      :class="{ active: activeTab === tab.key }"
      @click="emit('update:activeTab', tab.key)"
    >
      <span class="status-tab-label">{{ tab.label }}</span>
      <span class="status-tab-count">{{ counts[tab.key] }}</span>
    </button>
  </div>
</template>

<style scoped>
.status-tabs {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.375rem;
  margin-bottom: 0.625rem;
}

.status-tab {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.375rem 0.5rem;
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  background-color: color-mix(in srgb, var(--color-surface) 94%, #fff);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.status-tab:hover {
  border-color: color-mix(in srgb, var(--color-primary) 28%, var(--color-border));
  background-color: color-mix(in srgb, var(--color-primary-light) 35%, var(--color-surface));
}

.status-tab.active {
  border-color: var(--color-primary);
  background-color: color-mix(in srgb, var(--color-primary-light) 72%, var(--color-surface));
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-primary) 16%, transparent);
}

.status-tab-label {
  color: var(--color-text-primary, #1e293b);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
}

.status-tab-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.25rem;
  height: 1.125rem;
  padding: 0 0.375rem;
  border-radius: var(--radius-full, 9999px);
  background-color: color-mix(in srgb, var(--color-bg-tertiary) 88%, var(--color-surface));
  color: var(--color-text-secondary);
  font-size: 0.6875rem;
}

.status-tab.active .status-tab-count {
  background-color: color-mix(in srgb, var(--color-primary) 16%, var(--color-surface));
  color: var(--color-primary);
}

[data-theme='dark'] .status-tab {
  border-color: rgba(148, 163, 184, 0.18);
  background-color: rgba(15, 23, 42, 0.42);
}

[data-theme='dark'] .status-tab:hover {
  background-color: rgba(30, 64, 175, 0.18);
}

[data-theme='dark'] .status-tab.active {
  background-color: rgba(30, 64, 175, 0.3);
  box-shadow: 0 0 0 2px rgba(96, 165, 250, 0.16);
}

[data-theme='dark'] .status-tab-count {
  background-color: rgba(15, 23, 42, 0.62);
  color: rgba(226, 232, 240, 0.82);
}
</style>
