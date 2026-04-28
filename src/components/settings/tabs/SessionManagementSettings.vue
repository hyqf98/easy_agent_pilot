<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon, EaSelect } from '@/components/common'
import { useAgentStore } from '@/stores/agent'
import { useNotificationStore } from '@/stores/notification'
import { useProjectStore } from '@/stores/project'
import CliSessionBrowser from '@/components/settings/session-manager/CliSessionBrowser.vue'
import CliSessionDeleteModal from '@/components/settings/session-manager/CliSessionDeleteModal.vue'
import CliSessionDetailModal from '@/components/settings/session-manager/CliSessionDetailModal.vue'
import { buildCliDeleteErrorMessage } from '@/utils/sessionManager'
import type {
  AgentCliSessionProjectsResult,
  AgentCliSessionsResult,
  CliSessionDetail,
  DeleteCliSessionsResult,
  ScannedCliSession
} from '@/types/cliSessionManager'

const { t } = useI18n()
const agentStore = useAgentStore()
const projectStore = useProjectStore()
const notificationStore = useNotificationStore()

const selectedAgentId = ref('')
const selectedProjectPath = ref('')
const selectedUpdatedRange = ref<'all' | '24h' | '7d' | '30d'>('all')

const sessions = ref<ScannedCliSession[]>([])
const cliName = ref('')
const sessionRoot = ref('')
const availableProjects = ref<string[]>([])

const isLoadingSessions = ref(false)
const isLoadingProjects = ref(false)
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
const isPreparingCurrentProjectDelete = ref(false)

let projectListRequestId = 0
let sessionLoadRequestId = 0
let suppressProjectPathWatch = false

const cliAgents = computed(() => agentStore.agents.filter(agent => agent.type === 'cli'))
const hasCliAgents = computed(() => cliAgents.value.length > 0)
const currentProjectPath = computed(() => projectStore.currentProject?.path ?? '')
const selectedSessionPathSet = computed(() => new Set(selectedSessionPaths.value))
const updatedRangeOptions = computed(() => [
  { value: 'all', label: t('settings.sessionManager.updatedRangeAll') },
  { value: '24h', label: t('settings.sessionManager.updatedRange24h') },
  { value: '7d', label: t('settings.sessionManager.updatedRange7d') },
  { value: '30d', label: t('settings.sessionManager.updatedRange30d') }
])

const getUpdatedRangeCutoff = (range: 'all' | '24h' | '7d' | '30d') => {
  if (range === 'all') return null

  const now = Date.now()
  const hours = range === '24h' ? 24 : range === '7d' ? 24 * 7 : 24 * 30
  return now - hours * 60 * 60 * 1000
}

const filteredSessions = computed(() => {
  const cutoff = getUpdatedRangeCutoff(selectedUpdatedRange.value)
  if (cutoff === null) {
    return sessions.value
  }

  return sessions.value.filter(session => {
    const updatedAt = new Date(session.updated_at).getTime()
    return Number.isFinite(updatedAt) && updatedAt >= cutoff
  })
})

const selectedSessions = computed(() =>
  filteredSessions.value.filter(session => selectedSessionPathSet.value.has(session.session_path))
)
const selectedCount = computed(() => selectedSessions.value.length)
const allVisibleSelected = computed(() =>
  filteredSessions.value.length > 0 && selectedCount.value === filteredSessions.value.length
)
const sessionListLoading = computed(() => isLoadingProjects.value || isLoadingSessions.value)

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
  const seen = new Set<string>()
  const projectPaths = [...availableProjects.value]

  if (selectedProjectPath.value && !projectPaths.includes(selectedProjectPath.value)) {
    projectPaths.unshift(selectedProjectPath.value)
  }

  for (const path of projectPaths) {
    if (seen.has(path)) continue
    seen.add(path)

    const name = path.split('/').pop() || path.split('\\').pop() || path
    options.push({ value: path, label: name })
  }

  return options
})

const groupedSessions = computed(() => {
  const groups: Record<string, ScannedCliSession[]> = {}

  for (const session of filteredSessions.value) {
    const key = session.project_path || t('settings.sessionManager.noProject')
    if (!groups[key]) {
      groups[key] = []
    }
    groups[key].push(session)
  }

  for (const key of Object.keys(groups)) {
    groups[key].sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
  }

  return groups
})

const resolveNextProjectPath = (projectPaths: string[], preferredProjectPath?: string | null) => {
  if (preferredProjectPath === null) {
    return ''
  }

  if (typeof preferredProjectPath === 'string' && preferredProjectPath.trim()) {
    return preferredProjectPath
  }

  if (
    selectedProjectPath.value &&
    (projectPaths.includes(selectedProjectPath.value) || selectedProjectPath.value === currentProjectPath.value)
  ) {
    return selectedProjectPath.value
  }

  if (currentProjectPath.value && projectPaths.includes(currentProjectPath.value)) {
    return currentProjectPath.value
  }

  return projectPaths[0] || ''
}

const loadProjectPaths = async (preferredProjectPath?: string | null) => {
  sessionsError.value = ''

  if (!selectedAgentId.value) {
    availableProjects.value = []
    selectedProjectPath.value = ''
    return false
  }

  const requestId = ++projectListRequestId
  isLoadingProjects.value = true

  try {
    const result = await invoke<AgentCliSessionProjectsResult>('list_agent_cli_session_projects', {
      agentId: selectedAgentId.value
    })

    if (requestId !== projectListRequestId) {
      return false
    }

    availableProjects.value = result.project_paths
    cliName.value = result.cli_name
    sessionRoot.value = result.session_root

    const nextProjectPath = resolveNextProjectPath(result.project_paths, preferredProjectPath)
    const changed = nextProjectPath !== selectedProjectPath.value
    selectedProjectPath.value = nextProjectPath
    return changed
  } catch (error) {
    if (requestId !== projectListRequestId) {
      return false
    }

    availableProjects.value = []
    selectedProjectPath.value = ''
    sessions.value = []
    selectedSessionPaths.value = []
    sessionsError.value = String(error)
    return false
  } finally {
    if (requestId === projectListRequestId) {
      isLoadingProjects.value = false
    }
  }
}

