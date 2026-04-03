<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { SoloAgentOption, SoloCreateFormState, SoloModelOption } from './soloShared'

const props = defineProps<{
  visible: boolean
  form: SoloCreateFormState
  coordinatorOptions: SoloAgentOption[]
  expertOptions: SoloAgentOption[]
  modelOptions: SoloModelOption[]
  canCreate: boolean
}>()

const emit = defineEmits<{
  close: []
  browseExecutionPath: []
  createDraft: []
  createAndStart: []
  'update:form': [patch: Partial<SoloCreateFormState>]
}>()

const allSelected = computed(() =>
  props.expertOptions.length > 0
  && props.form.participantExpertIds.length === props.expertOptions.length
)
const showAllExperts = ref(false)
const collapsedExpertLimit = 6
const hiddenExpertCount = computed(() => Math.max(props.expertOptions.length - collapsedExpertLimit, 0))
const visibleExpertOptions = computed(() => (
  showAllExperts.value
    ? props.expertOptions
    : props.expertOptions.slice(0, collapsedExpertLimit)
))

function updateField<K extends keyof SoloCreateFormState>(key: K, value: SoloCreateFormState[K]) {
  emit('update:form', { [key]: value })
}

function toggleExpert(expertId: string) {
  const nextIds = props.form.participantExpertIds.includes(expertId)
    ? props.form.participantExpertIds.filter((id) => id !== expertId)
    : [...props.form.participantExpertIds, expertId]

  updateField('participantExpertIds', nextIds)
}

function toggleAllExperts() {
  updateField(
    'participantExpertIds',
    allSelected.value ? [] : props.expertOptions.map((option) => option.value)
  )
}

function toggleExpertOverflow() {
  showAllExperts.value = !showAllExperts.value
}

watch(
  () => props.visible,
  (visible) => {
    if (!visible) {
      showAllExperts.value = false
    }
  }
)
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="solo-create-overlay"
    >
      <div class="solo-create-dialog">
        <div class="solo-create-dialog__header">
          <div>
            <p class="solo-create-dialog__eyebrow">
              SOLO Mode
            </p>
            <h3>创建全程自主规划任务</h3>
            <p class="solo-create-dialog__subtitle">
              先指定本次 SOLO 使用的规划智能体与执行路径，再让它持续选择内置专家推进任务。
            </p>
          </div>
          <button
            class="solo-create-dialog__close"
            @click="emit('close')"
          >
            ×
          </button>
        </div>

        <div class="solo-create-dialog__body">
          <div class="solo-create-dialog__field">
            <label>运行名称</label>
            <input
              :value="form.name"
              type="text"
              placeholder="例如：让 AI 自主实现 SOLO 模式全链路"
              @input="updateField('name', ($event.target as HTMLInputElement).value)"
            >
          </div>

          <div class="solo-create-dialog__field">
            <label>需求说明</label>
            <textarea
              :value="form.requirement"
              rows="5"
              placeholder="描述背景、限制和当前问题，让 SOLO 协调者理解当前上下文。"
              @input="updateField('requirement', ($event.target as HTMLTextAreaElement).value)"
            />
          </div>

          <div class="solo-create-dialog__field">
            <label>实现目标</label>
            <textarea
              :value="form.goal"
              rows="4"
              placeholder="描述最终交付结果、成功标准和验收目标。"
              @input="updateField('goal', ($event.target as HTMLTextAreaElement).value)"
            />
          </div>

          <div class="solo-create-dialog__field">
            <label>规划智能体</label>
            <select
              class="solo-create-dialog__select"
              :value="form.coordinatorExpertId ?? ''"
              @change="updateField('coordinatorExpertId', (($event.target as HTMLSelectElement).value || null))"
            >
              <option
                v-for="option in coordinatorOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </div>

          <div class="solo-create-dialog__field">
            <label>执行路径</label>
            <div class="solo-create-dialog__path-row">
              <input
                :value="form.executionPath"
                class="solo-create-dialog__path-input"
                type="text"
                placeholder="选择项目路径或手动指定执行目录"
                @input="updateField('executionPath', ($event.target as HTMLInputElement).value)"
              >
              <button
                class="solo-create-dialog__picker-button"
                type="button"
                @click="emit('browseExecutionPath')"
              >
                选择文件夹
              </button>
            </div>
          </div>

          <div class="solo-create-dialog__grid">
            <div class="solo-create-dialog__field">
              <label>统一调度模型</label>
              <select
                class="solo-create-dialog__select"
                :value="form.coordinatorModelId"
                :disabled="modelOptions.length === 0"
                @change="updateField('coordinatorModelId', ($event.target as HTMLSelectElement).value)"
              >
                <option
                  v-if="modelOptions.length === 0"
                  value=""
                >
                  使用运行时默认模型
                </option>
                <option
                  v-for="option in modelOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </select>
            </div>

            <div class="solo-create-dialog__field">
              <label>最大调度层数</label>
              <input
                :value="form.maxDispatchDepth"
                class="solo-create-dialog__number-input"
                type="number"
                min="1"
                max="100"
                @input="updateField('maxDispatchDepth', Number(($event.target as HTMLInputElement).value || 1))"
              >
            </div>
          </div>

          <div class="solo-create-dialog__field">
            <div class="solo-create-dialog__label-row">
              <label>参与专家</label>
              <button
                class="solo-create-dialog__toggle-all"
                type="button"
                @click="toggleAllExperts"
              >
                {{ allSelected ? '清空选择' : '全部勾选' }}
              </button>
            </div>

            <div class="solo-create-dialog__experts">
              <button
                v-for="option in visibleExpertOptions"
                :key="option.value"
                class="solo-expert-chip"
                type="button"
                :class="{ 'solo-expert-chip--active': form.participantExpertIds.includes(option.value) }"
                @click="toggleExpert(option.value)"
              >
                <span class="solo-expert-chip__checkbox">
                  {{ form.participantExpertIds.includes(option.value) ? '✓' : '' }}
                </span>
                <span class="solo-expert-chip__content">
                  <strong>{{ option.label }}</strong>
                  <small>{{ option.description || '内置专家视角' }}</small>
                </span>
              </button>
            </div>

            <div
              v-if="hiddenExpertCount > 0"
              class="solo-create-dialog__experts-more"
            >
              <button
                class="solo-create-dialog__more-button"
                type="button"
                @click="toggleExpertOverflow"
              >
                {{ showAllExperts ? '收起' : `更多 +${hiddenExpertCount}` }}
              </button>
              <span v-if="!showAllExperts">已折叠剩余专家，避免表单无限拉长。</span>
            </div>
          </div>

          <div class="solo-create-dialog__notice">
            <span class="solo-create-dialog__notice-dot" />
            <p>SOLO 会持续轮询“调度决策 -> 执行步骤 -> 回写日志 -> 再决策”，直到达到目标、被阻塞或被你停止。</p>
          </div>
        </div>

        <div class="solo-create-dialog__footer">
          <button
            class="solo-create-dialog__button solo-create-dialog__button--ghost"
            @click="emit('close')"
          >
            取消
          </button>
          <button
            class="solo-create-dialog__button solo-create-dialog__button--secondary"
            :disabled="!canCreate"
            @click="emit('createDraft')"
          >
            创建草稿
          </button>
          <button
            class="solo-create-dialog__button solo-create-dialog__button--primary"
            :disabled="!canCreate"
            @click="emit('createAndStart')"
          >
            创建并启动
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.solo-create-overlay {
  position: fixed;
  inset: 0;
  z-index: 1400;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background:
    radial-gradient(circle at top left, color-mix(in srgb, var(--color-primary) 16%, transparent) 0%, transparent 44%),
    rgba(8, 15, 28, 0.52);
  backdrop-filter: blur(10px);
}

