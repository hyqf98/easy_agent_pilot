<script setup lang="ts">
import { computed, ref } from 'vue'
import { useAgentStore } from '@/stores/agent'
import { usePlanStore } from '@/stores/plan'
import { useTaskStore } from '@/stores/task'
import type { Task } from '@/types/plan'
import { resolvePlanTaskAgentSelection } from '@/utils/planExecutionProgress'
import AgentRoleBadge from './AgentRoleBadge.vue'
import TaskEditModal from './TaskEditModal.vue'

const agentStore = useAgentStore()
const planStore = usePlanStore()
const taskStore = useTaskStore()

// 当前任务
const currentTask = computed(() => taskStore.currentTask)

// 编辑弹窗状态
const isEditModalVisible = ref(false)

// 是否显示停止按钮
const showStopButton = computed(() => {
  return currentTask.value?.status === 'in_progress'
})

// 是否显示重试按钮
const showRetryButton = computed(() => {
  return currentTask.value?.status === 'blocked'
})

// 打开编辑弹窗
function openEditModal() {
  isEditModalVisible.value = true
}

// 停止任务
async function stopTask() {
  if (!currentTask.value) return
  try {
    await taskStore.stopTask(currentTask.value.id)
  } catch (error) {
    console.error('Failed to stop task:', error)
  }
}

// 重试任务
async function retryTask() {
  if (!currentTask.value) return
  try {
    await taskStore.retryTask(currentTask.value.id)
  } catch (error) {
    console.error('Failed to retry task:', error)
  }
}

// 格式化日期
function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

// 获取依赖任务
const dependencies = computed(() => {
  if (!currentTask.value?.dependencies) return []
  return currentTask.value.dependencies
    .map(id => taskStore.tasks.find(t => t.id === id))
    .filter((t): t is Task => t !== undefined)
})

const currentPlan = computed(() => {
  if (!currentTask.value) return null
  return planStore.plans.find(plan => plan.id === currentTask.value?.planId) || null
})

const executionConfig = computed(() => {
  if (!currentTask.value) {
    return {
      agentLabel: '未指定',
      modelLabel: '使用默认模型',
      sourceLabel: '',
      recommendedLabel: '',
      recommendationReason: ''
    }
  }

  const selection = resolvePlanTaskAgentSelection(
    {
      agent_id: currentTask.value.agentId ?? null,
      model_id: currentTask.value.modelId ?? null
    },
    currentPlan.value
  )

  const agent = selection.agentId
    ? agentStore.agents.find(item => item.id === selection.agentId)
    : null

  return {
    agentLabel: agent?.name || selection.agentId || '未指定',
    modelLabel: selection.modelId || '使用默认模型',
    sourceLabel: selection.source === 'plan' ? '来源于计划默认配置' : '',
    recommendedLabel: currentTask.value.recommendedAgentId
      ? `${currentTask.value.recommendedAgentId}${currentTask.value.recommendedModelId ? ` / ${currentTask.value.recommendedModelId}` : ''}`
      : '',
    recommendationReason: currentTask.value.recommendationReason || ''
  }
})

// 状态标签映射
const statusLabels: Record<string, string> = {
  pending: '待办',
  in_progress: '进行中',
  completed: '已完成',
  blocked: '已阻塞'
}

// 点击依赖任务跳转
function goToDependency(task: Task) {
  taskStore.setCurrentTask(task.id)
}
</script>

