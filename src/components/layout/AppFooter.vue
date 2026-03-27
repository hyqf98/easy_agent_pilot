<script setup lang="ts">
import { computed } from 'vue'
import { useProjectStore } from '@/stores/project'
import { useSessionStore } from '@/stores/session'

const projectStore = useProjectStore()
const sessionStore = useSessionStore()

// 当前项目名称
const currentProjectName = computed(() => {
  const project = projectStore.projects.find(p => p.id === projectStore.currentProjectId)
  return project?.name || ''
})

// 当前会话数量
const sessionCount = computed(() => {
  if (!projectStore.currentProjectId) return 0
  return sessionStore.sessionsByProject(projectStore.currentProjectId).length
})

// 当前智能体状态
const agentStatus = computed(() => {
  const currentSession = sessionStore.sessions.find(s => s.id === sessionStore.currentSessionId)
  if (!currentSession) return 'idle'
  return currentSession.status
})

// 状态颜色类
const statusClass = computed(() => {
  return `app-footer__status--${agentStatus.value}`
})
</script>

<template>
  <footer class="app-footer">
    <div class="app-footer__left">
      <!-- 当前项目 -->
      <span
        v-if="currentProjectName"
        class="app-footer__item"
      >
        <span class="app-footer__label">项目:</span>
        <span class="app-footer__value">{{ currentProjectName }}</span>
      </span>

      <!-- 会话数量 -->
      <span
        v-if="projectStore.currentProjectId"
        class="app-footer__item"
      >
        <span class="app-footer__label">会话:</span>
        <span class="app-footer__value">{{ sessionCount }}</span>
      </span>
    </div>

    <div class="app-footer__center">
      <!-- 状态显示 -->
      <span :class="['app-footer__status', statusClass]">
        <span class="app-footer__status-dot" />
        <span>就绪</span>
      </span>
    </div>

    <div class="app-footer__right">
      <!-- 版本信息 -->
      <span class="app-footer__version">v1.2.0</span>
    </div>
  </footer>
</template>

<style scoped>
.app-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--footer-height);
  padding: 0 var(--spacing-4);
  background-color: var(--color-surface);
  border-top: 1px solid var(--color-border);
  flex-shrink: 0;
}

.app-footer__left,
.app-footer__right {
  display: flex;
  align-items: center;
  gap: var(--spacing-4);
}

.app-footer__center {
  display: flex;
  align-items: center;
}

.app-footer__item {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  font-size: var(--font-size-xs);
}

.app-footer__label {
  color: var(--color-text-tertiary);
}

.app-footer__value {
  color: var(--color-text-secondary);
}

.app-footer__status {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.app-footer__status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: var(--color-text-tertiary);
}

.app-footer__status--idle .app-footer__status-dot {
  background-color: var(--color-text-tertiary);
}

.app-footer__status--running .app-footer__status-dot {
  background-color: var(--color-primary);
  animation: pulse 1.5s infinite;
}

.app-footer__status--completed .app-footer__status-dot {
  background-color: var(--color-success);
}

.app-footer__status--error .app-footer__status-dot {
  background-color: var(--color-error);
}

.app-footer__status--paused .app-footer__status-dot {
  background-color: var(--color-warning);
}

.app-footer__version {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}
</style>
