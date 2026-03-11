<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import type { CliTool } from '@/stores/settings'

interface Props {
  tools: CliTool[]
}

defineProps<Props>()

const { t } = useI18n()

function getStatusIcon(status: string): string {
  switch (status) {
    case 'available':
      return 'check-circle'
    case 'not_found':
      return 'x-circle'
    case 'error':
      return 'alert-triangle'
    default:
      return 'help-circle'
  }
}

function getStatusColor(status: string): string {
  switch (status) {
    case 'available':
      return 'var(--color-success)'
    case 'not_found':
      return 'var(--color-text-tertiary)'
    case 'error':
      return 'var(--color-warning)'
    default:
      return 'var(--color-text-tertiary)'
  }
}

function getStatusText(status: string): string {
  switch (status) {
    case 'available':
      return t('settings.cli.statusAvailable')
    case 'not_found':
      return t('settings.cli.statusNotFound')
    case 'error':
      return t('settings.cli.statusError')
    default:
      return t('settings.cli.statusNotFound')
  }
}
</script>

<template>
  <div class="cli-section">
    <h4 class="cli-section__title">
      {{ t('settings.cli.autoDetected') }}
    </h4>
    <div class="cli-list">
      <div
        v-for="tool in tools"
        :key="tool.name"
        class="cli-card"
      >
        <div class="cli-card__header">
          <div class="cli-card__name">
            <EaIcon
              name="terminal"
              :size="18"
            />
            <span>{{ tool.name }}</span>
          </div>
          <div
            class="cli-card__status"
            :style="{ color: getStatusColor(tool.status) }"
          >
            <EaIcon
              :name="getStatusIcon(tool.status)"
              :size="16"
            />
            <span>{{ getStatusText(tool.status) }}</span>
          </div>
        </div>

        <div class="cli-card__body">
          <div class="cli-card__row">
            <span class="cli-card__label">{{ t('settings.cli.path') }}</span>
            <span class="cli-card__value cli-card__path">
              {{ tool.path || '-' }}
            </span>
          </div>
          <div class="cli-card__row">
            <span class="cli-card__label">{{ t('settings.cli.version') }}</span>
            <span class="cli-card__value">
              {{ tool.version || '-' }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cli-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.cli-section__title {
  margin: 0;
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-secondary);
}

.cli-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.cli-card {
  overflow: hidden;
  border-radius: var(--radius-lg);
  background-color: var(--color-bg-secondary);
}

.cli-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4);
  border-bottom: 1px solid var(--color-border);
}

.cli-card__name {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  color: var(--color-text-primary);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
}

.cli-card__status {
  display: flex;
  align-items: center;
  gap: var(--spacing-1);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.cli-card__body {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
}

.cli-card__row {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-4);
}

.cli-card__label {
  width: 48px;
  flex-shrink: 0;
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.cli-card__value {
  display: flex;
  align-items: center;
  word-break: break-all;
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
}

.cli-card__path {
  padding: var(--spacing-1) var(--spacing-2);
  border-radius: var(--radius-sm);
  background-color: var(--color-surface);
  font-family: var(--font-family-mono);
}
</style>
