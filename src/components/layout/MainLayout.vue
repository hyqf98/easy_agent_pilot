<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'
import { useLayoutStore, PANEL_LIMITS } from '@/stores/layout'
import { useUIStore } from '@/stores/ui'
import { useProjectStore } from '@/stores/project'
import AppHeader from './AppHeader.vue'
import BottomTerminalPanel from './BottomTerminalPanel.vue'
import SideNavRail from './SideNavRail.vue'
import PanelContainer from './PanelContainer.vue'
import SessionTabs from './SessionTabs.vue'
import MessageArea from './messageArea/MessageArea.vue'
import PanelResizer from './PanelResizer.vue'
import WelcomePage from './WelcomePage.vue'
import { PlanModePanel } from '@/components/plan'
import { MemoryModePanel } from '@/components/memory'
import { SoloModePanel } from '@/components/solo'
import { FileEditorWorkspace } from '@/modules/fileEditor'

const layoutStore = useLayoutStore()
const uiStore = useUIStore()
const projectStore = useProjectStore()

// 面板拖拽
const handlePanelResize = (delta: number) => {
  const newWidth = layoutStore.panelWidth + delta
  layoutStore.setPanelWidth(newWidth)
}

const handlePanelResizeEnd = (width: number) => {
  layoutStore.setPanelWidth(width)
}

// 窗口大小变化时的响应式处理
let resizeTimeout: ReturnType<typeof setTimeout> | null = null
const handleWindowResize = () => {
  // 使用防抖避免频繁触发
  if (resizeTimeout) {
    clearTimeout(resizeTimeout)
  }
  resizeTimeout = setTimeout(() => {
    layoutStore.handleResize()
  }, 100)
}

// 加载项目数据
onMounted(async () => {
  // 初始调整
  layoutStore.handleResize()
  // 监听窗口大小变化
  window.addEventListener('resize', handleWindowResize)
  // 加载项目列表
  await projectStore.loadProjects()
})

onUnmounted(() => {
  window.removeEventListener('resize', handleWindowResize)
  if (resizeTimeout) {
    clearTimeout(resizeTimeout)
  }
})

watch(
  () => projectStore.currentProjectId,
  (nextProjectId, previousProjectId) => {
    if (!previousProjectId && nextProjectId && uiStore.projectCreateModalVisible) {
      uiStore.closeProjectCreateModal()
    }
  }
)
</script>

<template>
  <div class="main-layout">
    <!-- 欢迎页面：没有选中项目时显示（即使已有项目也显示，让用户手动选择） -->
    <template v-if="!projectStore.currentProjectId">
      <div class="main-layout__welcome">
        <WelcomePage />
      </div>
    </template>

    <!-- 正常布局：有选中项目时显示 -->
    <template v-else>
      <!-- 顶部 Header -->
      <AppHeader />

      <!-- 主体区域 -->
      <div class="main-layout__body">
        <SideNavRail />

        <div
          v-show="uiStore.appMode === 'plan'"
          class="main-layout__plan-panel"
        >
          <PlanModePanel />
        </div>

        <div
          v-show="uiStore.appMode === 'solo'"
          class="main-layout__solo-panel"
        >
          <SoloModePanel />
        </div>

        <div
          v-show="uiStore.appMode === 'memory'"
          class="main-layout__memory-panel"
        >
          <MemoryModePanel />
        </div>

        <div
          v-show="uiStore.appMode === 'chat'"
          class="main-layout__chat-shell"
        >
          <template v-if="layoutStore.isPanelOpen">
            <div
              class="main-layout__panel"
              :style="{ width: `${layoutStore.panelWidth}px` }"
            >
              <PanelContainer />
            </div>

            <PanelResizer
              direction="right"
              :min-width="PANEL_LIMITS.panel.minWidth"
              :max-width="PANEL_LIMITS.panel.maxWidth"
              :current-width="layoutStore.panelWidth"
              @resize="handlePanelResize"
              @resize-end="handlePanelResizeEnd"
            />
          </template>

          <div class="main-layout__main">
            <template v-if="uiStore.mainContentMode === 'chat'">
              <SessionTabs />
              <MessageArea />
            </template>
            <FileEditorWorkspace
              v-else
              class="main-layout__file-editor"
            />
          </div>
        </div>
      </div>

      <BottomTerminalPanel />
    </template>
  </div>
</template>

<style scoped>
.main-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: var(--color-bg-secondary);
}

.main-layout__welcome {
  flex: 1;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.main-layout__body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.main-layout__chat-shell {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.main-layout__panel {
  flex-shrink: 0;
  min-width: 0;
  overflow: hidden;
  transition: width var(--transition-normal) var(--easing-default);
}

.main-layout__main {
  flex: 1;
  min-width: 400px;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.main-layout__file-editor {
  flex: 1;
  min-height: 0;
}

.main-layout__plan-panel {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.main-layout__memory-panel {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.main-layout__solo-panel {
  flex: 1;
  min-width: 0;
  min-height: 0;
}
</style>
