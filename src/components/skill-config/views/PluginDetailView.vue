<script setup lang="ts">
import { computed, onUnmounted, watch, toRef } from 'vue'
import { useI18n } from 'vue-i18n'
import type { UnifiedPluginConfig } from '@/stores/skillConfig'
import { EaIcon } from '@/components/common'
import ConfigFileWorkspace from '@/components/skill-config/common/ConfigFileWorkspace.vue'
import PluginDetailSidebar from '@/components/skill-config/plugin-detail/PluginDetailSidebar.vue'
import PluginDetailToolbar from '@/components/skill-config/plugin-detail/PluginDetailToolbar.vue'
import { PLUGIN_SECTION_ICONS, PLUGIN_SECTION_KEYS } from '@/components/skill-config/plugin-detail/shared'
import type { PluginSection } from '@/components/skill-config/plugin-detail/shared'
import { usePluginDetail } from '@/components/skill-config/plugin-detail/usePluginDetail'

const props = defineProps<{
  plugin: UnifiedPluginConfig
}>()

const emit = defineEmits<{
  (e: 'back'): void
  (e: 'delete', plugin: UnifiedPluginConfig): void
}>()

const { t } = useI18n()
const pluginRef = toRef(props, 'plugin')
const {
  activeSection,
  currentList,
  editContent,
  fileContent,
  hasListItems,
  isEditMode,
  isLoading,
  isLoadingFile,
  isSidebarHovered,
  pluginDetails,
  selectedItem,
  dispose,
  handleSidebarMouseEnter,
  handleSidebarMouseLeave,
  loadPluginDetail,
  saveEdit,
  selectItem,
  switchSection,
  toggleEditMode,
} = usePluginDetail(pluginRef)

// 返回列表
function handleBack() {
  emit('back')
}

// 删除 Plugin
function handleDelete() {
  emit('delete', props.plugin)
}

const sectionTabs = computed<Array<{ key: PluginSection, label: string, icon: string, count: number }>>(() => ([
  { key: 'skills', label: t(PLUGIN_SECTION_KEYS.skills), icon: PLUGIN_SECTION_ICONS.skills, count: pluginDetails.value?.internal_skills.length || 0 },
  { key: 'commands', label: t(PLUGIN_SECTION_KEYS.commands), icon: PLUGIN_SECTION_ICONS.commands, count: pluginDetails.value?.internal_commands.length || 0 },
  { key: 'agents', label: t(PLUGIN_SECTION_KEYS.agents), icon: PLUGIN_SECTION_ICONS.agents, count: pluginDetails.value?.internal_agents.length || 0 },
]))

// 监听 plugin 变化
watch(() => props.plugin, () => {
  loadPluginDetail()
}, { immediate: true })

onUnmounted(() => {
  dispose()
})
</script>

<template>
  <div class="plugin-detail">
    <PluginDetailToolbar
      :plugin="plugin"
      :plugin-version="plugin.version"
      :selected-item="selectedItem"
      :active-section="activeSection"
      :current-list-count="currentList.length"
      :has-list-items="hasListItems"
      :is-edit-mode="isEditMode"
      :has-file-content="Boolean(fileContent)"
      @back="handleBack"
      @delete="handleDelete"
      @toggle-edit="toggleEditMode"
      @save="saveEdit"
    />

    <div class="plugin-detail__tabs">
      <button
        v-for="tab in sectionTabs"
        :key="tab.key"
        class="plugin-detail__tab"
        :class="{ 'plugin-detail__tab--active': activeSection === tab.key }"
        @click="switchSection(tab.key)"
      >
        <EaIcon :name="tab.icon" />
        {{ tab.label }}
        <span class="plugin-detail__tab-count">{{ tab.count }}</span>
      </button>
    </div>

    <div
      v-if="isLoading"
      class="plugin-detail__loading"
    >
      <EaIcon
        name="lucide:loader-2"
        class="plugin-detail__spinner"
      />
      {{ t('common.loading') }}
    </div>

    <!-- 主体 -->
    <div
      v-else
      class="plugin-detail__body"
    >
      <PluginDetailSidebar
        v-if="hasListItems"
        :active-section="activeSection"
        :current-list="currentList"
        :selected-item="selectedItem"
        :expanded="isSidebarHovered"
        @mouseenter="handleSidebarMouseEnter"
        @mouseleave="handleSidebarMouseLeave"
        @select="selectItem"
      />

      <div class="plugin-detail__main">
        <template v-if="selectedItem">
          <ConfigFileWorkspace
            :loading="isLoadingFile"
            :editing="isEditMode"
            :file="fileContent"
            :edit-content="editContent"
            :edit-placeholder="t('settings.skills.editPlaceholder')"
            :empty-text="t('settings.skills.noContent')"
            max-width="1200px"
            padding="var(--spacing-6)"
            @update:edit-content="editContent = $event"
          >
            <template #loading>
              {{ t('common.loading') }}
            </template>
          </ConfigFileWorkspace>
        </template>

        <template v-else>
          <div
            v-if="pluginDetails"
            class="plugin-detail__info"
          >
            <div class="plugin-detail__info-grid">
              <div
                v-if="pluginDetails.author"
                class="plugin-detail__info-item"
              >
                <span class="plugin-detail__info-label">
                  <EaIcon name="lucide:user" />
                  {{ t('settings.plugins.author') }}
                </span>
                <span class="plugin-detail__info-value">{{ pluginDetails.author }}</span>
              </div>
              <div
                v-if="pluginDetails.install_source"
                class="plugin-detail__info-item"
              >
                <span class="plugin-detail__info-label">
                  <EaIcon name="lucide:download" />
                  {{ t('settings.plugins.installedFrom') }}
                </span>
                <span class="plugin-detail__info-value">{{ pluginDetails.install_source }}</span>
              </div>
              <div class="plugin-detail__info-item">
                <span class="plugin-detail__info-label">
                  <EaIcon name="lucide:folder" />
                  {{ t('settings.plugins.path') }}
                </span>
                <code class="plugin-detail__info-value">{{ pluginDetails.path }}</code>
              </div>
            </div>

            <div
              v-if="pluginDetails.description"
              class="plugin-detail__description"
            >
              {{ pluginDetails.description }}
            </div>

            <div class="plugin-detail__stats">
              <div class="plugin-detail__stat">
                <EaIcon name="lucide:book-open" />
                <span>{{ pluginDetails.internal_skills.length }} {{ t('settings.plugins.internalSkills') }}</span>
              </div>
              <div class="plugin-detail__stat">
                <EaIcon name="lucide:terminal" />
                <span>{{ pluginDetails.internal_commands.length }} {{ t('settings.plugins.internalCommands') }}</span>
              </div>
              <div class="plugin-detail__stat">
                <EaIcon name="lucide:bot" />
                <span>{{ pluginDetails.internal_agents.length }} {{ t('settings.plugins.internalAgents') }}</span>
              </div>
            </div>

            <div
              v-if="currentList.length > 0"
              class="plugin-detail__select-hint"
            >
              <EaIcon
                name="lucide:mouse-pointer-click"
                class="plugin-detail__select-hint-icon"
              />
              <p>{{ t('settings.plugins.selectFromList') }}</p>
            </div>
          </div>

          <div
            v-else
            class="plugin-detail__empty-section"
          >
            <EaIcon
              name="lucide:inbox"
              class="plugin-detail__empty-icon"
            />
            <p>
              {{ activeSection === 'skills' ? t('settings.plugins.noInternalSkills') :
                activeSection === 'commands' ? t('settings.plugins.noInternalCommands') :
                t('settings.plugins.noInternalAgents') }}
            </p>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.plugin-detail {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--color-surface);
}

