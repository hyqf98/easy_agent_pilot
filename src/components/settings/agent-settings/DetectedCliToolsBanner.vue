<script setup lang="ts">
import type { CliTool } from '@/stores/agent'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon } from '@/components/common'

interface Props {
  tools: CliTool[]
  addingToolName: string | null
}

defineProps<Props>()

const emit = defineEmits<{
  quickAdd: [tool: CliTool]
}>()

const { t } = useI18n()
</script>

<template>
  <Transition name="banner">
    <div
      v-if="tools.length > 0"
      class="detected-tools"
    >
      <div class="detected-tools__header">
        <EaIcon
          name="scan"
          :size="18"
          class="detected-tools__icon"
        />
        <span class="detected-tools__title">
          {{ t('settings.agentList.detectedTools') }}
        </span>
      </div>
      <div class="detected-tools__list">
        <div
          v-for="tool in tools"
          :key="tool.path"
          class="detected-tool"
        >
          <div class="detected-tool__info">
            <EaIcon
              :name="tool.name === 'claude' ? 'bot' : tool.name === 'opencode' ? 'terminal' : 'code'"
              :size="16"
              class="detected-tool__icon"
            />
            <div class="detected-tool__details">
              <span class="detected-tool__name">
                {{ tool.name === 'claude' ? 'Claude CLI' : tool.name === 'opencode' ? 'OpenCode CLI' : 'Codex CLI' }}
              </span>
              <span class="detected-tool__path">{{ tool.path }}</span>
            </div>
            <span
              v-if="tool.version"
              class="detected-tool__version"
            >
              {{ tool.version }}
            </span>
          </div>
          <EaButton
            type="primary"
            size="small"
            :loading="addingToolName === tool.name"
            @click="emit('quickAdd', tool)"
          >
            <EaIcon
              name="plus"
              :size="14"
            />
            {{ t('settings.agentList.quickAdd') }}
          </EaButton>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.detected-tools {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  border: 1px solid rgba(34, 197, 94, 0.3);
  border-radius: var(--radius-lg);
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.08) 0%, rgba(16, 185, 129, 0.08) 100%);
}

.detected-tools__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.detected-tools__icon {
  color: var(--color-success, #22c55e);
}

.detected-tools__title {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.detected-tools__list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.detected-tool {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
  transition: all var(--transition-fast);
}

.detected-tool:hover {
  border-color: var(--color-primary);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.detected-tool__info {
  display: flex;
  flex: 1;
  min-width: 0;
  align-items: center;
  gap: var(--spacing-3);
}

.detected-tool__icon {
  flex-shrink: 0;
  color: var(--color-primary);
}

.detected-tool__details {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.detected-tool__name {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.detected-tool__path {
  overflow: hidden;
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
  font-family: var(--font-family-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detected-tool__version {
  flex-shrink: 0;
  padding: 2px var(--spacing-2);
  border-radius: var(--radius-sm);
  background-color: var(--color-primary-light);
  color: var(--color-primary);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.banner-enter-active,
.banner-leave-active {
  transition: all var(--transition-normal) var(--easing-default);
}

.banner-enter-from,
.banner-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
