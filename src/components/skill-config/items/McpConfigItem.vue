<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { computed } from 'vue'
import type { UnifiedMcpConfig } from '@/stores/skillConfig'
import { EaButton, EaIcon } from '@/components/common'

const props = defineProps<{
  config: UnifiedMcpConfig
  isReadOnly: boolean
}>()

const emit = defineEmits<{
  (e: 'test', config: UnifiedMcpConfig): void
  (e: 'edit', config: UnifiedMcpConfig): void
  (e: 'delete', config: UnifiedMcpConfig): void
}>()

const { t } = useI18n()

const isBuiltin = computed(() => props.config.transportType === 'builtin')

function getTransportIcon(transport: string) {
  switch (transport) {
    case 'stdio': return 'lucide:terminal'
    case 'sse': return 'lucide:radio'
    case 'http': return 'lucide:globe'
    case 'builtin': return 'lucide:cpu'
    default: return 'lucide:plug'
  }
}

function getTransportLabel(transport: string) {
  if (transport === 'builtin') {
    return 'BUILT-IN'
  }
  return transport.toUpperCase()
}

function getScopeLabel(scope: string) {
  return t(`settings.agent.scan.scopeTypes.${scope}`)
}

function getCommandDisplay() {
  if (props.config.transportType === 'builtin') {
    return t('settings.mcp.builtinServer')
  }
  if (props.config.url) {
    return props.config.url
  }
  if (props.config.command) {
    const parts = [props.config.command]
    if (props.config.args?.length) {
      parts.push(...props.config.args)
    }
    return parts.join(' ')
  }
  return '-'
}
</script>

<template>
  <div
    class="mcp-config-item"
    :class="{ 'mcp-config-item--disabled': !config.enabled }"
  >
    <div class="mcp-config-item__header">
      <div class="mcp-config-item__name">
        <EaIcon
          :name="isBuiltin ? 'lucide:cpu' : 'lucide:folder'"
          class="mcp-config-item__icon"
        />
        <span>{{ config.name }}</span>
      </div>
      <div v-if="!isBuiltin" class="mcp-config-item__actions">
        <EaButton
          size="small"
          variant="ghost"
          class="btn-test"
          @click="emit('test', config)"
        >
          <EaIcon name="lucide:play" />
          {{ t('settings.mcp.testConnection') }}
        </EaButton>
        <EaButton
          size="small"
          variant="ghost"
          class="btn-edit"
          @click="emit('edit', config)"
        >
          <EaIcon name="lucide:pencil" />
          {{ t('common.edit') }}
        </EaButton>
        <EaButton
          size="small"
          variant="ghost"
          class="btn-delete"
          @click="emit('delete', config)"
        >
          <EaIcon name="lucide:trash-2" />
          {{ t('common.delete') }}
        </EaButton>
      </div>
    </div>

    <div class="mcp-config-item__command">
      {{ getCommandDisplay() }}
    </div>

    <div class="mcp-config-item__meta">
      <span class="mcp-config-item__tag">
        <EaIcon :name="getTransportIcon(config.transportType)" />
        {{ getTransportLabel(config.transportType) }}
      </span>
      <span v-if="!isBuiltin" class="mcp-config-item__tag">
        <EaIcon name="lucide:map-pin" />
        {{ getScopeLabel(config.scope) }}
      </span>
      <span
        v-if="config.source === 'file'"
        class="mcp-config-item__tag mcp-config-item__tag--file"
      >
        <EaIcon name="lucide:file-text" />
        CLI
      </span>
    </div>
  </div>
</template>

<style scoped>
.mcp-config-item {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: var(--spacing-4);
  transition: border-color 0.2s;
}

.mcp-config-item:hover {
  border-color: var(--color-border-hover);
}

.mcp-config-item--disabled {
  opacity: 0.6;
}

.mcp-config-item__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--spacing-2);
}

.mcp-config-item__name {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  font-weight: var(--font-weight-medium);
  font-size: var(--font-size-base);
}

.mcp-config-item__icon {
  width: 16px;
  height: 16px;
  color: var(--color-primary);
}

.mcp-config-item__actions {
  display: flex;
  gap: var(--spacing-1);
}

.mcp-config-item__command {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  background: var(--color-background-secondary);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
  margin-bottom: var(--spacing-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mcp-config-item__meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
}

.mcp-config-item__tag {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-1);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  padding: 2px 8px;
  background: var(--color-background-secondary);
  border-radius: var(--radius-sm);
}

.mcp-config-item__tag--file {
  background: var(--color-warning-bg);
  color: var(--color-warning);
}

.mcp-config-item__tag svg {
  width: 12px;
  height: 12px;
}

/* 按钮颜色区分 */
.btn-test {
  background: rgba(34, 197, 94, 0.1) !important;
  color: #16a34a !important;
  border: 1px solid rgba(34, 197, 94, 0.2) !important;
}

.btn-test:hover {
  background: rgba(34, 197, 94, 0.2) !important;
  border-color: rgba(34, 197, 94, 0.4) !important;
}

.btn-edit {
  background: rgba(59, 130, 246, 0.1) !important;
  color: #2563eb !important;
  border: 1px solid rgba(59, 130, 246, 0.2) !important;
}

.btn-edit:hover {
  background: rgba(59, 130, 246, 0.2) !important;
  border-color: rgba(59, 130, 246, 0.4) !important;
}

.btn-delete {
  background: rgba(239, 68, 68, 0.1) !important;
  color: #dc2626 !important;
  border: 1px solid rgba(239, 68, 68, 0.2) !important;
}

.btn-delete:hover {
  background: rgba(239, 68, 68, 0.2) !important;
  border-color: rgba(239, 68, 68, 0.4) !important;
}
</style>
