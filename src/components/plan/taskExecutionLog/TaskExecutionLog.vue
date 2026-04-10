<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import ExecutionTimeline from '@/components/message/ExecutionTimeline.vue'
import StructuredContentRenderer from '@/components/message/StructuredContentRenderer.vue'
import DynamicForm from '@/components/plan/dynamicForm/DynamicForm.vue'
import { EaIcon } from '@/components/common'
import { formatTokenCount } from '@/stores/token'
import { useTaskExecutionLog } from './useTaskExecutionLog'
const { t } = useI18n()
const props = defineProps<{
  taskId: string
}>()
 
const {
  formatTodoStatusLabel,
  logContainerRef,
  task,
  tokenUsageWindow,
  tokenContextLimit,
  tokenUsageTotal,
  tokenUsagePercentage,
  tokenUsageLevel,
  tokenProgressStyle,
  logs,
  isTodoCollapsed,
  todoSnapshot,
  sortedTodoItems,
  todoCompletedCount,
  activeTodoItems,
  hiddenActiveTodoCount,
  isWaitingInput,
  effectiveStatus,
  isRunning,
  structuredResultContent,
  statusText,
  statusColor,
  handleStop,
  handleResume,
  handleClearLogs,
  handleInputSubmit,
  handleSkip,
  handleScroll,
  timelineEntries
} = useTaskExecutionLog({ props })
</script>

