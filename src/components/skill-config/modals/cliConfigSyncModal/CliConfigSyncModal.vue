<script setup lang="ts">
import { EaButton, EaIcon, EaModal, EaSelect } from '@/components/common'
import {
  useCliConfigSyncModal,
  type CliConfigSyncModalEmits,
  type CliConfigSyncModalProps,
} from './useCliConfigSyncModal'

const props = defineProps<CliConfigSyncModalProps>()
const emit = defineEmits<CliConfigSyncModalEmits>()

const {
  canSubmit,
  errorMessage,
  existingCount,
  handleSubmit,
  isAllSelected,
  isItemExisting,
  isLoadingSource,
  isLoadingTarget,
  isSyncing,
  newCount,
  selectedNames,
  sourceAgentId,
  sourceEmptyText,
  sourceItems,
  sourceOptions,
  syncResult,
  t,
  targetAgentId,
  targetEmptyText,
  targetItems,
  targetOptions,
  toggleAll,
  toggleItem,
} = useCliConfigSyncModal(props, emit)
</script>

<template>
  <EaModal
    :visible="visible"
    content-class="cli-sync-modal"
    @update:visible="emit('close')"
  >
    <template #header>
      <div class="cli-sync-modal__header">
        <div class="cli-sync-modal__headline">
          <div class="cli-sync-modal__eyebrow">
            {{ t('settings.integration.sync.button') }}
          </div>
          <h3 class="cli-sync-modal__title">
            {{ t('settings.integration.sync.title', { type: props.syncType === 'mcp' ? 'MCP' : t('settings.integration.tabs.skills') }) }}
          </h3>
          <p class="cli-sync-modal__subtitle">
            {{ t('settings.integration.sync.subtitle') }}
          </p>
        </div>
        <button
          class="cli-sync-modal__close"
          type="button"
          @click="emit('close')"
        >
          <EaIcon name="lucide:x" />
        </button>
      </div>
    </template>

    <div class="cli-sync-modal__toolbar">
      <div class="cli-sync-modal__field">
        <label class="cli-sync-modal__label">{{ t('settings.integration.sync.source') }}</label>
        <EaSelect
          :model-value="sourceAgentId"
          :options="sourceOptions"
          :placeholder="t('settings.integration.sync.selectSource')"
          @update:model-value="sourceAgentId = String($event)"
        />
      </div>

      <div class="cli-sync-modal__transfer-mark">
        <span class="cli-sync-modal__transfer-line" />
        <EaIcon name="lucide:arrow-right-left" />
        <span class="cli-sync-modal__transfer-line" />
      </div>

      <div class="cli-sync-modal__field">
        <label class="cli-sync-modal__label">{{ t('settings.integration.sync.target') }}</label>
        <EaSelect
          :model-value="targetAgentId"
          :options="targetOptions"
          :placeholder="t('settings.integration.sync.selectTarget')"
          :disabled="targetOptions.length === 0"
          @update:model-value="targetAgentId = String($event)"
        />
      </div>
    </div>

    <div
      v-if="targetOptions.length === 0"
      class="cli-sync-modal__notice cli-sync-modal__notice--warning"
    >
      <EaIcon name="lucide:triangle-alert" />
      {{ t('settings.integration.sync.noTarget') }}
    </div>

    <div class="cli-sync-modal__layout">
      <section class="cli-sync-panel cli-sync-panel--source">
        <div class="cli-sync-panel__header">
          <div>
            <div class="cli-sync-panel__title">
              {{ t('settings.integration.sync.sourcePanelTitle') }}
            </div>
            <div class="cli-sync-panel__subtitle">
              {{ t('settings.integration.sync.selectedCount', { n: selectedNames.length }) }}
            </div>
          </div>

          <EaButton
            size="small"
            type="secondary"
            :disabled="sourceItems.length === 0 || isLoadingSource"
            @click="toggleAll"
          >
            <EaIcon :name="isAllSelected ? 'lucide:square-minus' : 'lucide:check-square'" />
            {{ isAllSelected ? t('settings.integration.sync.deselectAll') : t('settings.integration.sync.selectAll') }}
          </EaButton>
        </div>

        <div
          v-if="isLoadingSource"
          class="cli-sync-panel__empty"
        >
          <EaIcon
            name="lucide:loader-circle"
            class="cli-sync-panel__spinner"
          />
          {{ t('settings.integration.sync.loading') }}
        </div>

        <div
          v-else-if="sourceItems.length === 0"
          class="cli-sync-panel__empty"
        >
          <EaIcon name="lucide:inbox" />
          {{ sourceEmptyText }}
        </div>

        <div
          v-else
          class="cli-sync-panel__list"
        >
          <label
            v-for="item in sourceItems"
            :key="item.name"
            class="cli-sync-item"
            :class="{
              'cli-sync-item--selected': selectedNames.includes(item.name),
              'cli-sync-item--existing': isItemExisting(item.name),
            }"
          >
            <input
              type="checkbox"
              :checked="selectedNames.includes(item.name)"
              @change="toggleItem(item.name)"
            >
            <div class="cli-sync-item__content">
              <div class="cli-sync-item__top">
                <span class="cli-sync-item__name">{{ item.name }}</span>
                <div class="cli-sync-item__badges">
                  <span
                    v-if="item.transportType"
                    class="cli-sync-badge cli-sync-badge--transport"
                  >
                    {{ item.transportType.toUpperCase() }}
                  </span>
                  <span
                    v-if="isItemExisting(item.name)"
                    class="cli-sync-badge cli-sync-badge--existing"
                  >
                    {{ t('settings.integration.sync.existsBadge') }}
                  </span>
                </div>
              </div>

              <div
                v-if="item.description"
                class="cli-sync-item__description"
              >
                {{ item.description }}
              </div>

              <div
                v-if="item.path"
                class="cli-sync-item__path"
              >
                {{ item.path }}
              </div>
            </div>
          </label>
        </div>
      </section>

      <section class="cli-sync-panel cli-sync-panel--target">
        <div class="cli-sync-panel__header">
          <div>
            <div class="cli-sync-panel__title">
              {{ t('settings.integration.sync.targetPanelTitle') }}
            </div>
            <div class="cli-sync-panel__subtitle">
              {{ t('settings.integration.sync.targetCount', { n: targetItems.length }) }}
            </div>
          </div>
        </div>

        <div class="cli-sync-stats">
          <div class="cli-sync-stats__card">
            <div class="cli-sync-stats__label">
              {{ t('settings.integration.sync.summary.newCount') }}
            </div>
            <div class="cli-sync-stats__value">
              {{ newCount }}
            </div>
          </div>
          <div class="cli-sync-stats__card cli-sync-stats__card--warning">
            <div class="cli-sync-stats__label">
              {{ t('settings.integration.sync.summary.existingCount') }}
            </div>
            <div class="cli-sync-stats__value">
              {{ existingCount }}
            </div>
          </div>
        </div>

        <div
          v-if="isLoadingTarget"
          class="cli-sync-panel__empty cli-sync-panel__empty--compact"
        >
          <EaIcon
            name="lucide:loader-circle"
            class="cli-sync-panel__spinner"
          />
          {{ t('settings.integration.sync.targetLoading') }}
        </div>

        <div
          v-else-if="targetItems.length === 0"
          class="cli-sync-panel__empty cli-sync-panel__empty--compact"
        >
          <EaIcon name="lucide:folder-open" />
          {{ targetEmptyText }}
        </div>

        <div
          v-else
          class="cli-sync-panel__list cli-sync-panel__list--target"
        >
          <div
            v-for="item in targetItems"
            :key="`target-${item.name}`"
            class="cli-sync-item cli-sync-item--readonly"
            :class="{ 'cli-sync-item--matched': selectedNames.includes(item.name) }"
          >
            <div class="cli-sync-item__content">
              <div class="cli-sync-item__top">
                <span class="cli-sync-item__name">{{ item.name }}</span>
                <div class="cli-sync-item__badges">
                  <span
                    v-if="selectedNames.includes(item.name)"
                    class="cli-sync-badge cli-sync-badge--skip"
                  >
                    {{ t('settings.integration.sync.skipBadge') }}
                  </span>
                </div>
              </div>

              <div
                v-if="item.description"
                class="cli-sync-item__description"
              >
                {{ item.description }}
              </div>

              <div
                v-if="item.path"
                class="cli-sync-item__path"
              >
                {{ item.path }}
              </div>
            </div>
          </div>
        </div>

        <div
          v-if="errorMessage"
          class="cli-sync-modal__notice cli-sync-modal__notice--error"
        >
          <EaIcon name="lucide:circle-alert" />
          {{ errorMessage }}
        </div>

        <div
          v-if="syncResult"
          class="cli-sync-result"
        >
          <div class="cli-sync-result__summary">
            <span>{{ t('settings.integration.sync.result.success', { n: syncResult.successCount }) }}</span>
            <span>{{ t('settings.integration.sync.result.skipped', { n: syncResult.skippedCount }) }}</span>
            <span>{{ t('settings.integration.sync.result.failed', { n: syncResult.failedCount }) }}</span>
          </div>

          <div
            v-if="syncResult.skippedItems.length > 0"
            class="cli-sync-result__list"
          >
            <div class="cli-sync-result__title">
              {{ t('settings.integration.sync.result.skippedList') }}
            </div>
            <div
              v-for="item in syncResult.skippedItems"
              :key="`skip-${item.name}`"
              class="cli-sync-result__item"
            >
              {{ item.name }} · {{ item.reason }}
            </div>
          </div>

          <div
            v-if="syncResult.failedItems.length > 0"
            class="cli-sync-result__list"
          >
            <div class="cli-sync-result__title">
              {{ t('settings.integration.sync.result.failedList') }}
            </div>
            <div
              v-for="item in syncResult.failedItems"
              :key="`fail-${item.name}`"
              class="cli-sync-result__item"
            >
              {{ item.name }} · {{ item.reason }}
            </div>
          </div>
        </div>
      </section>
    </div>

    <template #footer>
      <EaButton
        type="secondary"
        @click="emit('close')"
      >
        {{ t('common.close') }}
      </EaButton>
      <EaButton
        :loading="isSyncing"
        :disabled="!canSubmit"
        @click="handleSubmit"
      >
        <EaIcon name="lucide:refresh-cw" />
        {{ t('settings.integration.sync.submit') }}
      </EaButton>
    </template>
  </EaModal>
</template>

<style src="./modalStyles.css"></style>
<style scoped src="./styles.css"></style>
