<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useThemeStore } from '@/stores/theme'
import { useUIStore } from '@/stores/ui'
import { useProjectStore } from '@/stores/project'
import { useSessionStore } from '@/stores/session'
import { EaIcon } from '@/components/common'

const themeStore = useThemeStore()
const uiStore = useUIStore()
const projectStore = useProjectStore()
const sessionStore = useSessionStore()

/**
 * Logo 点击处理：返回欢迎页面
 * - 关闭所有模态框
 * - 清除当前项目选中状态，显示欢迎页面
 */
function handleLogoClick() {
  // 关闭所有模态框
  uiStore.closeSettings()

  // 清除当前项目，显示欢迎页面
  projectStore.setCurrentProject(null)
  sessionStore.setCurrentSession(null)
}

/**
 * 切换迷你面板显示状态
 * - 复用 Tauri 侧现有的 mini panel 显示/隐藏命令
 * - 失败时只记录日志，避免影响主窗口交互
 */
async function handleToggleMiniPanel() {
  try {
    const isMiniPanelVisible = await invoke<boolean>('toggle_mini_panel')
    if (isMiniPanelVisible) {
      await getCurrentWindow().hide()
    }
  } catch (error) {
    console.error('Failed to toggle mini panel:', error)
  }
}
</script>

<template>
  <header class="app-header">
    <div class="app-header__left">
      <div
        class="app-logo"
        title="返回欢迎页面"
        @click="handleLogoClick"
      >
        <EaIcon
          name="bot"
          :size="24"
        />
      </div>
    </div>

    <div class="app-header__right">
      <button
        class="header-btn"
        title="切换迷你面板"
        @click="handleToggleMiniPanel"
      >
        <EaIcon
          name="panel-left-open"
          :size="18"
        />
      </button>

      <!-- 主题切换 -->
      <button
        class="header-btn"
        title="切换主题"
        @click="themeStore.toggleTheme()"
      >
        <EaIcon
          :name="themeStore.isDark ? 'sun' : 'moon'"
          :size="18"
        />
      </button>

      <!-- 设置按钮 -->
      <button
        class="header-btn"
        title="设置"
        @click="uiStore.openSettings()"
      >
        <EaIcon
          name="settings"
          :size="18"
        />
      </button>
    </div>
  </header>
</template>

<style scoped>
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--header-height);
  padding: 0 var(--spacing-4);
  background-color: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.app-header__left,
.app-header__right {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.app-logo {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  color: var(--color-primary);
  font-weight: var(--font-weight-semibold);
  cursor: pointer;
  padding: var(--spacing-1) var(--spacing-2);
  border-radius: var(--radius-md);
  transition: background-color var(--transition-fast) var(--easing-default);
}

.app-logo:hover {
  background-color: var(--color-surface-hover);
}

.header-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
  transition: all var(--transition-fast) var(--easing-default);
}

.header-btn:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.header-btn:active {
  background-color: var(--color-surface-active);
}
</style>