.solo-create-dialog {
  width: min(860px, 100%);
  max-height: min(92vh, 860px);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--color-border) 76%, var(--color-primary) 24%);
  border-radius: 28px;
  background: linear-gradient(180deg, color-mix(in srgb, var(--color-surface) 92%, white 8%) 0%, var(--color-surface) 100%);
  box-shadow: 0 32px 90px rgba(3, 10, 22, 0.28);
}

.solo-create-dialog__header,
.solo-create-dialog__footer {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 22px 24px;
}

.solo-create-dialog__header {
  border-bottom: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
}

.solo-create-dialog__header h3 {
  margin: 4px 0 0;
  font-size: 22px;
  color: var(--color-text-primary);
}

.solo-create-dialog__subtitle {
  margin: 8px 0 0;
  max-width: 560px;
  font-size: 13px;
  line-height: 1.7;
  color: var(--color-text-secondary);
}

.solo-create-dialog__eyebrow {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.24em;
  text-transform: uppercase;
  color: var(--color-primary);
}

.solo-create-dialog__close {
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-surface-hover) 88%, transparent);
  color: var(--color-text-secondary);
  font-size: 24px;
  line-height: 1;
  cursor: pointer;
}

.solo-create-dialog__body {
  padding: 20px 24px 12px;
  overflow: auto;
}

.solo-create-dialog__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.solo-create-dialog__field {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 18px;
}

.solo-create-dialog__field label {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.02em;
  color: var(--color-text-secondary);
}

.solo-create-dialog__label-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.solo-create-dialog__toggle-all {
  border: none;
  background: transparent;
  color: var(--color-primary);
  font-size: 12px;
  cursor: pointer;
}

