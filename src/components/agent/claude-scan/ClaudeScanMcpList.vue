<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ScannedMcpServer } from '@/stores/skillConfigShared'

defineProps<{
  items: ScannedMcpServer[]
  selectedNames: string[]
}>()

defineEmits<{
  (e: 'toggle-all'): void
  (e: 'toggle-item', name: string): void
}>()

const { t } = useI18n()
</script>

<template>
  <div class="scan-list">
    <div
      v-if="items.length === 0"
      class="scan-list__empty"
    >
      {{ t('settings.agent.scan.noMcpFound') }}
    </div>
    <template v-else>
      <div class="scan-list__header">
        <label class="scan-checkbox">
          <input
            type="checkbox"
            :checked="selectedNames.length === items.length"
            @change="$emit('toggle-all')"
          >
          <span>{{ t('settings.agent.scan.serverName') }}</span>
        </label>
        <span class="scan-list__col scan-list__col--small">{{ t('settings.agent.scan.transport') }}</span>
        <span class="scan-list__col scan-list__col--small">{{ t('settings.agent.scan.scope') }}</span>
        <span class="scan-list__col">{{ t('settings.agent.scan.commandOrUrl') }}</span>
      </div>
      <div
        v-for="server in items"
        :key="server.name"
        class="scan-list__item"
        :class="{ 'scan-list__item--selected': selectedNames.includes(server.name) }"
        @click="$emit('toggle-item', server.name)"
      >
        <label class="scan-checkbox">
          <input
            type="checkbox"
            :checked="selectedNames.includes(server.name)"
            @click.stop
            @change="$emit('toggle-item', server.name)"
          >
          <span class="scan-list__item-name">{{ server.name }}</span>
        </label>
        <span class="scan-list__col scan-list__col--small">
          <span
            class="scan-badge"
            :class="`scan-badge--${server.transport}`"
          >
            {{ server.transport.toUpperCase() }}
          </span>
        </span>
        <span class="scan-list__col scan-list__col--small">
          <span
            class="scan-badge scan-badge--scope"
            :class="`scan-badge--${server.scope}`"
          >
            {{ t(`settings.agent.scan.scopeTypes.${server.scope}`) }}
          </span>
        </span>
        <span class="scan-list__col scan-list__item-command">
          <template v-if="server.transport === 'stdio'">
            {{ server.command }}
            <span
              v-if="server.args && server.args.length > 0"
              class="scan-list__item-args"
            >
              {{ server.args.join(' ') }}
            </span>
          </template>
          <template v-else>
            {{ server.url }}
          </template>
        </span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.scan-list {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.scan-list__empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-6);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-sm);
}

.scan-list__header {
  display: flex;
  align-items: center;
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-background);
  border-bottom: 1px solid var(--color-border);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
}

.scan-list__item {
  display: flex;
  align-items: center;
  padding: var(--spacing-3);
  border-bottom: 1px solid var(--color-border);
  cursor: pointer;
  transition: background-color var(--transition-fast) var(--easing-default);
}

.scan-list__item:last-child {
  border-bottom: none;
}

.scan-list__item:hover {
  background-color: var(--color-hover);
}

.scan-list__item--selected {
  background-color: rgba(var(--color-primary-rgb), 0.1);
}

.scan-list__col {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.scan-list__col--small {
  flex: 0 0 80px;
  text-align: center;
}

.scan-checkbox {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  cursor: pointer;
  min-width: 0;
  flex-shrink: 0;
  width: 40%;
}

.scan-checkbox input[type='checkbox'] {
  width: 16px;
  height: 16px;
  margin: 0;
  cursor: pointer;
  accent-color: var(--color-primary);
}

.scan-list__item-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.scan-list__item-command {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.scan-list__item-args {
  color: var(--color-text-tertiary);
}

.scan-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-1) var(--spacing-2);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  min-width: 50px;
}

.scan-badge--stdio {
  background-color: rgba(59, 130, 246, 0.1);
  color: rgb(59, 130, 246);
}

.scan-badge--sse {
  background-color: rgba(168, 85, 247, 0.1);
  color: rgb(168, 85, 247);
}

.scan-badge--http {
  background-color: rgba(34, 197, 94, 0.1);
  color: rgb(34, 197, 94);
}

.scan-badge--scope {
  background-color: rgba(100, 116, 139, 0.1);
  color: rgb(100, 116, 139);
}

.scan-badge--user {
  background-color: rgba(59, 130, 246, 0.1);
  color: rgb(59, 130, 246);
}

.scan-badge--local {
  background-color: rgba(251, 146, 60, 0.1);
  color: rgb(251, 146, 60);
}

.scan-badge--project {
  background-color: rgba(34, 197, 94, 0.1);
  color: rgb(34, 197, 94);
}
</style>
