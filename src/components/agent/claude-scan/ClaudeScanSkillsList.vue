<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ScannedSkill } from '@/stores/skillConfigShared'

defineProps<{
  items: ScannedSkill[]
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
      {{ t('settings.agent.scan.noSkillsFound') }}
    </div>
    <template v-else>
      <div class="scan-list__header">
        <label class="scan-checkbox">
          <input
            type="checkbox"
            :checked="selectedNames.length === items.length"
            @change="$emit('toggle-all')"
          >
          <span>{{ t('settings.agent.scan.skillName') }}</span>
        </label>
        <span class="scan-list__col scan-list__col--subdirs">{{ t('settings.agent.scan.subdirectories') }}</span>
        <span class="scan-list__col">{{ t('settings.agent.scan.description') }}</span>
      </div>
      <div
        v-for="skill in items"
        :key="skill.name"
        class="scan-list__item"
        :class="{ 'scan-list__item--selected': selectedNames.includes(skill.name) }"
        @click="$emit('toggle-item', skill.name)"
      >
        <label class="scan-checkbox">
          <input
            type="checkbox"
            :checked="selectedNames.includes(skill.name)"
            @click.stop
            @change="$emit('toggle-item', skill.name)"
          >
          <span class="scan-list__item-name">{{ skill.name }}</span>
        </label>
        <span class="scan-list__col scan-list__col--subdirs">
          <span
            v-if="skill.subdirectories.has_scripts"
            class="scan-subdir-badge"
            :title="t('settings.agent.scan.hasScripts')"
          >
            scripts
          </span>
          <span
            v-if="skill.subdirectories.has_references"
            class="scan-subdir-badge scan-subdir-badge--refs"
            :title="t('settings.agent.scan.hasReferences')"
          >
            refs
          </span>
          <span
            v-if="skill.subdirectories.has_assets"
            class="scan-subdir-badge scan-subdir-badge--assets"
            :title="t('settings.agent.scan.hasAssets')"
          >
            assets
          </span>
          <span
            v-if="!skill.subdirectories.has_scripts && !skill.subdirectories.has_references && !skill.subdirectories.has_assets"
            class="scan-subdir-badge--empty"
          >
            -
          </span>
        </span>
        <span class="scan-list__col scan-list__item-desc">
          {{ skill.description || '-' }}
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

.scan-list__item-desc {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.scan-subdir-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--spacing-2);
  border-radius: var(--radius-sm);
  font-size: 10px;
  font-weight: var(--font-weight-medium);
  background-color: rgba(59, 130, 246, 0.1);
  color: rgb(59, 130, 246);
}

.scan-subdir-badge--refs {
  background-color: rgba(168, 85, 247, 0.1);
  color: rgb(168, 85, 247);
}

.scan-subdir-badge--assets {
  background-color: rgba(251, 146, 60, 0.1);
  color: rgb(251, 146, 60);
}

.scan-subdir-badge--empty {
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}
</style>
