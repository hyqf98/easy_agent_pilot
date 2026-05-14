<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import EaIcon from '@/components/common/EaIcon.vue'
import EaSelect, { type SelectOption } from '@/components/common/EaSelect.vue'
import TerminalTabPane from './TerminalTabPane.vue'
import { useProjectStore } from '@/stores/project'
import { useTerminalStore } from '@/stores/terminal'

const { t } = useI18n()
const projectStore = useProjectStore()
const terminalStore = useTerminalStore()
const isResizing = ref(false)
let startY = 0
let startHeight = 0
let rafId = 0
let pendingDelta = 0
let hasPending = false

function flushResize() {
  if (hasPending) {
    terminalStore.setPanelHeight(startHeight + pendingDelta)
    hasPending = false
  }
}

const activeTab = computed(() => terminalStore.activeTab)
const projectOptions = computed<SelectOption[]>(() => [
  {
    value: '',
    label: t('terminal.unboundProject')
  },
  ...projectStore.projects.map(project => ({
    value: project.id,
    label: project.name
  }))
])
const activeProjectValue = computed({
  get: () => activeTab.value?.projectId ?? '',
  set: (value: string | number) => {
    if (!activeTab.value) {
      return
    }

    void terminalStore.changeTabProject(activeTab.value.id, String(value) || null)
  }
})
const activeProjectPath = computed(() => activeTab.value?.cwd || t('terminal.noProjectPath'))

async function ensureTerminalReady(projectId: string | null) {
  await terminalStore.bindEvents()
  await terminalStore.ensureFirstTab(projectId)
}

async function handleOpenPanel() {
  await ensureTerminalReady(projectStore.currentProjectId)
  terminalStore.setCollapsed(false)
}

function handleClosePanel() {
  terminalStore.setCollapsed(true)
}

async function handleCreateTab() {
  await terminalStore.createTab(projectStore.currentProjectId)
}

async function handleCloseTab(tabId: string) {
  await terminalStore.closeTab(tabId)
}

function handleResizeMove(event: MouseEvent) {
  if (!isResizing.value) {
    return
  }

  pendingDelta = startY - event.clientY
  if (!hasPending) {
    hasPending = true
    rafId = requestAnimationFrame(flushResize)
  }
}

function handleResizeEnd() {
  if (!isResizing.value) {
    return
  }

  cancelAnimationFrame(rafId)
  hasPending = false
  isResizing.value = false
  document.body.style.userSelect = ''
  document.removeEventListener('mousemove', handleResizeMove)
  document.removeEventListener('mouseup', handleResizeEnd)
}

function handleResizeStart(event: MouseEvent) {
  if (terminalStore.isCollapsed) {
    return
  }

  isResizing.value = true
  startY = event.clientY
  startHeight = terminalStore.panelHeight
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', handleResizeMove, { passive: true })
  document.addEventListener('mouseup', handleResizeEnd)
}

watch(() => projectStore.currentProjectId, async (projectId) => {
  if (!projectId) {
    return
  }

  if (terminalStore.tabs.length === 0) {
    return
  }

  await terminalStore.syncActiveTabToProject(projectId)
}, { immediate: true })

onMounted(async () => {
  await terminalStore.bindEvents()
})

onBeforeUnmount(() => {
  handleResizeEnd()
  void terminalStore.dispose()
})
</script>

