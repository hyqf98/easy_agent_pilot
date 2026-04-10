<script setup lang="ts">
import { EaIcon, EaButton, EaSkeleton, EaStateBlock } from '@/components/common'
import PanelHeader from '../PanelHeader.vue'
import SessionPanelDialogs from '../sessionPanelDialogs/SessionPanelDialogs.vue'
import SessionPanelItem from '../SessionPanelItem.vue'
import type { SessionPanelProps } from './useSessionPanel'
import { useSessionPanelView } from './useSessionPanel'

defineProps<SessionPanelProps>()

defineEmits<{
  toggle: []
}>()

const {
  cancelEditSessionName,
  clearSearch,
  closeClearMessagesConfirm,
  closeDeleteConfirm,
  closeErrorModal,
  closeSummaryModal,
  confirmClearMessages,
  confirmDelete,
  currentProjectSessions,
  deletingSession,
  editingSessionId,
  editingSessionName,
  errorSession,
  handleAdd,
  handleCreateSession,
  handleProjectChange,
  handleRefreshSessions,
  handleSearchInput,
  handleSelectSession,
  hasSearchQuery,
  isClearingMessages,
  isNewSessionFormValid,
  newSessionName,
  projectStore,
  rerunSummarySession,
  retryErroredSession,
  saveEditSessionName,
  searchInput,
  selectedProjectId,
  sessionActionMap,
  sessionStore,
  showClearMessagesConfirm,
  showDeleteConfirm,
  showErrorModal,
  showSummaryModal,
  summarySession,
  t,
  uiStore
} = useSessionPanelView()

</script>

