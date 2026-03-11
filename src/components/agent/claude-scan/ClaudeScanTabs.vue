<script setup lang="ts">
import type { ClaudeScanTab } from './shared'

defineProps<{
  activeTab: ClaudeScanTab
  counts: Record<ClaudeScanTab, number>
}>()

defineEmits<{
  (e: 'update:activeTab', value: ClaudeScanTab): void
}>()
</script>

<template>
  <div class="scan-modal-tabs">
    <button
      class="scan-tab"
      :class="{ 'scan-tab--active': activeTab === 'mcp' }"
      @click="$emit('update:activeTab', 'mcp')"
    >
      <span>MCP</span>
      <span
        v-if="counts.mcp > 0"
        class="scan-tab__count"
      >
        {{ counts.mcp }}
      </span>
    </button>
    <button
      class="scan-tab"
      :class="{ 'scan-tab--active': activeTab === 'skills' }"
      @click="$emit('update:activeTab', 'skills')"
    >
      <span>Skills</span>
      <span
        v-if="counts.skills > 0"
        class="scan-tab__count"
      >
        {{ counts.skills }}
      </span>
    </button>
    <button
      class="scan-tab"
      :class="{ 'scan-tab--active': activeTab === 'plugins' }"
      @click="$emit('update:activeTab', 'plugins')"
    >
      <span>Plugins</span>
      <span
        v-if="counts.plugins > 0"
        class="scan-tab__count"
      >
        {{ counts.plugins }}
      </span>
    </button>
  </div>
</template>

<style scoped>
.scan-modal-tabs {
  display: flex;
  gap: var(--spacing-2);
  margin-bottom: var(--spacing-4);
}

.scan-tab {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.scan-tab:hover {
  background-color: var(--color-hover);
  color: var(--color-text-primary);
}

.scan-tab--active {
  background-color: var(--color-primary);
  border-color: var(--color-primary);
  color: white;
}

.scan-tab__count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 20px;
  padding: 0 var(--spacing-1);
  background-color: rgba(255, 255, 255, 0.2);
  border-radius: 10px;
  font-size: var(--font-size-xs);
}
</style>
