<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import type { InternalItem, PluginSection } from './shared'
import { PLUGIN_SECTION_KEYS, getPluginItemIcon } from './shared'

const props = defineProps<{
  activeSection: PluginSection
  currentList: InternalItem[]
  selectedItem: InternalItem | null
  expanded: boolean
}>()

defineEmits<{
  (e: 'mouseenter'): void
  (e: 'mouseleave'): void
  (e: 'select', item: InternalItem): void
}>()

const { t } = useI18n()

const sectionTitle = computed(() => t(PLUGIN_SECTION_KEYS[props.activeSection]))
</script>

<template>
  <div
    class="plugin-detail-sidebar"
    :class="{ 'plugin-detail-sidebar--expanded': expanded }"
    @mouseenter="$emit('mouseenter')"
    @mouseleave="$emit('mouseleave')"
  >
    <div class="plugin-detail-sidebar__header">
      <h3>{{ sectionTitle }}</h3>
    </div>

    <div class="plugin-detail-sidebar__content">
      <div
        v-for="item in currentList"
        :key="item.path"
        class="plugin-detail-sidebar__item"
        :class="{ 'plugin-detail-sidebar__item--active': selectedItem?.path === item.path }"
        @click="$emit('select', item)"
      >
        <EaIcon
          :name="getPluginItemIcon(item.item_type)"
          class="plugin-detail-sidebar__icon"
        />
        <div class="plugin-detail-sidebar__info">
          <span class="plugin-detail-sidebar__name">{{ item.name }}</span>
          <span
            v-if="item.description"
            class="plugin-detail-sidebar__desc"
          >{{ item.description }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.plugin-detail-sidebar {
  width: 40px;
  background: var(--color-background-secondary);
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  z-index: 10;
  transition: width 0.25s ease;
  overflow: hidden;
  position: relative;
}

.plugin-detail-sidebar::before {
  content: '';
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 4px;
  height: 40px;
  background: var(--color-primary);
  border-radius: 0 4px 4px 0;
  opacity: 0.5;
  transition: opacity 0.2s;
}

.plugin-detail-sidebar:hover::before,
.plugin-detail-sidebar--expanded::before {
  opacity: 0;
}

.plugin-detail-sidebar--expanded {
  width: 280px;
}

.plugin-detail-sidebar__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3) var(--spacing-4);
  border-bottom: 1px solid var(--color-border);
  white-space: nowrap;
  overflow: hidden;
}

.plugin-detail-sidebar__header h3 {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  margin: 0;
  opacity: 0;
  transition: opacity 0.2s;
}

.plugin-detail-sidebar--expanded .plugin-detail-sidebar__header h3,
.plugin-detail-sidebar--expanded .plugin-detail-sidebar__content {
  opacity: 1;
}

.plugin-detail-sidebar__content {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-2);
  opacity: 0;
  transition: opacity 0.2s;
}

.plugin-detail-sidebar__item {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.plugin-detail-sidebar__item:hover {
  background: var(--color-background-tertiary);
}

.plugin-detail-sidebar__item--active {
  background: var(--color-primary-bg);
}

.plugin-detail-sidebar__icon {
  width: 18px;
  height: 18px;
  color: var(--color-text-tertiary);
  flex-shrink: 0;
  margin-top: 2px;
}

.plugin-detail-sidebar__item--active .plugin-detail-sidebar__icon {
  color: var(--color-primary);
}

.plugin-detail-sidebar__info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.plugin-detail-sidebar__name {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-detail-sidebar__desc {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
}
</style>
