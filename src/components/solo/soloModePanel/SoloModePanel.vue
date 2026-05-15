<script setup lang="ts">
import SoloExecutionLogPanel from '../SoloExecutionLogPanel.vue'
import SoloRunCreateDialog from '../soloRunCreateDialog/SoloRunCreateDialog.vue'
import SoloRunList from '../SoloRunList.vue'
import { useSoloModePanel } from './useSoloModePanel'

const {
  canCreate,
  canEditCurrentRun,
  closeCreateDialog,
  closeLogPanel,
  compactSoloSummary,
  completedCount,
  dialogMode,
  coordinatorExpertOptions,
  coordinatorLogCount,
  coordinatorModelLabel,
  coordinatorStatusLabel,
  coordinatorStatusBadge,
  coordinatorSummaryText,
  createForm,
  createRun,
  currentRunDurationLabel,
  currentRunHistoryMetrics,
  currentRunHistoryRows,
  currentRunHistorySummary,
  currentRun,
  currentRunCoordinatorLabel,
  currentRunParticipants,
  currentSteps,
  hasCoordinatorLogs,
  isCoordinatorSelected,
  timelineSteps,
  failedCount,
  blockedCount,
  formatTime,
  getStepExpertLabel,
  getStepLogCount,
  handleBrowseExecutionPath,
  handleDelete,
  handlePause,
  handleReset,
  handleReExecute,
  handleResume,
  handleRetry,
  handleStart,
  handleStop,
  isLogPanelOpen,
  isLogPanelResizing,
  logPanelWidth,
  openEditDialog,
  openCreateDialog,
  participantExpertOptions,
  runStatusLabel,
  runs,
  selectRun,
  selectedStep,
  selectedStepId,
  selectStep,
  showCreateDialog,
  soloRunStore,
  stepStatusLabel,
  saveRunEdits,
  updateCreateForm
  ,
  startLogPanelResize
} = useSoloModePanel()
</script>

