<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted } from 'vue'
import { usePlanStore } from '@/stores/plan'
import { useTaskStore } from '@/stores/task'
import { useAgentSchedulerStore } from '@/stores/agentScheduler'
import { useTaskExecutionStore } from '@/stores/taskExecution'
import PlanList from './PlanList.vue'
import TaskBoard from './TaskBoard.vue'
import TaskExecutionLog from './TaskExecutionLog.vue'
import TaskDetail from './TaskDetail.vue'
import PlanProgressDetail from './PlanProgressDetail.vue'
import AgentRoleBadge from './AgentRoleBadge.vue'
import type { Plan, Task } from '@/types/plan'

const planStore = usePlanStore()
const taskStore = useTaskStore()
const agentSchedulerStore = useAgentSchedulerStore()
const taskExecutionStore = useTaskExecutionStore()

type RightPanelView = 'plan_progress' | 'task_detail' | 'task_log'

const rightPanelOpen = ref(false)
const rightPanelView = ref<RightPanelView>('plan_progress')
const selectedPlanId = ref<string | null>(null)
const selectedTaskId = ref<string | null>(null)

// 右侧面板宽度拖拽相关
const rightPanelWidth = ref(380)
const isResizing = ref(false)
const minPanelWidth = 280
const maxPanelWidth = 600

// 当前活动角色
const activeRole = computed(() => agentSchedulerStore.activeRole)

// 监听计划变化，加载任务
watch(
  () => planStore.currentPlanId,
  (planId) => {
    if (planId) {
      void taskStore.loadTasks(planId)
    }
  }
)

function handlePlanClick(plan: Plan) {
  planStore.setCurrentPlan(plan.id)
  rightPanelOpen.value = true
  rightPanelView.value = 'plan_progress'
  selectedPlanId.value = plan.id
  selectedTaskId.value = null
}

function handleTaskClick(task: Task) {
  planStore.setCurrentPlan(task.planId)
  rightPanelOpen.value = true
  selectedTaskId.value = task.id
  selectedPlanId.value = task.planId
  taskExecutionStore.setCurrentViewingTask(task.id)

  // 根据任务状态决定右侧面板视图
  if (task.status === 'pending') {
    rightPanelView.value = 'task_detail'
    taskStore.setCurrentTask(task.id)
  } else {
    // in_progress, completed, blocked 都显示执行日志
    rightPanelView.value = 'task_log'
    void taskExecutionStore.loadTaskLogs(task.id)
  }
}

function handlePlanTaskSelect(taskId: string) {
  const task = taskStore.tasks.find(item => item.id === taskId)
  if (!task) return
  handleTaskClick(task)
}

function closeRightPanel() {
  rightPanelOpen.value = false
}

// 开始拖拽调整宽度
function startResize(e: MouseEvent) {
  isResizing.value = true
  e.preventDefault()
}

// 拖拽中
function handleResize(e: MouseEvent) {
  if (!isResizing.value) return

  const containerRect = document.querySelector('.plan-mode-panel')?.getBoundingClientRect()
  if (!containerRect) return

  // 计算新的宽度（从右边缘到鼠标位置）
  const newWidth = containerRect.right - e.clientX

  // 限制在最小和最大宽度之间
  rightPanelWidth.value = Math.min(maxPanelWidth, Math.max(minPanelWidth, newWidth))
}

// 结束拖拽
function stopResize() {
  isResizing.value = false
}

// 添加和移除全局事件监听
onMounted(() => {
  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
})

onUnmounted(() => {
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
})
</script>

