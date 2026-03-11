<script setup lang="ts">
import { computed, ref } from 'vue'
import type { UnifiedMcpConfig } from '@/stores/skillConfig'
import McpConfigEditView from '../mcp/McpConfigEditView.vue'
import McpConfigListView from '../mcp/McpConfigListView.vue'
import McpConfigTestView from '../mcp/McpConfigTestView.vue'

const props = defineProps<{
  configs: UnifiedMcpConfig[]
  isReadOnly: boolean
  isLoading: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'open-file'): void
  (e: 'save', config: Partial<UnifiedMcpConfig>, originalId?: string): void
  (e: 'delete', config: UnifiedMcpConfig): void
}>()

const testingConfig = ref<UnifiedMcpConfig | null>(null)
const editingConfig = ref<UnifiedMcpConfig | null>(null)

const isTesting = computed(() => testingConfig.value !== null)
const isEditing = computed(() => editingConfig.value !== null)
const showList = computed(() => !isTesting.value && !isEditing.value)

function handleAdd() {
  editingConfig.value = {
    id: '',
    name: '',
    enabled: true,
    source: 'database',
    isReadOnly: false,
    transportType: 'stdio',
    scope: 'user'
  } as UnifiedMcpConfig
}

function goBackToList() {
  testingConfig.value = null
  editingConfig.value = null
}

function handleSave(config: Partial<UnifiedMcpConfig>, originalId?: string) {
  emit('save', config, originalId)
  goBackToList()
}
</script>

<template>
  <div class="mcp-config-tab">
    <McpConfigListView
      v-if="showList"
      :configs="props.configs"
      :is-read-only="isReadOnly"
      :is-loading="isLoading"
      @add="handleAdd"
      @refresh="emit('refresh')"
      @open-file="emit('open-file')"
      @test="testingConfig = $event"
      @edit="editingConfig = $event"
      @delete="emit('delete', $event)"
    />

    <McpConfigEditView
      v-else-if="isEditing && editingConfig"
      :config="editingConfig"
      @back="goBackToList"
      @save="handleSave"
    />

    <McpConfigTestView
      v-else-if="isTesting && testingConfig"
      :config="testingConfig"
      @back="goBackToList"
    />
  </div>
</template>

<style scoped>
.mcp-config-tab {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
}
</style>
