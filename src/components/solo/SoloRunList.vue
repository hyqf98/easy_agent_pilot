<script setup lang="ts">
import { computed } from 'vue'
import type { SoloRun } from '@/types/solo'

const props = defineProps<{
  runs: SoloRun[]
  currentRunId: string | null
}>()

const emit = defineEmits<{
  select: [runId: string]
  create: []
}>()

const groupedRuns = computed(() => {
  const order: SoloRun['status'][] = ['running', 'blocked', 'paused', 'draft', 'failed', 'completed', 'stopped']
  return order
    .map((status) => ({
      status,
      items: props.runs.filter((run) => run.status === status)
    }))
    .filter((group) => group.items.length > 0)
})

function statusLabel(status: SoloRun['status']): string {
  switch (status) {
    case 'running': return '执行中'
    case 'blocked': return '待输入'
    case 'paused': return '已暂停'
    case 'draft': return '草稿'
    case 'failed': return '失败'
    case 'completed': return '完成'
    case 'stopped': return '已停止'
    default: return status
  }
}

function formatTime(value: string): string {
  const date = new Date(value)
  const diff = Date.now() - date.getTime()
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes} 分钟前`
  if (hours < 24) return `${hours} 小时前`
  if (days < 7) return `${days} 天前`
  return date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })
}
</script>

<template>
  <div class="solo-run-list">
    <div class="solo-run-list__header">
      <div>
        <p class="solo-run-list__eyebrow">
          SOLO
        </p>
        <h3>全程自主规划</h3>
      </div>
      <button
        class="solo-run-list__create"
        @click="emit('create')"
      >
        新建
      </button>
    </div>

    <div
      v-if="runs.length === 0"
      class="solo-run-list__empty"
    >
      <div class="solo-run-list__empty-copy">
        <p>还没有全程自主规划任务</p>
        <span>创建后，规划智能体会持续协调参与专家，自动推进步骤、回写结果并轮询执行直到达到目标。</span>
      </div>
    </div>

    <div
      v-else
      class="solo-run-list__groups"
    >
      <section
        v-for="group in groupedRuns"
        :key="group.status"
        class="solo-run-list__group"
      >
        <div class="solo-run-list__group-header">
          <span>{{ statusLabel(group.status) }}</span>
          <strong>{{ group.items.length }}</strong>
        </div>

        <button
          v-for="run in group.items"
          :key="run.id"
          class="solo-run-card"
          :class="[
            `solo-run-card--${run.status}`,
            { 'solo-run-card--active': run.id === currentRunId }
          ]"
          @click="emit('select', run.id)"
        >
          <div class="solo-run-card__title-row">
            <strong>{{ run.name }}</strong>
            <span class="solo-run-card__status">{{ statusLabel(run.status) }}</span>
          </div>
          <p class="solo-run-card__goal">
            {{ run.goal }}
          </p>
          <div class="solo-run-card__meta">
            <span>深度 {{ run.currentDepth }}/{{ run.maxDispatchDepth }}</span>
            <span>{{ formatTime(run.updatedAt) }}</span>
          </div>
        </button>
      </section>
    </div>
  </div>
</template>

<style scoped>
.solo-run-list {
  height: 100%;
  display: flex;
  flex-direction: column;
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--color-surface) 92%, white 8%) 0%, var(--color-surface) 100%);
}

.solo-run-list__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 20px 18px 16px;
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
}

.solo-run-list__eyebrow {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.24em;
  text-transform: uppercase;
  color: var(--color-primary);
}

.solo-run-list__header h3 {
  margin: 6px 0 0;
  font-size: 19px;
  color: var(--color-text-primary);
}

.solo-run-list__create {
  padding: 10px 16px;
  border: none;
  border-radius: 999px;
  color: white;
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-primary) 82%, white) 0%, var(--color-primary) 100%);
  cursor: pointer;
}

.solo-run-list__empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 28px 22px 34px;
  color: var(--color-text-secondary);
}

.solo-run-list__empty-copy {
  width: 100%;
  max-width: 236px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  text-align: left;
}

.solo-run-list__empty-copy::before {
  content: 'SOLO';
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: fit-content;
  padding: 6px 10px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  color: var(--color-primary);
  font-size: 11px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
}

.solo-run-list__empty p {
  margin: 0 0 8px;
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.solo-run-list__empty span {
  font-size: 13px;
  line-height: 1.6;
}

.solo-run-list__groups {
  flex: 1;
  overflow: auto;
  padding: 12px 12px 18px;
}

.solo-run-list__group + .solo-run-list__group {
  margin-top: 18px;
}

.solo-run-list__group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px 10px;
  font-size: 12px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.solo-run-card {
  width: 100%;
  text-align: left;
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
  border-radius: 20px;
  padding: 14px 14px 12px;
  background: color-mix(in srgb, var(--color-surface) 92%, transparent);
  cursor: pointer;
  transition: transform 160ms ease, border-color 160ms ease, box-shadow 160ms ease;
}

.solo-run-card + .solo-run-card {
  margin-top: 10px;
}

.solo-run-card:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--color-primary) 36%, var(--color-border) 64%);
}

.solo-run-card--active {
  border-color: color-mix(in srgb, var(--color-primary) 52%, var(--color-border) 48%);
  box-shadow: 0 16px 28px color-mix(in srgb, var(--color-primary) 12%, transparent);
}

.solo-run-card__title-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  color: var(--color-text-primary);
}

.solo-run-card__status {
  display: inline-flex;
  align-items: center;
  padding: 4px 8px;
  border-radius: 999px;
  font-size: 11px;
  white-space: nowrap;
  background: color-mix(in srgb, var(--color-surface-hover) 78%, transparent);
  color: var(--color-text-secondary);
}

.solo-run-card__goal {
  margin: 10px 0 12px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--color-text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.solo-run-card__meta {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.solo-run-card--running .solo-run-card__status {
  background: color-mix(in srgb, #14b8a6 20%, transparent);
  color: #0f766e;
}

.solo-run-card--blocked .solo-run-card__status {
  background: color-mix(in srgb, #f59e0b 18%, transparent);
  color: #b45309;
}

.solo-run-card--completed .solo-run-card__status {
  background: color-mix(in srgb, #22c55e 16%, transparent);
  color: #15803d;
}

.solo-run-card--failed .solo-run-card__status {
  background: color-mix(in srgb, #ef4444 14%, transparent);
  color: #b91c1c;
}

[data-theme='dark'] .solo-run-card--running .solo-run-card__status {
  color: #5eead4;
}

[data-theme='dark'] .solo-run-card--blocked .solo-run-card__status {
  color: #fcd34d;
}

[data-theme='dark'] .solo-run-card--completed .solo-run-card__status {
  color: #86efac;
}

[data-theme='dark'] .solo-run-card--failed .solo-run-card__status {
  color: #fca5a5;
}
</style>
