<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { UnifiedPluginConfig } from '@/stores/skillConfig'
import { EaButton, EaIcon } from '@/components/common'
import type { InternalItem, PluginSection } from './shared'
import { PLUGIN_SECTION_KEYS, getPluginItemIcon } from './shared'

const props = defineProps<{
  plugin: UnifiedPluginConfig
  pluginVersion?: string | null
  selectedItem: InternalItem | null
  activeSection: PluginSection
  currentListCount: number
  hasListItems: boolean
  isEditMode: boolean
  hasFileContent: boolean
}>()

defineEmits<{
  (e: 'back'): void
  (e: 'delete'): void
  (e: 'toggle-edit'): void
  (e: 'save'): void
}>()

const { t } = useI18n()

const currentSectionLabel = computed(() => t(PLUGIN_SECTION_KEYS[props.activeSection]))
</script>

<template>
  <div class="plugin-detail-toolbar">
    <div class="plugin-detail-toolbar__left">
      <EaButton
        variant="ghost"
        size="small"
        @click="$emit('back')"
      >
        <EaIcon name="lucide:arrow-left" />
        {{ t('common.back') }}
      </EaButton>
      <div class="plugin-detail-toolbar__breadcrumb">
        <EaIcon
          name="lucide:puzzle"
          class="plugin-detail-toolbar__icon"
        />
        <span class="plugin-detail-toolbar__name">{{ plugin.name }}</span>
        <span
          v-if="pluginVersion"
          class="plugin-detail-toolbar__version"
        >v{{ pluginVersion }}</span>
        <template v-if="selectedItem">
          <EaIcon
            name="lucide:chevron-right"
            class="plugin-detail-toolbar__chevron"
          />
          <EaIcon
            :name="getPluginItemIcon(selectedItem.item_type)"
            class="plugin-detail-toolbar__type-icon"
          />
          <span class="plugin-detail-toolbar__current-file">{{ selectedItem.name }}</span>
        </template>
      </div>
    </div>

    <div class="plugin-detail-toolbar__right">
      <div
        v-if="hasListItems && !selectedItem"
        class="plugin-detail-toolbar__list-hint"
      >
        <EaIcon name="lucide:list" />
        <span>{{ currentListCount }} {{ currentSectionLabel }}</span>
        <span class="plugin-detail-toolbar__hint-text">{{ t('settings.plugins.hoverToExpand') }}</span>
      </div>

      <EaButton
        v-if="hasFileContent && plugin.source === 'file'"
        :variant="isEditMode ? 'primary' : 'ghost'"
        size="small"
        @click="$emit('toggle-edit')"
      >
        <EaIcon :name="isEditMode ? 'lucide:eye' : 'lucide:pencil'" />
        {{ isEditMode ? t('common.view') : t('common.edit') }}
      </EaButton>

      <EaButton
        v-if="isEditMode"
        variant="primary"
        size="small"
        @click="$emit('save')"
      >
        <EaIcon name="lucide:save" />
        {{ t('common.save') }}
      </EaButton>

      <EaButton
        v-if="plugin.source === 'file'"
        variant="ghost"
        size="small"
        danger
        @click="$emit('delete')"
      >
        <EaIcon name="lucide:trash-2" />
      </EaButton>
    </div>
  </div>
</template>

<style scoped>
.plugin-detail-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3) var(--spacing-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-background-secondary);
}

.plugin-detail-toolbar__left,
.plugin-detail-toolbar__right,
.plugin-detail-toolbar__breadcrumb,
.plugin-detail-toolbar__list-hint {
  display: flex;
  align-items: center;
}

.plugin-detail-toolbar__left {
  gap: var(--spacing-3);
}

.plugin-detail-toolbar__right {
  gap: var(--spacing-2);
}

.plugin-detail-toolbar__breadcrumb,
.plugin-detail-toolbar__list-hint {
  gap: var(--spacing-2);
}

.plugin-detail-toolbar__icon {
  width: 20px;
  height: 20px;
  color: var(--color-warning);
}

.plugin-detail-toolbar__name {
  font-weight: var(--font-weight-semibold);
  font-size: var(--font-size-base);
}

.plugin-detail-toolbar__version {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  padding: 2px 6px;
  background: var(--color-background-tertiary);
  border-radius: var(--radius-sm);
}

.plugin-detail-toolbar__chevron {
  width: 14px;
  height: 14px;
  color: var(--color-text-tertiary);
}

.plugin-detail-toolbar__type-icon {
  width: 16px;
  height: 16px;
  color: var(--color-primary);
}

.plugin-detail-toolbar__current-file {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.plugin-detail-toolbar__list-hint {
  padding: var(--spacing-1) var(--spacing-3);
  background: var(--color-background-tertiary);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.plugin-detail-toolbar__hint-text {
  color: var(--color-text-tertiary);
}
</style>
