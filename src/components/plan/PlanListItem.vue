<script setup lang="ts">
import type { PlanListItemViewModel } from './planListShared'

defineProps<{
  item: PlanListItemViewModel
}>()

const emit = defineEmits<{
  select: []
  split: []
  edit: []
  delete: []
}>()
</script>

<template>
  <div
    class="plan-item"
    :class="{ active: item.isActive }"
    @click="emit('select')"
  >
    <div
      class="plan-status-bar"
      :class="item.statusColor"
    />
    <div class="plan-info">
      <div class="plan-name-row">
        <span class="plan-name">{{ item.plan.name }}</span>
        <span
          class="plan-status-chip"
          :class="item.statusColor"
        >{{ item.statusLabel }}</span>
        <span
          v-if="item.plan.scheduleStatus === 'scheduled'"
          class="plan-schedule-chip"
          :title="'定时计划: ' + new Date(item.plan.scheduledAt || '').toLocaleString('zh-CN')"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <circle
              cx="12"
              cy="12"
              r="10"
            />
            <polyline points="12,6 12,12 16,14" />
          </svg>
          {{ item.scheduledLabel }}
        </span>
      </div>
      <span
        v-if="item.plan.description"
        class="plan-desc"
      >{{ item.plan.description }}</span>
      <span class="plan-time">{{ item.relativeTimeLabel }}</span>
      <div class="plan-metrics">
        <span
          class="plan-metric split"
          title="已拆分任务总数"
        >拆分 {{ item.taskStats.total }}</span>
        <span
          class="plan-metric queue"
          title="待执行和执行中的任务数量"
        >执行列表 {{ item.taskStats.executionQueue }}</span>
        <span
          class="plan-metric done"
          title="已完成任务数量"
        >完成 {{ item.taskStats.completed }}</span>
        <span
          class="plan-metric failed"
          title="执行失败或已取消任务数量"
        >失败 {{ item.taskStats.failed }}</span>
      </div>
    </div>
    <div class="plan-actions">
      <button
        v-if="item.canSplit"
        class="btn-action btn-split"
        title="拆分任务"
        @click.stop="emit('split')"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M16 3h5v5M4 20L21 3M21 16v5h-5M15 15l6 6M4 4l5 5" />
        </svg>
      </button>
      <button
        v-if="item.canResumeSplit"
        class="btn-action btn-resume-split"
        title="继续拆分"
        @click.stop="emit('split')"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M21 12a9 9 0 1 1-3.35-6.94" />
          <path d="M21 3v6h-6" />
        </svg>
      </button>
      <button
        v-if="item.canEdit"
        class="btn-action btn-edit"
        title="编辑"
        @click.stop="emit('edit')"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7" />
          <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
      </button>
      <button
        class="btn-action btn-delete"
        title="删除"
        @click.stop="emit('delete')"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.plan-item {
  display: flex;
  align-items: stretch;
  padding: 0;
  border-radius: var(--radius-md, 8px);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms) var(--easing-default);
  background-color: var(--color-surface, #fff);
  border: 1px solid var(--color-border-light, #f1f5f9);
  overflow: hidden;
}

.plan-item:hover {
  border-color: var(--color-border, #e2e8f0);
  box-shadow: var(--shadow-sm, 0 1px 3px 0 rgb(0 0 0 / 0.1));
}

.plan-item.active {
  border-color: var(--color-primary, #60a5fa);
  box-shadow: 0 0 0 3px var(--color-primary-light, #dbeafe);
}

.plan-status-bar {
  width: 4px;
  flex-shrink: 0;
}

.plan-status-bar.gray { background-color: #94a3b8; }
.plan-status-bar.blue { background-color: #60a5fa; }
.plan-status-bar.green { background-color: #10b981; }
.plan-status-bar.purple { background-color: #8b5cf6; }
.plan-status-bar.orange { background-color: #f59e0b; }
.plan-status-bar.yellow { background-color: #fbbf24; }

.plan-info {
  flex: 1;
  min-width: 0;
  padding: var(--spacing-2, 0.5rem) var(--spacing-3, 0.75rem);
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.plan-name-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.plan-name {
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-primary, #1e293b);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.plan-status-chip {
  flex-shrink: 0;
  border-radius: var(--radius-full, 9999px);
  padding: 0.125rem 0.4rem;
  font-size: 0.625rem;
  font-weight: var(--font-weight-medium, 500);
}

.plan-status-chip.gray {
  color: #64748b;
  background-color: #f1f5f9;
}

.plan-status-chip.blue {
  color: #1d4ed8;
  background-color: #dbeafe;
}

.plan-status-chip.green {
  color: #166534;
  background-color: #dcfce7;
}

.plan-status-chip.purple {
  color: #7c3aed;
  background-color: #f3e8ff;
}

.plan-status-chip.orange {
  color: #b45309;
  background-color: #fef3c7;
}

.plan-status-chip.yellow {
  color: #92400e;
  background-color: #fef3c7;
}

.plan-schedule-chip {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  border-radius: var(--radius-full, 9999px);
  padding: 0.125rem 0.4rem;
  font-size: 0.625rem;
  font-weight: var(--font-weight-medium, 500);
  color: #1d4ed8;
  background-color: #dbeafe;
}

.plan-desc {
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-secondary, #64748b);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.plan-time {
  font-size: 0.6875rem;
  color: var(--color-text-tertiary, #94a3b8);
  margin-top: 0.125rem;
}

.plan-metrics {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
  margin-top: 0.375rem;
}

.plan-metric {
  display: inline-flex;
  align-items: center;
  padding: 0.125rem 0.375rem;
  border-radius: var(--radius-sm, 4px);
  font-size: 0.6875rem;
  font-weight: var(--font-weight-medium, 500);
  line-height: 1.2;
}

.plan-metric.split {
  color: #0f766e;
  background-color: #ccfbf1;
}

.plan-metric.queue {
  color: #1d4ed8;
  background-color: #dbeafe;
}

.plan-metric.done {
  color: #166534;
  background-color: #dcfce7;
}

.plan-metric.failed {
  color: #b91c1c;
  background-color: #fee2e2;
}

.plan-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 0 var(--spacing-1, 0.25rem);
  opacity: 1;
}

.btn-action {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius-sm, 4px);
  background: transparent;
  color: var(--color-text-tertiary, #94a3b8);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-action:hover {
  background-color: var(--color-bg-secondary, #f1f5f9);
}

.btn-split:hover {
  color: var(--color-primary, #3b82f6);
  background-color: var(--color-primary-light, #dbeafe);
}

.btn-resume-split:hover {
  color: #b45309;
  background-color: #fef3c7;
}

[data-theme='dark'] .plan-item {
  background-color: color-mix(in srgb, var(--color-surface) 92%, #0f172a);
  border-color: color-mix(in srgb, var(--color-border) 70%, transparent);
}

[data-theme='dark'] .plan-item:hover {
  border-color: color-mix(in srgb, var(--color-primary) 32%, var(--color-border));
  box-shadow: 0 10px 24px rgba(2, 6, 23, 0.24);
}

[data-theme='dark'] .plan-item.active {
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-primary) 34%, transparent);
}

[data-theme='dark'] .plan-schedule-chip {
  color: #bfdbfe;
  background-color: rgba(30, 64, 175, 0.28);
}

.btn-edit:hover {
  color: var(--color-success, #10b981);
  background-color: var(--color-success-light, #d1fae5);
}

.btn-delete:hover {
  color: var(--color-error, #ef4444);
  background-color: var(--color-error-light, #fee2e2);
}
</style>
