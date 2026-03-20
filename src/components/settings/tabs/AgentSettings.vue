<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon } from '@/components/common'
import AgentConfigForm from '@/components/agent/AgentConfigForm.vue'
import ModelManageModal from '@/components/agent/ModelManageModal.vue'
import AgentSettingsDeleteDialog from '@/components/settings/agent-settings/AgentSettingsDeleteDialog.vue'
import DetectedCliToolsBanner from '@/components/settings/agent-settings/DetectedCliToolsBanner.vue'
import AgentSettingsTable from '@/components/settings/agent-settings/AgentSettingsTable.vue'
import CliSettings from './CliSettings.vue'
import { useAgentSettingsPage } from '@/components/settings/agent-settings/useAgentSettingsPage'

const { t } = useI18n()
const {
  PAGE_SIZE,
  addingToolName,
  agentStore,
  currentPage,
  searchQuery,
  showModal,
  editingAgent,
  showDeleteConfirm,
  deletingAgent,
  showModelManageModal,
  managingModelAgent,
  testResult,
  showMigrationBanner,
  migrationPendingCount,
  isMigrating,
  migrationResult,
  showMigrationResultToast,
  filteredAgents,
  totalPages,
  paginatedAgents,
  pageNumbers,
  handleSearchChange,
  triggerMigrationCheck,
  handleMigration,
  handleMigrationLater,
  handleQuickAdd,
  handleAdd,
  handleEdit,
  handleDelete,
  confirmDelete,
  handleTest,
  handleSubmit,
  handleCancel,
  handleOpenModelManage,
  handleCloseModelManage,
  goToPage,
  clearSearch
} = useAgentSettingsPage()
</script>

