<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useLayoutStore } from '@/stores/layout'
import { useProjectStore } from '@/stores/project'
import { useSessionStore } from '@/stores/session'
import { useUIStore } from '@/stores/ui'
import { EaIcon } from '@/components/common'

const { t } = useI18n()
const layoutStore = useLayoutStore()
const projectStore = useProjectStore()
const sessionStore = useSessionStore()
const uiStore = useUIStore()

// 处理工作区点击
const handleWorkspaceClick = () => {
  // 从任意特殊模式返回工作区
  if (uiStore.appMode !== 'chat') {
    uiStore.setAppMode('chat')
  }
  layoutStore.togglePanel('unified')
}

// 判断工作区是否激活
const isWorkspaceActive = () => {
  return uiStore.appMode === 'chat' && layoutStore.activePanel === 'unified'
}

// 切换计划模式
const togglePlanMode = () => {
  if (uiStore.appMode === 'plan') {
    uiStore.setAppMode('chat')
  } else {
    uiStore.setAppMode('plan')
    // 关闭左侧面板
    layoutStore.closePanel()
  }
}

// 切换记忆模式
const toggleMemoryMode = () => {
  if (uiStore.appMode === 'memory') {
    uiStore.setAppMode('chat')
  } else {
    uiStore.setAppMode('memory')
    // 关闭左侧面板
    layoutStore.closePanel()
  }
}

// 判断计划模式是否激活
const isPlanModeActive = () => {
  return uiStore.appMode === 'plan'
}

// 切换 SOLO 模式
const toggleSoloMode = () => {
  if (uiStore.appMode === 'solo') {
    uiStore.setAppMode('chat')
  } else {
    uiStore.setAppMode('solo')
    layoutStore.closePanel()
  }
}

// 判断记忆模式是否激活
const isMemoryModeActive = () => {
  return uiStore.appMode === 'memory'
}

// 判断 SOLO 模式是否激活
const isSoloModeActive = () => {
  return uiStore.appMode === 'solo'
}
</script>

<template>
  <div class="side-nav-rail">
    <!-- 导航项 -->
    <nav class="side-nav-rail__nav">
      <!-- 工作区按钮 -->
      <button
        :class="['nav-item', { 'nav-item--active': isWorkspaceActive() }]"
        :title="t('panel.workspace')"
        @click="handleWorkspaceClick"
      >
        <EaIcon
          name="layout-grid"
          :size="20"
          class="nav-item__icon"
        />
        <span class="nav-item__label">{{ t('panel.workspace') }}</span>
        <!-- 激活指示器 -->
        <span
          v-if="isWorkspaceActive()"
          class="nav-item__indicator"
        />
      </button>

      <!-- 分隔线 -->
      <div class="nav-divider" />

      <!-- 计划模式按钮 -->
      <button
        :class="['nav-item', 'nav-item--plan', { 'nav-item--active': isPlanModeActive() }]"
        title="计划模式"
        @click="togglePlanMode"
      >
        <EaIcon
          name="clipboard-list"
          :size="20"
          class="nav-item__icon"
        />
        <span class="nav-item__label">计划</span>
        <!-- 激活指示器 -->
        <span
          v-if="isPlanModeActive()"
          class="nav-item__indicator"
        />
      </button>

      <button
        :class="['nav-item', 'nav-item--solo', { 'nav-item--active': isSoloModeActive() }]"
        title="SOLO 模式"
        @click="toggleSoloMode"
      >
        <EaIcon
          name="sparkles"
          :size="20"
          class="nav-item__icon"
        />
        <span class="nav-item__label">SOLO</span>
        <span
          v-if="isSoloModeActive()"
          class="nav-item__indicator"
        />
      </button>

      <!-- 记忆模式按钮 -->
      <button
        :class="['nav-item', 'nav-item--memory', { 'nav-item--active': isMemoryModeActive() }]"
        title="记忆管理"
        @click="toggleMemoryMode"
      >
        <EaIcon
          name="brain"
          :size="20"
          class="nav-item__icon"
        />
        <span class="nav-item__label">记忆</span>
        <!-- 激活指示器 -->
        <span
          v-if="isMemoryModeActive()"
          class="nav-item__indicator"
        />
      </button>
    </nav>

    <!-- 底部快捷操作 -->
    <div class="side-nav-rail__footer">
      <!-- 返回欢迎页 -->
      <button
        class="nav-stats nav-stats--button"
        title="返回项目列表"
        @click="projectStore.setCurrentProject(null)"
      >
        <EaIcon
          name="folder"
          :size="14"
        />
        <span>{{ projectStore.projects.length }}</span>
      </button>
      <!-- 会话数量（仅显示） -->
      <div
        class="nav-stats"
        :title="t('panel.sessions')"
      >
        <EaIcon
          name="message-square"
          :size="14"
        />
        <span>{{ sessionStore.sessions.length }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.side-nav-rail {
  display: flex;
  flex-direction: column;
  width: 56px;
  height: 100%;
  background-color: var(--color-surface);
  border-right: 1px solid var(--color-border);
  flex-shrink: 0;
}

.side-nav-rail__nav {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
  padding: var(--spacing-2);
  flex: 1;
}

.nav-item {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-1);
  width: 100%;
  height: 56px;
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
  background-color: transparent;
  transition: all var(--transition-fast) var(--easing-default);
  cursor: pointer;
  border: none;
  outline: none;
}

.nav-item:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.nav-item:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: -2px;
}

.nav-item--active {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

.nav-item--active:hover {
  background-color: var(--color-primary-light);
  color: var(--color-primary);
}

/* 暗色模式下的激活样式 */
[data-theme='dark'] .nav-item--active {
  background-color: var(--color-active-bg);
  color: var(--color-active-text);
}

[data-theme='dark'] .nav-item--active:hover {
  background-color: var(--color-active-bg-hover);
  color: var(--color-active-text);
}

.nav-item--plan {
  margin-top: var(--spacing-1);
}

.nav-item--memory {
  margin-top: var(--spacing-1);
}

.nav-item--solo {
  margin-top: var(--spacing-1);
}

.nav-item__icon {
  flex-shrink: 0;
  transition: transform var(--transition-fast) var(--easing-default);
}

.nav-item:hover .nav-item__icon {
  transform: scale(1.1);
}

.nav-item__label {
  font-size: 11px;
  font-weight: var(--font-weight-medium);
  line-height: 1;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 48px;
}

.nav-item__indicator {
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 24px;
  background-color: var(--color-primary);
  border-radius: 0 2px 2px 0;
}

.nav-divider {
  height: 1px;
  background-color: var(--color-border);
  margin: var(--spacing-1) var(--spacing-2);
}

/* 底部统计 */
.side-nav-rail__footer {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
  padding: var(--spacing-2);
  border-top: 1px solid var(--color-border);
}

.nav-stats {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-1);
  font-size: 11px;
  color: var(--color-text-tertiary);
  padding: var(--spacing-1);
  border-radius: var(--radius-sm);
}

.nav-stats:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-secondary);
}

.nav-stats--button {
  border: none;
  background: transparent;
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.nav-stats--button:active {
  transform: scale(0.95);
}
</style>
