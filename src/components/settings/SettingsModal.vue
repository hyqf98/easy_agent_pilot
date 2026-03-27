<script setup lang="ts">
import { nextTick, watch, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useUIStore } from '@/stores/ui'
import SettingsNav from './SettingsNav.vue'
import SettingsContent from './SettingsContent.vue'

const uiStore = useUIStore()
const router = useRouter()

// ESC 关闭
const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && uiStore.settingsModalVisible) {
    // 检查是否有输入框聚焦
    const activeElement = document.activeElement
    if (activeElement?.tagName === 'INPUT' || activeElement?.tagName === 'TEXTAREA') {
      return
    }
    uiStore.closeSettings()
  }
}

// Cmd/Ctrl + , 打开设置
const handleOpenSettings = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === ',') {
    // 检查是否有输入框聚焦
    const activeElement = document.activeElement
    if (activeElement?.tagName === 'INPUT' || activeElement?.tagName === 'TEXTAREA') {
      return
    }
    e.preventDefault()
    uiStore.toggleSettings()
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
  document.addEventListener('keydown', handleOpenSettings)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
  document.removeEventListener('keydown', handleOpenSettings)
})

// 打开时禁止背景滚动
watch(() => uiStore.settingsModalVisible, (visible) => {
  document.body.style.overflow = visible ? 'hidden' : ''
})

const handleOverlayClick = (e: MouseEvent) => {
  if (e.target === e.currentTarget) {
    uiStore.closeSettings()
  }
}

const openFullscreenSettings = async () => {
  uiStore.closeSettings()
  await nextTick()
  if (router.currentRoute.value.path !== '/settings') {
    await router.push('/settings')
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="settings-modal">
      <div
        v-if="uiStore.settingsModalVisible"
        class="settings-overlay"
        @click="handleOverlayClick"
      >
        <div
          :class="['settings-modal', { 'settings-modal--logs': uiStore.activeSettingsTab === 'logs' }]"
        >
          <div class="settings-modal__header">
            <h2 class="settings-modal__title">
              设置
            </h2>
            <div class="settings-modal__actions">
              <button
                class="settings-modal__action-btn"
                title="全屏打开设置"
                @click="openFullscreenSettings"
              >
                <svg
                  viewBox="0 0 24 24"
                  width="18"
                  height="18"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path d="M9 3H5a2 2 0 0 0-2 2v4" />
                  <path d="M15 3h4a2 2 0 0 1 2 2v4" />
                  <path d="M21 15v4a2 2 0 0 1-2 2h-4" />
                  <path d="M3 15v4a2 2 0 0 0 2 2h4" />
                </svg>
              </button>
              <button
                class="settings-modal__action-btn"
                title="关闭设置"
                @click="uiStore.closeSettings"
              >
                <svg
                  viewBox="0 0 24 24"
                  width="20"
                  height="20"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="M18 6L6 18M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>
          <div class="settings-modal__body">
            <SettingsNav />
            <SettingsContent />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.settings-overlay {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
}

.settings-modal {
  display: flex;
  flex-direction: column;
  width: min(96vw, 1440px);
  height: 94vh;
  background-color: var(--color-surface);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
  overflow: hidden;
}

.settings-modal--logs {
  width: min(98vw, 1560px);
  height: 96vh;
}

.settings-modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4) var(--spacing-6);
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.settings-modal__title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.settings-modal__actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.settings-modal__action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
  transition: all var(--transition-fast) var(--easing-default);
}

.settings-modal__action-btn:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.settings-modal__body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* 动画 */
.settings-modal-enter-active,
.settings-modal-leave-active {
  transition: opacity var(--transition-normal) var(--easing-default);
}

.settings-modal-enter-active .settings-modal,
.settings-modal-leave-active .settings-modal {
  transition: transform var(--transition-normal) var(--easing-default),
              opacity var(--transition-normal) var(--easing-default);
}

.settings-modal-enter-from,
.settings-modal-leave-to {
  opacity: 0;
}

.settings-modal-enter-from .settings-modal,
.settings-modal-leave-to .settings-modal {
  transform: scale(0.95);
  opacity: 0;
}
</style>
