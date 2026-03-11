<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon } from '@/components/common'
import type { CliPathEntry } from '@/stores/settings'

interface Props {
  entries: CliPathEntry[]
}

defineProps<Props>()

const emit = defineEmits<{
  add: []
  edit: [entry: CliPathEntry]
  delete: [entry: CliPathEntry]
}>()

const { t } = useI18n()
</script>

<template>
  <div class="cli-section">
    <div class="cli-section__header">
      <h4 class="cli-section__title">
        {{ t('settings.cli.manualConfig') }}
      </h4>
      <EaButton
        type="primary"
        size="small"
        @click="emit('add')"
      >
        <EaIcon
          name="plus"
          :size="16"
        />
        {{ t('settings.cli.addCli') }}
      </EaButton>
    </div>

    <div
      v-if="entries.length > 0"
      class="cli-list"
    >
      <div
        v-for="entry in entries"
        :key="entry.id"
        class="cli-card"
      >
        <div class="cli-card__header">
          <div class="cli-card__name">
            <EaIcon
              name="terminal"
              :size="18"
            />
            <span>{{ entry.name }}</span>
          </div>
          <div class="cli-card__actions">
            <button
              class="cli-card__action-btn"
              title="编辑"
              @click="emit('edit', entry)"
            >
              <EaIcon
                name="edit"
                :size="16"
              />
            </button>
            <button
              class="cli-card__action-btn cli-card__action-btn--danger"
              title="删除"
              @click="emit('delete', entry)"
            >
              <EaIcon
                name="trash"
                :size="16"
              />
            </button>
          </div>
        </div>

        <div class="cli-card__body">
          <div class="cli-card__row">
            <span class="cli-card__label">{{ t('settings.cli.path') }}</span>
            <span class="cli-card__value cli-card__path">
              {{ entry.path }}
            </span>
          </div>
          <div class="cli-card__row">
            <span class="cli-card__label">{{ t('settings.cli.version') }}</span>
            <span class="cli-card__value">
              <template v-if="entry.version">
                <EaIcon
                  name="check-circle"
                  :size="14"
                  class="cli-card__version-icon cli-card__version-icon--success"
                />
                {{ entry.version }}
              </template>
              <template v-else>
                <EaIcon
                  name="x-circle"
                  :size="14"
                  class="cli-card__version-icon cli-card__version-icon--error"
                />
                <span class="cli-card__version-error">
                  {{ t('settings.cli.verificationFailed') }}
                </span>
              </template>
            </span>
          </div>
        </div>
      </div>
    </div>

    <div
      v-else
      class="settings-empty settings-empty--compact"
    >
      <EaIcon
        name="folder-plus"
        :size="32"
        class="settings-empty__icon"
      />
      <p class="settings-empty__text">
        {{ t('settings.cli.noCustomPaths') }}
      </p>
      <p class="settings-empty__hint">
        {{ t('settings.cli.noCustomPathsHint') }}
      </p>
    </div>
  </div>
</template>

<style scoped>
.cli-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.cli-section__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
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

.cli-card__actions {
  display: flex;
  gap: var(--spacing-2);
}

.cli-card__action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius-sm);
  background-color: transparent;
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.cli-card__action-btn:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.cli-card__action-btn--danger:hover {
  background-color: var(--color-error-light, rgba(239, 68, 68, 0.1));
  color: var(--color-error, #ef4444);
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

.cli-card__version-icon {
  margin-right: 4px;
}

.cli-card__version-icon--success {
  color: var(--color-success);
}

.cli-card__version-icon--error,
.cli-card__version-error {
  color: var(--color-error, #ef4444);
}

.settings-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--spacing-10) var(--spacing-4);
  text-align: center;
}

.settings-empty--compact {
  padding: var(--spacing-6) var(--spacing-4);
  border-radius: var(--radius-lg);
  background-color: var(--color-bg-secondary);
}

.settings-empty__icon {
  margin-bottom: var(--spacing-3);
  color: var(--color-text-tertiary);
}

.settings-empty__text {
  margin-bottom: var(--spacing-1);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.settings-empty__hint {
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
}
</style>
