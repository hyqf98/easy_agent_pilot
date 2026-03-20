<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentConfig, AgentProvider, AgentType } from '@/stores/agent'
import { EaButton, EaIcon, EaStateBlock } from '@/components/common'

interface Props {
  agents: AgentConfig[]
  activeAgentId?: string | null
  searchQuery: string
  filteredCount: number
  currentPage: number
  totalPages: number
  pageNumbers: number[]
  pageSize: number
  testingAgentId: string | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  focusAgent: [agent: AgentConfig]
  test: [id: string]
  manageModels: [agent: AgentConfig]
  edit: [agent: AgentConfig]
  delete: [agent: AgentConfig]
  pageChange: [page: number]
}>()

const { t } = useI18n()

const showPagination = computed(() => props.filteredCount > props.pageSize)

function getTypeIcon(type: AgentType): string {
  return type === 'cli' ? 'terminal' : 'code'
}

function getProviderIcon(provider?: AgentProvider): string {
  if (!provider) return 'bot'
  return provider === 'claude' ? 'bot' : 'code'
}

function getProviderText(provider?: AgentProvider): string {
  if (!provider) return '-'
  return provider === 'claude' ? 'Claude' : 'Codex'
}

function getTypeText(type: AgentType): string {
  return type === 'cli' ? 'CLI' : 'SDK'
}

function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function changePage(page: number) {
  emit('pageChange', page)
}
</script>

<template>
  <div class="agent-settings-table">
    <div
      v-if="agents.length > 0"
      class="agent-table-container"
    >
      <table class="agent-table">
        <thead>
          <tr>
            <th class="agent-table__th agent-table__th--name">
              {{ t('settings.agentList.columnName') }}
            </th>
            <th class="agent-table__th agent-table__th--type">
              {{ t('settings.agentList.columnType') }}
            </th>
            <th class="agent-table__th agent-table__th--provider">
              {{ t('settings.agentList.columnProvider') }}
            </th>
            <th class="agent-table__th agent-table__th--model">
              {{ t('settings.agentList.columnModel') }}
            </th>
            <th class="agent-table__th agent-table__th--created">
              {{ t('settings.agentList.columnCreated') }}
            </th>
            <th class="agent-table__th agent-table__th--actions">
              {{ t('settings.agentList.columnActions') }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="agent in agents"
            :key="agent.id"
            class="agent-table__row"
            :class="{ 'agent-table__row--active': props.activeAgentId === agent.id }"
            @click="emit('focusAgent', agent)"
          >
            <td class="agent-table__td agent-table__td--name">
              <div class="agent-name-cell">
                <EaIcon
                  :name="getProviderIcon(agent.provider)"
                  :size="18"
                  class="agent-name-cell__icon"
                />
                <span class="agent-name-cell__text">{{ agent.name }}</span>
              </div>
            </td>
            <td class="agent-table__td agent-table__td--type">
              <div class="type-badge">
                <EaIcon
                  :name="getTypeIcon(agent.type)"
                  :size="14"
                />
                <span>{{ getTypeText(agent.type) }}</span>
              </div>
            </td>
            <td class="agent-table__td agent-table__td--provider">
              <span class="provider-text">{{ getProviderText(agent.provider) }}</span>
            </td>
            <td class="agent-table__td agent-table__td--model">
              <div
                v-if="agent.modelId"
                class="model-cell"
              >
                <span class="model-cell__name">{{ agent.modelId }}</span>
                <span
                  v-if="agent.customModelEnabled"
                  class="model-cell__badge"
                >
                  {{ t('settings.agent.customModel') }}
                </span>
              </div>
              <span
                v-else
                class="model-cell--empty"
              >-</span>
            </td>
            <td class="agent-table__td agent-table__td--created">
              <span class="created-text">{{ formatDate(agent.createdAt) }}</span>
            </td>
            <td class="agent-table__td agent-table__td--actions">
              <div class="action-buttons">
                <EaButton
                  type="ghost"
                  size="small"
                  :loading="testingAgentId === agent.id"
                  @click.stop="emit('test', agent.id)"
                >
                  <EaIcon
                    name="wifi"
                    :size="14"
                  />
                </EaButton>
                <EaButton
                  type="ghost"
                  size="small"
                  @click.stop="emit('manageModels', agent)"
                >
                  <EaIcon
                    name="cpu"
                    :size="14"
                  />
                </EaButton>
                <EaButton
                  type="ghost"
                  size="small"
                  @click.stop="emit('edit', agent)"
                >
                  <EaIcon
                    name="edit-2"
                    :size="14"
                  />
                </EaButton>
                <EaButton
                  type="ghost"
                  size="small"
                  class="action-buttons__delete"
                  @click.stop="emit('delete', agent)"
                >
                  <EaIcon
                    name="trash-2"
                    :size="14"
                  />
                </EaButton>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <EaStateBlock
      v-else
      class="agent-empty"
      icon="bot"
      :title="searchQuery ? t('settings.agentList.noMatchingAgents') : t('settings.agent.noAgents')"
      :description="t('settings.agent.noAgentsHint')"
    />

    <div
      v-if="showPagination"
      class="pagination"
    >
      <button
        class="pagination__btn"
        :disabled="currentPage === 1"
        @click="changePage(currentPage - 1)"
      >
        <EaIcon
          name="chevron-left"
          :size="16"
        />
      </button>

      <template
        v-for="(page, index) in pageNumbers"
        :key="index"
      >
        <span
          v-if="page === -1"
          class="pagination__ellipsis"
        >...</span>
        <button
          v-else
          :class="['pagination__btn', { 'pagination__btn--active': currentPage === page }]"
          @click="changePage(page)"
        >
          {{ page }}
        </button>
      </template>

      <button
        class="pagination__btn"
        :disabled="currentPage === totalPages"
        @click="changePage(currentPage + 1)"
      >
        <EaIcon
          name="chevron-right"
          :size="16"
        />
      </button>
    </div>
  </div>
</template>

<style scoped>
.agent-settings-table {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
}

.agent-table-container {
  overflow-x: auto;
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-border);
  background-color: var(--color-surface);
}

