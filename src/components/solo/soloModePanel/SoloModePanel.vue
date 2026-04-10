<script setup lang="ts">
import SoloExecutionLogPanel from '../SoloExecutionLogPanel.vue'
import SoloRunCreateDialog from '../soloRunCreateDialog/SoloRunCreateDialog.vue'
import SoloRunList from '../SoloRunList.vue'
import { useSoloModePanel } from './useSoloModePanel'

const {
  canCreate,
  closeCreateDialog,
  compactSoloSummary,
  completedCount,
  coordinatorExpertOptions,
  createDialogModelOptions,
  createForm,
  createRun,
  currentExecutionState,
  currentRun,
  currentRunCoordinatorLabel,
  currentRunParticipants,
  currentSteps,
  failedCount,
  blockedCount,
  formatTime,
  getStepExpertLabel,
  getStepLogCount,
  handleBrowseExecutionPath,
  handleDelete,
  handlePause,
  handleReset,
  handleResume,
  handleStart,
  handleStop,
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
  updateCreateForm
} = useSoloModePanel()
</script>

<template>
  <div class="solo-mode-panel">
    <div class="solo-mode-panel__list">
      <SoloRunList
        :runs="runs"
        :current-run-id="soloRunStore.currentRunId"
        @select="selectRun"
        @create="openCreateDialog"
      />
    </div>

    <div class="solo-mode-panel__main">
      <template v-if="currentRun">
        <div class="solo-run-header">
          <div class="solo-run-header__meta">
            <span class="solo-run-header__tag">执行路径</span>
            <strong>{{ currentRun.executionPath }}</strong>
          </div>

          <div class="solo-run-header__actions">
            <button
              v-if="['draft', 'failed', 'stopped'].includes(currentRun.status)"
              class="solo-run-header__button solo-run-header__button--primary"
              @click="handleStart"
            >
              启动
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
              <p>点击任一步骤卡片，右侧会切换到该步骤的执行日志流程。</p>
            </div>
            <div
              v-if="currentExecutionState?.status"
              class="solo-timeline__runtime"
            >
              <span>运行态</span>
              <strong>{{ currentExecutionState.status }}</strong>
            </div>
          </div>

          <div
            v-if="currentSteps.length === 0"
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
              v-for="step in currentSteps"
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
          <span>左侧查看当前项目下的 SOLO 运行，中间查看步骤时间线，右侧查看选中步骤日志。</span>
        </div>
      </div>
    </div>

    <div
      v-if="currentRun"
      class="solo-mode-panel__log"
    >
      <SoloExecutionLogPanel
        :run-id="currentRun.id"
        :step-id="selectedStep?.id ?? null"
      />
    </div>

    <SoloRunCreateDialog
      :visible="showCreateDialog"
      :form="createForm"
      :coordinator-options="coordinatorExpertOptions"
      :expert-options="participantExpertOptions"
      :model-options="createDialogModelOptions"
      :can-create="canCreate"
      @browse-execution-path="handleBrowseExecutionPath"
      @close="closeCreateDialog"
      @create-draft="createRun(false)"
      @create-and-start="createRun(true)"
      @update:form="updateCreateForm"
    />
  </div>
</template>

<style scoped src="./styles.css"></style>
