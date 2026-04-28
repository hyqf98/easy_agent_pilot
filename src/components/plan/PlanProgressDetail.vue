<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfirmDialog } from '@/composables'
import { useAgentStore } from '@/stores/agent'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { usePlanStore } from '@/stores/plan'
import { useTaskExecutionStore } from '@/stores/taskExecution'
import { useTaskStore } from '@/stores/task'
import type { PlanExecutionProgress, PlanExecutionTaskProgress } from '@/types/taskExecution'
import {
  buildPlanExecutionSnapshot,
  resolvePlanTaskAgentSelection
} from '@/utils/planExecutionProgress'

const props = defineProps<{
  planId: string
}>()

const emit = defineEmits<{
  (e: 'task-select', taskId: string): void
}>()

const taskExecutionStore = useTaskExecutionStore()
const taskStore = useTaskStore()
const planStore = usePlanStore()
const agentStore = useAgentStore()
const agentTeamsStore = useAgentTeamsStore()
const confirmDialog = useConfirmDialog()
const { t, locale } = useI18n()

const isLoading = ref(false)
const isClearing = ref(false)
const progress = ref<PlanExecutionProgress | null>(null)

const plan = computed(() => planStore.plans.find(item => item.id === props.planId) || null)

const snapshot = computed(() =>
  buildPlanExecutionSnapshot(progress.value, plan.value?.currentTaskId ?? null)
)

const summaryStats = computed(() => ({
  total: snapshot.value.totalTasks,
  pending: progress.value?.pending_count ?? 0,
  inProgress: progress.value?.in_progress_count ?? 0,
  completed: progress.value?.completed_count ?? 0,
  blocked: progress.value?.blocked_count ?? 0,
  failed: snapshot.value.failedTasks.length,
  cancelled: progress.value?.cancelled_count ?? 0
}))

const statusCards = computed(() => [
  { key: 'total', label: t('taskBoard.planOverview.totalTasks'), value: summaryStats.value.total, tone: 'neutral' },
  { key: 'pending', label: t('taskBoard.planOverview.pendingTasks'), value: summaryStats.value.pending, tone: 'pending' },
  { key: 'in_progress', label: t('taskBoard.planOverview.inProgressTasks'), value: summaryStats.value.inProgress, tone: 'progress' },
  { key: 'completed', label: t('taskBoard.columns.completed'), value: summaryStats.value.completed, tone: 'success' },
  { key: 'blocked', label: t('taskBoard.planOverview.blockedTasks'), value: summaryStats.value.blocked, tone: 'warning' },
  { key: 'failed', label: t('taskBoard.planOverview.statusFailed'), value: summaryStats.value.failed, tone: 'danger' }
])

const failureTasks = computed(() =>
  snapshot.value.failedTasks
    .map(task => ({
      id: task.task_id,
      title: task.title,
      reason: task.last_fail_reason || task.last_result_summary || t('taskBoard.planOverview.noFailureReason')
    }))
)

const overviewContent = computed(() =>
  progress.value?.execution_overview?.trim()
  || plan.value?.executionOverview?.trim()
  || ''
)

const overviewUpdatedAt = computed(() =>
  progress.value?.execution_overview_updated_at
  || plan.value?.executionOverviewUpdatedAt
  || null
)

function formatPlanStatus(status?: string): string {
  switch (status) {
    case 'draft': return t('taskBoard.planOverview.planStatuses.draft')
    case 'planning': return t('taskBoard.planOverview.planStatuses.planning')
    case 'ready': return t('taskBoard.planOverview.planStatuses.ready')
    case 'executing': return t('taskBoard.planOverview.planStatuses.executing')
    case 'completed': return t('taskBoard.planOverview.planStatuses.completed')
    case 'paused': return t('taskBoard.planOverview.planStatuses.paused')
    default: return status || t('taskBoard.planOverview.planStatuses.unknown')
  }
}

