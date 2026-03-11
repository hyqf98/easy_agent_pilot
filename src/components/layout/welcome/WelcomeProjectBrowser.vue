<script setup lang="ts">
import type { Project } from '@/stores'
import { EaIcon } from '@/components/common'
import type { WelcomeAction } from './welcomeShared'

defineProps<{
  recentProjects: Project[]
  sortedProjects: Project[]
  quickActions: WelcomeAction[]
  formatTime: (dateStr: string) => string
}>()

defineEmits<{
  (e: 'select-project', projectId: string): void
  (e: 'project-context-menu', event: MouseEvent, project: Project): void
}>()
</script>

<template>
  <div class="welcome-project-browser">
    <div class="welcome-header">
      <div class="welcome-logo">
        <div class="welcome-logo__icon">
          <EaIcon
            name="bot"
            :size="40"
          />
        </div>
      </div>
      <h1 class="welcome-title">
        选择项目
      </h1>
      <p class="welcome-subtitle">
        选择一个项目开始工作，或导入新项目
      </p>
    </div>

    <div
      v-if="recentProjects.length > 0"
      class="recent-section"
    >
      <h3 class="section-title">
        最近使用
      </h3>
      <div class="project-grid">
        <div
          v-for="(project, index) in recentProjects"
          :key="project.id"
          class="project-card project-card--recent"
          :style="{ '--delay': `${0.1 + index * 0.05}s` }"
          @click="$emit('select-project', project.id)"
          @contextmenu.prevent="$emit('project-context-menu', $event, project)"
        >
          <div class="project-card__icon">
            <EaIcon
              name="folder"
              :size="24"
            />
          </div>
          <div class="project-card__content">
            <div class="project-card__name">
              {{ project.name }}
            </div>
            <div class="project-card__meta">
              <span class="project-card__path">{{ project.path }}</span>
            </div>
          </div>
          <div class="project-card__arrow">
            <EaIcon
              name="arrow-right"
              :size="16"
            />
          </div>
        </div>
      </div>
    </div>

    <div class="all-projects-section">
      <h3 class="section-title">
        所有项目
      </h3>
      <div class="project-list">
        <div
          v-for="(project, index) in sortedProjects"
          :key="project.id"
          class="project-card"
          :style="{ '--delay': `${0.1 + index * 0.05}s` }"
          @click="$emit('select-project', project.id)"
          @contextmenu.prevent="$emit('project-context-menu', $event, project)"
        >
          <div class="project-card__icon">
            <EaIcon
              name="folder"
              :size="24"
            />
          </div>
          <div class="project-card__content">
            <div class="project-card__name">
              {{ project.name }}
            </div>
            <div class="project-card__meta">
              <span class="project-card__path">{{ project.path }}</span>
              <span class="project-card__time">{{ formatTime(project.updatedAt) }}</span>
            </div>
          </div>
          <div class="project-card__arrow">
            <EaIcon
              name="arrow-right"
              :size="16"
            />
          </div>
        </div>
      </div>
    </div>

    <div class="quick-actions">
      <button
        v-for="(action, index) in quickActions"
        :key="index"
        class="quick-action-btn"
        :style="{ '--delay': `${0.3 + index * 0.05}s` }"
        @click="action.action"
      >
        <EaIcon
          :name="action.icon"
          :size="16"
        />
        <span>{{ action.title }}</span>
        <kbd v-if="action.shortcut">{{ action.shortcut }}</kbd>
      </button>
    </div>
  </div>
</template>

<style scoped>
.welcome-project-browser {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.welcome-header {
  text-align: center;
  margin-bottom: var(--spacing-8);
  --delay: 0s;
}

.welcome-logo {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  margin-bottom: var(--spacing-4);
}

.welcome-logo__icon {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 80px;
  height: 80px;
  background: linear-gradient(
    135deg,
    var(--color-primary) 0%,
    var(--color-accent) 100%
  );
  border-radius: var(--radius-2xl);
  color: white;
  box-shadow:
    0 8px 32px -8px rgba(59, 130, 246, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.2);
}

.welcome-title {
  margin: 0 0 var(--spacing-2);
  font-size: 28px;
  font-weight: 700;
  line-height: 1.2;
  letter-spacing: -0.02em;
  color: var(--color-text-primary);
}

.welcome-subtitle {
  margin: 0;
  font-size: var(--font-size-base);
  color: var(--color-text-secondary);
  font-weight: 400;
}

.recent-section,
.all-projects-section {
  width: 100%;
  max-width: 500px;
}

.recent-section {
  margin-bottom: var(--spacing-8);
}

.all-projects-section {
  margin-top: var(--spacing-2);
}

.section-title {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-3);
}

.project-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--spacing-2);
}

.project-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  width: 100%;
  max-width: 500px;
  margin-bottom: var(--spacing-6);
}

.project-card {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-3) var(--spacing-4);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  cursor: pointer;
  transition:
    background-color var(--transition-fast) var(--easing-default),
    border-color var(--transition-fast) var(--easing-default),
    transform var(--transition-fast) var(--easing-default),
    box-shadow var(--transition-fast) var(--easing-default);
}

.project-card:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-primary);
  transform: translateX(4px);
  box-shadow: var(--shadow-md);
}

.project-card--recent {
  padding: var(--spacing-2) var(--spacing-3);
}

.project-card__icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  background: var(--color-primary-light);
  border-radius: var(--radius-md);
  color: var(--color-primary);
  transition:
    background-color var(--transition-fast) var(--easing-default),
    transform var(--transition-fast) var(--easing-default);
}

.project-card:hover .project-card__icon {
  background: var(--color-primary);
  color: white;
  transform: scale(1.05);
}

[data-theme='dark'] .project-card__icon {
  background: rgba(96, 165, 250, 0.15);
}

[data-theme='dark'] .project-card:hover .project-card__icon {
  background: var(--color-primary);
}

.project-card__content {
  flex: 1;
  min-width: 0;
}

.project-card__name {
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
  margin-bottom: 2px;
}

.project-card--recent .project-card__name {
  font-size: var(--font-size-sm);
}

.project-card__meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.project-card--recent .project-card__meta {
  font-size: var(--font-size-xs);
}

.project-card__path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-family-mono);
}

.project-card__time {
  flex-shrink: 0;
}

.project-card__arrow {
  flex-shrink: 0;
  color: var(--color-text-tertiary);
  opacity: 0;
  transform: translateX(-8px);
  transition:
    opacity var(--transition-fast) var(--easing-default),
    transform var(--transition-fast) var(--easing-default);
}

.project-card:hover .project-card__arrow {
  opacity: 1;
  transform: translateX(0);
  color: var(--color-primary);
}

.quick-actions {
  display: flex;
  gap: var(--spacing-3);
  --delay: 0.3s;
}

.quick-action-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-4);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-full);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition:
    background-color var(--transition-fast) var(--easing-default),
    border-color var(--transition-fast) var(--easing-default),
    color var(--transition-fast) var(--easing-default);
}

.quick-action-btn:hover {
  background: var(--color-primary-light);
  border-color: var(--color-primary);
  color: var(--color-primary);
}

[data-theme='dark'] .quick-action-btn:hover {
  background: rgba(96, 165, 250, 0.15);
}

.quick-action-btn kbd {
  padding: 2px 6px;
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-sm);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
}

@media (max-width: 640px) {
  .welcome-title {
    font-size: 24px;
  }

  .welcome-logo__icon {
    width: 64px;
    height: 64px;
  }

  .project-card__meta {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--spacing-1);
  }

  .quick-actions {
    flex-direction: column;
  }
}
</style>
