<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useUIStore, useProjectStore, useWindowManagerStore, type Project } from '@/stores'
import { EaModal } from '@/components/common'
import { ProjectCreateModal } from '@/components/project'
import WelcomeBackground from '@/components/layout/welcome/WelcomeBackground.vue'
import WelcomeEmptyState from '@/components/layout/welcome/WelcomeEmptyState.vue'
import WelcomeProjectBrowser from '@/components/layout/welcome/WelcomeProjectBrowser.vue'
import type { WelcomeAction, WelcomeFeature } from '@/components/layout/welcome/welcomeShared'

import { useMessage } from 'naive-ui'

const router = useRouter()
const uiStore = useUIStore()
const projectStore = useProjectStore()
const windowManagerStore = useWindowManagerStore()
const message = useMessage()

// 动画状态
const isLoaded = ref(false)

// 加载项目数据
onMounted(async () => {
  await projectStore.loadProjects()
  // 加载最近访问的项目
  await projectStore.getRecentProjectIds()
  window.addEventListener('pointerdown', hideContextMenu)
  setTimeout(() => {
    isLoaded.value = true
  }, 100)
})

onUnmounted(() => {
  window.removeEventListener('pointerdown', hideContextMenu)
})

// 是否有项目
const hasProjects = computed(() => projectStore.projectCount > 0)

// 项目列表（按更新时间排序）
const sortedProjects = computed(() => {
  return [...projectStore.projects]
    .sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())
    .slice(0, 6) // 最多显示6个
})

// 最近访问的项目
const recentProjects = computed(() => {
  const recentIds = projectStore.recentProjectIds
  return recentIds
    .slice(0, 2) // 只显示最近使用的 2 个项目
    .map(id => projectStore.projects.find(p => p.id === id))
    .filter((p): p is Project => p !== undefined)
})

// 右键菜单状态
const showContextMenuFlag = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const selectedProject = ref<Project | null>(null)

const contextMenuOptions = [
  { label: '在新窗口中打开', key: 'open-in-new-window' },
  { type: 'divider' },
  { label: '重命名', key: 'rename' },
  { label: '删除', key: 'delete' }
]

function showProjectContextMenu(e: MouseEvent, project: Project) {
  e.preventDefault()
  selectedProject.value = project
  contextMenuX.value = e.clientX
  contextMenuY.value = e.clientY
  showContextMenuFlag.value = true
}

function hideContextMenu() {
  showContextMenuFlag.value = false
}

async function handleContextMenuSelect(key: string) {
  hideContextMenu()
  if (!selectedProject.value) return

  switch (key) {
    case 'open-in-new-window':
      await windowManagerStore.openProjectInNewWindow(selectedProject.value.id)
      message.success('已在新窗口中打开')
      break
    case 'rename':
      // TODO: 调用现有的重命名逻辑
      break
    case 'delete':
      // TODO: 调用现有的删除逻辑
      break
  }
}

// 快捷操作
const quickActions = computed<WelcomeAction[]>(() => {
  if (hasProjects.value) {
    return [
      {
        icon: 'folder-plus',
        title: '导入新项目',
        description: '添加另一个项目到工作区',
        action: () => uiStore.openProjectCreateModal(),
        shortcut: '⌘N'
      },
      {
        icon: 'settings',
        title: '配置智能体',
        description: '设置 API 密钥和智能体配置',
        action: () => router.push('/settings'),
        shortcut: '⌘,'
      }
    ]
  }
  return [
    {
      icon: 'folder-plus',
      title: '导入项目',
      description: '从本地目录导入现有项目',
      action: () => uiStore.openProjectCreateModal(),
      shortcut: '⌘N'
    },
    {
      icon: 'settings',
      title: '配置智能体',
      description: '设置 API 密钥和智能体配置',
      action: () => router.push('/settings'),
      shortcut: '⌘,'
    },
    {
      icon: 'book-open',
      title: '使用文档',
      description: '查看详细的使用指南',
      action: () => openExternalLink('https://github.com/anthropics/claude-code'),
      shortcut: ''
    }
  ]
})