/* 标签页 */
.plugin-detail__tabs {
  display: flex;
  gap: var(--spacing-1);
  padding: var(--spacing-2) var(--spacing-4);
  background: var(--color-background-tertiary);
  border-bottom: 1px solid var(--color-border);
}

.plugin-detail__tab {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border: none;
  background: transparent;
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.plugin-detail__tab:hover {
  background: var(--color-background-secondary);
  color: var(--color-text);
}

.plugin-detail__tab--active {
  background: var(--color-surface);
  color: var(--color-text);
  box-shadow: var(--shadow-sm);
}

.plugin-detail__tab svg {
  width: 14px;
  height: 14px;
}

.plugin-detail__tab-count {
  font-size: var(--font-size-xs);
  padding: 2px 6px;
  background: var(--color-background-tertiary);
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
}

.plugin-detail__tab--active .plugin-detail__tab-count {
  background: var(--color-primary-bg);
  color: var(--color-primary);
}

/* 主体 */
.plugin-detail__body {
  display: flex;
  flex: 1;
  overflow: hidden;
  position: relative;
}

/* 主面板 */
.plugin-detail__main {
  flex: 1;
  overflow: hidden;
  min-width: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.plugin-detail__info {
  padding: var(--spacing-8);
  max-width: 1200px;
  margin: 0 auto;
  flex: 1;
  overflow-y: auto;
}

.plugin-detail__info-grid {
  display: grid;
  gap: var(--spacing-3);
  margin-bottom: var(--spacing-4);
}

.plugin-detail__info-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-2) var(--spacing-3);
  background: var(--color-background-secondary);
  border-radius: var(--radius-md);
}

.plugin-detail__info-label {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  min-width: 100px;
}

.plugin-detail__info-label svg {
  width: 14px;
  height: 14px;
}

.plugin-detail__info-value {
  font-size: var(--font-size-sm);
  color: var(--color-text);
}

.plugin-detail__info-value code {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.plugin-detail__description {
  font-size: var(--font-size-base);
  color: var(--color-text-secondary);
  line-height: 1.7;
  margin-bottom: var(--spacing-4);
}

.plugin-detail__stats {
  display: flex;
  gap: var(--spacing-4);
  padding: var(--spacing-3);
  background: var(--color-background-secondary);
  border-radius: var(--radius-lg);
}

.plugin-detail__stat {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.plugin-detail__stat svg {
  width: 16px;
  height: 16px;
  color: var(--color-text-tertiary);
}

/* 加载状态 */
.plugin-detail__loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  padding: var(--spacing-8);
  color: var(--color-text-tertiary);
}

.plugin-detail__spinner {
  width: 20px;
  height: 20px;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.plugin-detail__empty-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-3);
  height: 100%;
  color: var(--color-text-tertiary);
  padding: var(--spacing-6);
}

.plugin-detail__empty-icon {
  width: 48px;
  height: 48px;
  opacity: 0.5;
}

/* 选择提示 */
.plugin-detail__select-hint {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-3) var(--spacing-4);
  margin-top: var(--spacing-4);
  background: var(--color-primary-bg);
  border-radius: var(--radius-md);
  color: var(--color-primary);
  font-size: var(--font-size-sm);
}

.plugin-detail__select-hint-icon {
  width: 18px;
  height: 18px;
}
</style>