function formatTaskStatus(status: string): string {
  switch (status) {
    case 'pending': return t('taskDetail.statuses.pending')
    case 'in_progress': return t('taskDetail.statuses.in_progress')
    case 'completed': return t('taskDetail.statuses.completed')
    case 'blocked': return t('taskDetail.statuses.blocked')
    case 'failed': return t('taskDetail.statuses.failed')
    case 'cancelled': return t('taskDetail.statuses.cancelled')
    default: return status
  }
}

function formatRelativeTime(date: string | null | undefined): string {
  if (!date) return t('common.none')

  const target = new Date(date)
  if (Number.isNaN(target.getTime())) return t('common.none')

  const diff = Date.now() - target.getTime()
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)

  if (minutes <= 0) return t('common.justNow')
  if (minutes < 60) return t('common.minutesAgo', { n: minutes })
  if (hours < 24) return t('common.hoursAgo', { n: hours })
  if (days < 7) return t('common.daysAgo', { n: days })

  return target.toLocaleString(locale.value === 'zh-CN' ? 'zh-CN' : 'en-US', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

function compactText(content: string | null | undefined, fallback: string = ''): string {
  const normalized = (content || '').replace(/\s+/g, ' ').trim()
  if (!normalized) return fallback
  return normalized.length > 96 ? `${normalized.slice(0, 96)}...` : normalized
}

function resolveAgentLabel(task: PlanExecutionTaskProgress): string {
  const selection = resolvePlanTaskAgentSelection(task, plan.value)
  const expert = selection.expertId
    ? agentTeamsStore.getExpertById(selection.expertId)
    : null
  if (expert) {
    return selection.modelId ? `${expert.name} / ${selection.modelId}` : expert.name
  }
  if (!selection.agentId) {
    return t('taskDetail.unspecified')
  }

  const agent = agentStore.agents.find(item => item.id === selection.agentId)
  const baseLabel = agent?.name || selection.agentId
  return selection.modelId ? `${baseLabel} / ${selection.modelId}` : baseLabel
}

const taskRows = computed(() =>
  snapshot.value.orderedTasks.map(task => ({
    id: task.task_id,
    title: task.title,
    statusLabel: formatTaskStatus(task.status),
    statusClass: `status-${task.status}`,
    agentLabel: resolveAgentLabel(task),
    summary: compactText(
      task.last_result_summary,
      task.status === 'completed'
        ? t('taskBoard.planOverview.noSummaryCompleted')
        : t('taskBoard.planOverview.noResult')
    ),
    failReason: compactText(task.last_fail_reason, ''),
    updatedAt: formatRelativeTime(task.updated_at),
    isActive: snapshot.value.activeTask?.task_id === task.task_id
  }))
)

async function loadProgress() {
  if (!props.planId) return
  isLoading.value = true
  try {
    progress.value = await taskExecutionStore.getPlanExecutionProgress(props.planId)
  } finally {
    isLoading.value = false
  }
}

async function handleClearLogs() {
  const confirmed = await confirmDialog.danger(
    t('taskBoard.planOverview.clearProgressConfirm'),
    t('taskBoard.planOverview.clearProgressTitle')
  )

  if (!confirmed) return

  isClearing.value = true
  try {
    await taskExecutionStore.clearPlanExecutionResults(props.planId)
    await loadProgress()
  } finally {
    isClearing.value = false
  }
}

watch(
  () => props.planId,
  () => {
    void loadProgress()
  }
)

watch(
  () => [
    plan.value?.currentTaskId,
    plan.value?.updatedAt,
    taskStore.tasks
      .filter(task => task.planId === props.planId)
      .map(task => `${task.id}:${task.status}:${task.updatedAt}:${task.agentId || ''}:${task.modelId || ''}`)
      .join('|')
  ],
  () => {
    void loadProgress()
  }
)

onMounted(async () => {
  if (agentStore.agents.length === 0) {
    await agentStore.loadAgents()
  }
  if (agentTeamsStore.experts.length === 0) {
    await agentTeamsStore.loadExperts()
  }
  await loadProgress()
})
</script>

<template>
  <div class="plan-progress-detail">
    <div class="detail-header">
      <div>
        <h4 class="detail-title">
          {{ t('taskBoard.planOverview.detailTitle') }}
        </h4>
        <p class="detail-subtitle">
          {{ plan?.name || t('taskBoard.planOverview.unnamedPlan') }}
        </p>
      </div>
      <div class="header-actions">
        <button
          class="btn-action"
          :disabled="isLoading"
          @click="loadProgress"
        >
          {{ t('common.refresh') }}
        </button>
        <button
          :disabled="isLoading || isClearing || summaryStats.total === 0"
          @click="handleClearLogs"
        >
          {{ t('taskBoard.planOverview.clearProgress') }}
        </button>
      </div>
    </div>

    <div class="detail-content">
      <div class="summary-panel">
        <div class="summary-meta">
          <span class="meta-chip">{{ formatPlanStatus(plan?.status) }}</span>
          <span class="meta-chip meta-chip--muted">
            {{
              plan?.executionStatus === 'running'
                ? t('taskBoard.planOverview.executionStatuses.running')
                : plan?.executionStatus === 'completed'
                  ? t('taskBoard.planOverview.executionStatuses.completed')
                  : t('taskBoard.planOverview.executionStatuses.idle')
            }}
          </span>
          <span class="meta-chip meta-chip--muted">{{ t('taskBoard.planOverview.updatedAt', { time: formatRelativeTime(plan?.updatedAt) }) }}</span>
        </div>

        <div
          v-if="snapshot.activeTask"
          class="active-task-card"
        >
          <div class="active-task-card__label">
            {{ t('taskBoard.planOverview.currentPosition') }}
          </div>
          <div class="active-task-card__title">
            {{ t('taskBoard.planOverview.currentTaskProgress', { current: snapshot.currentTaskIndex, total: snapshot.totalTasks, title: snapshot.activeTask.title }) }}
          </div>
          <div class="active-task-card__hint">
            {{ t('taskBoard.planOverview.currentStatus', { status: formatTaskStatus(snapshot.activeTask.status) }) }}
          </div>
        </div>

        <div
          v-else
          class="active-task-card active-task-card--empty"
        >
          <div class="active-task-card__label">
            {{ t('taskBoard.planOverview.currentPosition') }}
          </div>
          <div class="active-task-card__title">
            {{ t('taskBoard.planOverview.noActiveTask') }}
          </div>
          <div class="active-task-card__hint">
            {{ t('taskBoard.planOverview.taskCardHint') }}
          </div>
        </div>

        <div class="stats-grid">
          <div
            v-for="card in statusCards"
            :key="card.key"
            class="stat-card"
            :class="`stat-card--${card.tone}`"
          >
            <span class="stat-card__label">{{ card.label }}</span>
            <span class="stat-card__value">{{ card.value }}</span>
          </div>
        </div>
      </div>

      <div class="overview-panel">
        <div class="overview-panel__head">
          <h5>{{ t('taskBoard.planOverview.title') }}</h5>
          <span>{{ overviewUpdatedAt ? t('taskBoard.planOverview.updatedAt', { time: formatRelativeTime(overviewUpdatedAt) }) : t('common.none') }}</span>
        </div>
        <div
          v-if="overviewContent"
          class="overview-panel__content"
        >
          {{ overviewContent }}
        </div>
        <div
          v-else
          class="overview-panel__empty"
        />
      </div>

      <div
        v-if="isLoading"
        class="placeholder"
      >
        {{ t('taskBoard.planOverview.loadingProgress') }}
      </div>

      <template v-else>
        <div
          v-if="taskRows.length === 0"
          class="placeholder"
        >
          {{ t('taskBoard.planOverview.emptyTasks') }}
        </div>

        <div
          v-else
          class="detail-section"
        >
          <div class="section-header">
            <h5>{{ t('taskBoard.planOverview.progressTitle') }}</h5>
            <span>{{ t('taskBoard.planOverview.itemCount', { count: taskRows.length }) }}</span>
          </div>

          <div class="task-list">
            <button
              v-for="task in taskRows"
              :key="task.id"
              class="task-row"
              :class="{ 'task-row--active': task.isActive }"
              @click="emit('task-select', task.id)"
            >
              <div class="task-row__main">
                <div class="task-row__title-line">
                  <span class="task-row__title">{{ task.title }}</span>
                  <span
                    class="task-row__status"
                    :class="task.statusClass"
                  >{{ task.statusLabel }}</span>
                </div>
                <div class="task-row__summary">
                  {{ task.summary }}
                </div>
                <div
                  v-if="task.failReason"
                  class="task-row__failure"
                >
                  {{ t('taskBoard.planOverview.failureReason', { reason: task.failReason }) }}
                </div>
              </div>
              <div class="task-row__meta">
                <span
                  class="task-row__agent"
                  :title="task.agentLabel"
                >{{ task.agentLabel }}</span>
                <span class="task-row__time">{{ task.updatedAt }}</span>
              </div>
            </button>
          </div>
        </div>

        <div
          v-if="failureTasks.length > 0"
          class="detail-section"
        >
          <div class="section-header">
            <h5>{{ t('taskBoard.planOverview.failureTitle') }}</h5>
            <span>{{ t('taskBoard.planOverview.itemCount', { count: failureTasks.length }) }}</span>
          </div>
          <div class="failure-list">
            <div
              v-for="item in failureTasks"
              :key="item.id"
              class="failure-item"
            >
              <div class="failure-item__title">
                {{ item.title }}
              </div>
              <div class="failure-item__reason">
                {{ compactText(item.reason) }}
              </div>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.plan-progress-detail {
  container-type: inline-size;
  container-name: plan-progress-detail;
  display: flex;
  flex-direction: column;
  height: 100%;
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--color-surface, #fff) 98%, #ffffff),
      color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 92%, var(--color-surface, #fff))
    );
}

.detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.9rem 1rem;
  border-bottom: 1px solid color-mix(in srgb, var(--color-border, #e2e8f0) 72%, transparent);
}

.detail-header > div:first-child {
  min-width: 0;
  flex: 1;
}

.detail-title {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--color-text-primary, #0f172a);
}

.detail-subtitle {
  margin: 0.2rem 0 0;
  font-size: 0.78rem;
  color: var(--color-text-secondary, #64748b);
  word-break: break-word;
}

.header-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.5rem;
}

.btn-action {
  border: 1px solid color-mix(in srgb, var(--color-border, #e2e8f0) 70%, transparent);
  background: color-mix(in srgb, var(--color-surface, #fff) 96%, #ffffff);
  color: var(--color-text-secondary, #475569);
  border-radius: 0.7rem;
  padding: 0.45rem 0.75rem;
  cursor: pointer;
}

.btn-action:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-action--danger {
  color: var(--color-error, #dc2626);
  border-color: color-mix(in srgb, var(--color-error, #dc2626) 22%, transparent);
}

.detail-content {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.summary-panel,
.detail-section,
.overview-panel {
  border: 1px solid color-mix(in srgb, var(--color-border, #e2e8f0) 68%, transparent);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--color-surface, #fff) 94%, #ffffff);
  box-shadow: 0 14px 30px rgba(15, 23, 42, 0.06);
  padding: 0.95rem;
}

.summary-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-bottom: 0.85rem;
}

.meta-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border-radius: 999px;
  padding: 0.28rem 0.68rem;
  background: rgba(14, 165, 233, 0.1);
  color: var(--color-primary, #0284c7);
  font-size: 0.72rem;
  font-weight: 600;
}

.meta-chip--muted {
  background: color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 86%, var(--color-surface, #fff));
  color: var(--color-text-secondary, #64748b);
}

.active-task-card {
  border-radius: 0.9rem;
  padding: 0.85rem 0.95rem;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.12), rgba(14, 165, 233, 0.08));
  border: 1px solid rgba(59, 130, 246, 0.16);
}

.active-task-card--empty {
  background: color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 80%, var(--color-surface, #fff));
  border-color: color-mix(in srgb, var(--color-border, #e2e8f0) 72%, transparent);
}

.active-task-card__label {
  font-size: 0.72rem;
  color: var(--color-text-secondary, #64748b);
  margin-bottom: 0.35rem;
}

.active-task-card__title {
  font-size: 0.9rem;
  font-weight: 700;
  line-height: 1.45;
  color: var(--color-text-primary, #0f172a);
  word-break: break-word;
}

.active-task-card__hint {
  margin-top: 0.35rem;
  font-size: 0.78rem;
  color: var(--color-text-secondary, #64748b);
}

.stats-grid {
  margin-top: 0.85rem;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.65rem;
}

.stat-card {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.75rem 0.8rem;
  border-radius: 0.85rem;
  background: color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 84%, var(--color-surface, #fff));
}

.stat-card__label {
  font-size: 0.72rem;
  color: var(--color-text-secondary, #64748b);
}

.stat-card__value {
  font-size: 1.15rem;
  font-weight: 700;
  color: var(--color-text-primary, #0f172a);
}

.stat-card--success .stat-card__value { color: #16a34a; }
.stat-card--danger .stat-card__value { color: #dc2626; }
.stat-card--warning .stat-card__value { color: #d97706; }
.stat-card--progress .stat-card__value { color: #2563eb; }
.stat-card--pending .stat-card__value { color: #475569; }

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.8rem;
}

.section-header h5 {
  margin: 0;
  font-size: 0.84rem;
  font-weight: 700;
  color: var(--color-text-primary, #0f172a);
}

.section-header span {
  font-size: 0.72rem;
  color: var(--color-text-secondary, #64748b);
}

.task-list,
.failure-list {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.overview-panel {
  padding: 1rem;
}

.overview-panel__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}

.overview-panel__head h5 {
  margin: 0;
  font-size: 0.84rem;
  font-weight: 700;
  color: var(--color-text-primary, #0f172a);
}

.overview-panel__head span {
  font-size: 0.72rem;
  color: var(--color-text-secondary, #64748b);
}

.overview-panel__content {
  margin: 0;
  font-size: 0.83rem;
  line-height: 1.75;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--color-text-primary, #0f172a);
}

.overview-panel__empty {
  min-height: 84px;
  display: flex;
  align-items: center;
  color: var(--color-text-secondary, #64748b);
  font-size: 0.8rem;
  line-height: 1.6;
}

.task-row {
  display: flex;
  flex-direction: column;
  gap: 0.72rem;
  width: 100%;
  border: 1px solid color-mix(in srgb, var(--color-border, #e2e8f0) 66%, transparent);
  background: color-mix(in srgb, var(--color-surface, #fff) 94%, #ffffff);
  border-radius: 0.9rem;
  padding: 0.85rem;
  text-align: left;
  cursor: pointer;
}

.task-row--active {
  border-color: color-mix(in srgb, var(--color-primary, #3b82f6) 34%, transparent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary, #3b82f6) 10%, transparent);
}

.task-row__main {
  min-width: 0;
  flex: 1;
}

.task-row__title-line {
  display: flex;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.task-row__title {
  min-width: 0;
  flex: 1 1 12rem;
  font-size: 0.84rem;
  font-weight: 700;
  line-height: 1.5;
  color: var(--color-text-primary, #0f172a);
  word-break: break-word;
}

.task-row__status {
  flex-shrink: 0;
  padding: 0.16rem 0.5rem;
  border-radius: 999px;
  font-size: 0.68rem;
  font-weight: 700;
}

.status-pending { background: #e2e8f0; color: #475569; }
.status-in_progress { background: #dbeafe; color: #2563eb; }
.status-completed { background: #dcfce7; color: #15803d; }
.status-blocked { background: #fef3c7; color: #b45309; }
.status-failed { background: #fee2e2; color: #b91c1c; }
.status-cancelled { background: #f1f5f9; color: #64748b; }

.task-row__summary,
.task-row__failure,
.failure-item__reason {
  margin-top: 0.45rem;
  font-size: 0.76rem;
  line-height: 1.55;
  color: var(--color-text-secondary, #475569);
}

.task-row__failure,
.failure-item__reason {
  color: var(--color-error, #dc2626);
}

.task-row__meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.4rem 0.75rem;
  min-width: 0;
  font-size: 0.72rem;
  color: var(--color-text-tertiary, #94a3b8);
}

.task-row__agent {
  min-width: 0;
  flex: 1 1 10rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-row__time {
  flex-shrink: 0;
}

.failure-item {
  border-radius: 0.85rem;
  padding: 0.75rem 0.85rem;
  background: color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 84%, var(--color-surface, #fff));
}

.failure-item__title {
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--color-text-primary, #0f172a);
}

.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 160px;
  border-radius: 1rem;
  border: 1px dashed color-mix(in srgb, var(--color-border, #e2e8f0) 70%, transparent);
  color: var(--color-text-secondary, #64748b);
  background: color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 78%, var(--color-surface, #fff));
}

.placeholder--compact {
  min-height: 96px;
  padding: 0.9rem 1rem;
  text-align: center;
}

[data-theme='dark'] .plan-progress-detail {
  background:
    linear-gradient(
      180deg,
      rgba(15, 23, 42, 0.96),
      rgba(2, 6, 23, 0.94)
    );
}

[data-theme='dark'] .detail-header,
[data-theme='dark'] .summary-panel,
[data-theme='dark'] .detail-section,
[data-theme='dark'] .overview-panel,
[data-theme='dark'] .task-row,
[data-theme='dark'] .failure-item,
[data-theme='dark'] .placeholder {
  border-color: rgba(148, 163, 184, 0.16);
}

[data-theme='dark'] .summary-panel,
[data-theme='dark'] .detail-section,
[data-theme='dark'] .overview-panel,
[data-theme='dark'] .task-row,
[data-theme='dark'] .failure-item {
  background: rgba(15, 23, 42, 0.72);
  box-shadow: 0 18px 36px rgba(2, 6, 23, 0.28);
}

[data-theme='dark'] .detail-title,
[data-theme='dark'] .active-task-card__title,
[data-theme='dark'] .section-header h5,
[data-theme='dark'] .overview-panel__head h5,
[data-theme='dark'] .task-row__title,
[data-theme='dark'] .failure-item__title {
  color: #f8fafc;
}

[data-theme='dark'] .detail-subtitle,
[data-theme='dark'] .meta-chip--muted,
[data-theme='dark'] .active-task-card__hint,
[data-theme='dark'] .overview-panel__head span,
[data-theme='dark'] .overview-panel__content,
[data-theme='dark'] .overview-panel__empty,
[data-theme='dark'] .task-row__summary,
[data-theme='dark'] .task-row__meta,
[data-theme='dark'] .section-header span,
[data-theme='dark'] .placeholder {
  color: rgba(226, 232, 240, 0.72);
}

[data-theme='dark'] .btn-action {
  background: rgba(15, 23, 42, 0.76);
  color: rgba(226, 232, 240, 0.88);
}

[data-theme='dark'] .active-task-card {
  background: linear-gradient(135deg, rgba(37, 99, 235, 0.22), rgba(14, 165, 233, 0.12));
  border-color: rgba(96, 165, 250, 0.24);
}

[data-theme='dark'] .active-task-card--empty,
[data-theme='dark'] .stat-card,
[data-theme='dark'] .placeholder {
  background: rgba(30, 41, 59, 0.58);
}

@container plan-progress-detail (max-width: 430px) {
  .detail-header {
    flex-direction: column;
  }

  .header-actions {
    width: 100%;
    justify-content: stretch;
  }

  .btn-action {
    flex: 1 1 0;
    text-align: center;
  }

  .stats-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .overview-panel__head {
    flex-direction: column;
    align-items: flex-start;
  }

  .task-row__meta {
    flex-direction: column;
    align-items: flex-start;
  }

  .task-row__agent {
    width: 100%;
    flex-basis: auto;
  }
}
</style>
