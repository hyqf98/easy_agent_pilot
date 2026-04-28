<script setup lang="ts">
import { computed, toRefs } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton } from '@/components/common'
import type { CliInstallerCard, InstallLogEvent } from './types'
import type { CliName } from '@/stores/cliInstaller'

interface Props {
  cards: CliInstallerCard[]
  installLogs: InstallLogEvent[]
  activeOperations: Partial<Record<CliName, 'install' | 'upgrade'>>
  checkingUpdates: Partial<Record<CliName, boolean>>
  visibleLogs: Partial<Record<CliName, boolean>>
  loading?: boolean
}

const props = defineProps<Props>()
const { cards } = toRefs(props)

const emit = defineEmits<{
  install: [cliName: string, method: string]
  upgrade: [cliName: string]
  checkUpdate: [cliName: string]
  clearLogs: []
  clearLogsForCli: [cliName: CliName]
}>()

const { t } = useI18n()

const logsByCli = computed(() => {
  const map = new Map<string, InstallLogEvent[]>()
  for (const log of props.installLogs) {
    const list = map.get(log.cli_name) || []
    list.push(log)
    map.set(log.cli_name, list)
  }
  return map
})

function getLogsForCli(cliName: string): InstallLogEvent[] {
  return logsByCli.value.get(cliName) || []
}

function formatTime(timestamp: string): string {
  return new Date(timestamp).toLocaleTimeString()
}

function isInstalling(cliName: string): boolean {
  return props.activeOperations[cliName as CliName] === 'install'
}

function isUpgrading(cliName: string): boolean {
  return props.activeOperations[cliName as CliName] === 'upgrade'
}

function isCheckingUpdate(cliName: string): boolean {
  return props.checkingUpdates[cliName as CliName] === true
}

function hasActiveOperation(cliName: string): boolean {
  return Boolean(props.activeOperations[cliName as CliName])
}

function shouldShowLogs(cliName: string): boolean {
  return props.visibleLogs[cliName as CliName] === true
}
</script>

<template>
  <div class="cli-section">
    <h4 class="cli-section__title">
      {{ t('settings.cli.installer.title') }}
    </h4>

    <template v-if="loading">
      <div
        v-for="i in 3"
        :key="i"
        class="install-card install-card--skeleton"
      >
        <div class="install-card__header">
          <div class="skeleton skeleton--text" />
          <div class="skeleton skeleton--badge" />
        </div>
      </div>
    </template>

    <template v-else>
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
              :loading="isUpgrading(card.key)"
              :disabled="hasActiveOperation(card.key)"
              @click="emit('upgrade', card.key)"
            >
              {{ t('settings.cli.installer.upgrade') }}
            </EaButton>
            <EaButton
              v-else
              type="secondary"
              size="small"
              :loading="isCheckingUpdate(card.key)"
              :disabled="hasActiveOperation(card.key)"
              @click="emit('checkUpdate', card.key)"
            >
              {{ isCheckingUpdate(card.key) ? t('settings.cli.installer.checking') : t('settings.cli.installer.checkUpdate') }}
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
              :disabled="!option.available || hasActiveOperation(card.key)"
              :loading="isInstalling(card.key)"
              @click="emit('install', card.key, option.method)"
            >
              {{ t('settings.cli.installer.install') }}
            </EaButton>
          </div>
        </div>

        <div
          v-if="shouldShowLogs(card.key) && getLogsForCli(card.key).length > 0"
          class="install-logs install-logs--per-card"
        >
          <div class="install-logs__header">
            <span>{{ t('settings.cli.installer.logs') }}</span>
            <button
              class="install-logs__clear"
              @click="emit('clearLogsForCli', card.key as CliName)"
            >
              {{ t('settings.cli.installer.clearLogs') }}
            </button>
          </div>
          <div class="install-logs__content">
            <div
              v-for="(log, index) in [...getLogsForCli(card.key)].reverse()"
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

.install-card--skeleton {
  pointer-events: none;
}

.skeleton {
  border-radius: var(--radius-sm);
  background: linear-gradient(90deg, var(--color-surface-hover) 25%, var(--color-border) 50%, var(--color-surface-hover) 75%);
  background-size: 200% 100%;
  animation: skeleton-shimmer 1.5s ease-in-out infinite;
}

.skeleton--text {
  width: 100px;
  height: 16px;
}

.skeleton--badge {
  width: 60px;
  height: 20px;
  margin-left: auto;
}

@keyframes skeleton-shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
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

.install-logs--per-card {
  margin-top: 0;
  border-top: 1px solid var(--color-border);
  border-radius: 0;
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
  max-height: 360px;
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