<template>
  <div class="agent-list-page">
    <!-- 迁移提示横幅 -->
    <Transition name="banner">
      <div
        v-if="showMigrationBanner"
        class="migration-banner"
      >
        <div class="migration-banner__content">
          <EaIcon
            name="info"
            :size="20"
            class="migration-banner__icon"
          />
          <div class="migration-banner__text">
            <h4 class="migration-banner__title">
              {{ t('settings.agentList.migrationTitle') }}
            </h4>
            <p class="migration-banner__description">
              {{ t('settings.agentList.migrationAvailable', { n: migrationPendingCount }) }}
            </p>
          </div>
        </div>
        <div class="migration-banner__actions">
          <EaButton
            type="ghost"
            size="small"
            :disabled="isMigrating"
            @click="handleMigrationLater"
          >
            {{ t('settings.agentList.migrationLater') }}
          </EaButton>
          <EaButton
            type="primary"
            size="small"
            :loading="isMigrating"
            @click="handleMigration"
          >
            {{ isMigrating ? t('settings.agentList.migrationProcessing') : t('settings.agentList.migrationButton') }}
          </EaButton>
        </div>
      </div>
    </Transition>

    <!-- 迁移结果提示 -->
    <Transition name="toast">
      <div
        v-if="showMigrationResultToast && migrationResult"
        class="migration-result-toast"
        :class="migrationResult.success ? 'migration-result-toast--success' : 'migration-result-toast--error'"
      >
        <EaIcon
          :name="migrationResult.success ? 'check-circle' : 'x-circle'"
          :size="18"
        />
        <span class="migration-result-toast__message">
          {{ migrationResult.success
            ? t('settings.agentList.migrationSuccess', { migrated: migrationResult.migrated_count, skipped: migrationResult.skipped_count })
            : t('settings.agentList.migrationError') }}
        </span>
        <button
          class="migration-result-toast__close"
          @click="showMigrationResultToast = false"
        >
          <EaIcon
            name="x"
            :size="14"
          />
        </button>
      </div>
    </Transition>

    <!-- 页面标题和操作栏 -->
    <div class="agent-list-page__header">
      <h3 class="agent-list-page__title">
        {{ t('settings.agentList.title') }}
      </h3>
      <EaButton
        type="primary"
        size="small"
        @click="handleAdd"
      >
        <EaIcon
          name="plus"
          :size="16"
        />
        {{ t('settings.agent.addAgent') }}
      </EaButton>
    </div>

    <DetectedCliToolsBanner
      :tools="agentStore.availableToolsToAdd"
      :adding-tool-name="addingToolName"
      @quick-add="handleQuickAdd"
    />

    <!-- 搜索和过滤栏 -->
    <div class="agent-list-page__toolbar">
      <div class="search-box">
        <EaIcon
          name="search"
          :size="16"
          class="search-box__icon"
        />
        <input
          v-model="searchQuery"
          type="text"
          class="search-box__input"
          :placeholder="t('settings.agentList.searchPlaceholder')"
          @input="handleSearchChange"
        >
        <button
          v-if="searchQuery"
          class="search-box__clear"
          @click="searchQuery = ''; handleSearchChange()"
        >
          <EaIcon
            name="x"
            :size="14"
          />
        </button>
      </div>

      <div class="filter-group">
        <EaButton
          v-if="searchQuery"
          type="ghost"
          size="small"
          @click="clearSearch"
        >
          <EaIcon
            name="x"
            :size="14"
          />
          {{ t('common.clearSearch') }}
        </EaButton>
      </div>

      <div class="toolbar-actions">
        <EaButton
          v-if="!showMigrationBanner"
          type="ghost"
          size="small"
          @click="triggerMigrationCheck"
        >
          <EaIcon
            name="refresh-cw"
            :size="14"
          />
        </EaButton>
      </div>

      <div class="agent-count">
        {{ t('settings.agentList.agentCount', { n: filteredAgents.length }) }}
      </div>
    </div>

    <!-- 测试结果提示 -->
    <Transition name="toast">
      <div
        v-if="testResult.visible"
        class="test-result-toast"
        :class="testResult.success ? 'test-result-toast--success' : 'test-result-toast--error'"
      >
        <EaIcon
          :name="testResult.success ? 'check-circle' : 'x-circle'"
          :size="18"
        />
        <span class="test-result-toast__message">{{ testResult.message }}</span>
        <button
          class="test-result-toast__close"
          @click="testResult.visible = false"
        >
          <EaIcon
            name="x"
            :size="14"
          />
        </button>
      </div>
    </Transition>

    <AgentSettingsTable
      :agents="paginatedAgents"
      :search-query="searchQuery"
      :filtered-count="filteredAgents.length"
      :current-page="currentPage"
      :total-pages="totalPages"
      :page-numbers="pageNumbers"
      :page-size="PAGE_SIZE"
      :testing-agent-id="agentStore.testingAgentId"
      @test="handleTest"
      @manage-models="handleOpenModelManage"
      @edit="handleEdit"
      @delete="handleDelete"
      @page-change="goToPage"
    />

    <CliSettings embedded />

    <!-- 配置表单弹框 -->
    <Teleport to="body">
      <Transition name="modal">
        <div
          v-if="showModal"
          class="modal-overlay"
          @click="showModal = false"
        >
          <div
            class="modal-container"
            @click.stop
          >
            <AgentConfigForm
              :agent="editingAgent"
              @submit="handleSubmit"
              @cancel="handleCancel"
            />
          </div>
        </div>
      </Transition>
    </Teleport>

    <AgentSettingsDeleteDialog
      v-model:visible="showDeleteConfirm"
      :agent-name="deletingAgent?.name || ''"
      @cancel="deletingAgent = null"
      @confirm="confirmDelete"
    />

    <!-- 模型管理弹窗 -->
    <Teleport to="body">
      <Transition name="modal">
        <div
          v-if="showModelManageModal && managingModelAgent"
          class="modal-overlay"
          @click="handleCloseModelManage"
        >
          <div
            class="modal-container modal-container--lg"
            @click.stop
          >
            <ModelManageModal
              :agent-id="managingModelAgent.id"
              @close="handleCloseModelManage"
            />
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.agent-list-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.agent-list-page__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.agent-list-page__title {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

/* 搜索和过滤栏 */
.agent-list-page__toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-3);
  background-color: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
}