.solo-create-dialog__field input,
.solo-create-dialog__field textarea,
.solo-create-dialog__field select {
  width: 100%;
  box-sizing: border-box;
  padding: 12px 14px;
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
  border-radius: 16px;
  background: color-mix(in srgb, var(--color-surface) 86%, transparent);
  color: var(--color-text-primary);
  font-size: 14px;
  transition: border-color 160ms ease, box-shadow 160ms ease, transform 160ms ease;
}

.solo-create-dialog__field select {
  appearance: none;
  padding-right: 40px;
  background-image:
    linear-gradient(45deg, transparent 50%, color-mix(in srgb, var(--color-text-secondary) 88%, transparent) 50%),
    linear-gradient(135deg, color-mix(in srgb, var(--color-text-secondary) 88%, transparent) 50%, transparent 50%);
  background-position: calc(100% - 18px) calc(50% - 2px), calc(100% - 12px) calc(50% - 2px);
  background-size: 6px 6px, 6px 6px;
  background-repeat: no-repeat;
}

.solo-create-dialog__field textarea {
  resize: vertical;
  min-height: 96px;
}

.solo-create-dialog__path-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 10px;
}

.solo-create-dialog__path-input,
.solo-create-dialog__select,
.solo-create-dialog__number-input {
  min-height: 48px;
}

.solo-create-dialog__picker-button {
  min-width: 124px;
  padding: 0 16px;
  border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
  border-radius: 16px;
  background: color-mix(in srgb, var(--color-surface-hover) 86%, transparent);
  color: var(--color-text-primary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 160ms ease, background 160ms ease, transform 160ms ease;
}

.solo-create-dialog__picker-button:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--color-primary) 38%, var(--color-border) 62%);
}

.solo-create-dialog__field input:focus,
.solo-create-dialog__field textarea:focus,
.solo-create-dialog__field select:focus {
  outline: none;
  border-color: color-mix(in srgb, var(--color-primary) 72%, var(--color-border) 28%);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--color-primary) 14%, transparent);
}

.solo-create-dialog__experts {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.solo-create-dialog__experts-more {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 12px;
  color: var(--color-text-secondary);
  font-size: 12px;
}

.solo-create-dialog__more-button {
  padding: 8px 14px;
  border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-surface-hover) 82%, transparent);
  color: var(--color-text-primary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.solo-expert-chip {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 14px;
  border: 1px solid color-mix(in srgb, var(--color-border) 82%, transparent);
  border-radius: 18px;
  background: color-mix(in srgb, var(--color-surface) 88%, transparent);
  text-align: left;
  cursor: pointer;
  transition: transform 160ms ease, border-color 160ms ease, box-shadow 160ms ease;
}

.solo-expert-chip:hover {
  transform: translateY(-1px);
}

.solo-expert-chip--active {
  border-color: color-mix(in srgb, var(--color-primary) 54%, var(--color-border) 46%);
  box-shadow: 0 12px 24px color-mix(in srgb, var(--color-primary) 12%, transparent);
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-primary) 8%, transparent) 0%, color-mix(in srgb, var(--color-surface) 92%, transparent) 100%);
}

.solo-expert-chip__checkbox {
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: color-mix(in srgb, var(--color-surface-hover) 90%, transparent);
  color: var(--color-primary);
  font-size: 12px;
  flex: 0 0 auto;
}

.solo-expert-chip__content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.solo-expert-chip__content strong {
  color: var(--color-text-primary);
}

.solo-expert-chip__content small {
  line-height: 1.5;
  color: var(--color-text-secondary);
}

.solo-create-dialog__notice {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  padding: 16px 18px;
  border-radius: 18px;
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
  color: var(--color-text-secondary);
}

.solo-create-dialog__notice-dot {
  width: 10px;
  height: 10px;
  margin-top: 6px;
  border-radius: 999px;
  background: var(--color-primary);
  flex: 0 0 auto;
}

.solo-create-dialog__notice p {
  margin: 0;
  line-height: 1.7;
}

.solo-create-dialog__footer {
  align-items: center;
  justify-content: flex-end;
  border-top: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
}

.solo-create-dialog__button {
  padding: 10px 18px;
  border: none;
  border-radius: 999px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

.solo-create-dialog__button--ghost {
  background: color-mix(in srgb, var(--color-surface-hover) 88%, transparent);
  color: var(--color-text-primary);
}

.solo-create-dialog__button--secondary {
  background: color-mix(in srgb, #0ea5e9 14%, transparent);
  color: color-mix(in srgb, #0369a1 90%, white 10%);
}

.solo-create-dialog__button--primary {
  color: white;
  background: linear-gradient(135deg, color-mix(in srgb, var(--color-primary) 82%, white) 0%, var(--color-primary) 100%);
}

.solo-create-dialog__button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

@media (max-width: 900px) {
  .solo-create-dialog__grid,
  .solo-create-dialog__experts {
    grid-template-columns: 1fr;
  }

  .solo-create-dialog__path-row {
    grid-template-columns: 1fr;
  }
}
</style>
