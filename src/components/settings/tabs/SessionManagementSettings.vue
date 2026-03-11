<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon, EaSelect } from '@/components/common'
import { useAgentStore } from '@/stores/agent'
import CliSessionBrowser from '@/components/settings/session-manager/CliSessionBrowser.vue'
import CliSessionDeleteModal from '@/components/settings/session-manager/CliSessionDeleteModal.vue'
import CliSessionDetailModal from '@/components/settings/session-manager/CliSessionDetailModal.vue'
import {
  buildCliDeleteErrorMessage,
} from '@/utils/sessionManager'
import type {
  AgentCliSessionsResult,
  CliSessionDetail,
  DeleteCliSessionsResult,
  ScannedCliSession
} from '@/types/cliSessionManager'

const { t } = useI18n()
const agentStore = useAgentStore()

const selectedAgentId = ref('')
const selectedProjectPath = ref<string>('')

const sessions = ref<ScannedCliSession[]>([])
const cliName = ref('')
const sessionRoot = ref('')
const availableProjects = ref<string[]>([])

const isLoadingSessions = ref(false)
const sessionsError = ref('')

const showDetailModal = ref(false)
const detailLoading = ref(false)
const detailError = ref('')
const currentDetail = ref<CliSessionDetail | null>(null)

const showDeleteModal = ref(false)
const deleting = ref(false)
const pendingDeleteSessions = ref<ScannedCliSession[]>([])
const deleteError = ref('')
const selectedSessionPaths = ref<string[]>([])

const cliAgents = computed(() => agentStore.agents.filter(agent => agent.type === 'cli'))
const hasCliAgents = computed(() => cliAgents.value.length > 0)
const selectedSessionPathSet = computed(() => new Set(selectedSessionPaths.value))
const selectedSessions = computed(() =>
  sessions.value.filter(session => selectedSessionPathSet.value.has(session.session_path))
)
const selectedCount = computed(() => selectedSessions.value.length)
const allVisibleSelected = computed(() =>
  sessions.value.length > 0 && selectedCount.value === sessions.value.length
)

const agentOptions = computed(() =>
  cliAgents.value.map(agent => {
    const provider = agent.provider ? agent.provider.toUpperCase() : 'CLI'
    return {
      value: agent.id,
      label: `${agent.name} (${provider})`
    }
  })
)

const projectOptions = computed(() => {
  const options = [{ value: '', label: t('settings.sessionManager.allProjects') }]
  for (const path of availableProjects.value) {
    // 显示项目名称而非完整路径
    const name = path.split('/').pop() || path.split('\\').pop() || path
    options.push({ value: path, label: name })
  }
  return options
})

// 按项目分组会话
const groupedSessions = computed(() => {
  const groups: Record<string, ScannedCliSession[]> = {}

  for (const session of sessions.value) {
    const key = session.project_path || t('settings.sessionManager.noProject')
    if (!groups[key]) {
      groups[key] = []
    }
    groups[key].push(session)
  }

  // 按更新时间排序每个分组内的会话
  for (const key of Object.keys(groups)) {
    groups[key].sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
  }

  return groups
})

const loadSessions = async () => {
  sessionsError.value = ''
  sessions.value = []
  availableProjects.value = []
  selectedSessionPaths.value = []

  if (!selectedAgentId.value) {
    return
  }

  isLoadingSessions.value = true
  try {
    const projectPath = selectedProjectPath.value || null
    const result = await invoke<AgentCliSessionsResult>('list_agent_cli_sessions', {
      agentId: selectedAgentId.value,
      projectPath
    })
    sessions.value = result.sessions
    cliName.value = result.cli_name
    sessionRoot.value = result.session_root

    // 只在首次加载或没有选择项目时更新项目列表
    if (!selectedProjectPath.value) {
      availableProjects.value = result.project_paths
    }
  } catch (error) {
    sessionsError.value = String(error)
  } finally {
    isLoadingSessions.value = false
  }
}

const openDetail = async (session: ScannedCliSession) => {
  showDetailModal.value = true
  detailLoading.value = true
  detailError.value = ''
  currentDetail.value = null

  try {
    const detail = await invoke<CliSessionDetail>('read_cli_session_detail', {
      agentId: selectedAgentId.value,
      sessionPath: session.session_path
    })
    currentDetail.value = detail
  } catch (error) {
    detailError.value = String(error)
  } finally {
    detailLoading.value = false
  }
}

const requestDelete = (session: ScannedCliSession) => {
  pendingDeleteSessions.value = [session]
  deleteError.value = ''
  showDeleteModal.value = true
}

const requestDeleteSelected = () => {
  if (!selectedSessions.value.length) return
  pendingDeleteSessions.value = [...selectedSessions.value]
  deleteError.value = ''
  showDeleteModal.value = true
}

const closeDeleteModal = () => {
  showDeleteModal.value = false
  pendingDeleteSessions.value = []
  deleteError.value = ''
}

const toggleSessionSelection = (sessionPath: string, checked?: boolean) => {
  const next = new Set(selectedSessionPaths.value)
  const shouldSelect = checked ?? !next.has(sessionPath)

  if (shouldSelect) {
    next.add(sessionPath)
  } else {
    next.delete(sessionPath)
  }

  selectedSessionPaths.value = Array.from(next)
}

const handleSessionSelectionChange = (sessionPath: string, event: Event) => {
  const target = event.target as HTMLInputElement | null
  toggleSessionSelection(sessionPath, target?.checked ?? false)
}

const toggleSelectAllSessions = () => {
  if (allVisibleSelected.value) {
    selectedSessionPaths.value = []
    return
  }

  selectedSessionPaths.value = sessions.value.map(session => session.session_path)
}

