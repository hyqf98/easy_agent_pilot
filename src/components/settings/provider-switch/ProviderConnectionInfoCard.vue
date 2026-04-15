<script setup lang="ts">
import { computed } from 'vue'
import { EaIcon } from '@/components/common'
import type { CliConnectionInfo } from '@/stores/providerProfile'
import { useI18n } from 'vue-i18n'
import type { DefaultCliConfigLocateTarget } from '@/composables/useDefaultCliConfigEditor'

const props = defineProps<{
  loading: boolean
  connection: CliConnectionInfo | null
  showApiKey: boolean
}>()

const emit = defineEmits<{
  toggleApiKey: []
  openConfigEditor: [target?: DefaultCliConfigLocateTarget]
}>()

const { t } = useI18n()

const locateActions = computed(() => {
  if (!props.connection) {
    return []
  }

  const actions: Array<DefaultCliConfigLocateTarget & { key: string }> = []
  const providerOrBaseUrl = props.connection.cliType === 'opencode'
    ? props.connection.providerName?.trim()
    : props.connection.baseUrl?.trim()
  const providerOrBaseUrlLabel = props.connection.cliType === 'opencode'
    ? t('settings.providerSwitch.form.providerName')
    : t('settings.providerSwitch.form.baseUrl')
  const mainModel = props.connection.mainModel?.trim()

  if (providerOrBaseUrl) {
    actions.push({
      key: 'provider-or-base-url',
      label: providerOrBaseUrlLabel,
      query: providerOrBaseUrl
    })
  }

  if (mainModel) {
    actions.push({
      key: 'main-model',
      label: t('settings.providerSwitch.form.mainModel'),
      query: mainModel
    })
  }

  return actions
})
</script>

<template>
  <div class="cli-connection section">
    <h3 class="section-title">
      {{ t('settings.providerSwitch.currentFileConfig') }}
    </h3>
    <div
      v-if="loading"
      class="loading"
    >
      <EaIcon
        name="loading"
        spin
        :size="20"
      />
      <span>{{ t('common.loading') }}</span>
    </div>
    <div
      v-else-if="connection"
      class="connection-card"
    >
      <div class="connection-header">
        <div class="connection-name">
          <EaIcon
            :name="connection.isValid ? 'check-circle' : 'alert-circle'"
            :class="connection.isValid ? 'valid-icon' : 'invalid-icon'"
            :size="18"
          />
          {{ connection.displayName }}
        </div>
        <span
          class="status-badge"
          :class="connection.isValid ? 'status-valid' : 'status-invalid'"
        >
          {{ connection.isValid ? t('settings.providerSwitch.connectionValid') : t('settings.providerSwitch.connectionInvalid') }}
        </span>
      </div>
      <div class="connection-header__actions">
        <button
          class="connection-open-btn"
          @click="emit('openConfigEditor')"
        >
          <EaIcon
            name="file-text"
            :size="14"
          />
          {{ t('settings.providerSwitch.openDefaultConfig') }}
        </button>
        <button
          v-for="action in locateActions"
          :key="action.key"
          class="connection-open-btn connection-open-btn--secondary"
          @click="emit('openConfigEditor', action)"
        >
          <EaIcon
            name="crosshair"
            :size="14"
          />
          {{ t('settings.providerSwitch.locateInConfig', { field: action.label }) }}
        </button>
      </div>
      <div class="connection-body">
        <div class="connection-row">
          <span class="connection-label">{{ t('settings.providerSwitch.configFile') }}</span>
          <span class="connection-value mono">{{ connection.configFile }}</span>
        </div>
        <div class="connection-row">
          <span class="connection-label">{{ t('settings.providerSwitch.settingsFile') }}</span>
          <span class="connection-value mono">{{ connection.settingsFile }}</span>
        </div>
        <div class="connection-row">
          <span class="connection-label">{{ connection.cliType === 'opencode' ? t('settings.providerSwitch.form.providerName') : t('settings.providerSwitch.form.baseUrl') }}</span>
          <span class="connection-value mono">{{ connection.cliType === 'opencode' ? (connection.providerName || '-') : (connection.baseUrl || '-') }}</span>
        </div>
        <div
          v-if="connection.mainModel"
          class="connection-row"
        >
          <span class="connection-label">{{ t('settings.providerSwitch.form.mainModel') }}</span>
          <span class="connection-value mono">{{ connection.mainModel }}</span>
        </div>
        <div class="connection-row">
          <span class="connection-label">{{ t('settings.providerSwitch.form.apiKey') }}</span>
          <div class="connection-value-with-action">
            <template v-if="connection.apiKeyMasked">
              <span class="connection-value mono masked">{{ showApiKey ? connection.apiKey : connection.apiKeyMasked }}</span>
              <button
                class="toggle-visibility-btn"
                :title="showApiKey ? '隐藏 API Key' : '显示 API Key'"
                @click="emit('toggleApiKey')"
              >
                <EaIcon
                  :name="showApiKey ? 'eye-off' : 'eye'"
                  :size="14"
                />
              </button>
            </template>
            <span
              v-else
              class="connection-value mono"
            >
              -
            </span>
          </div>
        </div>
        <div
          v-if="connection.errorMessage"
          class="connection-error"
        >
          <EaIcon
            name="alert-triangle"
            :size="14"
          />
          <span>{{ connection.errorMessage }}</span>
        </div>
      </div>
    </div>
    <div
      v-else
      class="no-connection"
    >
      <EaIcon
        name="info"
        :size="16"
      />
      <span>{{ t('settings.providerSwitch.noConnectionInfo') }}</span>
    </div>
  </div>
