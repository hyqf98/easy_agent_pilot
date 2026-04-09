import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentConfig } from '@/stores/agent'
import {
  useSkillConfigStore,
  type CliSyncPreviewItem,
  type CliSyncResult,
  type SyncConfigType,
} from '@/stores/skillConfig'
import type { SelectOption } from '@/components/common'

export interface CliConfigSyncModalProps {
  visible: boolean
  syncType: SyncConfigType
  agents: AgentConfig[]
  selectedAgent: AgentConfig | null
}

export type CliConfigSyncModalEmits = {
  (e: 'close'): void
  (e: 'completed', payload: { targetAgentId: string; result: CliSyncResult }): void
}

/**
 * 管理 CLI 配置同步弹窗的源目标扫描、选择集和同步结果回写。
 */
export function useCliConfigSyncModal(
  props: Readonly<CliConfigSyncModalProps>,
  emit: CliConfigSyncModalEmits
) {
  const { t } = useI18n()
  const skillConfigStore = useSkillConfigStore()

  const sourceAgentId = ref('')
  const targetAgentId = ref('')
  const sourceItems = ref<CliSyncPreviewItem[]>([])
  const targetItems = ref<CliSyncPreviewItem[]>([])
  const selectedNames = ref<string[]>([])
  const syncResult = ref<CliSyncResult | null>(null)
  const isLoadingSource = ref(false)
  const isLoadingTarget = ref(false)
  const isSyncing = ref(false)
  const errorMessage = ref('')

  const eligibleAgents = computed(() =>
    props.agents.filter(
      (agent) =>
        agent.type === 'cli'
        && !!agent.cliPath
        && (agent.provider === 'claude' || agent.provider === 'codex' || agent.provider === 'opencode')
    )
  )

  const sourceOptions = computed<SelectOption[]>(() =>
    eligibleAgents.value.map((agent) => ({
      value: agent.id,
      label: `${agent.name} · ${getProviderLabel(agent)}`,
    }))
  )

  const sourceAgent = computed(
    () => eligibleAgents.value.find((agent) => agent.id === sourceAgentId.value) || null
  )

  const targetCandidates = computed(() => {
    const currentSource = sourceAgent.value
    if (!currentSource) {
      return []
    }

    return eligibleAgents.value.filter(
      (agent) => agent.id !== currentSource.id && agent.provider !== currentSource.provider
    )
  })

  const targetOptions = computed<SelectOption[]>(() =>
    targetCandidates.value.map((agent) => ({
      value: agent.id,
      label: `${agent.name} · ${getProviderLabel(agent)}`,
    }))
  )

  const targetAgent = computed(
    () => targetCandidates.value.find((agent) => agent.id === targetAgentId.value) || null
  )

  const targetNames = computed(() => new Set(targetItems.value.map((item) => item.name)))

  const isAllSelected = computed(
    () => sourceItems.value.length > 0 && selectedNames.value.length === sourceItems.value.length
  )

  const existingCount = computed(
    () => selectedNames.value.filter((name) => targetNames.value.has(name)).length
  )

  const newCount = computed(() =>
    Math.max(selectedNames.value.length - existingCount.value, 0)
  )

  const canSubmit = computed(() =>
    !!sourceAgent.value?.cliPath
    && !!targetAgent.value?.cliPath
    && selectedNames.value.length > 0
    && !isLoadingSource.value
    && !isLoadingTarget.value
    && !isSyncing.value
  )

  const sourceEmptyText = computed(() =>
    props.syncType === 'mcp'
      ? t('settings.integration.emptyMcp')
      : t('settings.integration.emptySkills')
  )

  const targetEmptyText = computed(() =>
    props.syncType === 'mcp'
      ? t('settings.integration.sync.targetEmptyMcp')
      : t('settings.integration.sync.targetEmptySkills')
  )

  function getProviderLabel(agent: AgentConfig) {
    return agent.provider === 'claude' ? 'Claude CLI' : agent.provider === 'codex' ? 'Codex CLI' : 'OpenCode CLI'
  }

  function isItemExisting(name: string) {
    return targetNames.value.has(name)
  }

  function initialiseState() {
    const defaultSource = props.selectedAgent
      && eligibleAgents.value.some((agent) => agent.id === props.selectedAgent?.id)
      ? props.selectedAgent
      : eligibleAgents.value[0] || null

    sourceAgentId.value = defaultSource?.id || ''
    targetAgentId.value = ''
    sourceItems.value = []
    targetItems.value = []
    selectedNames.value = []
    syncResult.value = null
    errorMessage.value = ''
  }

  async function loadSourceItems() {
    if (!props.visible || !sourceAgent.value?.cliPath) {
      sourceItems.value = []
      selectedNames.value = []
      return
    }

    isLoadingSource.value = true
    errorMessage.value = ''
    syncResult.value = null

    try {
      sourceItems.value = await skillConfigStore.scanCliItemsForSync(
        sourceAgent.value.cliPath,
        props.syncType,
        sourceAgent.value.provider
      )
      selectedNames.value = sourceItems.value.map((item) => item.name)
    } catch (error) {
      errorMessage.value = String(error)
      sourceItems.value = []
      selectedNames.value = []
    } finally {
      isLoadingSource.value = false
    }
  }

  async function loadTargetItems() {
    if (!props.visible || !targetAgent.value?.cliPath) {
      targetItems.value = []
      return
    }

    isLoadingTarget.value = true

    try {
      targetItems.value = await skillConfigStore.scanCliItemsForSync(
        targetAgent.value.cliPath,
        props.syncType,
        targetAgent.value.provider
      )
    } catch (error) {
      errorMessage.value = String(error)
      targetItems.value = []
    } finally {
      isLoadingTarget.value = false
    }
  }

  async function refreshPanels() {
    await Promise.all([loadSourceItems(), loadTargetItems()])
  }

  function toggleAll() {
    selectedNames.value = isAllSelected.value ? [] : sourceItems.value.map((item) => item.name)
  }

  function toggleItem(name: string) {
    if (selectedNames.value.includes(name)) {
      selectedNames.value = selectedNames.value.filter((item) => item !== name)
      return
    }

    selectedNames.value = [...selectedNames.value, name]
  }

  async function handleSubmit() {
    if (!sourceAgent.value?.cliPath || !targetAgent.value?.cliPath || selectedNames.value.length === 0) {
      return
    }

    isSyncing.value = true
    errorMessage.value = ''

    try {
      const result = await skillConfigStore.syncCliItems({
        sourceCliPath: sourceAgent.value.cliPath,
        targetCliPath: targetAgent.value.cliPath,
        sourceCliType: sourceAgent.value.provider,
        targetCliType: targetAgent.value.provider,
        configType: props.syncType,
        itemNames: selectedNames.value,
      })

      syncResult.value = result
      await loadTargetItems()
      emit('completed', {
        targetAgentId: targetAgent.value.id,
        result,
      })
    } catch (error) {
      errorMessage.value = String(error)
    } finally {
      isSyncing.value = false
    }
  }

  watch(
    () => props.visible,
    (visible) => {
      if (!visible) return

      initialiseState()
      targetAgentId.value = targetCandidates.value[0]?.id || ''
      void refreshPanels()
    }
  )

  watch(sourceAgentId, () => {
    targetAgentId.value = targetCandidates.value[0]?.id || ''
    void refreshPanels()
  })

  watch(targetAgentId, () => {
    void loadTargetItems()
  })

  watch(
    () => props.syncType,
    () => {
      if (props.visible) {
        void refreshPanels()
      }
    }
  )

  return {
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
    refreshPanels,
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
  }
}
