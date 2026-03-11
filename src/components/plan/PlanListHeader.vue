<script setup lang="ts">
import type { ProjectOption } from './planListShared'

defineProps<{
  visiblePlanCount: number
  selectedProjectId: string | null
  selectedProjectPath: string
  projectOptions: ProjectOption[]
}>()

const emit = defineEmits<{
  create: []
  'update:selectedProjectId': [value: string]
}>()
</script>

<template>
  <div class="list-header">
    <div class="list-header-top">
      <h3 class="title">
        <span class="title-icon">📋</span>
        计划列表
        <span
          v-if="visiblePlanCount > 0"
          class="title-count"
        >
          {{ visiblePlanCount }} 项
        </span>
      </h3>
      <button
        class="btn-create"
        title="新建计划"
        @click="emit('create')"
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M12 5v14M5 12h14" />
        </svg>
      </button>
    </div>

    <div class="project-switcher">
      <div class="project-switcher-meta">
        <span class="project-switcher-label">当前项目</span>
        <span
          v-if="selectedProjectPath"
          class="project-switcher-path"
          :title="selectedProjectPath"
        >
          {{ selectedProjectPath }}
        </span>
      </div>
      <div class="project-switcher-control">
        <select
          :value="selectedProjectId || ''"
          class="project-switcher-select"
          :title="selectedProjectPath || '请选择项目'"
          :disabled="projectOptions.length === 0"
          @change="emit('update:selectedProjectId', ($event.target as HTMLSelectElement).value)"
        >
          <option
            value=""
            disabled
          >
            请选择项目
          </option>
          <option
            v-for="option in projectOptions"
            :key="option.value"
            :value="option.value"
          >
            {{ option.label }}
          </option>
        </select>
        <span class="project-switcher-chevron">
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M6 9l6 6 6-6" />
          </svg>
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.list-header {
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  padding: 0.75rem 0.875rem;
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  background: linear-gradient(180deg, #ffffff 0%, #f8fafc 100%);
}

.list-header-top {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.title {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
  margin: 0;
  color: var(--color-text-primary, #1e293b);
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-semibold, 600);
}

.title-icon {
  font-size: 1rem;
}

.title-count {
  display: inline-flex;
  align-items: center;
  height: 1.125rem;
  padding: 0 0.375rem;
  border: 1px solid #e2e8f0;
  border-radius: var(--radius-full, 9999px);
  background-color: #eef2ff;
  color: var(--color-text-secondary, #64748b);
  font-size: 0.625rem;
  font-weight: var(--font-weight-medium, 500);
}

.btn-create {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid #bfdbfe;
  border-radius: var(--radius-md, 8px);
  background-color: #eff6ff;
  color: #3b82f6;
  cursor: pointer;
  transition: all var(--transition-fast, 150ms) var(--easing-default);
}

.btn-create:hover {
  transform: translateY(-1px);
  border-color: #93c5fd;
  background-color: #dbeafe;
  color: #2563eb;
}

.project-switcher {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  width: 100%;
  padding: 0.625rem;
  border: 1px solid #e2e8f0;
  border-radius: var(--radius-md, 8px);
  background-color: #ffffff;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.7);
}

.project-switcher-meta {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  width: 100%;
}

.project-switcher-label {
  color: var(--color-text-secondary, #64748b);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
}

.project-switcher-path {
  max-width: 100%;
  overflow: hidden;
  color: #94a3b8;
  font-size: 0.6875rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-switcher-control {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
}

.project-switcher-select {
  width: 100%;
  min-width: 0;
  height: 2rem;
  padding: 0 2rem 0 0.625rem;
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  background-color: var(--color-surface, #fff);
  color: var(--color-text-primary, #1e293b);
  font-size: var(--font-size-xs, 12px);
  cursor: pointer;
  appearance: none;
  transition: all var(--transition-fast, 150ms);
}

.project-switcher-select:focus {
  outline: none;
  border-color: var(--color-primary, #60a5fa);
  box-shadow: 0 0 0 3px var(--color-primary-light, #dbeafe);
}

.project-switcher-select:disabled {
  cursor: not-allowed;
  opacity: 0.6;
  background-color: #f8fafc;
}

.project-switcher-chevron {
  position: absolute;
  right: 0.625rem;
  top: 50%;
  transform: translateY(-50%);
  color: #94a3b8;
  pointer-events: none;
}

[data-theme='dark'] .list-header {
  border-bottom-color: rgba(148, 163, 184, 0.16);
  background: linear-gradient(180deg, rgba(15, 23, 42, 0.96) 0%, rgba(15, 23, 42, 0.9) 100%);
}

[data-theme='dark'] .title,
[data-theme='dark'] .project-switcher-label {
  color: #f8fafc;
}

[data-theme='dark'] .title-count {
  border-color: rgba(96, 165, 250, 0.18);
  background-color: rgba(30, 64, 175, 0.24);
  color: #bfdbfe;
}

[data-theme='dark'] .btn-create {
  border-color: rgba(96, 165, 250, 0.26);
  background-color: rgba(30, 64, 175, 0.18);
  color: #bfdbfe;
}

[data-theme='dark'] .btn-create:hover {
  border-color: rgba(96, 165, 250, 0.4);
  background-color: rgba(30, 64, 175, 0.32);
  color: #eff6ff;
}

[data-theme='dark'] .project-switcher {
  border-color: rgba(148, 163, 184, 0.16);
  background-color: rgba(15, 23, 42, 0.72);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
}

[data-theme='dark'] .project-switcher-path,
[data-theme='dark'] .project-switcher-chevron {
  color: rgba(148, 163, 184, 0.78);
}

[data-theme='dark'] .project-switcher-select {
  border-color: rgba(148, 163, 184, 0.18);
  background-color: rgba(15, 23, 42, 0.92);
  color: #f8fafc;
}

[data-theme='dark'] .project-switcher-select:focus {
  border-color: rgba(96, 165, 250, 0.48);
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.18);
}

[data-theme='dark'] .project-switcher-select:disabled {
  background-color: rgba(30, 41, 59, 0.72);
}
</style>
