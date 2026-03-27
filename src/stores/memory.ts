import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { useNotificationStore } from './notification'
import { useAgentStore, type AgentConfig } from './agent'
import { getErrorMessage } from '@/utils/api'
import { memoryMergeService } from '@/services/memory'
import type {
  BatchDeleteRawMemoryRecordsInput,
  BatchDeleteRawMemoryRecordsResult,
  CaptureUserMessageInput,
  CreateMemoryLibraryInput,
  CreateRawMemoryRecordInput,
  ListMemoryMergeRunsQuery,
  ListRawMemoryRecordsQuery,
  MemoryLibrary,
  MemoryMergeRun,
  MergeRawMemoriesIntoLibraryResult,
  RecordSessionMemoryReferencesInput,
  RawMemoryRecord,
  SearchMemorySuggestionsInput,
  SearchMemorySuggestionsResult,
  UpdateMemoryLibraryInput,
  UpdateRawMemoryRecordInput
} from '@/types/memory'

interface MergeIntoLibraryOptions {
  libraryId?: string
  recordIds?: string[]
  agentId?: string
}

export const useMemoryStore = defineStore('memory', () => {
  const notificationStore = useNotificationStore()

  const libraries = ref<MemoryLibrary[]>([])
  const rawRecords = ref<RawMemoryRecord[]>([])
  const mergeRuns = ref<MemoryMergeRun[]>([])
  const activeLibraryId = ref<string | null>(null)
  const selectedRecordIds = ref<string[]>([])
  const lastRawQuery = ref<ListRawMemoryRecordsQuery>({})
  const isInitializing = ref(false)
  const isLoadingLibraries = ref(false)
  const isLoadingRecords = ref(false)
  const isLoadingMergeRuns = ref(false)
  const isMerging = ref(false)
  const isSavingLibrary = ref(false)
  const isDeletingRecords = ref(false)

  const activeLibrary = computed(() =>
    libraries.value.find((library) => library.id === activeLibraryId.value) ?? null
  )

  const selectedRecords = computed(() =>
    rawRecords.value.filter((record) => selectedRecordIds.value.includes(record.id))
  )

  function upsertLibrary(library: MemoryLibrary) {
    const index = libraries.value.findIndex((entry) => entry.id === library.id)
    if (index === -1) {
      libraries.value.unshift(library)
      return
    }
    libraries.value[index] = library
  }

  function upsertRawRecord(record: RawMemoryRecord) {
    const index = rawRecords.value.findIndex((entry) => entry.id === record.id)
    if (index === -1) {
      rawRecords.value.unshift(record)
      return
    }
    rawRecords.value[index] = record
  }

  function matchesCurrentQuery(record: RawMemoryRecord) {
    const query = lastRawQuery.value
    if (query.projectId && record.projectId !== query.projectId) {
      return false
    }
    if (query.sessionId && record.sessionId !== query.sessionId) {
      return false
    }
    if (query.search) {
      const keyword = query.search.trim().toLowerCase()
      if (keyword && !record.content.toLowerCase().includes(keyword)) {
        return false
      }
    }
    return true
  }

  async function initialize() {
    isInitializing.value = true
    try {
      await Promise.all([
        loadLibraries(),
        loadRawRecords({})
      ])

      if (!activeLibraryId.value && libraries.value.length > 0) {
        activeLibraryId.value = libraries.value[0].id
      }

      if (activeLibraryId.value) {
        await loadMergeRuns({ libraryId: activeLibraryId.value })
      }
    } finally {
      isInitializing.value = false
    }
  }

  async function loadLibraries() {
    isLoadingLibraries.value = true
    try {
      libraries.value = await invoke<MemoryLibrary[]>('list_memory_libraries')
      libraries.value.sort((left, right) =>
        new Date(right.updatedAt).getTime() - new Date(left.updatedAt).getTime()
      )

      if (activeLibraryId.value && !libraries.value.some((library) => library.id === activeLibraryId.value)) {
        activeLibraryId.value = libraries.value[0]?.id ?? null
      }
    } catch (error) {
      notificationStore.networkError('加载记忆库', getErrorMessage(error), loadLibraries)
    } finally {
      isLoadingLibraries.value = false
    }
  }

  async function loadRawRecords(query: ListRawMemoryRecordsQuery = {}) {
    isLoadingRecords.value = true
    lastRawQuery.value = { ...query }
    try {
      rawRecords.value = await invoke<RawMemoryRecord[]>('list_raw_memory_records', { query })
    } catch (error) {
      notificationStore.networkError('加载原始记忆', getErrorMessage(error), () => loadRawRecords(query))
    } finally {
      isLoadingRecords.value = false
    }
  }

  async function loadMergeRuns(query?: ListMemoryMergeRunsQuery) {
    const libraryId = query?.libraryId ?? activeLibraryId.value
    if (!libraryId) {
      mergeRuns.value = []
      return
    }

    isLoadingMergeRuns.value = true
    try {
      mergeRuns.value = await invoke<MemoryMergeRun[]>('list_memory_merge_runs', {
        query: { libraryId }
      })
    } catch (error) {
      notificationStore.networkError('加载合并记录', getErrorMessage(error), () => loadMergeRuns({ libraryId }))
    } finally {
      isLoadingMergeRuns.value = false
    }
  }

  async function setActiveLibrary(libraryId: string | null) {
    activeLibraryId.value = libraryId
    await loadMergeRuns(libraryId ? { libraryId } : undefined)
  }

  async function createLibrary(input: CreateMemoryLibraryInput) {
    try {
      const library = await invoke<MemoryLibrary>('create_memory_library', { input })
      upsertLibrary(library)
      activeLibraryId.value = library.id
      await loadMergeRuns({ libraryId: library.id })
      return library
    } catch (error) {
      notificationStore.databaseError('创建记忆库失败', getErrorMessage(error), async () => {
        await createLibrary(input)
      })
      throw error
    }
  }

  async function updateLibrary(id: string, input: UpdateMemoryLibraryInput) {
    isSavingLibrary.value = true
    try {
      const library = await invoke<MemoryLibrary>('update_memory_library', { id, input })
      upsertLibrary(library)
      return library
    } catch (error) {
      notificationStore.databaseError('更新记忆库失败', getErrorMessage(error), async () => {
        await updateLibrary(id, input)
      })
      throw error
    } finally {
      isSavingLibrary.value = false
    }
  }

  async function deleteLibrary(id: string) {
    try {
      await invoke('delete_memory_library', { id })
      libraries.value = libraries.value.filter((library) => library.id !== id)

      if (activeLibraryId.value === id) {
        const nextId = libraries.value[0]?.id ?? null
        activeLibraryId.value = nextId
        await loadMergeRuns(nextId ? { libraryId: nextId } : undefined)
      }
    } catch (error) {
      notificationStore.databaseError('删除记忆库失败', getErrorMessage(error), async () => {
        await deleteLibrary(id)
      })
      throw error
    }
  }

  async function createRawRecord(input: CreateRawMemoryRecordInput) {
    try {
      const record = await invoke<RawMemoryRecord>('create_raw_memory_record', { input })
      if (matchesCurrentQuery(record)) {
        upsertRawRecord(record)
      }
      return record
    } catch (error) {
      notificationStore.databaseError('新增原始记忆失败', getErrorMessage(error), async () => {
        await createRawRecord(input)
      })
      throw error
    }
  }

  async function updateRawRecord(id: string, input: UpdateRawMemoryRecordInput) {
    try {
      const record = await invoke<RawMemoryRecord>('update_raw_memory_record', { id, input })
      if (matchesCurrentQuery(record)) {
        upsertRawRecord(record)
      } else {
        rawRecords.value = rawRecords.value.filter((entry) => entry.id !== id)
      }
      return record
    } catch (error) {
      notificationStore.databaseError('更新原始记忆失败', getErrorMessage(error), async () => {
        await updateRawRecord(id, input)
      })
      throw error
    }
  }

  async function deleteRawRecord(id: string) {
    try {
      await invoke('delete_raw_memory_record', { id })
      rawRecords.value = rawRecords.value.filter((record) => record.id !== id)
      selectedRecordIds.value = selectedRecordIds.value.filter((recordId) => recordId !== id)
    } catch (error) {
      notificationStore.databaseError('删除原始记忆失败', getErrorMessage(error), async () => {
        await deleteRawRecord(id)
      })
      throw error
    }
  }

  async function batchDeleteRawRecords(input: Omit<BatchDeleteRawMemoryRecordsInput, keyof ListRawMemoryRecordsQuery>) {
    isDeletingRecords.value = true
    const request: BatchDeleteRawMemoryRecordsInput = {
      ...lastRawQuery.value,
      ...input
    }

    try {
      const result = await invoke<BatchDeleteRawMemoryRecordsResult>('batch_delete_raw_memory_records', {
        input: request
      })
      selectedRecordIds.value = selectedRecordIds.value.filter(recordId => !result.deletedIds.includes(recordId))
      await loadRawRecords(lastRawQuery.value)

      if (result.deletedCount > 0) {
        notificationStore.success(`已批量删除 ${result.deletedCount} 条原始记忆`)
      } else {
        notificationStore.warning('没有匹配到可删除的原始记忆')
      }

      return result
    } catch (error) {
      notificationStore.databaseError('批量删除原始记忆失败', getErrorMessage(error), async () => {
        await batchDeleteRawRecords(input)
      })
      throw error
    } finally {
      isDeletingRecords.value = false
    }
  }

  async function captureUserMessage(input: CaptureUserMessageInput) {
    try {
      const record = await invoke<RawMemoryRecord>('capture_user_message', { input })
      if (matchesCurrentQuery(record)) {
        upsertRawRecord(record)
      }
      return record
    } catch (error) {
      console.error('Failed to capture raw memory:', error)
      return null
    }
  }

  /**
   * 基于当前输入内容检索可引用的记忆建议。
   *
   * 封装目的：统一调度后端轻量 FTS 检索，并在失败时返回空结果，避免打断输入体验。
   */
  async function searchSuggestions(input: SearchMemorySuggestionsInput): Promise<SearchMemorySuggestionsResult> {
    try {
      return await invoke<SearchMemorySuggestionsResult>('search_memory_suggestions', { input })
    } catch (error) {
      console.error('Failed to search memory suggestions:', error)
      return {
        librarySuggestions: [],
        rawSuggestions: []
      }
    }
  }

  /**
   * 记录本会话已成功引用并发送的记忆，用于后续去重。
   *
   * 副作用：写入 session_memory_reference_history；失败仅记录日志，不阻断消息发送成功态。
   */
  async function recordSessionMemoryReferences(input: RecordSessionMemoryReferencesInput): Promise<void> {
    if (input.references.length === 0) {
      return
    }

    try {
      await invoke('record_session_memory_references', { input })
    } catch (error) {
      console.error('Failed to record session memory references:', error)
    }
  }

  async function resolveMergeAgent(preferredAgentId?: string): Promise<AgentConfig | null> {
    const agentStore = useAgentStore()
    if (agentStore.agents.length === 0) {
      await agentStore.loadAgents()
    }

    if (preferredAgentId) {
      const preferred = agentStore.agents.find((agent) => agent.id === preferredAgentId)
      if (preferred) return preferred
    }

    if (agentStore.currentAgentId) {
      const current = agentStore.agents.find((agent) => agent.id === agentStore.currentAgentId)
      if (current) return current
    }

    return agentStore.agents[0] ?? null
  }

  async function mergeIntoLibrary(options: MergeIntoLibraryOptions = {}) {
    const libraryId = options.libraryId ?? activeLibraryId.value
    if (!libraryId) {
      throw new Error('请先选择目标记忆库')
    }

    const library = libraries.value.find((entry) => entry.id === libraryId)
    if (!library) {
      throw new Error('目标记忆库不存在')
    }

    const sourceRecords = options.recordIds?.length
      ? rawRecords.value.filter((record) => options.recordIds?.includes(record.id))
      : selectedRecords.value

    if (sourceRecords.length === 0) {
      throw new Error('请先勾选要压缩的原始记忆')
    }

    const agent = await resolveMergeAgent(options.agentId)
    if (!agent) {
      throw new Error('当前没有可用的智能体用于记忆压缩')
    }

    isMerging.value = true
    try {
      const mergedContentMd = await memoryMergeService.mergeLibrary({
        agent,
        library,
        records: sourceRecords
      })

      const result = await invoke<MergeRawMemoriesIntoLibraryResult>('merge_raw_memories_into_library', {
        input: {
          libraryId,
          sourceRecordIds: sourceRecords.map((record) => record.id),
          mergedContentMd,
          agentId: agent.id,
          modelId: agent.modelId
        }
      })

      upsertLibrary(result.library)
      activeLibraryId.value = result.library.id
      await loadMergeRuns({ libraryId: result.library.id })
      selectedRecordIds.value = []
      notificationStore.success(`已合并 ${sourceRecords.length} 条原始记忆到「${result.library.name}」`)
      return result
    } catch (error) {
      notificationStore.error('记忆压缩失败', getErrorMessage(error))
      throw error
    } finally {
      isMerging.value = false
    }
  }

  function toggleRecordSelection(recordId: string) {
    if (selectedRecordIds.value.includes(recordId)) {
      selectedRecordIds.value = selectedRecordIds.value.filter((id) => id !== recordId)
      return
    }
    selectedRecordIds.value = [...selectedRecordIds.value, recordId]
  }

  function clearSelectedRecords() {
    selectedRecordIds.value = []
  }

  function setSelectedRecordIds(recordIds: string[]) {
    selectedRecordIds.value = [...recordIds]
  }

  return {
    libraries,
    rawRecords,
    mergeRuns,
    activeLibraryId,
    activeLibrary,
    selectedRecordIds,
    selectedRecords,
    isInitializing,
    isLoadingLibraries,
    isLoadingRecords,
    isLoadingMergeRuns,
    isMerging,
    isSavingLibrary,
    isDeletingRecords,
    initialize,
    loadLibraries,
    loadRawRecords,
    loadMergeRuns,
    setActiveLibrary,
    createLibrary,
    updateLibrary,
    deleteLibrary,
    createRawRecord,
    updateRawRecord,
    deleteRawRecord,
    batchDeleteRawRecords,
    captureUserMessage,
    searchSuggestions,
    recordSessionMemoryReferences,
    mergeIntoLibrary,
    toggleRecordSelection,
    clearSelectedRecords,
    setSelectedRecordIds
  }
})
