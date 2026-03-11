<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { EaButton } from '@/components/common'
import type { CliInstallerCard, InstallLogEvent } from './types'

interface Props {
  cards: CliInstallerCard[]
  installLogs: InstallLogEvent[]
  isInstalling: boolean
  currentInstallingCli: string | null
  isCheckingUpdate: string | null
  isUpgrading: string | null
}

defineProps<Props>()

const emit = defineEmits<{
  install: [cliName: string, method: string]
  upgrade: [cliName: string]
  checkUpdate: [cliName: string]
  clearLogs: []
}>()

const { t } = useI18n()

function formatTime(timestamp: string): string {
  return new Date(timestamp).toLocaleTimeString()
}
</script>

<template>
  <div class="cli-section">
    <h4 class="cli-section__title">
      {{ t('settings.cli.installer.title') }}
    </h4>

    <div
      v-for="card in cards"
      :key="card.key"
      class="install-card"
    >
      <div class="install-card__header">
        <div class="install-card__info">
          <span class="install-card__name">{{ card.label }}</span>
          <span
            class="install-card__status"
            :class="{ 'is-installed': card.info?.installed }"
          >
            {{ card.info?.installed ? t('settings.cli.installer.installed') : t('settings.cli.installer.notInstalled') }}
          </span>
        </div>

        <div
          v-if="card.info?.installed"
          class="install-card__version"
        >
          <span
            v-if="card.versionInfo"
            class="install-card__version-text"
          >
            v{{ card.versionInfo.current || '?' }}
            <span
              v-if="card.versionInfo.has_update"
              class="install-card__update-badge"
            >
              {{ t('settings.cli.installer.newVersion') }}: v{{ card.versionInfo.latest }}
            </span>
          </span>
          <EaButton
            v-if="card.versionInfo?.has_update"
            type="primary"
            size="small"
            :loading="isUpgrading === card.key"
            @click="emit('upgrade', card.key)"
          >
            {{ t('settings.cli.installer.upgrade') }}
          </EaButton>
          <EaButton
            v-else
            type="secondary"
            size="small"
            :loading="isCheckingUpdate === card.key"
            @click="emit('checkUpdate', card.key)"
          >
            {{ t('settings.cli.installer.checkUpdate') }}
          </EaButton>
        </div>
      </div>

      <div
        v-if="!card.info?.installed"
        class="install-card__options"
      >
        <div
          v-for="option in card.info?.install_options"
          :key="option.method"
          class="install-option"
          :class="{ 'is-recommended': option.recommended, 'is-unavailable': !option.available }"
        >
          <div class="install-option__info">
            <span class="install-option__method">{{ option.display_name }}</span>
            <span
              v-if="option.recommended"
              class="install-option__badge"
            >
              {{ t('settings.cli.installer.recommended') }}
            </span>
          </div>
          <code class="install-option__command">{{ option.command }}</code>
          <EaButton
            type="primary"
            size="small"
            :disabled="!option.available || isInstalling"
            :loading="isInstalling && currentInstallingCli === card.key"
            @click="emit('install', card.key, option.method)"
          >
            {{ t('settings.cli.installer.install') }}
          </EaButton>
        </div>
      </div>
    </div>

    <div
      v-if="installLogs.length > 0"
      class="install-logs"
    >
      <div class="install-logs__header">
        <span>{{ t('settings.cli.installer.logs') }}</span>
        <button
          class="install-logs__clear"
          @click="emit('clearLogs')"
        >
          {{ t('settings.cli.installer.clearLogs') }}
        </button>
      </div>
      <div class="install-logs__content">
        <div
          v-for="(log, index) in installLogs"
          :key="index"
          class="install-logs__item"
        >
          <span class="install-logs__time">{{ formatTime(log.timestamp) }}</span>
          <span class="install-logs__message">{{ log.message }}</span>
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

.install-card {
  margin-bottom: var(--spacing-4);
  overflow: hidden;
  border-radius: var(--radius-lg);
  background-color: var(--color-bg-secondary);
}

.install-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-4);
  border-bottom: 1px solid var(--color-border);
}

.install-card__info {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.install-card__name {
  color: var(--color-text-primary);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
}

.install-card__status {
  padding: var(--spacing-1) var(--spacing-2);
  border-radius: var(--radius-sm);
  background-color: var(--color-surface-hover);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}

.install-card__status.is-installed {
  background-color: var(--color-success-light, rgba(34, 197, 94, 0.1));
  color: var(--color-success);
}

.install-card__version {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.install-card__version-text {
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.install-card__update-badge {
  margin-left: var(--spacing-2);
  padding: var(--spacing-1) var(--spacing-2);
  border-radius: var(--radius-sm);
  background-color: var(--color-primary-light);
  color: var(--color-primary);
  font-size: var(--font-size-xs);
}

.install-card__options {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
}

.install-option {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background-color: var(--color-surface);
}

.install-option.is-recommended {
  border-color: var(--color-primary);
  background-color: var(--color-primary-light);
}

.install-option.is-unavailable {
  opacity: 0.6;
}

.install-option__info {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.install-option__method {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
}

.install-option__badge {
  padding: 2px var(--spacing-2);
  border-radius: var(--radius-sm);
  background-color: var(--color-primary);
  color: white;
  font-size: var(--font-size-xs);
}

.install-option__command {
  padding: var(--spacing-2);
  border-radius: var(--radius-sm);
  background-color: var(--color-surface-hover);
  color: var(--color-text-tertiary);
  word-break: break-all;
  font-size: var(--font-size-xs);
  font-family: var(--font-family-mono);
}

.install-logs {
  margin-top: var(--spacing-4);
  overflow: hidden;
  border-radius: var(--radius-lg);
  background-color: var(--color-bg-secondary);
}

.install-logs__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3) var(--spacing-4);
  border-bottom: 1px solid var(--color-border);
  background-color: var(--color-surface);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.install-logs__clear {
  border: none;
  border-radius: var(--radius-sm);
  background-color: transparent;
  color: var(--color-primary);
  cursor: pointer;
  font-size: var(--font-size-xs);
  padding: var(--spacing-1) var(--spacing-2);
  transition: background-color var(--transition-fast);
}

.install-logs__clear:hover {
  background-color: var(--color-surface-hover);
}

.install-logs__content {
  max-height: 200px;
  overflow-y: auto;
  padding: var(--spacing-3) var(--spacing-4);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
}

.install-logs__item {
  display: flex;
  gap: var(--spacing-2);
  padding: var(--spacing-1) 0;
}

.install-logs__time {
  flex-shrink: 0;
  color: var(--color-text-tertiary);
}

.install-logs__message {
  word-break: break-all;
  color: var(--color-text-primary);
}
</style>
