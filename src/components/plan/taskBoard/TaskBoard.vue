<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Task } from '@/types/plan'
import KanbanColumn from '../KanbanColumn.vue'
import TaskEditModal from '../taskEditModal/TaskEditModal.vue'
import { useTaskBoard } from './useTaskBoard'
const { t } = useI18n()
const emit = defineEmits<{
  (e: 'task-click', task: Task): void
}>()
 
const {
  showEditModal,
  editingTask,
  showCreateModal,
  showPlanOverview,
  currentPlanId,
  currentPlan,
  isCurrentPlanPaused,
  isManualMode,
  newTaskTemplate,
  tasks,
  tasksByStatus,
  taskStats,
  planOverviewTasks,
  hasPlanResults,
  resolveTaskStatusClass,
  resolveTaskStatusLabel,
  columns,
  handleTaskDrop,
  handleTaskReorder,
  selectTask,
  handleTaskEdit,
  handleTaskStop,
  handleTaskStart,
  handleTaskResume,
  handleTaskRetry,
  handleTaskDelete,
  handleExecuteAll,
  handleStartExecution,
  handleToggleGlobalExecution,
  handleEditSaved,
  openCreateTaskModal,
  handleTaskCreated,
  markPlanAsReady
} = useTaskBoard({ emit })
</script>

<template>
  <div class="task-board">
    <div class="board-header">
      <div class="header-left">
        <h3 class="title">
          {{ t('taskBoard.title') }}
        </h3>
      </div>
      <div class="header-right">
        <button
          v-if="isManualMode && currentPlan?.status === 'planning' && tasks.length > 0"
          class="btn btn-secondary"
          @click="markPlanAsReady"
        >
          {{ t('taskBoard.actions.markSplitReady') }}
        </button>

        <div class="task-stats">
          <span class="stat-item completed">{{ t('taskBoard.stats.completed', { count: taskStats.completed }) }}</span>
          <span class="stat-item in-progress">{{ t('taskBoard.stats.inProgress', { count: taskStats.inProgress }) }}</span>
          <span class="stat-item blocked">{{ t('taskBoard.stats.blocked', { count: taskStats.blocked }) }}</span>
          <span class="stat-item pending">{{ t('taskBoard.stats.pending', { count: taskStats.pending }) }}</span>
          <span class="stat-item failed">{{ t('taskBoard.stats.failed', { count: taskStats.failed }) }}</span>
        </div>
      </div>
    </div>

    <div
      v-if="!currentPlanId"
      class="empty-state"
    >
      <span>{{ t('taskBoard.emptyNoPlan') }}</span>
    </div>

    <template v-else>
      <div
        v-if="hasPlanResults"
        class="plan-overview"
      >
        <div
          class="plan-overview-header"
          @click="showPlanOverview = !showPlanOverview"
        >
          <span class="plan-overview-title">{{ t('taskBoard.planOverview.title') }}</span>
          <div class="plan-overview-stats">
            <span class="ov-stat ov-stat-completed">{{ t('taskBoard.planOverview.completedCount', { count: taskStats.completed }) }}</span>
            <span
              v-if="taskStats.failed > 0"
              class="ov-stat ov-stat-failed"
            >{{ t('taskBoard.planOverview.failedCount', { count: taskStats.failed }) }}</span>
            <span class="ov-stat ov-stat-pending">{{ t('taskBoard.planOverview.pendingCount', { count: taskStats.pending + taskStats.inProgress }) }}</span>
          </div>
          <span class="plan-overview-toggle">{{ showPlanOverview ? '▲' : '▼' }}</span>
        </div>
        <div
          v-if="showPlanOverview"
          class="plan-overview-body"
        >
          <div
            v-for="(task, index) in planOverviewTasks"
            :key="index"
            class="plan-overview-task"
          >
            <div class="ov-task-header">
              <span :class="['ov-task-status', resolveTaskStatusClass(task.status)]">{{ resolveTaskStatusLabel(task.status) }}</span>
              <span class="ov-task-title">{{ task.title }}</span>
              <span
                v-if="task.expertName"
                class="ov-task-expert"
              >{{ task.expertName }}</span>
            </div>
            <div
              v-if="task.summary"
              class="ov-task-summary"
            >
              {{ task.summary }}
            </div>
            <div
              v-if="task.files.length > 0"
              class="ov-task-files"
            >
              <span class="ov-files-label">{{ t('taskBoard.planOverview.files') }}:</span>
              <span
                v-for="file in task.files.slice(0, 5)"
                :key="file"
                class="ov-file-tag"
              >{{ file }}</span>
              <span
                v-if="task.files.length > 5"
                class="ov-file-more"
              >+{{ task.files.length - 5 }}</span>
            </div>
            <div
              v-if="task.failReason"
              class="ov-task-fail"
            >
              {{ t('taskBoard.planOverview.failReason') }}: {{ task.failReason }}
            </div>
          </div>
        </div>
      </div>

      <div class="board-columns">
        <KanbanColumn
          v-for="column in columns"
          :key="column.status"
          :status="column.status"
          :title="column.label"
          :color="column.color"
          :tasks="tasksByStatus[column.status] || []"
          :global-paused="column.status === 'in_progress' ? isCurrentPlanPaused : false"
          @task-drop="handleTaskDrop"
          @task-click="selectTask"
          @task-reorder="handleTaskReorder"
          @task-edit="handleTaskEdit"
          @task-start="handleTaskStart"
          @task-stop="handleTaskStop"
          @task-resume="handleTaskResume"
          @task-retry="handleTaskRetry"
          @task-delete="handleTaskDelete"
          @execute-all="handleExecuteAll"
          @start-execution="handleStartExecution"
          @toggle-global-execution="handleToggleGlobalExecution"
          @add-task="openCreateTaskModal"
        />
      </div>
    </template>

    <TaskEditModal
      v-if="editingTask"
      v-model:visible="showEditModal"
      :task="editingTask"
      @saved="handleEditSaved"
    />

    <TaskEditModal
      v-if="showCreateModal"
      v-model:visible="showCreateModal"
      :task="newTaskTemplate as Task"
      mode="create"
      @saved="handleTaskCreated"
    />
  </div>
</template>

<style scoped src="./styles.css"></style>