<template>
  <div class="task-execution-log">
    <div class="log-header">
      <div class="header-left">
        <h4
          class="log-title"
          :title="task?.title || t('taskExecution.unnamedTask')"
        >
          {{ task?.title || t('taskExecution.unnamedTask') }}
        </h4>
        <span
          class="status-badge"
          :class="statusColor"
        >
          {{ statusText }}
        </span>
      </div>
      <div class="header-actions">
        <button
          v-if="isRunning"
          class="btn-stop"
          :title="t('taskExecution.stop')"
          @click="handleStop"
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
          <span class="btn-label">{{ t('taskExecution.stop') }}</span>
        </button>
        <button
          v-else-if="effectiveStatus === 'stopped'"
          class="btn-resume"
          :title="t('taskExecution.resume')"
          @click="handleResume"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polygon points="5 3 19 12 5 21 5 3" />
          </svg>
          <span class="btn-label">{{ t('taskExecution.resume') }}</span>
        </button>
        <button
          v-if="logs.length > 0"
          class="btn-clear"
          :title="t('taskExecution.clearLogs')"
          @click="handleClearLogs"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          </svg>
          <span class="btn-label">{{ t('taskExecution.clearLogs') }}</span>
        </button>
      </div>
    </div>

    <div
      v-if="tokenUsageTotal > 0 || tokenUsageWindow.model"
      class="token-usage-panel"
    >
      <div class="token-usage-panel__meta">
        <div class="token-usage-panel__title">
          <span>{{ t('taskExecution.tokenUsage') }}</span>
          <span
            v-if="tokenUsageWindow.model"
            class="token-usage-panel__model"
          >
            {{ tokenUsageWindow.model }}
          </span>
        </div>
        <div class="token-usage-panel__stats">
          <span>{{ formatTokenCount(tokenUsageTotal) }} / {{ formatTokenCount(tokenContextLimit) }}</span>
          <span v-if="tokenUsageWindow.resetCount > 0">{{ t('taskExecution.tokenResetCount', { count: tokenUsageWindow.resetCount }) }}</span>
        </div>
      </div>
      <div
        class="token-usage-panel__bar"
        :class="`token-usage-panel__bar--${tokenUsageLevel}`"
      >
        <div
          class="token-usage-panel__fill"
          :style="tokenProgressStyle"
        />
      </div>
      <div class="token-usage-panel__breakdown">
        <span>{{ t('taskExecution.inputTokens', { count: formatTokenCount(tokenUsageWindow.inputTokens) }) }}</span>
        <span>{{ t('taskExecution.outputTokens', { count: formatTokenCount(tokenUsageWindow.outputTokens) }) }}</span>
        <span>{{ Math.round(tokenUsagePercentage) }}%</span>
      </div>
    </div>
    <!-- 待办列表面板 -->
    <section
      v-if="todoSnapshot && sortedTodoItems.length > 0"
      class="task-todo-panel"
      :class="{ 'task-todo-panel--expanded': !isTodoCollapsed }"
    >
      <button
        type="button"
        class="task-todo-panel__head"
        :aria-expanded="!isTodoCollapsed"
        @click="isTodoCollapsed = !isTodoCollapsed"
      >
        <div class="task-todo-panel__head-main">
          <div class="task-todo-panel__title">
            <EaIcon
              name="list-todo"
              :size="14"
            />
            <span>待办列表</span>
          </div>
          <div
            v-if="isTodoCollapsed && activeTodoItems.length > 0"
            class="task-todo-panel__active-strip"
          >
            <span
              v-for="item in activeTodoItems"
              :key="item.id"
              class="task-todo-panel__active-chip"
              :class="`task-todo-panel__active-chip--${item.status}`"
            >
              <span class="task-todo-panel__active-chip-dot" />
              <span class="task-todo-panel__active-chip-text">{{ item.content }}</span>
            </span>
            <span
              v-if="hiddenActiveTodoCount > 0"
              class="task-todo-panel__active-more"
            >
              +{{ hiddenActiveTodoCount }}
            </span>
          </div>
        </div>
        <div class="task-todo-panel__summary">
          {{ todoCompletedCount }}/{{ sortedTodoItems.length }}
          <EaIcon
            :name="isTodoCollapsed ? 'chevron-down' : 'chevron-up'"
            :size="14"
          />
        </div>
      </button>

      <div
        v-if="!isTodoCollapsed"
        class="task-todo-panel__items"
      >
        <div class="task-todo-panel__items-inner">
          <div
            v-for="(item, index) in sortedTodoItems"
            :key="item.id"
            class="task-todo-panel__item"
            :class="`task-todo-panel__item--${item.status}`"
            :style="{ '--todo-item-index': index }"
          >
            <span class="task-todo-panel__dot" />
            <div class="task-todo-panel__content">
              <div class="task-todo-panel__text">
                {{ item.content }}
              </div>
              <div
                v-if="item.activeForm"
                class="task-todo-panel__hint"
              >
                {{ item.activeForm }}
              </div>
            </div>
            <span class="task-todo-panel__status">
              {{ formatTodoStatusLabel(item.status) }}
            </span>
          </div>
        </div>
      </div>
    </section>

    <!-- 等待用户输入表单区域 -->
    <div
      v-if="isWaitingInput && task?.inputRequest"
      class="input-form-section"
    >
      <h5 class="section-title">
        {{ task.inputRequest.question || t('taskExecution.defaultQuestion') }}
      </h5>
      <DynamicForm
        :schema="task.inputRequest.formSchema"
        @submit="handleInputSubmit"
      />
      <button
        class="btn-skip"
        @click="handleSkip"
      >
        {{ t('taskExecution.skipAndContinue') }}
      </button>
    </div>

    <!-- 日志内容 -->
    <div
      ref="logContainerRef"
      class="log-content"
      @scroll="handleScroll"
    >
      <div
        v-if="structuredResultContent"
        class="result-summary"
      >
        <StructuredContentRenderer :content="structuredResultContent" />
      </div>

      <div
        v-if="logs.length === 0"
        class="empty-state"
      >
        <span v-if="isRunning">{{ t('task.execution.running') }}</span>
        <span v-else>{{ t('taskExecution.noLogs') }}</span>
      </div>

      <div
        v-else
        class="log-entries"
      >
        <ExecutionTimeline :entries="timelineEntries" />
      </div>

      <!-- 运行指示器 -->
      <div
        v-if="isRunning"
        class="running-indicator"
      >
        <span class="indicator-dot" />
        <span class="indicator-text">{{ t('taskExecution.aiRunning') }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped src="./styles.css"></style>