.search-box {
  position: relative;
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 200px;
  max-width: 320px;
}

.search-box__icon {
  position: absolute;
  left: var(--spacing-3);
  color: var(--color-text-tertiary);
  pointer-events: none;
}

.search-box__input {
  width: 100%;
  padding: var(--spacing-2) var(--spacing-8);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  transition: border-color var(--transition-fast);
}

.search-box__input:focus {
  border-color: var(--color-primary);
  outline: none;
}

.search-box__input::placeholder {
  color: var(--color-text-tertiary);
}

.search-box__clear {
  position: absolute;
  right: var(--spacing-2);
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  background: none;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.search-box__clear:hover {
  background-color: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.filter-group {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  margin-left: auto;
}

.agent-count {
  margin-left: auto;
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

/* 测试结果提示 */
.test-result-toast {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-3) var(--spacing-4);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  animation: slide-in 0.3s ease-out;
}

.test-result-toast--success {
  background-color: rgba(34, 197, 94, 0.1);
  border: 1px solid var(--color-success, #22c55e);
  color: var(--color-success, #22c55e);
}

.test-result-toast--error {
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-error, #ef4444);
  color: var(--color-error, #ef4444);
}

.test-result-toast__message {
  flex: 1;
}

.test-result-toast__close {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-1);
  background: none;
  border: none;
  cursor: pointer;
  color: inherit;
  opacity: 0.7;
  transition: opacity var(--transition-fast);
}

.test-result-toast__close:hover {
  opacity: 1;
}

@keyframes slide-in {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}

/* 弹框样式 */
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
}

.modal-container {
  width: 480px;
  max-width: 90vw;
  background-color: var(--color-surface);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
}

.modal-container--lg {
  width: 720px;
  max-height: 85vh;
}

/* 动画 */
.modal-enter-active,
.modal-leave-active {
  transition: opacity var(--transition-normal) var(--easing-default);
}

.modal-enter-active .modal-container,
.modal-leave-active .modal-container {
  transition: transform var(--transition-normal) var(--easing-default),
              opacity var(--transition-normal) var(--easing-default);
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-container,
.modal-leave-to .modal-container {
  transform: scale(0.95);
  opacity: 0;
}

/* 迁移提示横幅 */
.migration-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: var(--spacing-3);
  padding: var(--spacing-4);
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.1) 0%, rgba(139, 92, 246, 0.1) 100%);
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: var(--radius-lg);
}

.migration-banner__content {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-3);
}

.migration-banner__icon {
  color: var(--color-primary);
  flex-shrink: 0;
  margin-top: 2px;
}

.migration-banner__text {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-1);
}

.migration-banner__title {
  margin: 0;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.migration-banner__description {
  margin: 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.migration-banner__actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

/* 迁移结果提示 */
.migration-result-toast {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
  padding: var(--spacing-3) var(--spacing-4);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  animation: slide-in 0.3s ease-out;
}

.migration-result-toast--success {
  background-color: rgba(34, 197, 94, 0.1);
  border: 1px solid var(--color-success, #22c55e);
  color: var(--color-success, #22c55e);
}

.migration-result-toast--error {
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--color-error, #ef4444);
  color: var(--color-error, #ef4444);
}

.migration-result-toast__message {
  flex: 1;
}

.migration-result-toast__close {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-1);
  background: none;
  border: none;
  cursor: pointer;
  color: inherit;
  opacity: 0.7;
  transition: opacity var(--transition-fast);
}

.migration-result-toast__close:hover {
  opacity: 1;
}

/* 横幅动画 */
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