<template>
  <div class="plan-mode-panel">
    <!-- 左侧：计划列表 -->
    <div class="plan-list-container">
      <PlanList @plan-click="handlePlanClick" />
    </div>

    <!-- 中间：任务看板 -->
    <div
      class="task-board-container"
      :class="{ 'task-board-container--with-right': rightPanelOpen }"
    >
      <TaskBoard @task-click="handleTaskClick" />
    </div>

    <!-- 右侧：按需展开 -->
    <div
      v-if="rightPanelOpen"
      class="task-detail-container"
      :style="{
        width: rightPanelWidth + 'px',
        '--detail-panel-width': rightPanelWidth + 'px'
      }"
    >
      <!-- 拖拽调整宽度手柄 -->
      <div
        class="resize-handle"
        :class="{ 'resize-handle--active': isResizing }"
        @mousedown="startResize"
      />

      <!-- 收起按钮 - 放在拖拽手柄上 -->
      <button
        class="collapse-button"
        title="收起详情面板"
        @click="closeRightPanel"
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <polyline points="9 18 15 12 9 6" />
        </svg>
      </button>
      <PlanProgressDetail
        v-if="rightPanelView === 'plan_progress' && selectedPlanId"
        :plan-id="selectedPlanId"
        @task-select="handlePlanTaskSelect"
      />
      <TaskDetail
        v-else-if="rightPanelView === 'task_detail' && selectedTaskId"
      />
      <TaskExecutionLog
        v-else-if="rightPanelView === 'task_log' && selectedTaskId"
        :task-id="selectedTaskId"
      />
    </div>

    <!-- 活动角色指示器 -->
    <div
      v-if="activeRole"
      class="active-role-indicator"
    >
      <AgentRoleBadge
        :role="activeRole"
        size="lg"
      />
    </div>
  </div>
</template>

<style scoped>
.plan-mode-panel {
  display: flex;
  height: 100%;
  background-color: var(--bg-primary, #fff);
  position: relative;
}

.plan-list-container {
  width: 320px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-color, #e5e7eb);
}

.task-board-container {
  flex: 1;
  min-width: 0;
}

.task-board-container--with-right {
  border-right: 1px solid var(--border-color, #e5e7eb);
}

.task-detail-container {
  --detail-panel-surface: var(--color-surface, #ffffff);
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--border-color, #e5e7eb);
  position: relative;
  min-width: 280px;
  max-width: 600px;
  min-height: 0;
  overflow: hidden;
  background:
    linear-gradient(
      180deg,
      color-mix(in srgb, var(--detail-panel-surface) 96%, #ffffff) 0%,
      color-mix(in srgb, var(--detail-panel-surface) 90%, var(--color-bg-secondary, #f8fafc)) 100%
    );
}

.task-detail-container > :not(.resize-handle):not(.collapse-button) {
  width: 100%;
  min-width: 0;
  min-height: 0;
  flex: 1 1 auto;
}

/* 拖拽调整宽度手柄 */
.resize-handle {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 6px;
  cursor: ew-resize;
  z-index: 20;
  background: transparent;
  transition: background-color 0.15s ease;
}

.resize-handle:hover,
.resize-handle--active {
  background-color: var(--color-primary, #3b82f6);
}

.resize-handle::after {
  content: '';
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 2px;
  height: 32px;
  background-color: var(--color-text-tertiary, #94a3b8);
  border-radius: 1px;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.resize-handle:hover::after,
.resize-handle--active::after {
  opacity: 1;
  background-color: white;
}

.collapse-button {
  position: absolute;
  left: 6px;
  top: 50%;
  transform: translateY(-50%);
  width: 20px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--color-surface, #fff);
  border: 1px solid var(--border-color, #e5e7eb);
  border-left: none;
  border-radius: 0 6px 6px 0;
  cursor: pointer;
  z-index: 25;
  color: var(--color-text-secondary, #64748b);
  transition: all 0.15s ease;
}

.collapse-button:hover {
  background-color: var(--color-bg-secondary, #f8fafc);
  color: var(--color-text-primary, #1e293b);
  width: 24px;
}

.active-role-indicator {
  position: absolute;
  top: 0.75rem;
  right: 0.75rem;
  z-index: 10;
}

[data-theme='dark'] .plan-mode-panel {
  background-color: rgba(2, 6, 23, 0.92);
}

[data-theme='dark'] .plan-list-container,
[data-theme='dark'] .task-board-container--with-right,
[data-theme='dark'] .task-detail-container {
  border-color: rgba(148, 163, 184, 0.14);
}

[data-theme='dark'] .task-detail-container {
  --detail-panel-surface: rgba(15, 23, 42, 0.96);
  background:
    linear-gradient(
      180deg,
      rgba(15, 23, 42, 0.98) 0%,
      rgba(2, 6, 23, 0.94) 100%
    );
}

[data-theme='dark'] .collapse-button {
  background-color: rgba(15, 23, 42, 0.96);
  border-color: rgba(148, 163, 184, 0.18);
  color: rgba(226, 232, 240, 0.78);
}

[data-theme='dark'] .collapse-button:hover {
  background-color: rgba(30, 41, 59, 0.96);
  color: #f8fafc;
}
</style>