.agent-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: auto;
}

.agent-table__th {
  padding: var(--spacing-3) var(--spacing-4);
  text-align: left;
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-secondary);
  background-color: var(--color-bg-secondary);
  border-bottom: 1px solid var(--color-border);
  white-space: nowrap;
}

.agent-table__th--actions {
  text-align: center;
  width: 140px;
}

.agent-table__row {
  transition: background-color var(--transition-fast);
}

.agent-table__row:hover {
  background-color: var(--color-surface-hover);
}

.agent-table__row--active {
  background-color: var(--color-primary-light);
}

.agent-table__td {
  padding: var(--spacing-3) var(--spacing-4);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  border-bottom: 1px solid var(--color-border);
  vertical-align: middle;
}

.agent-table__row:last-child .agent-table__td {
  border-bottom: none;
}

.agent-table__td--actions {
  text-align: center;
}

.agent-name-cell {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.agent-name-cell__icon {
  color: var(--color-primary);
  flex-shrink: 0;
}

.agent-name-cell__text {
  font-weight: var(--font-weight-medium);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.type-badge {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-1);
  padding: 2px var(--spacing-2);
  background-color: var(--color-primary-light);
  color: var(--color-primary);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.provider-text {
  color: var(--color-text-secondary);
}

.model-cell {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.model-cell__name {
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-secondary);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
}

.model-cell__badge {
  flex-shrink: 0;
  padding: 1px var(--spacing-2);
  background-color: var(--color-warning-light, rgba(234, 179, 8, 0.1));
  color: var(--color-warning, #eab308);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
}

.model-cell--empty {
  color: var(--color-text-tertiary);
}

.created-text {
  color: var(--color-text-tertiary);
  font-size: var(--font-size-xs);
  white-space: nowrap;
}

.action-buttons {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-1);
}

.action-buttons__delete:hover {
  color: var(--color-error, #ef4444);
}

.agent-empty {
  padding: var(--spacing-7) var(--spacing-5);
}

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-1);
  padding: var(--spacing-3);
}

.pagination__btn {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 32px;
  height: 32px;
  padding: 0 var(--spacing-2);
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.pagination__btn:hover:not(:disabled) {
  background-color: var(--color-surface-hover);
  border-color: var(--color-border-dark);
}

.pagination__btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.pagination__btn--active {
  background-color: var(--color-primary);
  border-color: var(--color-primary);
  color: var(--color-text-inverse);
}

.pagination__ellipsis {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 32px;
  height: 32px;
  color: var(--color-text-tertiary);
}
</style>