const buildDeleteErrorMessage = (failedPaths: string[]) =>
  buildCliDeleteErrorMessage(failedPaths, n => t('settings.sessionManager.partialDeleteFailed', { n }))

const confirmDelete = async () => {
  if (!pendingDeleteSessions.value.length) return

  deleting.value = true
  deleteError.value = ''

  const sessionPaths = pendingDeleteSessions.value.map(session => session.session_path)
  const shouldCloseDetail = !!currentDetail.value && sessionPaths.includes(currentDetail.value.session_path)

  try {
    if (sessionPaths.length === 1) {
      await invoke('delete_cli_session', {
        agentId: selectedAgentId.value,
        sessionPath: sessionPaths[0],
        cleanupEmptyDirs: true
      })
    } else {
      const result = await invoke<DeleteCliSessionsResult>('delete_cli_sessions', {
        agentId: selectedAgentId.value,
        sessionPaths,
        cleanupEmptyDirs: true
      })

      if (result.failed_paths.length > 0) {
        deleteError.value = buildDeleteErrorMessage(result.failed_paths)
        await loadSessions()
        if (shouldCloseDetail) {
          showDetailModal.value = false
          currentDetail.value = null
        }
        return
      }
    }

    if (shouldCloseDetail) {
      showDetailModal.value = false
      currentDetail.value = null
    }

    closeDeleteModal()
    await loadSessions()
  } catch (error) {
    deleteError.value = String(error)
  } finally {
    deleting.value = false
  }
}

watch(cliAgents, (agents) => {
  if (!agents.length) {
    selectedAgentId.value = ''
    sessions.value = []
    return
  }

  if (!agents.some(agent => agent.id === selectedAgentId.value)) {
    selectedAgentId.value = agents[0].id
  }
}, { immediate: true })

// 当切换智能体时，重置项目选择并加载会话
watch(selectedAgentId, () => {
  selectedProjectPath.value = ''
  loadSessions()
})

// 当切换项目时，加载会话
watch(selectedProjectPath, () => {
  loadSessions()
})

onMounted(async () => {
  if (!agentStore.agents.length) {
    await agentStore.loadAgents()
  }

  if (cliAgents.value.length && !selectedAgentId.value) {
    selectedAgentId.value = cliAgents.value[0].id
  } else if (selectedAgentId.value && !isLoadingSessions.value && sessions.value.length === 0) {
    await loadSessions()
  }
})
</script>

<template>
  <div class="settings-page">
    <h3 class="settings-page__title">
      {{ t('settings.nav.sessions') }}
    </h3>

    <div class="settings-card">
      <h4 class="settings-card__title">
        {{ t('settings.sessionManager.agentSelection') }}
      </h4>

      <div
        v-if="hasCliAgents"
        class="toolbar"
      >
        <div class="toolbar__item">
          <label class="toolbar__label">{{ t('settings.sessionManager.agentLabel') }}</label>
          <EaSelect
            v-model="selectedAgentId"
            :options="agentOptions"
          />
        </div>

        <div class="toolbar__item">
          <label class="toolbar__label">{{ t('settings.sessionManager.projectLabel') }}</label>
          <EaSelect
            v-model="selectedProjectPath"
            :options="projectOptions"
            :disabled="availableProjects.length === 0"
          />
        </div>

        <EaButton
          type="ghost"
          size="small"
          :disabled="isLoadingSessions"
          @click="loadSessions"
        >
          <EaIcon
            name="refresh-cw"
            :size="14"
            :class="{ 'is-spinning': isLoadingSessions }"
          />
          {{ t('common.refresh') }}
        </EaButton>
      </div>

      <div
        v-else
        class="empty-state"
      >
        <EaIcon
          name="terminal"
          :size="24"
        />
        <span>{{ t('settings.sessionManager.noCliAgents') }}</span>
      </div>
    </div>

    <CliSessionBrowser
      v-if="hasCliAgents"
      :cli-name="cliName"
      :session-root="sessionRoot"
      :sessions="sessions"
      :grouped-sessions="groupedSessions"
      :is-loading-sessions="isLoadingSessions"
      :sessions-error="sessionsError"
      :selected-session-paths="selectedSessionPaths"
      :selected-count="selectedCount"
      :all-visible-selected="allVisibleSelected"
      @refresh="loadSessions"
      @toggle-select-all="toggleSelectAllSessions"
      @request-delete-selected="requestDeleteSelected"
      @selection-change="handleSessionSelectionChange"
      @open-detail="openDetail"
      @request-delete="requestDelete"
    />

    <CliSessionDetailModal
      v-model:visible="showDetailModal"
      :loading="detailLoading"
      :error="detailError"
      :detail="currentDetail"
    />

    <CliSessionDeleteModal
      v-model:visible="showDeleteModal"
      :deleting="deleting"
      :sessions="pendingDeleteSessions"
      :error="deleteError"
      @confirm="confirmDelete"
      @update:visible="value => !value && closeDeleteModal()"
    />
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-6);
}

.settings-page__title {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.settings-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-4);
  padding: var(--spacing-5);
  background-color: var(--color-bg-secondary);
  border-radius: var(--radius-lg);
}

.settings-card__title {
  margin: 0;
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  padding-bottom: var(--spacing-3);
  border-bottom: 1px solid var(--color-border);
}

.toolbar {
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  gap: var(--spacing-3);
  align-items: end;
}

.toolbar__item {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.toolbar__label {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.is-spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.loading,
.error,
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  min-height: 120px;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.error {
  color: var(--color-error);
  white-space: pre-wrap;
}
@media (max-width: 860px) {
  .toolbar {
    grid-template-columns: 1fr;
  }
}
</style>