<template>
  <div class="task-detail">
    <template v-if="currentTask">
      <!-- 头部 -->
      <div class="detail-header">
        <div class="header-left">
          <h3 class="title">
            任务详情
          </h3>
          <button
            class="btn-edit"
            @click="openEditModal"
          >
            编辑
          </button>
        </div>
        <AgentRoleBadge
          v-if="currentTask.assignee"
          :role="currentTask.assignee"
          size="md"
        />
      </div>

      <!-- 内容 -->
      <div class="detail-body">
        <!-- 基本信息 -->
        <div class="section">
          <h4 class="task-title">
            {{ currentTask.title }}
          </h4>
          <p
            v-if="currentTask.description"
            class="task-desc"
          >
            {{ currentTask.description }}
          </p>
        </div>

        <div class="section">
          <h5 class="section-title">
            执行配置
          </h5>
          <div class="info-item">
            <span class="info-label">执行智能体</span>
            <span class="info-value">{{ executionConfig.agentLabel }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">执行模型</span>
            <span class="info-value">{{ executionConfig.modelLabel }}</span>
          </div>
          <p
            v-if="executionConfig.sourceLabel"
            class="info-hint"
          >
            {{ executionConfig.sourceLabel }}
          </p>
          <div
            v-if="executionConfig.recommendedLabel"
            class="info-item"
          >
            <span class="info-label">推荐执行</span>
            <span class="info-value">{{ executionConfig.recommendedLabel }}</span>
          </div>
          <p
            v-if="executionConfig.recommendationReason"
            class="info-hint"
          >
            {{ executionConfig.recommendationReason }}
          </p>
        </div>

        <!-- 控制按钮 -->
        <div
          v-if="showStopButton || showRetryButton"
          class="section"
        >
          <div class="control-buttons">
            <button
              v-if="showStopButton"
              class="control-btn stop-btn"
              @click="stopTask"
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <rect
                  x="6"
                  y="6"
                  width="12"
                  height="12"
                />
              </svg>
              停止执行
            </button>
            <button
              v-if="showRetryButton"
              class="control-btn retry-btn"
              @click="retryTask"
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path d="M1 4v6h6" />
                <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
              </svg>
              重试任务
            </button>
          </div>
        </div>

        <!-- 重试信息 -->
        <div
          v-if="currentTask.retryCount > 0 || currentTask.errorMessage"
          class="section"
        >
          <h5 class="section-title">
            执行信息
          </h5>
          <div
            v-if="currentTask.retryCount > 0"
            class="info-item"
          >
            <span class="info-label">重试次数</span>
            <span class="info-value">{{ currentTask.retryCount }} / {{ currentTask.maxRetries }}</span>
          </div>
          <div
            v-if="currentTask.errorMessage"
            class="error-message"
          >
            <span class="error-label">错误信息</span>
            <p class="error-text">
              {{ currentTask.errorMessage }}
            </p>
          </div>
        </div>

        <!-- 实现步骤 -->
        <div
          v-if="currentTask.implementationSteps?.length"
          class="section"
        >
          <h5 class="section-title">
            实现步骤
          </h5>
          <ol class="steps-list">
            <li
              v-for="(step, index) in currentTask.implementationSteps"
              :key="index"
            >
              {{ step }}
            </li>
          </ol>
        </div>

        <!-- 测试步骤 -->
        <div
          v-if="currentTask.testSteps?.length"
          class="section"
        >
          <h5 class="section-title">
            测试步骤
          </h5>
          <ol class="steps-list">
            <li
              v-for="(step, index) in currentTask.testSteps"
              :key="index"
            >
              {{ step }}
            </li>
          </ol>
        </div>

        <!-- 验收标准 -->
        <div
          v-if="currentTask.acceptanceCriteria?.length"
          class="section"
        >
          <h5 class="section-title">
            验收标准
          </h5>
          <ul class="criteria-list">
            <li
              v-for="(criteria, index) in currentTask.acceptanceCriteria"
              :key="index"
            >
              {{ criteria }}
            </li>
          </ul>
        </div>

        <!-- 依赖 -->
        <div
          v-if="dependencies.length > 0"
          class="section"
        >
          <h5 class="section-title">
            依赖任务
          </h5>
          <div class="dependency-list">
            <div
              v-for="dep in dependencies"
              :key="dep.id"
              class="dependency-item"
              :class="dep.status"
              @click="goToDependency(dep)"
            >
              <span class="dep-status-dot" />
              <span class="dep-title">{{ dep.title }}</span>
              <span class="dep-status-label">{{ statusLabels[dep.status] || dep.status }}</span>
            </div>
          </div>
        </div>

        <!-- 执行信息 -->
        <div
          v-if="currentTask.sessionId"
          class="section"
        >
          <h5 class="section-title">
            执行信息
          </h5>
          <div class="info-item">
            <span class="info-label">会话 ID</span>
            <span class="info-value">{{ currentTask.sessionId }}</span>
          </div>
          <div
            v-if="currentTask.progressFile"
            class="info-item"
          >
            <span class="info-label">进度文件</span>
            <a
              href="#"
              class="info-link"
            >查看进度</a>
          </div>
        </div>

        <!-- 时间信息 -->
        <div class="section">
          <h5 class="section-title">
            时间信息
          </h5>
          <div class="info-item">
            <span class="info-label">创建时间</span>
            <span class="info-value">{{ formatDate(currentTask.createdAt) }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">更新时间</span>
            <span class="info-value">{{ formatDate(currentTask.updatedAt) }}</span>
          </div>
        </div>
      </div>
    </template>

    <!-- 空状态 -->
    <div
      v-else
      class="empty-state"
    >
      <p>选择一个任务查看详情</p>
    </div>

    <!-- 编辑弹窗 -->
    <TaskEditModal
      v-if="currentTask"
      v-model:visible="isEditModalVisible"
      :task="currentTask"
      @saved="isEditModalVisible = false"
    />
  </div>
</template>

<style scoped>
.task-detail {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: var(--color-surface, #fff);
  border-left: 1px solid var(--color-border, #e2e8f0);
}

.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3, 0.75rem) var(--spacing-4, 1rem);
  border-bottom: 1px solid var(--color-border, #e2e8f0);
  background-color: var(--color-surface, #fff);
}

.header-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-3, 0.75rem);
}

.title {
  margin: 0;
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
}

.btn-edit {
  padding: var(--spacing-1, 0.25rem) var(--spacing-2, 0.5rem);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  background: transparent;
  color: var(--color-text-secondary, #64748b);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-edit:hover {
  background-color: var(--color-surface-hover, #f8fafc);
  border-color: var(--color-primary, #3b82f6);
  color: var(--color-primary, #3b82f6);
}

.detail-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-4, 1rem);
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb, var(--color-border, #e2e8f0)) var(--scrollbar-track, transparent);
}

.detail-body::-webkit-scrollbar {
  width: var(--scrollbar-size, 6px);
}

.detail-body::-webkit-scrollbar-track {
  background: var(--scrollbar-track, transparent);
}

.detail-body::-webkit-scrollbar-thumb {
  background-color: var(--scrollbar-thumb, var(--color-border, #e2e8f0));
  border-radius: var(--radius-full, 9999px);
  border: 1px solid transparent;
  background-clip: padding-box;
}

.section {
  margin-bottom: var(--spacing-5, 1.25rem);
  padding-bottom: var(--spacing-4, 1rem);
  border-bottom: 1px solid var(--color-border-light, #f1f5f9);
}

.section:last-child {
  border-bottom: none;
  margin-bottom: 0;
}

.section-title {
  margin: 0 0 var(--spacing-3, 0.75rem);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-secondary, #64748b);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
}

.section-title::before {
  content: '';
  width: 3px;
  height: 12px;
  background-color: var(--color-primary, #3b82f6);
  border-radius: var(--radius-full, 9999px);
}

.task-title {
  margin: 0 0 var(--spacing-2, 0.5rem);
  font-size: var(--font-size-lg, 16px);
  font-weight: var(--font-weight-semibold, 600);
  color: var(--color-text-primary, #1e293b);
  line-height: 1.4;
}

.task-desc {
  margin: 0;
  font-size: var(--font-size-sm, 13px);
  color: var(--color-text-secondary, #64748b);
  line-height: 1.6;
}

.dependency-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2, 0.5rem);
}

.dependency-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-2, 0.5rem);
  padding: var(--spacing-2, 0.5rem) var(--spacing-3, 0.75rem);
  background-color: var(--color-bg-secondary, #f8fafc);
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-sm, 13px);
  border: 1px solid var(--color-border-light, #f1f5f9);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.dependency-item:hover {
  background-color: var(--color-surface-hover, #f1f5f9);
  border-color: var(--color-primary, #3b82f6);
}

.dep-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #94a3b8;
  flex-shrink: 0;
}

.dependency-item.completed .dep-status-dot {
  background-color: #10b981;
}

.dependency-item.in_progress .dep-status-dot {
  background-color: #3b82f6;
}

.dependency-item.blocked .dep-status-dot {
  background-color: #ef4444;
}

.dep-title {
  flex: 1;
  color: var(--color-text-primary, #1e293b);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dep-status-label {
  font-size: var(--font-size-xs, 11px);
  padding: 2px 6px;
  border-radius: var(--radius-sm, 4px);
  background-color: var(--color-bg-tertiary, #e2e8f0);
  color: var(--color-text-secondary, #64748b);
  flex-shrink: 0;
}

.dependency-item.completed .dep-status-label {
  background-color: #d1fae5;
  color: #059669;
}

.dependency-item.in_progress .dep-status-label {
  background-color: #dbeafe;
  color: #2563eb;
}

.dependency-item.blocked .dep-status-label {
  background-color: #fee2e2;
  color: #dc2626;
}

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--spacing-2, 0.5rem) 0;
  font-size: var(--font-size-sm, 13px);
}

.info-item + .info-item {
  border-top: 1px solid var(--color-border-light, #f1f5f9);
}

.info-label {
  color: var(--color-text-secondary, #64748b);
}

.info-value {
  color: var(--color-text-primary, #1e293b);
  font-family: var(--font-family-mono, monospace);
  font-size: var(--font-size-xs, 12px);
  background-color: var(--color-bg-tertiary, #f1f5f9);
  padding: var(--spacing-1, 0.25rem) var(--spacing-2, 0.5rem);
  border-radius: var(--radius-sm, 4px);
}

.info-link {
  color: var(--color-primary, #3b82f6);
  text-decoration: none;
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
}

.info-link:hover {
  text-decoration: underline;
}

.info-hint {
  margin: var(--spacing-2, 0.5rem) 0 0;
  font-size: var(--font-size-xs, 12px);
  color: var(--color-text-tertiary, #94a3b8);
}

.form-field {
  margin-bottom: var(--spacing-4, 1rem);
}

.form-field label {
  display: block;
  margin-bottom: var(--spacing-2, 0.5rem);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-text-secondary, #64748b);
}

.form-field input,
.form-field textarea,
.form-field select {
  width: 100%;
  padding: var(--spacing-2, 0.5rem) var(--spacing-3, 0.75rem);
  border: 1px solid var(--color-border, #e2e8f0);
  border-radius: var(--radius-md, 8px);
  background-color: var(--color-surface, #fff);
  color: var(--color-text-primary, #1e293b);
  font-size: var(--font-size-sm, 13px);
  transition: all var(--transition-fast, 150ms);
}

.form-field input:focus,
.form-field textarea:focus,
.form-field select:focus {
  outline: none;
  border-color: var(--color-primary, #3b82f6);
  box-shadow: 0 0 0 3px var(--color-primary-light, #dbeafe);
}

.edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-2, 0.5rem);
  margin-top: var(--spacing-4, 1rem);
  padding-top: var(--spacing-4, 1rem);
  border-top: 1px solid var(--color-border-light, #f1f5f9);
}

.btn {
  padding: var(--spacing-2, 0.5rem) var(--spacing-4, 1rem);
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-sm, 13px);
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.btn-primary {
  background-color: var(--color-primary, #3b82f6);
  color: white;
  border: none;
}

.btn-primary:hover {
  background-color: var(--color-primary-hover, #2563eb);
}

.btn-secondary {
  background-color: var(--color-surface, #fff);
  color: var(--color-text-primary, #1e293b);
  border: 1px solid var(--color-border, #e2e8f0);
}

.btn-secondary:hover {
  background-color: var(--color-surface-hover, #f8fafc);
  border-color: var(--color-border-dark, #cbd5e1);
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--color-text-tertiary, #94a3b8);
  font-size: var(--font-size-sm, 13px);
  gap: var(--spacing-2, 0.5rem);
}

.empty-state::before {
  content: '📋';
  font-size: 2rem;
  opacity: 0.5;
}

.control-buttons {
  display: flex;
  gap: var(--spacing-2, 0.5rem);
  margin-top: var(--spacing-3, 0.75rem);
}

.control-btn {
  display: flex;
  align-items: center;
  gap: var(--spacing-1, 0.25rem);
  padding: var(--spacing-2, 0.5rem) var(--spacing-3, 0.75rem);
  border: none;
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
}

.stop-btn {
  background-color: var(--color-warning-light, #fef3c7);
  color: var(--color-warning, #f59e0b);
}

.stop-btn:hover {
  background-color: var(--color-warning, #f59e0b);
  color: white;
}

.retry-btn {
  background-color: var(--color-primary-light, #dbeafe);
  color: var(--color-primary, #3b82f6);
}

.retry-btn:hover {
  background-color: var(--color-primary, #3b82f6);
  color: white;
}

.error-message {
  margin-top: var(--spacing-2, 0.5rem);
  padding: var(--spacing-3, 0.75rem);
  background-color: #fef2f2;
  border: 1px solid var(--color-error-light, #fecaca);
  border-radius: var(--radius-md, 8px);
}

.error-label {
  display: block;
  font-size: var(--font-size-xs, 12px);
  font-weight: var(--font-weight-medium, 500);
  color: var(--color-error, #ef4444);
  margin-bottom: var(--spacing-1, 0.25rem);
}

.error-text {
  margin: 0;
  font-size: var(--font-size-sm, 13px);
  color: #991b1b;
  line-height: 1.5;
  word-break: break-word;
}

.steps-list,
.criteria-list {
  margin: 0;
  padding-left: var(--spacing-5, 1.25rem);
  font-size: var(--font-size-sm, 13px);
  color: var(--color-text-primary, #1e293b);
  line-height: 1.6;
}

.steps-list li,
.criteria-list li {
  margin-bottom: var(--spacing-2, 0.5rem);
  padding-left: var(--spacing-1, 0.25rem);
}

.steps-list li:last-child,
.criteria-list li:last-child {
  margin-bottom: 0;
}

.criteria-list li::marker {
  color: var(--color-success, #10b981);
}
</style>
