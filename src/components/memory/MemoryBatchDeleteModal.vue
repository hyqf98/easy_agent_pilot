<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { EaButton, EaModal } from '@/components/common'
import EaSelect, { type SelectOption } from '@/components/common/EaSelect.vue'
import type { RawMemoryDeleteOrder } from '@/types/memory'

const props = defineProps<{
  visible: boolean
  loading?: boolean
  visibleCount: number
  projectLabel?: string
  searchKeyword?: string
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  confirm: [payload: {
    startAt?: string
    endAt?: string
    limit?: number
    deleteOrder?: RawMemoryDeleteOrder
  }]
}>()

const form = reactive({
  startAt: '',
  endAt: '',
  limit: '',
  deleteOrder: 'oldest' as RawMemoryDeleteOrder
})

const orderOptions = computed<SelectOption[]>(() => [
  { value: 'oldest', label: '优先删除最早的数据' },
  { value: 'latest', label: '优先删除最新的数据' }
])

const hasTimeRange = computed(() => Boolean(form.startAt || form.endAt))
const parsedLimit = computed(() => {
  const value = Number(form.limit)
  if (!form.limit.trim() || Number.isNaN(value) || value <= 0) {
    return undefined
  }
  return Math.floor(value)
})
const validationError = computed(() => {
  if (!hasTimeRange.value && !parsedLimit.value) {
    return '请至少设置时间范围或删除条数'
  }

  if (form.startAt && form.endAt && new Date(form.startAt).getTime() > new Date(form.endAt).getTime()) {
    return '开始时间不能晚于结束时间'
  }

  if (form.limit.trim() && !parsedLimit.value) {
    return '删除条数必须是大于 0 的整数'
  }

  return ''
})

const scopeSummary = computed(() => {
  const parts: string[] = []
  if (props.projectLabel && props.projectLabel !== '全部项目') {
    parts.push(`项目：${props.projectLabel}`)
  }
  if (props.searchKeyword) {
    parts.push(`搜索：${props.searchKeyword}`)
  }
  if (parts.length === 0) {
    return '当前会对全部原始记忆生效。'
  }
  return `当前筛选范围：${parts.join(' / ')}`
})

watch(
  () => props.visible,
  (visible) => {
    if (!visible) return
    form.startAt = ''
    form.endAt = ''
    form.limit = ''
    form.deleteOrder = 'oldest'
  }
)

function toIsoString(localValue: string): string | undefined {
  if (!localValue) return undefined
  const date = new Date(localValue)
  if (Number.isNaN(date.getTime())) return undefined
  return date.toISOString()
}

function handleConfirm() {
  if (validationError.value) return

  emit('confirm', {
    startAt: toIsoString(form.startAt),
    endAt: toIsoString(form.endAt),
    limit: parsedLimit.value,
    deleteOrder: parsedLimit.value ? form.deleteOrder : undefined
  })
}
</script>

<template>
  <EaModal
    :visible="visible"
    content-class="memory-dialog"
    @update:visible="emit('update:visible', $event)"
  >
    <template #header>
      <h3 class="memory-dialog__title">
        批量删除原始记忆
      </h3>
    </template>

    <div class="memory-dialog__body">
      <p class="memory-delete-note">
        {{ scopeSummary }}
        当前列表共 {{ visibleCount }} 条数据。可按时间范围删除，也可按条数删除最早或最新的数据。
      </p>

      <div class="memory-delete-grid">
        <label class="memory-dialog__field">
          <span>开始时间</span>
          <input
            v-model="form.startAt"
            type="datetime-local"
            class="memory-dialog__input"
          >
        </label>

        <label class="memory-dialog__field">
          <span>结束时间</span>
          <input
            v-model="form.endAt"
            type="datetime-local"
            class="memory-dialog__input"
          >
        </label>
      </div>

      <div class="memory-delete-grid memory-delete-grid--compact">
        <label class="memory-dialog__field">
          <span>删除条数</span>
          <input
            v-model="form.limit"
            type="number"
            min="1"
            step="1"
            class="memory-dialog__input"
            placeholder="例如 100"
          >
        </label>

        <label class="memory-dialog__field">
          <span>删除顺序</span>
          <EaSelect
            v-model="form.deleteOrder"
            :options="orderOptions"
            :disabled="!parsedLimit"
          />
        </label>
      </div>

      <p
        v-if="validationError"
        class="memory-dialog__error"
      >
        {{ validationError }}
      </p>
    </div>

    <template #footer>
      <EaButton
        type="secondary"
        @click="emit('update:visible', false)"
      >
        取消
      </EaButton>
      <EaButton
        type="danger"
        :loading="loading"
        :disabled="Boolean(validationError)"
        @click="handleConfirm"
      >
        确认删除
      </EaButton>
    </template>
  </EaModal>
</template>

<style scoped>
.memory-dialog__title {
  margin: 0;
  font-size: 22px;
  font-family: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Georgia, serif;
}

.memory-dialog__body {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.memory-delete-note {
  margin: 0;
  padding: 14px 16px;
  border-radius: 16px;
  background: linear-gradient(135deg, rgba(185, 28, 28, 0.08), rgba(15, 23, 42, 0.04));
  color: var(--color-text-secondary);
  line-height: 1.7;
}

.memory-delete-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}

.memory-delete-grid--compact {
  align-items: end;
}

.memory-dialog__field {
  display: flex;
  flex-direction: column;
  gap: 8px;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.memory-dialog__field span {
  color: var(--color-text-primary);
  font-weight: 600;
}

.memory-dialog__input {
  width: 100%;
  min-height: 36px;
  padding: 0 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  color: var(--color-text-primary);
  font: inherit;
  outline: none;
}

.memory-dialog__input:focus {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-alpha-20);
}

.memory-dialog__error {
  margin: 0;
  color: var(--color-error);
  font-size: var(--font-size-sm);
}

@media (max-width: 720px) {
  .memory-delete-grid {
    grid-template-columns: 1fr;
  }
}
</style>