<template>
  <section
    class="bottom-terminal"
    :class="{ 'bottom-terminal--collapsed': terminalStore.isCollapsed }"
    :style="terminalStore.isCollapsed ? undefined : { height: `${terminalStore.panelHeight}px` }"
  >
    <div
      v-if="!terminalStore.isCollapsed"
      class="bottom-terminal__resizer"
      @mousedown="handleResizeStart"
    />

    <header class="bottom-terminal__toolbar">
      <button
        type="button"
        class="bottom-terminal__title bottom-terminal__title-button"
        :title="terminalStore.isCollapsed ? t('terminal.expand') : t('terminal.collapse')"
        @click="terminalStore.isCollapsed ? handleOpenPanel() : handleClosePanel()"
      >
        <EaIcon
          name="terminal"
          :size="16"
        />
        <span>{{ t('terminal.title') }}</span>
        <span class="bottom-terminal__count">{{ terminalStore.tabs.length }}</span>
      </button>

      <div class="bottom-terminal__toolbar-actions">
        <div
          v-if="activeTab && !terminalStore.isCollapsed"
          class="bottom-terminal__project-select"
        >
          <span class="bottom-terminal__label">{{ t('terminal.projectLabel') }}</span>
          <EaSelect
            v-model="activeProjectValue"
            :options="projectOptions"
            size="small"
          />
        </div>

        <button
          v-if="!terminalStore.isCollapsed"
          type="button"
          class="bottom-terminal__toolbar-btn"
          :title="t('terminal.newTab')"
          @click="handleCreateTab"
        >
          <EaIcon
            name="plus"
            :size="15"
          />
        </button>

        <button
          type="button"
          class="bottom-terminal__toolbar-btn"
          :title="terminalStore.isCollapsed ? t('terminal.expand') : t('terminal.collapse')"
          @click="terminalStore.isCollapsed ? handleOpenPanel() : handleClosePanel()"
        >
          <EaIcon
            :name="terminalStore.isCollapsed ? 'chevron-up' : 'chevron-down'"
            :size="15"
          />
        </button>
      </div>
    </header>

    <div
      v-show="!terminalStore.isCollapsed"
      class="bottom-terminal__body"
    >
      <div class="bottom-terminal__tabs">
        <button
          v-for="tab in terminalStore.tabs"
          :key="tab.id"
          type="button"
          class="bottom-terminal__tab"
          :class="{
            'bottom-terminal__tab--active': tab.id === terminalStore.activeTabId,
            'bottom-terminal__tab--closed': tab.status === 'closed'
          }"
          @click="terminalStore.setActiveTab(tab.id)"
        >
          <span class="bottom-terminal__tab-dot" />
          <span class="bottom-terminal__tab-title">{{ t('terminal.tabTitle', { count: tab.sequence }) }}</span>
          <span class="bottom-terminal__tab-shell">{{ tab.shell }}</span>
          <span
            class="bottom-terminal__tab-close"
            @click.stop="handleCloseTab(tab.id)"
          >
            <EaIcon
              name="x"
              :size="13"
            />
          </span>
        </button>
      </div>

      <div class="bottom-terminal__meta">
        <span class="bottom-terminal__label">{{ t('terminal.pathLabel') }}</span>
        <span class="bottom-terminal__path">{{ activeProjectPath }}</span>
      </div>

      <div class="bottom-terminal__pane-shell">
        <div
          v-if="terminalStore.tabs.length === 0"
          class="bottom-terminal__empty"
        >
          <div class="bottom-terminal__empty-title">
            {{ t('terminal.emptyTitle') }}
          </div>
          <div class="bottom-terminal__empty-text">
            {{ t('terminal.emptyDescription') }}
          </div>
          <button
            type="button"
            class="bottom-terminal__empty-btn"
            @click="handleCreateTab"
          >
            {{ t('terminal.newTab') }}
          </button>
        </div>

        <TerminalTabPane
          v-for="tab in terminalStore.tabs"
          v-show="tab.id === terminalStore.activeTabId"
          :key="tab.id"
          :tab="tab"
          :active="tab.id === terminalStore.activeTabId"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.bottom-terminal {
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  border-top: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--color-bg-secondary) 72%, white), var(--color-bg-primary));
  min-height: 48px;
  --terminal-surface-bg: #0f172a;
  --terminal-surface-border: rgba(96, 165, 250, 0.18);
  --terminal-surface-text: #e2e8f0;
}

.bottom-terminal--collapsed {
  height: 48px;
}

.bottom-terminal__resizer {
  height: 8px;
  cursor: row-resize;
  background:
    linear-gradient(180deg, transparent, color-mix(in srgb, var(--color-border) 60%, transparent), transparent);
}

.bottom-terminal__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 48px;
  padding: 0 16px;
}

.bottom-terminal__title {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  font-weight: 700;
  color: var(--color-text-primary);
}