const loadSessions = async () => {
  sessionsError.value = ''
  sessions.value = []
  selectedSessionPaths.value = []

  if (!selectedAgentId.value) {
    return
  }

  const requestId = ++sessionLoadRequestId
  isLoadingSessions.value = true

  try {
    const result = await invoke<AgentCliSessionsResult>('list_agent_cli_sessions', {
      agentId: selectedAgentId.value,
      projectPath: selectedProjectPath.value || null
    })

    if (requestId !== sessionLoadRequestId) {
      return
    }

    sessions.value = result.sessions
    cliName.value = result.cli_name
    sessionRoot.value = result.session_root

    if (result.project_paths.length > 0) {
      availableProjects.value = result.project_paths
    }
  } catch (error) {
    if (requestId !== sessionLoadRequestId) {
      return
    }

    sessionsError.value = String(error)
  } finally {
    if (requestId === sessionLoadRequestId) {
      isLoadingSessions.value = false
    }
  }
}

const handleRefresh = async () => {
  if (!selectedAgentId.value) return

  const preserveSelection = selectedProjectPath.value || null
  suppressProjectPathWatch = true
  try {
    const selectionChanged = await loadProjectPaths(preserveSelection)
    if (!selectionChanged) {
      await loadSessions()
    }
  } finally {
    suppressProjectPathWatch = false
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

const requestDeleteCurrentProjectSessions = async () => {
  if (!selectedAgentId.value || !currentProjectPath.value) return

  isPreparingCurrentProjectDelete.value = true
  deleteError.value = ''

  try {
    const result = await invoke<AgentCliSessionsResult>('list_agent_cli_sessions', {
      agentId: selectedAgentId.value,
      projectPath: currentProjectPath.value
    })

    if (!result.sessions.length) {
      notificationStore.info(t('settings.sessionManager.noCurrentProjectSessions'))
      return
    }

    pendingDeleteSessions.value = result.sessions
    showDeleteModal.value = true
  } catch (error) {
    notificationStore.error(
      t('settings.sessionManager.loadCurrentProjectSessionsFailed'),
      String(error)
    )
  } finally {
    isPreparingCurrentProjectDelete.value = false
  }
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

  selectedSessionPaths.value = filteredSessions.value.map(session => session.session_path)
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
    await loadProjectPaths(selectedProjectPath.value || null)
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

watch(selectedAgentId, async () => {
  selectedProjectPath.value = ''
  sessions.value = []
  selectedSessionPaths.value = []
  availableProjects.value = []

  if (!selectedAgentId.value) {
    return
  }

  suppressProjectPathWatch = true
  try {
    await loadProjectPaths()
    await loadSessions()
  } finally {
    suppressProjectPathWatch = false
  }
})

watch(selectedProjectPath, async (next, prev) => {
  if (suppressProjectPathWatch || !selectedAgentId.value || next === prev) {
    return
  }

  await loadSessions()
})

watch(filteredSessions, nextSessions => {
  if (selectedSessionPaths.value.length === 0) {
    return
  }

  const visiblePaths = new Set(nextSessions.map(session => session.session_path))
  selectedSessionPaths.value = selectedSessionPaths.value.filter(path => visiblePaths.has(path))
}, { deep: true })

onMounted(async () => {
  if (!agentStore.agents.length) {
    await agentStore.loadAgents()
  }

  if (cliAgents.value.length && !selectedAgentId.value) {
    selectedAgentId.value = cliAgents.value[0].id
    return
  }

  if (selectedAgentId.value) {
    await handleRefresh()
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
          />
        </div>

        <div class="toolbar__item">
          <label class="toolbar__label">{{ t('settings.sessionManager.updatedRangeLabel') }}</label>
          <EaSelect
            v-model="selectedUpdatedRange"
            :options="updatedRangeOptions"
          />
        </div>

        <div class="toolbar__actions">
          <EaButton
            type="danger"
            size="small"
            :disabled="!currentProjectPath || isPreparingCurrentProjectDelete"
            :loading="isPreparingCurrentProjectDelete"
            @click="requestDeleteCurrentProjectSessions"
          >
            {{ t('settings.sessionManager.deleteCurrentProjectSessions') }}
          </EaButton>

          <EaButton
            type="ghost"
            size="small"
            :disabled="sessionListLoading"
            @click="handleRefresh"
          >
            <EaIcon
              name="refresh-cw"
              :size="14"
              :class="{ 'is-spinning': sessionListLoading }"
            />
            {{ t('common.refresh') }}
          </EaButton>
        </div>
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
      :sessions="filteredSessions"
      :grouped-sessions="groupedSessions"
      :is-loading-sessions="sessionListLoading"
      :sessions-error="sessionsError"
      :selected-session-paths="selectedSessionPaths"
      :selected-count="selectedCount"
      :all-visible-selected="allVisibleSelected"
      @refresh="handleRefresh"
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
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
  gap: var(--spacing-3);
  align-items: end;
}

.toolbar__item {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
}

.toolbar__actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--spacing-2);
  flex-wrap: wrap;
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

  .toolbar__actions {
    justify-content: flex-start;
  }
}
</style>
