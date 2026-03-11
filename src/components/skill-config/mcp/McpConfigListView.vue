<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { UnifiedMcpConfig } from '@/stores/skillConfig'
import McpConfigItem from '../items/McpConfigItem.vue'
import { EaButton, EaIcon, EaStateBlock } from '@/components/common'

defineProps<{
  configs: UnifiedMcpConfig[]
  isReadOnly: boolean
  isLoading: boolean
}>()

const emit = defineEmits<{
  add: []
  refresh: []
  'open-file': []
  test: [config: UnifiedMcpConfig]
  edit: [config: UnifiedMcpConfig]
  delete: [config: UnifiedMcpConfig]
}>()

const { t } = useI18n()
</script>

<template>
  <div class="mcp-config-list">
    <div class="mcp-config-list__header">
      <h3 class="mcp-config-list__title">
        {{ t('settings.sdkConfig.mcp.title') }}
      </h3>
      <div class="mcp-config-list__actions">
        <EaButton
          size="small"
          @click="emit('add')"
        >
          <EaIcon name="lucide:plus" />
          {{ t('settings.sdkConfig.mcp.add') }}
        </EaButton>
        <template v-if="isReadOnly">
          <EaButton
            size="small"
            variant="ghost"
            @click="emit('refresh')"
          >
            <EaIcon name="lucide:refresh-cw" />
            {{ t('common.refresh') }}
          </EaButton>
          <EaButton
            size="small"
            variant="ghost"
            @click="emit('open-file')"
          >
            <EaIcon name="lucide:external-link" />
            {{ t('settings.agentConfig.cliConfigCardTitle') }}
          </EaButton>
        </template>
      </div>
    </div>

    <EaStateBlock
      v-if="isLoading"
      variant="loading"
      :title="t('common.loading')"
    />

    <EaStateBlock
      v-else-if="configs.length === 0"
      icon="lucide:server"
      :description="t('settings.sdkConfig.mcp.noConfigs')"
    />

    <div
      v-else
      class="mcp-config-list__items"
    >
      <McpConfigItem
        v-for="config in configs"
        :key="config.id"
        :config="config"
        :is-read-only="isReadOnly"
        @test="emit('test', $event)"
        @edit="emit('edit', $event)"
        @delete="emit('delete', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.mcp-config-list {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  gap: var(--spacing-4);
}

.mcp-config-list__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
}

.mcp-config-list__title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.mcp-config-list__actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
}

.mcp-config-list__items {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--spacing-3);
  min-height: 0;
  overflow-y: auto;
}
</style>