.bottom-terminal__title-button {
  padding: 0;
  background: transparent;
}

.bottom-terminal__count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 22px;
  height: 22px;
  padding: 0 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
  color: var(--color-primary-active);
  font-size: 11px;
}

.bottom-terminal__toolbar-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.bottom-terminal__project-select {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-width: 220px;
}

.bottom-terminal__label {
  flex-shrink: 0;
  color: var(--color-text-secondary);
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.bottom-terminal__toolbar-btn,
.bottom-terminal__empty-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 32px;
  border-radius: 10px;
  border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
  background: color-mix(in srgb, var(--color-surface) 92%, white);
  color: var(--color-text-secondary);
  transition: all var(--transition-fast) var(--easing-default);
}

.bottom-terminal__toolbar-btn {
  width: 32px;
}

.bottom-terminal__toolbar-btn:hover,
.bottom-terminal__empty-btn:hover {
  border-color: color-mix(in srgb, var(--color-primary) 40%, var(--color-border));
  color: var(--color-text-primary);
  background: color-mix(in srgb, var(--color-primary-light) 34%, white);
}

.bottom-terminal__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  padding: 0 14px 14px;
  flex: 1;
}

.bottom-terminal__tabs {
  display: flex;
  align-items: center;
  gap: 8px;
  overflow-x: auto;
  padding-bottom: 2px;
}

.bottom-terminal__tab {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  max-width: 20rem;
  height: 34px;
  padding: 0 10px;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--color-border) 76%, transparent);
  background: color-mix(in srgb, var(--color-surface) 94%, white);
  color: var(--color-text-secondary);
}

.bottom-terminal__tab--active {
  border-color: color-mix(in srgb, var(--color-primary) 36%, var(--color-border));
  background: color-mix(in srgb, var(--color-primary-light) 48%, white);
  color: var(--color-text-primary);
}

.bottom-terminal__tab--closed {
  opacity: 0.72;
}

.bottom-terminal__tab-dot {
  width: 7px;
  height: 7px;
  border-radius: 999px;
  background: #22c55e;
  flex-shrink: 0;
}

.bottom-terminal__tab--closed .bottom-terminal__tab-dot {
  background: #94a3b8;
}

.bottom-terminal__tab-title,
.bottom-terminal__path {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.bottom-terminal__tab-title {
  font-size: 12px;
  font-weight: 600;
}

.bottom-terminal__tab-shell {
  max-width: 8rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-tertiary);
  font-size: 11px;
}

.bottom-terminal__tab-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 999px;
}

.bottom-terminal__tab-close:hover {
  background: color-mix(in srgb, var(--color-bg-secondary) 80%, transparent);
}

.bottom-terminal__meta {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  padding: 0 2px;
}

.bottom-terminal__path {
  font-family: var(--font-family-mono);
  font-size: 11px;
  color: var(--color-text-secondary);
}

.bottom-terminal__pane-shell {
  flex: 1;
  min-height: 0;
  display: flex;
}

.bottom-terminal__empty {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  border-radius: 16px;
  border: 1px dashed color-mix(in srgb, var(--color-border) 70%, transparent);
  background: color-mix(in srgb, var(--color-bg-secondary) 72%, white);
  text-align: center;
}

.bottom-terminal__empty-title {
  color: var(--color-text-primary);
  font-size: 14px;
  font-weight: 700;
}

.bottom-terminal__empty-text {
  max-width: 32rem;
  color: var(--color-text-secondary);
  font-size: 12px;
  line-height: 1.6;
}

.bottom-terminal__empty-btn {
  min-width: 132px;
  padding: 0 14px;
}

:deep(.xterm) {
  height: 100%;
}

:deep(.xterm-viewport) {
  scrollbar-color: rgba(148, 163, 184, 0.42) transparent;
}

@media (max-width: 960px) {
  .bottom-terminal__toolbar {
    flex-wrap: wrap;
    padding: 10px 12px;
  }

  .bottom-terminal__project-select {
    min-width: 0;
    width: 100%;
  }

  .bottom-terminal__tab-shell {
    display: none;
  }
}
</style>
