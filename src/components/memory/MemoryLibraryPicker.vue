<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useMemoryStore } from '@/stores/memory'

const props = withDefaults(defineProps<{
  modelValue?: string[]
  title?: string
  hint?: string
  emptyText?: string
}>(), {
  modelValue: () => [],
  title: '挂载记忆库',
  hint: '已选 0 个',
  emptyText: '暂无可挂载的记忆库，请先在记忆管理中创建。'
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string[]): void
}>()

const memoryStore = useMemoryStore()

const selectedIds = computed(() => props.modelValue ?? [])

const selectedCountLabel = computed(() => `已选 ${selectedIds.value.length} 个`)

function handleToggle(libraryId: string, checked: boolean) {
  const nextIds = checked
    ? Array.from(new Set([...selectedIds.value, libraryId]))
    : selectedIds.value.filter((id) => id !== libraryId)

  emit('update:modelValue', nextIds)
}

onMounted(async () => {
  if (memoryStore.libraries.length === 0 && !memoryStore.isLoadingLibraries) {
    await memoryStore.loadLibraries()
  }
})
</script>

<template>
  <div class="memory-library-picker">
    <div class="memory-library-picker__header">
      <label class="memory-library-picker__title">{{ title }}</label>
      <span class="memory-library-picker__hint">
        {{ selectedCountLabel }}
      </span>
    </div>

    <div
      v-if="hint"
      class="memory-library-picker__subhint"
    >
      {{ hint }}
    </div>

    <div
      v-if="memoryStore.isLoadingLibraries"
      class="memory-library-picker__state"
    >
      正在加载记忆库...
    </div>

    <div
      v-else-if="memoryStore.libraries.length === 0"
      class="memory-library-picker__state"
    >
      {{ emptyText }}
    </div>

    <div
      v-else
      class="memory-library-picker__grid"
    >
      <label
        v-for="library in memoryStore.libraries"
        :key="library.id"
        class="memory-library-picker__option"
      >
        <input
          :checked="selectedIds.includes(library.id)"
          type="checkbox"
          @change="handleToggle(library.id, ($event.target as HTMLInputElement).checked)"
        >
        <div class="memory-library-picker__content">
          <span class="memory-library-picker__name">{{ library.name }}</span>
          <span class="memory-library-picker__description">
            {{ library.description || '无说明' }}
          </span>
        </div>
      </label>
    </div>
  </div>
</template>

<style scoped>
.memory-library-picker {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.memory-library-picker__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.memory-library-picker__title {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--color-text-secondary, #64748b);
}

.memory-library-picker__hint {
  font-size: 0.75rem;
  color: var(--color-text-tertiary, #94a3b8);
}

.memory-library-picker__subhint {
  font-size: 0.75rem;
  line-height: 1.55;
  color: var(--color-text-secondary, #64748b);
}

.memory-library-picker__state {
  padding: 0.85rem 1rem;
  border: 1px dashed color-mix(in srgb, var(--color-border, #e2e8f0) 80%, transparent);
  border-radius: 0.85rem;
  font-size: 0.8125rem;
  color: var(--color-text-secondary, #64748b);
  background: color-mix(in srgb, var(--color-bg-secondary, #f8fafc) 80%, transparent);
}

.memory-library-picker__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 0.75rem;
  max-height: 14rem;
  overflow-y: auto;
  padding-right: 0.25rem;
  align-content: start;
  scrollbar-gutter: stable;
}

.memory-library-picker__option {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.85rem 0.95rem;
  border: 1px solid color-mix(in srgb, var(--color-border, #e2e8f0) 82%, transparent);
  border-radius: 0.95rem;
  background: color-mix(in srgb, var(--color-surface, #fff) 94%, transparent);
  cursor: pointer;
  transition: border-color 160ms ease, box-shadow 160ms ease, transform 160ms ease;
}

.memory-library-picker__option:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--color-primary, #3b82f6) 28%, var(--color-border, #e2e8f0));
  box-shadow: 0 12px 24px rgba(15, 23, 42, 0.06);
}

.memory-library-picker__option input {
  margin-top: 0.15rem;
  accent-color: var(--color-primary, #3b82f6);
}

.memory-library-picker__content {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.memory-library-picker__name {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-text-primary, #0f172a);
}

.memory-library-picker__description {
  font-size: 0.75rem;
  line-height: 1.55;
  color: var(--color-text-secondary, #64748b);
  word-break: break-word;
}
</style>