<template>
  <div :class="['session-panel', { 'session-panel--collapsed': collapsed }]">
    <PanelHeader
      :title="t('panel.sessions')"
      icon="message-square"
      :collapsed="collapsed"
      :show-toggle="showHeaderToggle"
      show-add
      @toggle="$emit('toggle')"
      @add="handleAdd"
    />

    <div
      v-if="!collapsed"
      class="session-panel__content"
    >
      <!-- 无项目提示 -->
      <div
        v-if="!projectStore.currentProjectId"
        class="session-empty"
      >
        <EaStateBlock
          icon="folder"
          :description="t('session.noProjectSelected')"
        />
      </div>

      <!-- 加载状态 -->
      <template v-else-if="sessionStore.isLoading">
        <!-- 搜索框骨架屏 -->
        <div class="session-search session-search--loading">
          <EaSkeleton
            variant="circle"
            height="14px"
            width="14px"
            animation="wave"
          />
          <EaSkeleton
            variant="text"
            height="14px"
            width="60%"
            animation="wave"
          />
        </div>
        <!-- 会话列表骨架屏 -->
        <div class="session-loading">
          <div
            v-for="i in 3"
            :key="i"
            class="session-skeleton"
          >
            <div class="session-skeleton__header">
              <EaSkeleton
                variant="circle"
                height="14px"
                width="14px"
                animation="wave"
              />
              <EaSkeleton
                variant="text"
                height="14px"
                :width="`${40 + Math.random() * 30}%`"
                animation="wave"
              />
            </div>
            <div class="session-skeleton__meta">
              <EaSkeleton
                variant="text"
                height="12px"
                width="50px"
                animation="wave"
              />
            </div>
            <div class="session-skeleton__preview">
              <EaSkeleton
                variant="text"
                height="12px"
                :width="`${70 + Math.random() * 20}%`"
                animation="wave"
              />
            </div>
          </div>
        </div>
      </template>

      <!-- 错误状态 -->
      <div
        v-else-if="sessionStore.loadError"
        class="session-error"
      >
        <EaStateBlock
          variant="error"
          :title="t('common.loadFailed')"
          :description="sessionStore.loadError"
        >
          <template #actions>
            <EaButton
              type="primary"
              size="small"
              @click="handleRefreshSessions"
            >
              <EaIcon
                name="refresh-cw"
                :size="14"
              />
              {{ t('common.retry') }}
            </EaButton>
          </template>
        </EaStateBlock>
      </div>

      <!-- 项目和智能体选择 -->
      <div
        v-else
        class="session-filters"
      >
        <!-- 项目选择 -->
        <div class="session-filter">
          <select
            v-model="selectedProjectId"
            class="session-filter__select"
            @change="handleProjectChange(($event.target as HTMLSelectElement).value)"
          >
            <option
              value=""
              disabled
            >
              {{ t('session.selectProject') }}
            </option>
            <option
              v-for="project in projectStore.projects"
              :key="project.id"
              :value="project.id"
            >
              {{ project.name }}
            </option>
          </select>
          <EaIcon
            name="chevron-down"
            :size="14"
            class="session-filter__icon"
          />
        </div>
      </div>

      <!-- 搜索框 -->
      <div class="session-search">
        <EaIcon
          name="search"
          :size="14"
          class="session-search__icon"
        />
        <input
          :value="searchInput"
          type="text"
          class="session-search__input"
          :placeholder="t('session.searchSessions')"
          @input="handleSearchInput(($event.target as HTMLInputElement).value)"
        >
        <button
          v-if="hasSearchQuery"
          class="session-search__clear"
          :title="t('common.clearSearch')"
          @click="clearSearch"
        >
          <EaIcon
            name="x"
            :size="14"
          />
        </button>
      </div>

      <!-- 搜索无结果 -->
      <div
        v-if="projectStore.currentProjectId && hasSearchQuery && currentProjectSessions.length === 0"
        class="session-empty"
      >
        <EaStateBlock
          icon="search"
          :description="t('session.noMatchingSessions')"
        >
          <template #actions>
            <EaButton
              type="secondary"
              size="small"
              @click="clearSearch"
            >
              {{ t('common.clearSearch') }}
            </EaButton>
          </template>
        </EaStateBlock>
      </div>

      <!-- 空状态（无会话） -->
      <div
        v-else-if="projectStore.currentProjectId && !hasSearchQuery && currentProjectSessions.length === 0"
        class="session-empty"
      >
        <EaStateBlock
          icon="message-square-plus"
          :description="t('session.noSessions')"
        >
          <template #actions>
            <EaButton
              type="primary"
              size="small"
              @click="handleAdd"
            >
              <EaIcon
                name="plus"
                :size="14"
              />
              {{ t('session.createSession') }}
            </EaButton>
          </template>
        </EaStateBlock>
      </div>

      <!-- 会话列表 -->
      <div
        v-else-if="projectStore.currentProjectId"
        class="session-list"
        role="list"
      >
        <SessionPanelItem
          v-for="session in currentProjectSessions"
          :key="session.id"
          :session="session"
          :active="session.id === sessionStore.currentSessionId"
          :editing-session-id="editingSessionId"
          :editing-session-name="editingSessionName"
          :search-query="sessionStore.searchQuery"
          :actions="(sessionActionMap.get(session.id) ?? []).map(({ key, title, icon, danger, warning }) => ({ key, title, icon, danger, warning }))"
          @select="handleSelectSession"
          @save-name="saveEditSessionName"
          @cancel-edit="cancelEditSessionName"
          @update-name="editingSessionName = $event"
          @action="(key, targetSession) => sessionActionMap.get(targetSession.id)?.find(action => action.key === key)?.handler(targetSession)"
        />
      </div>
    </div>

    <SessionPanelDialogs
      :session-create-modal-visible="uiStore.sessionCreateModalVisible"
      :new-session-name="newSessionName"
      :is-new-session-form-valid="isNewSessionFormValid"
      :show-delete-confirm="showDeleteConfirm"
      :deleting-session="deletingSession"
      :show-clear-messages-confirm="showClearMessagesConfirm"
      :is-clearing-messages="isClearingMessages"
      :show-error-modal="showErrorModal"
      :error-session="errorSession"
      :show-summary-modal="showSummaryModal"
      :summary-session="summarySession"
      @close-create="uiStore.closeSessionCreateModal()"
      @submit-create="handleCreateSession(newSessionName.trim())"
      @update-new-session-name="newSessionName = $event"
      @close-delete="closeDeleteConfirm"
      @confirm-delete="confirmDelete"
      @close-clear-messages="closeClearMessagesConfirm"
      @confirm-clear-messages="confirmClearMessages"
      @close-error="closeErrorModal"
      @retry-error="retryErroredSession"
      @close-summary="closeSummaryModal"
      @rerun-summary="rerunSummarySession"
    />
  </div>
</template>

<style scoped src="./styles.css"></style>