<template>
  <div
    class="solo-mode-panel"
    :class="{ 'solo-mode-panel--with-log': currentRun && isLogPanelOpen }"
    :style="currentRun && isLogPanelOpen ? { '--solo-log-panel-width': `${logPanelWidth}px` } : undefined"
  >
    <div class="solo-mode-panel__list">
      <SoloRunList
        :runs="runs"
        :current-run-id="soloRunStore.currentRunId"
        @select="selectRun"
        @create="openCreateDialog"
      />
    </div>

    <div
      class="solo-mode-panel__main"
      :class="{ 'solo-mode-panel__main--with-log': currentRun && isLogPanelOpen }"
    >
      <template v-if="currentRun">
        <div class="solo-run-header">
          <div class="solo-run-header__meta">
            <span class="solo-run-header__tag">执行路径</span>
            <strong>{{ currentRun.executionPath }}</strong>
          </div>

          <div class="solo-run-header__actions">
            <button
              v-if="currentRun.status === 'draft'"
              class="solo-run-header__button solo-run-header__button--primary"
              @click="handleStart"
            >
              启动
            </button>
            <button
              v-if="['failed', 'stopped'].includes(currentRun.status)"
              class="solo-run-header__button solo-run-header__button--primary"
              @click="handleRetry"
            >
              重试
            </button>
            <button
              v-if="currentRun.status === 'failed'"
              class="solo-run-header__button solo-run-header__button--ghost"
              @click="handleStart"
            >
              启动
            </button>
            <button
              v-if="currentRun.status === 'completed'"
              class="solo-run-header__button solo-run-header__button--primary"
              @click="handleReExecute"
            >
              重新执行
            </button>
            <button
              v-if="currentRun.status === 'running'"
              class="solo-run-header__button solo-run-header__button--secondary"
              @click="handlePause"
            >
              暂停
            </button>
            <button
              v-if="['paused', 'blocked'].includes(currentRun.status)"
              class="solo-run-header__button solo-run-header__button--primary"
              :disabled="currentRun.status === 'blocked' && Boolean(currentRun.inputRequest)"
              @click="handleResume"
            >
              继续
            </button>
            <button
              v-if="['running', 'paused', 'blocked'].includes(currentRun.status)"
              class="solo-run-header__button solo-run-header__button--danger"
              @click="handleStop"
            >
              停止
            </button>
            <button
              v-if="canEditCurrentRun"
              class="solo-run-header__button solo-run-header__button--ghost"
              @click="openEditDialog"
            >
              编辑
            </button>
            <button
              class="solo-run-header__button solo-run-header__button--ghost"
              @click="handleReset"
            >
              清空进度
            </button>
            <button
              class="solo-run-header__button solo-run-header__button--ghost"
              @click="handleDelete"
            >
              删除
            </button>
          </div>
        </div>

        <div class="solo-summary-card">
          <div class="solo-summary-card__content">
            <p class="solo-summary-card__eyebrow">
              SOLO Timeline
            </p>
            <h2>{{ currentRun.name }}</h2>
            <p class="solo-summary-card__goal">
              {{ currentRun.goal }}
            </p>
            <div class="solo-summary-card__chips">
              <span class="solo-summary-card__chip">{{ runStatusLabel(currentRun.status) }}</span>
              <span class="solo-summary-card__chip">深度 {{ currentRun.currentDepth }}/{{ currentRun.maxDispatchDepth }}</span>
              <span class="solo-summary-card__chip">步骤 {{ currentSteps.length }}</span>
              <span class="solo-summary-card__chip">专家 {{ currentRunParticipants.length }}</span>
              <span class="solo-summary-card__chip">{{ currentRunCoordinatorLabel }}</span>
              <span
                v-if="currentRun.coordinatorModelId"
                class="solo-summary-card__chip"
              >
                {{ currentRun.coordinatorModelId }}
              </span>
            </div>
          </div>

          <div class="solo-summary-card__stats">
            <article class="solo-stat-card">
              <span>已完成</span>
              <strong>{{ completedCount }}</strong>
            </article>
            <article class="solo-stat-card solo-stat-card--warning">
              <span>待输入</span>
              <strong>{{ blockedCount }}</strong>
            </article>
            <article class="solo-stat-card solo-stat-card--danger">
              <span>失败</span>
              <strong>{{ failedCount }}</strong>
            </article>
          </div>
        </div>

        <div class="solo-history-grid">
          <section class="solo-history-card">
            <header>
              <span>历史执行信息</span>
              <small>{{ currentRunDurationLabel }}</small>
            </header>
            <div class="solo-history-card__grid">
              <article
                v-for="item in currentRunHistoryRows"
                :key="item.label"
                class="solo-history-card__item"
              >
                <span>{{ item.label }}</span>
                <strong>{{ item.value }}</strong>
              </article>
            </div>
          </section>

          <section class="solo-history-card solo-history-card--highlight">
            <header>
              <span>本轮历史结论</span>
              <small>{{ runStatusLabel(currentRun.status) }}</small>
            </header>
            <p class="solo-history-card__summary">
              {{ currentRunHistorySummary }}
            </p>
            <div class="solo-history-card__metrics">
              <span
                v-for="item in currentRunHistoryMetrics"
                :key="item.label"
              >
                {{ item.label }} {{ item.value }}
              </span>
            </div>
          </section>
        </div>

        <div class="solo-brief-grid">
          <section class="solo-brief-card">
            <header>
              <span>需求说明</span>
              <small>{{ formatTime(currentRun.updatedAt) }}</small>
            </header>
            <p>{{ currentRun.requirement }}</p>
          </section>
          <section class="solo-brief-card solo-brief-card--experts">
            <header>
              <span>执行路径与专家</span>
              <small>{{ currentRun.executionPath }}</small>
            </header>
            <div class="solo-brief-card__chips">
              <span>{{ currentRunCoordinatorLabel }}</span>
              <span
                v-for="expert in currentRunParticipants"
                :key="expert.id"
              >
                {{ expert.name }}
              </span>
            </div>
          </section>
        </div>

        <div class="solo-timeline">
          <div class="solo-timeline__header">
            <div>
              <h3>任务过程时间线</h3>
              <p>点击任一步骤卡片，右侧会展开该步骤的执行日志流程。</p>
            </div>
            <div class="solo-timeline__header-side" />
          </div>

          <div
            v-if="currentSteps.length === 0 && !hasCoordinatorLogs"
            class="solo-timeline__empty"
          >
            <p>还没有任何步骤。</p>
            <span>启动后，内置协调 AI 会自动派发第一步并持续轮询执行。</span>
          </div>

          <div
            v-else
            class="solo-timeline__track"
          >
            <article
              class="solo-step-card solo-step-card--coordinator"
              :class="{ 'solo-step-card--active': selectedStepId === '__coordinator__' }"
              @click="selectStep('__coordinator__')"
            >
              <div class="solo-step-card__connector" />
              <div class="solo-step-card__marker solo-step-card__marker--coordinator" />
              <div class="solo-step-card__surface">
                <div class="solo-step-card__top">
                  <div>
                    <span class="solo-step-card__depth">调度器</span>
                    <h4>{{ currentRunCoordinatorLabel }}</h4>
                  </div>
                  <span
                    class="solo-step-card__status"
                    :class="coordinatorStatusBadge"
                  >
                    {{ coordinatorStatusLabel }}
                  </span>
                </div>

                <p class="solo-step-card__summary">
                  {{ coordinatorSummaryText }}
                </p>

                <div class="solo-step-card__meta">
                  <span>{{ coordinatorLogCount }} 条日志</span>
                  <span v-if="coordinatorModelLabel">{{ coordinatorModelLabel }}</span>
                </div>
              </div>
            </article>

            <article
              v-for="step in timelineSteps"
              :key="step.id"
              class="solo-step-card"
              :class="[
                `solo-step-card--${step.status}`,
                { 'solo-step-card--active': selectedStepId === step.id }
              ]"
              @click="selectStep(step.id)"
            >
              <div class="solo-step-card__connector" />
              <div class="solo-step-card__marker" />
              <div class="solo-step-card__surface">
                <div class="solo-step-card__top">
                  <div>
                    <span class="solo-step-card__depth">Depth {{ step.depth }}</span>
                    <h4>{{ step.title }}</h4>
                  </div>
                  <span class="solo-step-card__status">{{ stepStatusLabel(step.status) }}</span>
                </div>

                <p class="solo-step-card__summary">
                  {{ compactSoloSummary(step.resultSummary || step.summary || step.description, '等待执行摘要') }}
                </p>

                <div class="solo-step-card__meta">
                  <span>{{ getStepExpertLabel(step) }}</span>
                  <span>{{ getStepLogCount(step.id) }} 条日志</span>
                  <span>{{ formatTime(step.updatedAt) }}</span>
                </div>

                <div
                  v-if="step.resultFiles.length > 0"
                  class="solo-step-card__files"
                >
                  <span
                    v-for="file in step.resultFiles.slice(0, 4)"
                    :key="file"
                  >
                    {{ file }}
                  </span>
                </div>
              </div>
            </article>
          </div>

          <div
            v-if="currentRun.lastError"
            class="solo-timeline__error"
          >
            {{ currentRun.lastError }}
          </div>
        </div>
      </template>

      <div
        v-else
        class="solo-mode-panel__placeholder"
      >
        <div>
          <p class="solo-mode-panel__placeholder-eyebrow">
            SOLO Mode
          </p>
          <h2>选择一个运行，或者新建一个统一调度任务。</h2>
          <span>规划智能体会持续协调参与专家，自动推进步骤、回写结果并轮询执行直到达到目标。</span>
        </div>
      </div>
    </div>

    <div
      v-if="currentRun && isLogPanelOpen"
      class="solo-mode-panel__log"
      :style="{ width: `${logPanelWidth}px` }"
    >
      <div
        class="solo-mode-panel__log-resizer"
        :class="{ 'solo-mode-panel__log-resizer--active': isLogPanelResizing }"
        @mousedown="startLogPanelResize"
      />
      <button
        class="solo-mode-panel__log-collapse"
        title="收起日志面板"
        @click="closeLogPanel"
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
      <SoloExecutionLogPanel
        :run-id="currentRun.id"
        :step-id="isCoordinatorSelected ? null : selectedStep?.id ?? null"
        :force-coordinator-scope="isCoordinatorSelected"
      />
    </div>

    <SoloRunCreateDialog
      :visible="showCreateDialog"
      :mode="dialogMode"
      :form="createForm"
      :coordinator-options="coordinatorExpertOptions"
      :expert-options="participantExpertOptions"
      :can-create="canCreate"
      @browse-execution-path="handleBrowseExecutionPath"
      @close="closeCreateDialog"
      @create-draft="createRun(false)"
      @create-and-start="createRun(true)"
      @save="saveRunEdits"
      @update:form="updateCreateForm"
    />
  </div>
</template>

<style scoped src="./styles.css"></style>