// 特性列表（仅在无项目时显示）
const features: WelcomeFeature[] = [
  {
    icon: 'sparkles',
    title: '智能对话',
    description: '与 Claude AI 进行自然语言对话'
  },
  {
    icon: 'folder-tree',
    title: '项目管理',
    description: '轻松管理多个项目'
  },
  {
    icon: 'git-branch',
    title: '版本控制',
    description: '集成 Git 操作'
  },
  {
    icon: 'terminal',
    title: '终端集成',
    description: '执行命令行操作'
  }
]

// 选择项目
function selectProject(projectId: string) {
  projectStore.setCurrentProject(projectId)
}

// 打开外部链接
function openExternalLink(url: string) {
  window.open(url, '_blank')
}

// 格式化时间
function formatTime(dateStr: string) {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))

  if (days === 0) return '今天'
  if (days === 1) return '昨天'
  if (days < 7) return `${days}天前`
  return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
}

// 项目创建成功处理
async function handleProjectSubmit(data: { name: string; path: string; description?: string }) {
  try {
    const newProject = await projectStore.createProject(data)
    uiStore.closeProjectCreateModal()
    // 自动选中新创建的项目
    projectStore.setCurrentProject(newProject.id)
  } catch (error) {
    console.error('Failed to create project:', error)
  }
}
</script>

<template>
  <div class="welcome-page">
    <WelcomeBackground />

    <div
      class="welcome-content"
      :class="{ 'welcome-content--loaded': isLoaded }"
    >
      <template v-if="hasProjects">
        <WelcomeProjectBrowser
          :recent-projects="recentProjects"
          :sorted-projects="sortedProjects"
          :quick-actions="quickActions"
          :format-time="formatTime"
          @select-project="selectProject"
          @project-context-menu="showProjectContextMenu"
        />
      </template>

      <template v-else>
        <WelcomeEmptyState
          :quick-actions="quickActions"
          :features="features"
        />
      </template>
    </div>

    <!-- 项目创建弹窗 -->
    <EaModal
      :visible="uiStore.projectCreateModalVisible"
      :width="480"
      @update:visible="(v) => !v && uiStore.closeProjectCreateModal()"
    >
      <ProjectCreateModal
        @submit="handleProjectSubmit"
        @cancel="uiStore.closeProjectCreateModal()"
      />
    </EaModal>

    <!-- 右键菜单 -->
    <Teleport to="body">
      <div
        v-if="showContextMenuFlag"
        class="context-menu"
        :style="{ left: `${contextMenuX}px`, top: `${contextMenuY}px` }"
        @click.stop
      >
        <div
          v-for="option in contextMenuOptions"
          :key="option.key || option.type"
          :class="['context-menu__item', { 'context-menu__divider': option.type === 'divider' }]"
          @click="option.key && handleContextMenuSelect(option.key)"
        >
          {{ option.label }}
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.welcome-page {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background-color: var(--color-bg-primary);
}

/* ========== 主内容 ========== */
.welcome-content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--spacing-8);
  max-width: 800px;
  width: 100%;
}

.welcome-content > * {
  opacity: 0;
  transform: translateY(20px);
  transition:
    opacity 0.6s var(--easing-out),
    transform 0.6s var(--easing-out);
  transition-delay: var(--delay, 0s);
}

.welcome-content--loaded > * {
  opacity: 1;
  transform: translateY(0);
}
/* 右键菜单样式 */
.context-menu {
  position: fixed;
  z-index: 1000;
  min-width: 160px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg);
  padding: var(--spacing-1);
}

.context-menu__item {
  padding: var(--spacing-2) var(--spacing-3);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: background-color var(--transition-fast);
}

.context-menu__item:hover {
  background: var(--color-surface-hover);
}

.context-menu__divider {
  height: 1px;
  background: var(--color-border);
  margin: var(--spacing-1) 0;
}

@media (max-width: 640px) {
  .welcome-content {
    padding: var(--spacing-4);
  }
}
</style>
