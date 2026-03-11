<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ScannedPlugin } from '@/stores/skillConfigShared'

defineProps<{
  items: ScannedPlugin[]
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
      {{ t('settings.agent.scan.noPluginsFound') }}
    </div>
    <template v-else>
      <div class="scan-list__header">
        <label class="scan-checkbox">
          <input
            type="checkbox"
            :checked="selectedNames.length === items.length"
            @change="$emit('toggle-all')"
          >
          <span>{{ t('settings.agent.scan.pluginName') }}</span>
        </label>
        <span class="scan-list__col scan-list__col--small">{{ t('settings.agent.scan.version') }}</span>
        <span class="scan-list__col scan-list__col--subdirs">{{ t('settings.agent.scan.components') }}</span>
        <span class="scan-list__col scan-list__col--small">{{ t('settings.agent.scan.status') }}</span>
      </div>
      <div
        v-for="plugin in items"
        :key="plugin.name"
        class="scan-list__item"
        :class="{ 'scan-list__item--selected': selectedNames.includes(plugin.name) }"
        @click="$emit('toggle-item', plugin.name)"
      >
        <label class="scan-checkbox">
          <input
            type="checkbox"
            :checked="selectedNames.includes(plugin.name)"
            @click.stop
            @change="$emit('toggle-item', plugin.name)"
          >
          <span class="scan-list__item-name">{{ plugin.name }}</span>
        </label>
        <span class="scan-list__col scan-list__col--small scan-list__item-version">
          {{ plugin.version || '-' }}
        </span>
        <span class="scan-list__col scan-list__col--subdirs">
          <span
            v-if="plugin.subdirectories.has_agents"
            class="scan-plugin-badge"
            :title="t('settings.agent.scan.hasAgents')"
          >
            agents
          </span>
          <span
            v-if="plugin.subdirectories.has_commands"
            class="scan-plugin-badge scan-plugin-badge--commands"
            :title="t('settings.agent.scan.hasCommands')"
          >
            cmds
          </span>
          <span
            v-if="plugin.subdirectories.has_skills"
            class="scan-plugin-badge scan-plugin-badge--skills"
            :title="t('settings.agent.scan.hasSkills')"
          >
            skills
          </span>
          <span
            v-if="plugin.subdirectories.has_hooks"
            class="scan-plugin-badge scan-plugin-badge--hooks"
            :title="t('settings.agent.scan.hasHooks')"
          >
            hooks
          </span>
          <span
            v-if="plugin.subdirectories.has_scripts"
            class="scan-plugin-badge scan-plugin-badge--scripts"
            :title="t('settings.agent.scan.hasScripts')"
          >
            scripts
          </span>
          <span
            v-if="!plugin.subdirectories.has_agents && !plugin.subdirectories.has_commands && !plugin.subdirectories.has_skills && !plugin.subdirectories.has_hooks && !plugin.subdirectories.has_scripts"
            class="scan-subdir-badge--empty"
          >
            -
          </span>
        </span>
        <span class="scan-list__col scan-list__col--small">
          <span
            class="scan-status-badge"
            :class="plugin.enabled ? 'scan-status-badge--enabled' : 'scan-status-badge--disabled'"
          >
            {{ plugin.enabled ? t('settings.agent.scan.enabled') : t('settings.agent.scan.disabled') }}
          </span>
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

.scan-list__col--subdirs {
  flex: 0 0 120px;
  display: flex;
  gap: var(--spacing-1);
  flex-wrap: wrap;
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

.scan-list__item-version {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.scan-status-badge {
  display: inline-flex;
  align-items: center;
  padding: var(--spacing-1) var(--spacing-2);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.scan-status-badge--enabled {
  background-color: rgba(34, 197, 94, 0.1);
  color: rgb(34, 197, 94);
}

.scan-status-badge--disabled {
  background-color: rgba(239, 68, 68, 0.1);
  color: rgb(239, 68, 68);
}

.scan-plugin-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--spacing-2);
  border-radius: var(--radius-sm);
  font-size: 10px;
  font-weight: var(--font-weight-medium);
  background-color: rgba(34, 197, 94, 0.1);
  color: rgb(34, 197, 94);
}

.scan-plugin-badge--commands {
  background-color: rgba(59, 130, 246, 0.1);
  color: rgb(59, 130, 246);
}

.scan-plugin-badge--skills {
  background-color: rgba(168, 85, 247, 0.1);
  color: rgb(168, 85, 247);
}

.scan-plugin-badge--hooks {
  background-color: rgba(251, 146, 60, 0.1);
  color: rgb(251, 146, 60);
}

.scan-plugin-badge--scripts {
  background-color: rgba(236, 72, 153, 0.1);
  color: rgb(236, 72, 153);
}

.scan-subdir-badge--empty {
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}
</style>