</template>

<style scoped>
.section {
  margin-bottom: 24px;
}

.section-title {
  margin: 0 0 16px;
  color: var(--color-text-primary, #1a1a1a);
  font-size: 16px;
  font-weight: 600;
}

.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 32px;
  color: var(--color-text-secondary, #666);
}

.connection-card {
  overflow: hidden;
  border: 1px solid var(--color-border, #d7e1ec);
  border-radius: 12px;
  background: var(--color-bg-secondary, #f8fbff);
}

:global(.dark) .connection-card {
  border-color: var(--color-border, #334155);
  background: var(--color-bg-secondary, #1f2937);
}

.connection-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border, #d7e1ec);
  background: var(--color-surface, #ffffff);
}

.connection-header__actions {
  display: flex;
  justify-content: flex-end;
  padding: 0 16px 12px;
  background: var(--color-surface, #ffffff);
}

:global(.dark) .connection-header {
  border-color: var(--color-border, #334155);
  background: var(--color-bg-tertiary, #253142);
}

:global(.dark) .connection-header__actions {
  background: var(--color-bg-tertiary, #253142);
}

.connection-name {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--color-text-primary, #1a1a1a);
  font-weight: 600;
}

.connection-open-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--color-border, #d7e1ec);
  border-radius: 999px;
  padding: 6px 12px;
  background: var(--color-surface, #ffffff);
  color: var(--color-text-primary, #1a1a1a);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.connection-open-btn:hover {
  border-color: #93c5fd;
  color: var(--color-primary, #2563eb);
}

.connection-open-btn--secondary {
  font-weight: 500;
}

:global(.dark) .connection-open-btn {
  border-color: var(--color-border, #334155);
  background: var(--color-bg-secondary, #1f2937);
  color: var(--color-text-primary, #ffffff);
}

:global(.dark) .connection-name {
  color: var(--color-text-primary, #ffffff);
}

.valid-icon {
  color: var(--color-success, #22c55e);
}

.invalid-icon {
  color: var(--color-warning, #f59e0b);
}

.status-badge {
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 500;
}

.status-valid {
  border: 1px solid rgba(34, 197, 94, 0.25);
  background: rgba(34, 197, 94, 0.12);
  color: #15803d;
}

.status-invalid {
  border: 1px solid rgba(239, 68, 68, 0.24);
  background: rgba(239, 68, 68, 0.12);
  color: #dc2626;
}

.connection-body {
  padding: 12px 16px;
}

.connection-row {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid var(--color-border, #d7e1ec);
}

.connection-row:last-child {
  border-bottom: none;
}

:global(.dark) .connection-row {
  border-color: var(--color-border, #334155);
}

.connection-label {
  width: 100px;
  flex-shrink: 0;
  color: var(--color-text-secondary, #666);
  font-size: 13px;
}

.connection-value {
  word-break: break-all;
  color: var(--color-text-primary, #1a1a1a);
  font-size: 13px;
}

:global(.dark) .connection-value {
  color: var(--color-text-primary, #ffffff);
}

.connection-value.mono {
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--color-surface, #ffffff);
  font-family: var(--font-family-mono, monospace);
  font-size: 12px;
}

:global(.dark) .connection-value.mono {
  background: var(--color-bg-tertiary, #253142);
}

.connection-value.masked {
  color: var(--color-text-tertiary, #888);
}

.connection-value-with-action {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 8px;
}

.connection-value-with-action .connection-value {
  flex: 1;
}

.toggle-visibility-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border: none;
  border-radius: 6px;
  background: var(--color-surface-hover, rgba(15, 23, 42, 0.05));
  color: var(--color-text-secondary, #666);
  cursor: pointer;
  transition: all 0.2s ease;
}

.toggle-visibility-btn:hover {
  background: rgba(37, 99, 235, 0.1);
  color: #2563eb;
}

.connection-error {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
  padding: 8px 12px;
  border-radius: 6px;
  background: rgba(239, 68, 68, 0.1);
  color: var(--color-error, #dc2626);
  font-size: 13px;
}

.no-connection {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px;
  border: 1px dashed var(--color-border, #d7e1ec);
  border-radius: 10px;
  background: var(--color-bg-secondary, #f8fbff);
  color: var(--color-text-secondary, #666);
}

:global(.dark) .no-connection {
  border-color: var(--color-border, #334155);
  background: var(--color-bg-secondary, #1f2937);
  color: var(--color-text-secondary, #cbd5e1);
}
</style>
