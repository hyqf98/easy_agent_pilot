<script setup lang="ts">
import { computed } from 'vue'
import MemoryLibraryPicker from '@/components/memory/MemoryLibraryPicker.vue'
import {
  useSoloRunCreateDialog,
  type SoloRunCreateDialogEmits,
  type SoloRunCreateDialogProps,
} from './useSoloRunCreateDialog'

const props = defineProps<SoloRunCreateDialogProps>()
const emit = defineEmits<SoloRunCreateDialogEmits>()

const {
  allSelected,
  hiddenExpertCount,
  showAllExperts,
  toggleAllExperts,
  toggleExpert,
  toggleExpertOverflow,
  updateField,
  visibleExpertOptions,
} = useSoloRunCreateDialog(props, emit)

const isEditMode = computed(() => props.mode === 'edit')
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
            <h3>{{ isEditMode ? '编辑 SOLO 运行' : '创建全程自主规划任务' }}</h3>
            <p class="solo-create-dialog__subtitle">
              {{
                isEditMode
                  ? '仅在非执行状态下允许修改运行配置，保存后可以继续重试或再次启动。'
                  : '先指定本次 SOLO 使用的规划智能体与执行路径，再让它持续选择内置专家推进任务。'
              }}
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
            <MemoryLibraryPicker
              :model-value="form.memoryLibraryIds"
              hint="挂载后会作为 SOLO 协调者和步骤执行的长期记忆上下文。"
              @update:model-value="updateField('memoryLibraryIds', $event)"
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
            v-if="isEditMode"
            class="solo-create-dialog__button solo-create-dialog__button--primary"
            :disabled="!canCreate"
            @click="emit('save')"
          >
            保存修改
          </button>
          <template v-else>
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
          </template>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped src="./styles.css"></style>
