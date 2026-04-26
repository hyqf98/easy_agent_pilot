import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  toValue,
  watch,
  type MaybeRefOrGetter
} from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { inferAgentProvider, useAgentStore } from '@/stores/agent'
import { useMessageStore, type MessageAttachment } from '@/stores/message'
import { useNotificationStore } from '@/stores/notification'
import { useProjectStore } from '@/stores/project'
import {
  useSessionExecutionStore,
  type ComposerMemoryReference,
  type ComposerFileMention,
  type PendingImageAttachment,
  type QueuedMessageDraft
} from '@/stores/sessionExecution'
import { useSessionStore } from '@/stores/session'
import { useSettingsStore } from '@/stores/settings'
import { useTokenStore, type CompressionStrategy, type TokenLevel } from '@/stores/token'
import { useMemoryStore } from '@/stores/memory'
import { useAgentTeamsStore } from '@/stores/agentTeams'
import { compressionService } from '@/services/compression'
import { conversationService } from '@/services/conversation'
import { writeFrontendRuntimeLog } from '@/services/runtimeLog/client'
import { getErrorMessage } from '@/utils/api'
import {
  executeSlashCommand,
  parseSlashCommandInput,
  searchSlashCommands,
  type ParsedSlashCommand,
  type SlashCommandDescriptor,
  type SlashCommandPanelType
} from '@/services/slashCommands'
import { useSafeOutsideClick } from '@/composables/useSafeOutsideClick'
import { FILE_MENTION_PATTERN, getMentionDisplayText, getMentionTitle, isGlobalMentionPath } from '@/utils/fileMention'
import { createComposerFileMention, formatMentionLiteral } from '@/utils/composerFileMention'
import { resolveSessionAgent, resolveSessionAgentId } from '@/utils/sessionAgent'
import { resolveAttachmentPreviewUrl } from '@/utils/attachmentPreview'
import type { MemorySuggestion, MemorySuggestionSourceType } from '@/types/memory'
import {
  buildExpertSystemPrompt,
  resolveExpertById,
  resolveExpertRuntime,
  resolveFallbackCliAgent
} from '@/services/agentTeams/runtime'

interface TextSegment {
  type: 'text' | 'file' | 'slash' | 'memory'
  content: string
  displayContent?: string
  fullPath?: string
  titleContent?: string
}

interface UploadImageInput {
  fileName?: string
  mimeType: string
  bytes: number[]
}

interface UploadSessionImagesResponse {
  attachments: MessageAttachment[]
}

interface MemoryPreviewPayload {
  key: string
  sourceType: MemorySuggestionSourceType
  title: string
  sourceLabel: string
  fullContent: string
}

interface UseConversationComposerOptions {
  panelType: SlashCommandPanelType
  sessionId: MaybeRefOrGetter<string | null | undefined>
  projectPath?: MaybeRefOrGetter<string | null | undefined>
  defaultFileMentionScope?: 'project' | 'global'
  workingDirectory?: MaybeRefOrGetter<string | null | undefined>
  setWorkingDirectory?: (path: string) => Promise<string>
}

const MEMORY_REFERENCE_TOKEN_PATTERN = /\[\[memory-ref:(library_chunk|raw_record):([^\]]+)\]\]/g
const MEMORY_SUGGESTION_DEBOUNCE_MS = 350
const MEMORY_SUGGESTION_AUTO_HIDE_MS = 3000
const MEMORY_SUGGESTION_EMPTY_STATE_DELAY_MS = 3000
const MEMORY_SUGGESTION_EMPTY_STATE_RECHECK_MS = 240
const MEMORY_SUGGESTION_KEYBOARD_ACTIVE_MS = 800
const MEMORY_PREVIEW_HIDE_DELAY_MS = 120

function buildMemoryReferenceToken(sourceType: MemorySuggestionSourceType, sourceId: string): string {
  return `[[memory-ref:${sourceType}:${sourceId}]]`
}

function buildMemoryReferenceKey(sourceType: MemorySuggestionSourceType, sourceId: string): string {
  return `${sourceType}:${sourceId}`
}

function reconcileMemoryReferences(text: string, references: ComposerMemoryReference[]): ComposerMemoryReference[] {
  const buckets = new Map<string, ComposerMemoryReference[]>()

  references.forEach(reference => {
    const key = buildMemoryReferenceKey(reference.sourceType, reference.sourceId)
    const bucket = buckets.get(key) ?? []
    bucket.push(reference)
    buckets.set(key, bucket)
  })

  const next: ComposerMemoryReference[] = []
  MEMORY_REFERENCE_TOKEN_PATTERN.lastIndex = 0
  let match: RegExpExecArray | null
  while ((match = MEMORY_REFERENCE_TOKEN_PATTERN.exec(text)) !== null) {
    const key = buildMemoryReferenceKey(match[1] as MemorySuggestionSourceType, match[2] || '')
    const mapped = buckets.get(key)?.shift()
    if (mapped) {
      next.push(mapped)
    }
  }

  return next
}

function sanitizeMemorySearchText(value: string): string {
  const withoutTokens = value.replace(MEMORY_REFERENCE_TOKEN_PATTERN, ' ')
  const withoutMentions = withoutTokens.replace(FILE_MENTION_PATTERN, ' ')
  const withoutCommands = withoutMentions.replace(/^\/[^\n]*$/m, ' ')
  return withoutCommands.replace(/\s+/g, ' ').trim()
}

function removeMemoryReferenceTokens(text: string): string {
  return text.replace(MEMORY_REFERENCE_TOKEN_PATTERN, ' ').replace(/[ \t]{2,}/g, ' ').replace(/\n{3,}/g, '\n\n').trim()
}

function formatMemoryReferenceSource(reference: ComposerMemoryReference): string {
  if (reference.sourceType === 'library_chunk') {
    return reference.libraryName ? `记忆库《${reference.libraryName}》` : reference.title
  }

  if (reference.sessionName) {
    return `原始记忆（${reference.sessionName}）`
  }

  return reference.title
}

function buildMemoryAnnotatedMessage(
  expandedInput: string,
  references: ComposerMemoryReference[]
): { content: string; previewContent: string } {
  const orderedReferences = reconcileMemoryReferences(expandedInput, references)
  const cleanInput = removeMemoryReferenceTokens(expandedInput)

  if (orderedReferences.length === 0) {
    return {
      content: cleanInput,
      previewContent: cleanInput
    }
  }

  const memoryBlock = orderedReferences.map((reference, index) => {
    return [
      `${index + 1}. 来源：${formatMemoryReferenceSource(reference)}`,
      `内容：${reference.fullContent.trim()}`
    ].join('\n')
  }).join('\n\n')

  const content = [
    '[用户主动引用的历史记忆]',
    memoryBlock,
    '',
    '[用户当前输入]',
    cleanInput
  ].join('\n')

  return {
    content,
    previewContent: cleanInput || orderedReferences[0]?.snippet || orderedReferences[0]?.title || ''
  }
}

function sanitizeComposerText(value: string): string {
  let sanitized = ''

  for (const char of value) {
    const code = char.charCodeAt(0)
    const isControlChar = (code >= 0x00 && code <= 0x08)
      || code === 0x0B
      || code === 0x0C
      || (code >= 0x0E && code <= 0x1F)
      || code === 0x7F

    if (!isControlChar) {
      sanitized += char
    }
  }

  return sanitized
}

function getLeadingSlashSegment(text: string): { content: string; length: number } | null {
  if (!text.startsWith('/')) {
    return null
  }

  const matched = text.match(/^\/[^\s\n]*/)
  if (!matched || !matched[0]) {
    return null
  }

  return {
    content: matched[0],
    length: matched[0].length
  }
}

export function useConversationComposer(options: UseConversationComposerOptions) {
  const { t } = useI18n()
  const messageStore = useMessageStore()
  const sessionStore = useSessionStore()
  const settingsStore = useSettingsStore()
  const notificationStore = useNotificationStore()
  const projectStore = useProjectStore()
  const agentStore = useAgentStore()
  const agentConfigStore = useAgentConfigStore()
  const sessionExecutionStore = useSessionExecutionStore()
  const tokenStore = useTokenStore()
  const memoryStore = useMemoryStore()
  const agentTeamsStore = useAgentTeamsStore()

  const showCompressionDialog = ref(false)
  const isCompressing = ref(false)
  const isAgentDropdownOpen = ref(false)
  const agentDropdownRef = ref<HTMLElement | null>(null)
  const isModelDropdownOpen = ref(false)
  const modelDropdownRef = ref<HTMLElement | null>(null)
  const selectedModelId = ref<string>('')
  const textareaRef = ref<HTMLTextAreaElement | null>(null)
  const fileInputRef = ref<HTMLInputElement | null>(null)
  const renderLayerRef = ref<HTMLDivElement | null>(null)
  const showFileMention = ref(false)
  const fileMentionPosition = ref({ x: 0, y: 0, width: 0, height: 0 })
  const mentionStart = ref(-1)
  const mentionSearchText = ref('')
  const showSlashCommand = ref(false)
  const slashCommandPosition = ref({ x: 0, y: 0, width: 0, height: 0 })
  const slashCommandQuery = ref('')
  const showCdPathSuggestions = ref(false)
  const cdPathPosition = ref({ x: 0, y: 0, width: 0, height: 0 })
  const cdPathQuery = ref('')
  const isInputComposing = ref(false)
  const activeMemorySuggestionIndex = ref(-1)
  const hoveredMemoryPreview = ref<MemoryPreviewPayload | null>(null)
  const isMemorySuggestionPanelActive = ref(false)
  const isMemorySuggestionPending = ref(false)
  const memorySuggestionEmptyStateVisible = ref(false)
  const memorySuggestionPendingEmptyQuery = ref('')
  const isMemorySuggestionPointerActive = ref(false)
  const lastMemorySuggestionKeyboardAt = ref(0)
  let memorySuggestionTimer: ReturnType<typeof setTimeout> | null = null
  let memorySuggestionAutoHideTimer: ReturnType<typeof setTimeout> | null = null
  let memorySuggestionEmptyTimer: ReturnType<typeof setTimeout> | null = null
  let memoryPreviewHideTimer: ReturnType<typeof setTimeout> | null = null
  let memorySuggestionRequestId = 0

  const currentSessionId = computed(() => toValue(options.sessionId) || null)
  const currentSession = computed(() =>
    sessionStore.sessions.find(session => session.id === currentSessionId.value) || null
  )
  const currentProjectPath = computed(() => {
    const overridePath = toValue(options.projectPath)
    if (overridePath) {
      return overridePath
    }

    const projectId = currentSession.value?.projectId
    if (!projectId) {
      return null
    }

    return projectStore.projects.find(project => project.id === projectId)?.path || null
  })
  const currentWorkingDirectory = computed(() => toValue(options.workingDirectory) || currentProjectPath.value)

  const agentOptions = computed(() =>
    agentTeamsStore.enabledExperts.map(expert => {
      const runtime = resolveExpertRuntime(expert, agentStore.agents)
      const runtimeAgent = runtime?.agent
      const provider = runtimeAgent?.provider || inferAgentProvider(runtimeAgent)
      return {
        label: expert.name,
        value: expert.id,
        modelId: runtime?.modelId,
        provider,
        type: runtimeAgent?.type || 'cli',
        isCustom: runtimeAgent?.customModelEnabled || false
      }
    })
  )

  const currentExpertId = computed(() => {
    const explicitExpertId = currentSession.value?.expertId?.trim()
    return explicitExpertId || null
  })

  const currentExpert = computed(() =>
    resolveExpertById(currentExpertId.value, agentTeamsStore.experts)
  )

  const currentAgentId = computed(() => currentExpertId.value || currentAgent.value?.id || null)

  const currentAgent = computed(() => {
    const expertRuntime = resolveExpertRuntime(currentExpert.value, agentStore.agents)
    if (expertRuntime) {
      return expertRuntime.agent
    }
    return resolveSessionAgent(currentSession.value, agentStore.agents) || resolveFallbackCliAgent(agentStore.agents)
  })

  const currentAgentName = computed(() => {
    if (currentExpert.value) {
      return currentExpert.value.name
    }
    if (currentAgent.value) {
      return currentAgent.value.name
    }
    return '选择专家'
  })

  const modelOptions = computed(() => {
    const agentId = currentAgent.value?.id
    if (!agentId) return []

    return agentConfigStore.getModelsConfigs(agentId)
      .filter(config => config.enabled)
      .map(config => ({
        value: config.modelId,
        label: config.displayName,
        isDefault: config.isDefault
      }))
  })

  const presetModelOptions = computed(() => modelOptions.value)

  const inputText = computed({
    get: () => currentSessionId.value ? sessionExecutionStore.getInputText(currentSessionId.value) : '',
    set: (value) => {
      if (currentSessionId.value) {
        sessionExecutionStore.setInputText(currentSessionId.value, value)
      }
    }
  })

  const isSending = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getIsBusy(currentSessionId.value) : false
  )

  const pendingImages = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getPendingImages(currentSessionId.value) : []
  )

  const queuedMessages = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getQueuedMessages(currentSessionId.value) : []
  )

  const isUploadingImages = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getIsUploadingImages(currentSessionId.value) : false
  )
  const dispatchingSessionId = ref<string | null>(null)
  const isCurrentSessionDispatching = computed(() => (
    Boolean(currentSessionId.value) && dispatchingSessionId.value === currentSessionId.value
  ))
  const currentFileMentions = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getFileMentions(currentSessionId.value) : []
  )
  const currentMemoryReferences = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getMemoryReferences(currentSessionId.value) : []
  )
  const memorySuggestions = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getMemorySuggestions(currentSessionId.value) : {
      librarySuggestions: [],
      rawSuggestions: []
    }
  )
  const isSearchingMemory = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getIsSearchingMemory(currentSessionId.value) : false
  )
  const dismissedMemorySuggestionKeys = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getDismissedMemorySuggestionKeys(currentSessionId.value) : []
  )

  const createComposerMention = (fullPath: string): ComposerFileMention => {
    return createComposerFileMention({ fullPath })
  }

  const createComposerMemoryReference = (suggestion: MemorySuggestion): ComposerMemoryReference => ({
    sourceType: suggestion.sourceType,
    sourceId: suggestion.sourceId,
    title: suggestion.title,
    fullContent: suggestion.fullContent,
    snippet: suggestion.snippet,
    libraryId: suggestion.libraryId,
    libraryName: suggestion.libraryName,
    sessionId: suggestion.sessionId,
    sessionName: suggestion.sessionName,
    projectId: suggestion.projectId,
    projectName: suggestion.projectName,
    createdAt: suggestion.createdAt
  })

  const buildMemoryPreviewFromSuggestion = (suggestion: MemorySuggestion): MemoryPreviewPayload => ({
    key: buildMemoryReferenceKey(suggestion.sourceType, suggestion.sourceId),
    sourceType: suggestion.sourceType,
    title: suggestion.title,
    sourceLabel: suggestion.sourceType === 'library_chunk'
      ? t('message.memorySourceLibrary')
      : t('message.memorySourceRaw'),
    fullContent: suggestion.fullContent.trim()
  })

  const buildMemoryPreviewFromReference = (reference: ComposerMemoryReference): MemoryPreviewPayload => ({
    key: buildMemoryReferenceKey(reference.sourceType, reference.sourceId),
    sourceType: reference.sourceType,
    title: reference.title,
    sourceLabel: reference.sourceType === 'library_chunk'
      ? t('message.memorySourceLibrary')
      : t('message.memorySourceRaw'),
    fullContent: reference.fullContent.trim()
  })

  const countMentionsInText = (text: string) => {
    let count = 0
    FILE_MENTION_PATTERN.lastIndex = 0
    while (FILE_MENTION_PATTERN.exec(text) !== null) {
      count += 1
    }
    return count
  }

  const areFileMentionsEqual = (left: ComposerFileMention[], right: ComposerFileMention[]) => (
    left.length === right.length && left.every((mention, index) =>
      mention.id === right[index]?.id
      && mention.displayText === right[index]?.displayText
      && mention.fullPath === right[index]?.fullPath
      && mention.insertText === right[index]?.insertText
    )
  )

  const reconcileFileMentions = (text: string, mentions: ComposerFileMention[]) => {
    const mentionBuckets = new Map<string, ComposerFileMention[]>()

    mentions.forEach((mention) => {
      const bucket = mentionBuckets.get(mention.displayText) ?? []
      bucket.push(mention)
      mentionBuckets.set(mention.displayText, bucket)
    })

    const nextMentions: ComposerFileMention[] = []
    FILE_MENTION_PATTERN.lastIndex = 0
    let match: RegExpExecArray | null

    while ((match = FILE_MENTION_PATTERN.exec(text)) !== null) {
      const literal = match[0]
      const path = match[1] ?? match[2]
      const mappedMention = mentionBuckets.get(literal)?.shift()

      if (mappedMention) {
        nextMentions.push(mappedMention)
        continue
      }

      if (isGlobalMentionPath(path)) {
        nextMentions.push(createComposerMention(path))
      }
    }

    return nextMentions
  }

  const syncFileMentions = (text: string, mentions = currentFileMentions.value) => {
    if (!currentSessionId.value) {
      return
    }

    const nextMentions = reconcileFileMentions(text, mentions)
    if (!areFileMentionsEqual(nextMentions, currentFileMentions.value)) {
      sessionExecutionStore.setFileMentions(currentSessionId.value, nextMentions)
    }
  }

  const expandComposerMentions = (text: string, mentions: ComposerFileMention[]) => {
    const mentionBuckets = new Map<string, ComposerFileMention[]>()

    mentions.forEach((mention) => {
      const bucket = mentionBuckets.get(mention.displayText) ?? []
      bucket.push(mention)
      mentionBuckets.set(mention.displayText, bucket)
    })

    FILE_MENTION_PATTERN.lastIndex = 0
    return text.replace(FILE_MENTION_PATTERN, (literal) => {
      const mappedMention = mentionBuckets.get(literal)?.shift()
      return mappedMention?.insertText ?? literal
    })
  }

  const parsedInputText = computed<TextSegment[]>(() => {
    const text = inputText.value
    if (!text) return []

    const segments: TextSegment[] = []
    const leadingSlash = getLeadingSlashSegment(text)
    let lastIndex = 0
    let match: RegExpExecArray | null

    if (leadingSlash) {
      segments.push({
        type: 'slash',
        content: leadingSlash.content
      })
      lastIndex = leadingSlash.length
    }

    FILE_MENTION_PATTERN.lastIndex = 0
    MEMORY_REFERENCE_TOKEN_PATTERN.lastIndex = 0
    const tokenMatches: Array<
      { kind: 'file'; match: RegExpExecArray } |
      { kind: 'memory'; match: RegExpExecArray }
    > = []

    while ((match = FILE_MENTION_PATTERN.exec(text)) !== null) {
      tokenMatches.push({ kind: 'file', match })
    }

    let memoryMatch: RegExpExecArray | null
    while ((memoryMatch = MEMORY_REFERENCE_TOKEN_PATTERN.exec(text)) !== null) {
      tokenMatches.push({ kind: 'memory', match: memoryMatch })
    }

    tokenMatches.sort((left, right) => left.match.index - right.match.index)

    for (const entry of tokenMatches) {
      const nextMatch = entry.match
      if (nextMatch.index < lastIndex) {
        continue
      }

      if (nextMatch.index > lastIndex) {
        const content = text.slice(lastIndex, nextMatch.index)
        if (content) {
          segments.push({
            type: 'text',
            content
          })
        }
      }

      if (entry.kind === 'file') {
        match = entry.match
        const literal = match[0]
        const fullPath = match[1] ?? match[2]
        const mappedMention = currentFileMentions.value.find(mention => mention.displayText === literal)

        segments.push({
          type: 'file',
          content: literal,
          displayContent: mappedMention?.displayText ?? getMentionDisplayText(literal, fullPath),
          fullPath: mappedMention?.fullPath ?? fullPath,
          titleContent: mappedMention?.titleText ?? getMentionTitle(fullPath)
        })

        lastIndex = match.index + match[0].length
        continue
      }

      memoryMatch = entry.match
      const sourceType = memoryMatch[1] as MemorySuggestionSourceType
      const sourceId = memoryMatch[2] || ''
      const mappedReference = currentMemoryReferences.value.find(reference =>
        reference.sourceType === sourceType && reference.sourceId === sourceId
      )

      segments.push({
        type: 'memory',
        content: memoryMatch[0],
        displayContent: mappedReference?.title ?? '记忆引用',
        titleContent: mappedReference?.snippet ?? mappedReference?.fullContent ?? ''
      })

      lastIndex = memoryMatch.index + memoryMatch[0].length
    }

    if (lastIndex < text.length) {
      const content = text.slice(lastIndex)
      if (content) {
        segments.push({
          type: 'text',
          content
        })
      }
    }

    return segments
  })

  const visibleMemorySuggestions = computed(() => {
    const selectedKeys = new Set(
      currentMemoryReferences.value.map(reference => buildMemoryReferenceKey(reference.sourceType, reference.sourceId))
    )
    const dismissedKeys = new Set(dismissedMemorySuggestionKeys.value)
    const filterGroup = (items: MemorySuggestion[]) => items.filter(item => {
      const key = buildMemoryReferenceKey(item.sourceType, item.sourceId)
      return !selectedKeys.has(key) && !dismissedKeys.has(key)
    })

    return {
      librarySuggestions: filterGroup(memorySuggestions.value.librarySuggestions),
      rawSuggestions: filterGroup(memorySuggestions.value.rawSuggestions)
    }
  })

  const flatVisibleMemorySuggestions = computed(() => [
    ...visibleMemorySuggestions.value.librarySuggestions,
    ...visibleMemorySuggestions.value.rawSuggestions
  ])
  const hasVisibleMemorySuggestions = computed(() => flatVisibleMemorySuggestions.value.length > 0)

  const activeMemorySuggestion = computed(() => {
    if (activeMemorySuggestionIndex.value < 0) {
      return null
    }

    return flatVisibleMemorySuggestions.value[activeMemorySuggestionIndex.value] ?? null
  })

  const activeMemorySuggestionKey = computed(() => (
    activeMemorySuggestion.value
      ? buildMemoryReferenceKey(activeMemorySuggestion.value.sourceType, activeMemorySuggestion.value.sourceId)
      : ''
  ))

  const currentMemoryPreview = computed(() => (
    hoveredMemoryPreview.value
    ?? (activeMemorySuggestion.value ? buildMemoryPreviewFromSuggestion(activeMemorySuggestion.value) : null)
  ))

  const shouldShowMemorySuggestions = computed(() => {
    return options.panelType === 'main' && isMemorySuggestionPanelActive.value
  })

  const isMemorySuggestionLoading = computed(() => (
    shouldShowMemorySuggestions.value && (isMemorySuggestionPending.value || isSearchingMemory.value)
  ))

  const shouldShowMemorySuggestionEmptyState = computed(() => (
    shouldShowMemorySuggestions.value
    && memorySuggestionEmptyStateVisible.value
    && !!memorySuggestionPendingEmptyQuery.value
    && !hasVisibleMemorySuggestions.value
    && !isMemorySuggestionLoading.value
  ))

  const shouldShowMemorySuggestionIdleHint = computed(() => false)

  const setActiveMemorySuggestionIndex = (index: number) => {
    const total = flatVisibleMemorySuggestions.value.length
    if (total === 0) {
      activeMemorySuggestionIndex.value = -1
      return
    }

    const normalized = ((index % total) + total) % total
    activeMemorySuggestionIndex.value = normalized
  }

  const moveActiveMemorySuggestion = (step: number) => {
    const total = flatVisibleMemorySuggestions.value.length
    if (total === 0) {
      activeMemorySuggestionIndex.value = -1
      return
    }

    if (activeMemorySuggestionIndex.value < 0) {
      activeMemorySuggestionIndex.value = step > 0 ? 0 : total - 1
      return
    }

    setActiveMemorySuggestionIndex(activeMemorySuggestionIndex.value + step)
  }

  const isActiveMemorySuggestion = (suggestion: MemorySuggestion) => (
    activeMemorySuggestionKey.value === buildMemoryReferenceKey(suggestion.sourceType, suggestion.sourceId)
  )

  const previewMemorySuggestion = (suggestion: MemorySuggestion) => {
    if (memoryPreviewHideTimer) {
      clearTimeout(memoryPreviewHideTimer)
      memoryPreviewHideTimer = null
    }
    hoveredMemoryPreview.value = buildMemoryPreviewFromSuggestion(suggestion)
    armMemorySuggestionAutoHide()
  }

  const previewMemoryReference = (reference: ComposerMemoryReference) => {
    if (memoryPreviewHideTimer) {
      clearTimeout(memoryPreviewHideTimer)
      memoryPreviewHideTimer = null
    }
    hoveredMemoryPreview.value = buildMemoryPreviewFromReference(reference)
  }

  const clearMemoryPreview = () => {
    if (memoryPreviewHideTimer) {
      clearTimeout(memoryPreviewHideTimer)
      memoryPreviewHideTimer = null
    }
    hoveredMemoryPreview.value = null
  }

  const scheduleClearMemoryPreview = () => {
    if (memoryPreviewHideTimer) {
      clearTimeout(memoryPreviewHideTimer)
    }

    memoryPreviewHideTimer = setTimeout(() => {
      hoveredMemoryPreview.value = null
      memoryPreviewHideTimer = null
    }, MEMORY_PREVIEW_HIDE_DELAY_MS)
  }

  const handleMemoryPreviewPointerEnter = () => {
    if (memoryPreviewHideTimer) {
      clearTimeout(memoryPreviewHideTimer)
      memoryPreviewHideTimer = null
    }
  }

  const handleMemoryPreviewPointerLeave = () => {
    scheduleClearMemoryPreview()
  }

  const clearMemorySuggestionTimer = () => {
    if (memorySuggestionTimer) {
      clearTimeout(memorySuggestionTimer)
      memorySuggestionTimer = null
    }
  }

  const clearMemorySuggestionAutoHideTimer = () => {
    if (memorySuggestionAutoHideTimer) {
      clearTimeout(memorySuggestionAutoHideTimer)
      memorySuggestionAutoHideTimer = null
    }
  }

  const clearMemorySuggestionEmptyTimer = () => {
    if (memorySuggestionEmptyTimer) {
      clearTimeout(memorySuggestionEmptyTimer)
      memorySuggestionEmptyTimer = null
    }
  }

  const isMemorySuggestionKeyboardActive = () => (
    Date.now() - lastMemorySuggestionKeyboardAt.value < MEMORY_SUGGESTION_KEYBOARD_ACTIVE_MS
  )

  const hideMemorySuggestionPanel = () => {
    clearMemorySuggestionAutoHideTimer()
    clearMemoryPreview()
    activeMemorySuggestionIndex.value = -1
    isMemorySuggestionPanelActive.value = false
    isMemorySuggestionPending.value = false
    memorySuggestionEmptyStateVisible.value = false
    memorySuggestionPendingEmptyQuery.value = ''
    isMemorySuggestionPointerActive.value = false
    lastMemorySuggestionKeyboardAt.value = 0
  }

  const armMemorySuggestionAutoHide = () => {
    clearMemorySuggestionAutoHideTimer()

    if (!shouldShowMemorySuggestions.value) {
      return
    }

    memorySuggestionAutoHideTimer = setTimeout(() => {
      if (!shouldShowMemorySuggestions.value) {
        return
      }

      if (isMemorySuggestionPointerActive.value || isMemorySuggestionKeyboardActive()) {
        armMemorySuggestionAutoHide()
        return
      }

      hideMemorySuggestionPanel()
    }, MEMORY_SUGGESTION_AUTO_HIDE_MS)
  }

  const markMemorySuggestionKeyboardInteraction = () => {
    lastMemorySuggestionKeyboardAt.value = Date.now()
    if (shouldShowMemorySuggestions.value) {
      armMemorySuggestionAutoHide()
    }
  }

  const handleMemorySuggestionPointerEnter = () => {
    isMemorySuggestionPointerActive.value = true
    if (shouldShowMemorySuggestions.value) {
      armMemorySuggestionAutoHide()
    }
  }

  const handleMemorySuggestionPointerLeave = () => {
    isMemorySuggestionPointerActive.value = false
    if (shouldShowMemorySuggestions.value) {
      armMemorySuggestionAutoHide()
    }
  }

  const beginMemorySuggestionSearchCycle = () => {
    if (options.panelType !== 'main') {
      return
    }

    isMemorySuggestionPanelActive.value = true
    isMemorySuggestionPending.value = true
    memorySuggestionEmptyStateVisible.value = false
    memorySuggestionPendingEmptyQuery.value = ''
    clearMemorySuggestionEmptyTimer()
    armMemorySuggestionAutoHide()
  }

  const settleMemorySuggestionResults = (suggestionCount: number) => {
    isMemorySuggestionPending.value = false
    memorySuggestionEmptyStateVisible.value = false
    memorySuggestionPendingEmptyQuery.value = ''
    clearMemorySuggestionEmptyTimer()

    if (suggestionCount === 0) {
      activeMemorySuggestionIndex.value = -1
    }

    armMemorySuggestionAutoHide()
  }

  const armMemorySuggestionEmptyState = (
    sessionId: string,
    searchText: string,
    requestId: number,
    delay = MEMORY_SUGGESTION_EMPTY_STATE_DELAY_MS
  ) => {
    clearMemorySuggestionEmptyTimer()
    memorySuggestionPendingEmptyQuery.value = searchText
    memorySuggestionEmptyStateVisible.value = false
    isMemorySuggestionPending.value = true

    memorySuggestionEmptyTimer = setTimeout(() => {
      if (requestId !== memorySuggestionRequestId || currentSessionId.value !== sessionId) {
        return
      }

      if (sanitizeMemorySearchText(inputText.value) !== searchText) {
        return
      }

      if (isSearchingMemory.value) {
        armMemorySuggestionEmptyState(sessionId, searchText, requestId, MEMORY_SUGGESTION_EMPTY_STATE_RECHECK_MS)
        return
      }

      sessionExecutionStore.setMemorySuggestions(sessionId, {
        librarySuggestions: [],
        rawSuggestions: []
      }, searchText)
      hideMemorySuggestionPanel()
    }, delay)
  }

  const resetMemorySuggestionState = () => {
    const sessionId = currentSessionId.value
    if (!sessionId) {
      return
    }

    clearMemorySuggestionTimer()
    clearMemorySuggestionAutoHideTimer()
    clearMemorySuggestionEmptyTimer()
    memorySuggestionRequestId += 1
    hideMemorySuggestionPanel()
    sessionExecutionStore.setIsSearchingMemory(sessionId, false)
    sessionExecutionStore.clearMemorySuggestions(sessionId)
    sessionExecutionStore.clearDismissedMemorySuggestionKeys(sessionId)
  }

  const searchMemorySuggestions = async (draftText: string, requestId: number) => {
    const sessionId = currentSessionId.value
    if (!sessionId || options.panelType !== 'main') {
      return
    }

    const searchText = sanitizeMemorySearchText(draftText)
    if (searchText.length < 4) {
      if (requestId === memorySuggestionRequestId) {
        resetMemorySuggestionState()
      }
      return
    }

    const suggestions = await memoryStore.searchSuggestions({
      sessionId,
      projectId: currentSession.value?.projectId,
      draftText: searchText,
      limit: 6
    })

    if (requestId !== memorySuggestionRequestId || currentSessionId.value !== sessionId) {
      return
    }

    if (sanitizeMemorySearchText(inputText.value) !== searchText) {
      return
    }

    sessionExecutionStore.setIsSearchingMemory(sessionId, false)
    clearMemorySuggestionEmptyTimer()

    const hasSuggestions = suggestions.librarySuggestions.length > 0 || suggestions.rawSuggestions.length > 0
    if (hasSuggestions) {
      sessionExecutionStore.setMemorySuggestions(sessionId, suggestions, searchText)
      settleMemorySuggestionResults(suggestions.librarySuggestions.length + suggestions.rawSuggestions.length)
      return
    }

    armMemorySuggestionEmptyState(sessionId, searchText, requestId)
  }

  const scheduleMemorySuggestionSearch = (draftText: string) => {
    const sessionId = currentSessionId.value
    if (!sessionId || options.panelType !== 'main') {
      return
    }

    const searchText = sanitizeMemorySearchText(draftText)
    if (searchText.length < 4) {
      resetMemorySuggestionState()
      return
    }

    beginMemorySuggestionSearchCycle()
    clearMemorySuggestionTimer()
    clearMemorySuggestionEmptyTimer()
    const requestId = ++memorySuggestionRequestId
    memorySuggestionTimer = setTimeout(() => {
      if (requestId !== memorySuggestionRequestId || currentSessionId.value !== sessionId) {
        return
      }

      sessionExecutionStore.setIsSearchingMemory(sessionId, true)
      void searchMemorySuggestions(draftText, requestId)
    }, MEMORY_SUGGESTION_DEBOUNCE_MS)
  }

  const tokenUsage = computed(() => {
    if (!currentSessionId.value) {
      return { used: 0, limit: 0, percentage: 0, level: 'safe' as TokenLevel }
    }
    return tokenStore.getTokenUsage(currentSessionId.value)
  })

  const messageCount = computed(() => {
    if (!currentSessionId.value) return 0
    return messageStore.messagesBySession(currentSessionId.value).length
  })

  const shouldShowCompressButton = computed(() => {
    return tokenUsage.value.percentage >= 50 && messageCount.value > 0
  })

  const inputPlaceholder = computed(() => {
    const shortcut = settingsStore.settings.sendOnEnter
      ? t('message.shortcutEnter')
      : t('message.shortcutModifierEnter')
    return t('message.inputPlaceholder', { shortcut })
  })

  const slashCommands = computed(() =>
    searchSlashCommands(options.panelType, slashCommandQuery.value)
  )

  watch(() => currentAgent.value?.id, async (agentId) => {
    if (agentId) {
      const provider = inferAgentProvider(agentStore.agents.find(agent => agent.id === agentId))
      await agentConfigStore.ensureModelsConfigs(agentId, provider)
    }
  }, { immediate: true })

  watch([currentExpert, currentAgent], async ([expert, agent]) => {
    if (agent?.id) {
      await agentConfigStore.ensureModelsConfigs(agent.id, inferAgentProvider(agent))
      const configs = agentConfigStore.getModelsConfigs(agent.id)
      const preferredModel = expert?.defaultModelId
      const defaultModel = configs.find(config => config.enabled && config.modelId === preferredModel)
        || configs.find(config => config.isDefault && config.enabled)
        || configs.find(config => config.enabled)
      selectedModelId.value = defaultModel?.modelId || ''
    } else {
      selectedModelId.value = ''
    }
  }, { immediate: true })

  function focusInput() {
    nextTick(() => {
      textareaRef.value?.focus()
    })
  }

  watch(currentSessionId, (sessionId) => {
    clearMemorySuggestionTimer()
    clearMemorySuggestionAutoHideTimer()
    clearMemorySuggestionEmptyTimer()
    memorySuggestionRequestId += 1
    hideMemorySuggestionPanel()
    if (sessionId) {
      focusInput()
      sessionExecutionStore.clearMemorySuggestions(sessionId)
      sessionExecutionStore.clearDismissedMemorySuggestionKeys(sessionId)
    }
  }, { immediate: true })

  watch(inputText, (value) => {
    const sanitizedValue = sanitizeComposerText(value)
    if (sanitizedValue !== value) {
      inputText.value = sanitizedValue
      return
    }

    syncFileMentions(sanitizedValue)
    const nextMemoryReferences = reconcileMemoryReferences(sanitizedValue, currentMemoryReferences.value)
    if (currentSessionId.value) {
      sessionExecutionStore.setMemoryReferences(currentSessionId.value, nextMemoryReferences)
    }

    if (isInputComposing.value) {
      clearMemorySuggestionTimer()
      if (currentSessionId.value) {
        sessionExecutionStore.setIsSearchingMemory(currentSessionId.value, false)
      }
      if (isMemorySuggestionPanelActive.value) {
        isMemorySuggestionPending.value = true
        memorySuggestionEmptyStateVisible.value = false
        clearMemorySuggestionEmptyTimer()
      }
      return
    }

    if (!sanitizedValue.trim()) {
      resetMemorySuggestionState()
      return
    }

    scheduleMemorySuggestionSearch(sanitizedValue)
  })

  watch(flatVisibleMemorySuggestions, (items) => {
    if (items.length === 0) {
      activeMemorySuggestionIndex.value = -1
      return
    }

    if (activeMemorySuggestionIndex.value >= items.length) {
      activeMemorySuggestionIndex.value = items.length - 1
    }
  })

  onMounted(async () => {
    try {
      await Promise.all([
        agentStore.loadAgents(),
        agentTeamsStore.loadExperts(true)
      ])
      if (currentAgent.value?.id) {
        const provider = inferAgentProvider(agentStore.agents.find(agent => agent.id === currentAgent.value?.id))
        await agentConfigStore.ensureModelsConfigs(currentAgent.value.id, provider)
      }
    } catch (error) {
      console.error('Failed to load experts or agents:', error)
    }
  })

  onUnmounted(() => {
    clearMemorySuggestionTimer()
    clearMemorySuggestionAutoHideTimer()
    clearMemorySuggestionEmptyTimer()
    clearMemoryPreview()
  })

  useSafeOutsideClick(
    () => [agentDropdownRef.value, modelDropdownRef.value],
    () => {
      isAgentDropdownOpen.value = false
      isModelDropdownOpen.value = false
    }
  )

  const syncScroll = () => {
    if (textareaRef.value && renderLayerRef.value) {
      renderLayerRef.value.scrollTop = textareaRef.value.scrollTop
    }
  }

  const toggleAgentDropdown = () => {
    isAgentDropdownOpen.value = !isAgentDropdownOpen.value
    if (isAgentDropdownOpen.value) {
      isModelDropdownOpen.value = false
    }
  }

  const selectAgent = async (expertId: string) => {
    const sessionId = currentSessionId.value
    if (!sessionId) {
      isAgentDropdownOpen.value = false
      return
    }

    try {
      const expert = resolveExpertById(expertId, agentTeamsStore.experts)
      const runtime = resolveExpertRuntime(expert, agentStore.agents)
      const agent = runtime?.agent
      if (agent?.id) {
        await agentConfigStore.ensureModelsConfigs(agent.id, inferAgentProvider(agent))
      }
      await sessionStore.updateSession(sessionId, {
        expertId,
        agentId: agent?.id,
        agentType: agent?.provider || agent?.type || 'claude',
        cliSessionId: '',
        cliSessionProvider: ''
      })
      const configs = agent?.id
        ? agentConfigStore.getModelsConfigs(agent.id).filter(config => config.enabled)
        : []
      const matchedModel = configs.find(config => config.modelId === runtime?.modelId)
        || configs.find(config => config.isDefault)
        || configs[0]
      selectedModelId.value = matchedModel?.modelId || ''
      isAgentDropdownOpen.value = false
    } catch (error) {
      console.error('Failed to update session expert:', error)
    }
  }

  const toggleModelDropdown = () => {
    isModelDropdownOpen.value = !isModelDropdownOpen.value
    if (isModelDropdownOpen.value) {
      isAgentDropdownOpen.value = false
    }
  }

  const selectModel = async (modelId: string) => {
    if (!currentAgent.value) return

    selectedModelId.value = modelId
    isModelDropdownOpen.value = false

    try {
      const runtimeAgentId = currentAgent.value.id
      const configs = agentConfigStore.getModelsConfigs(runtimeAgentId)
      const selectedConfig = configs.find(config => config.modelId === modelId)
      if (selectedConfig) {
        await agentConfigStore.updateModelConfig(selectedConfig.id, runtimeAgentId, {
          isDefault: true
        })
      }
      if (currentExpert.value?.id) {
        await agentTeamsStore.updateExpert(currentExpert.value.id, { defaultModelId: modelId || undefined })
      }
    } catch (error) {
      console.error('Failed to update expert model:', error)
    }
  }

  const getModelLabel = (modelId: string) => {
    const model = modelOptions.value.find(item => item.value === modelId)
    return model ? model.label : modelId || '使用默认模型'
  }

  const handleOpenCompress = () => {
    showCompressionDialog.value = true
  }

  const handleConfirmCompress = async (strategy: CompressionStrategy) => {
    if (!currentSessionId.value) return

    const agentId = resolveSessionAgentId(currentSession.value, agentStore.agents) || currentAgent.value?.id

    if (!agentId) {
      notificationStore.smartError('压缩失败', new Error('未找到可用专家运行时'))
      showCompressionDialog.value = false
      return
    }

    showCompressionDialog.value = false
    isCompressing.value = true

    try {
      const result = await compressionService.compressSession(
        currentSessionId.value,
        agentId,
        { strategy }
      )

      if (result.success) {
        notificationStore.success(t('compression.success'))
      } else {
        notificationStore.error(t('compression.failed'), result.error)
      }
    } catch (error) {
      notificationStore.smartError('压缩失败', error instanceof Error ? error : new Error(String(error)))
    } finally {
      isCompressing.value = false
      showCompressionDialog.value = false
    }
  }

  const handleCancelCompress = () => {
    showCompressionDialog.value = false
  }

  const closeFileMention = () => {
    showFileMention.value = false
    fileMentionPosition.value = { x: 0, y: 0, width: 0, height: 0 }
    mentionStart.value = -1
    mentionSearchText.value = ''
  }

  const closeSlashCommand = () => {
    showSlashCommand.value = false
    slashCommandPosition.value = { x: 0, y: 0, width: 0, height: 0 }
    slashCommandQuery.value = ''
  }

  const closeCdPathSuggestions = () => {
    showCdPathSuggestions.value = false
    cdPathPosition.value = { x: 0, y: 0, width: 0, height: 0 }
    cdPathQuery.value = ''
  }

  const openFileMention = (x: number, y: number, query: string, start: number) => {
    if (!currentSessionId.value || !currentProjectPath.value) {
      return
    }

    closeSlashCommand()
    closeCdPathSuggestions()
    showFileMention.value = true
    fileMentionPosition.value = { x, y, width: 280, height: 0 }
    mentionStart.value = start
    mentionSearchText.value = query
  }

  const openSlashCommand = (x: number, y: number, query: string) => {
    closeCdPathSuggestions()
    closeFileMention()
    showSlashCommand.value = true
    slashCommandPosition.value = { x, y, width: 320, height: 0 }
    slashCommandQuery.value = query
  }

  const openCdPathSuggestions = (x: number, y: number, query: string) => {
    closeSlashCommand()
    closeFileMention()
    showCdPathSuggestions.value = true
    cdPathPosition.value = { x, y, width: 360, height: 0 }
    cdPathQuery.value = query
  }

  const handleFileSelect = (insertPath: string, mentionStartPos: number) => {
    closeFileMention()

    const textarea = textareaRef.value
    const cursorPos = textarea ? textarea.selectionStart : inputText.value.length
    const beforeAt = inputText.value.slice(0, mentionStartPos)
    const afterSearch = inputText.value.slice(cursorPos)
    const nextMention = isGlobalMentionPath(insertPath) ? createComposerMention(insertPath) : null
    const insertText = `${nextMention?.displayText ?? formatMentionLiteral(insertPath)} `
    const newText = sanitizeComposerText(beforeAt + insertText + afterSearch)
    const newPosition = beforeAt.length + insertText.length
    const nextMentions = [...reconcileFileMentions(inputText.value, currentFileMentions.value)]

    if (nextMention) {
      nextMentions.splice(countMentionsInText(beforeAt), 0, nextMention)
      if (currentSessionId.value) {
        sessionExecutionStore.setFileMentions(currentSessionId.value, nextMentions)
      }
    }

    // 先同步 textarea 的 DOM 值，避免光标位置错乱
    if (textarea) {
      textarea.value = newText
    }

    inputText.value = newText

    requestAnimationFrame(() => {
      if (textarea) {
        textarea.focus()
        textarea.setSelectionRange(newPosition, newPosition)
        if (renderLayerRef.value) {
          renderLayerRef.value.scrollTop = textarea.scrollTop
        }
      }
    })
  }

  const insertMemoryReference = (suggestion: MemorySuggestion) => {
    const sessionId = currentSessionId.value
    const textarea = textareaRef.value
    if (!sessionId || !textarea) {
      return
    }

    const reference = createComposerMemoryReference(suggestion)
    const token = buildMemoryReferenceToken(reference.sourceType, reference.sourceId)
    if (
      currentMemoryReferences.value.some(item => item.sourceType === reference.sourceType && item.sourceId === reference.sourceId)
      || inputText.value.includes(token)
    ) {
      return
    }

    const start = textarea.selectionStart ?? inputText.value.length
    const end = textarea.selectionEnd ?? inputText.value.length
    const before = inputText.value.slice(0, start)
    const after = inputText.value.slice(end)
    const needsLeadingSpace = before.length > 0 && !/\s$/.test(before)
    const needsTrailingSpace = after.length === 0 || !/^\s/.test(after)
    const insertedToken = `${needsLeadingSpace ? ' ' : ''}${token}${needsTrailingSpace ? ' ' : ''}`
    const newText = sanitizeComposerText(`${before}${insertedToken}${after}`)
    const newPosition = before.length + insertedToken.length

    textarea.value = newText
    inputText.value = newText
    sessionExecutionStore.appendMemoryReference(sessionId, reference)
    hideMemorySuggestionPanel()
    hoveredMemoryPreview.value = buildMemoryPreviewFromReference(reference)

    requestAnimationFrame(() => {
      textarea.focus()
      textarea.setSelectionRange(newPosition, newPosition)
      if (renderLayerRef.value) {
        renderLayerRef.value.scrollTop = textarea.scrollTop
      }
    })
  }

  const dismissMemorySuggestion = (suggestion: MemorySuggestion) => {
    if (!currentSessionId.value) {
      return
    }

    if (isActiveMemorySuggestion(suggestion)) {
      activeMemorySuggestionIndex.value = -1
    }
    if (hoveredMemoryPreview.value?.key === buildMemoryReferenceKey(suggestion.sourceType, suggestion.sourceId)) {
      clearMemoryPreview()
    }
    sessionExecutionStore.dismissMemorySuggestion(currentSessionId.value, suggestion)
    if (!hasVisibleMemorySuggestions.value) {
      hideMemorySuggestionPanel()
      return
    }

    armMemorySuggestionAutoHide()
  }

  const removeMemoryReferenceFromDraft = (reference: ComposerMemoryReference) => {
    const sessionId = currentSessionId.value
    if (!sessionId) {
      return
    }

    const token = buildMemoryReferenceToken(reference.sourceType, reference.sourceId)
    const newText = sanitizeComposerText(
      inputText.value
        .split(token)
        .join(' ')
        .replace(/[ \t]{2,}/g, ' ')
    ).trimStart()

    if (textareaRef.value) {
      textareaRef.value.value = newText
    }

    inputText.value = newText
    sessionExecutionStore.removeMemoryReference(sessionId, reference.sourceType, reference.sourceId)
    if (hoveredMemoryPreview.value?.key === buildMemoryReferenceKey(reference.sourceType, reference.sourceId)) {
      clearMemoryPreview()
    }
    focusInput()
  }

  const handleSlashCommandSelect = async (command: SlashCommandDescriptor) => {
    if (command.name === 'compact') {
      await runSlashCommand({
        name: command.name,
        argsText: ''
      })
      focusInput()
      return
    }

    const textarea = textareaRef.value
    const nextText = command.insertText.endsWith(' ')
      ? command.insertText
      : `${command.insertText} `
    const nextPosition = nextText.length

    if (textarea) {
      textarea.value = nextText
    }

    inputText.value = nextText
    closeSlashCommand()

    requestAnimationFrame(() => {
      if (textarea) {
        textarea.focus()
        textarea.setSelectionRange(nextPosition, nextPosition)
        if (renderLayerRef.value) {
          renderLayerRef.value.scrollTop = textarea.scrollTop
        }
        updateSlashCommandState(textarea, nextText, nextPosition)
      } else {
        focusInput()
      }
    })
  }

  const handleCdPathSelect = (insertPath: string) => {
    const textarea = textareaRef.value
    const newText = `/cd ${insertPath}`
    const nextPosition = newText.length

    if (textarea) {
      textarea.value = newText
    }

    inputText.value = newText

    requestAnimationFrame(() => {
      if (textarea) {
        textarea.focus()
        textarea.setSelectionRange(nextPosition, nextPosition)
        if (renderLayerRef.value) {
          renderLayerRef.value.scrollTop = textarea.scrollTop
        }
        updateSlashCommandState(textarea, newText, nextPosition)
      } else {
        closeCdPathSuggestions()
      }
    })
  }

  const getCaretCoordinates = (textarea: HTMLTextAreaElement, position: number) => {
    const text = textarea.value.substring(0, position)
    const lines = text.split('\n')
    const currentLine = lines.length - 1
    const currentCol = lines[lines.length - 1].length

    const style = window.getComputedStyle(textarea)
    const lineHeight = parseFloat(style.lineHeight)
    const paddingTop = parseFloat(style.paddingTop)
    const paddingLeft = parseFloat(style.paddingLeft)
    const fontSize = parseFloat(style.fontSize)
    const fontFamily = style.fontFamily

    const lineHeightActual = lineHeight || fontSize * 1.5
    const y = paddingTop + currentLine * lineHeightActual

    const canvas = document.createElement('canvas')
    const context = canvas.getContext('2d')
    if (context) {
      context.font = `${fontSize}px ${fontFamily}`
      const textWidth = context.measureText(lines[lines.length - 1]).width
      canvas.remove()
      return { x: paddingLeft + textWidth, y }
    }
    canvas.remove()
    return { x: paddingLeft + currentCol * (fontSize * 0.6), y }
  }

  const updateSlashCommandState = (target: HTMLTextAreaElement, value: string, cursorPosition: number) => {
    if (!value.startsWith('/')) {
      closeSlashCommand()
      closeCdPathSuggestions()
      return
    }

    const currentLineValue = value.slice(0, cursorPosition)
    if (currentLineValue.includes('\n')) {
      closeSlashCommand()
      closeCdPathSuggestions()
      return
    }

    if (options.panelType === 'mini' && currentLineValue.startsWith('/cd ')) {
      const rect = target.getBoundingClientRect()
      const caretPos = getCaretCoordinates(target, cursorPosition)
      openCdPathSuggestions(rect.left + caretPos.x, rect.top + caretPos.y + 18, currentLineValue.slice(4))
      return
    }

    closeCdPathSuggestions()

    const body = value.slice(1, cursorPosition)
    if (!body || /\s/.test(body)) {
      if (value === '/') {
        const rect = target.getBoundingClientRect()
        const caretPos = getCaretCoordinates(target, cursorPosition)
        openSlashCommand(rect.left + caretPos.x, rect.top + caretPos.y + 18, '')
      } else {
        closeSlashCommand()
      }
      return
    }

    const rect = target.getBoundingClientRect()
    const caretPos = getCaretCoordinates(target, cursorPosition)
    openSlashCommand(rect.left + caretPos.x, rect.top + caretPos.y + 18, body)
  }

  const handleInput = (event: Event) => {
    const target = event.target as HTMLTextAreaElement
    let value = target.value
    let cursorPosition = target.selectionStart || 0
    const sanitizedValue = sanitizeComposerText(value)

    if (sanitizedValue !== value) {
      cursorPosition = sanitizeComposerText(value.slice(0, cursorPosition)).length
      value = sanitizedValue
      target.value = sanitizedValue
      target.setSelectionRange(cursorPosition, cursorPosition)
    }

    if (showFileMention.value && mentionStart.value >= 0) {
      if (value[mentionStart.value] !== '@') {
        closeFileMention()
      } else if (cursorPosition < mentionStart.value || cursorPosition > mentionStart.value + 100) {
        closeFileMention()
      } else {
        mentionSearchText.value = value.slice(mentionStart.value + 1, cursorPosition)
      }
      inputText.value = value
      return
    }

    if (value.length > 0 && cursorPosition > 0 && value[cursorPosition - 1] === '@') {
      const rect = target.getBoundingClientRect()
      const caretPos = getCaretCoordinates(target, cursorPosition - 1)
      openFileMention(rect.left + caretPos.x, rect.top + caretPos.y + 20, '', cursorPosition - 1)
    }

    inputText.value = value
    updateSlashCommandState(target, value, cursorPosition)
  }

  const handleCompositionStart = () => {
    isInputComposing.value = true
    clearMemorySuggestionTimer()
    if (options.panelType === 'main' && sanitizeMemorySearchText(inputText.value).length >= 4) {
      beginMemorySuggestionSearchCycle()
    }
    if (currentSessionId.value) {
      sessionExecutionStore.setIsSearchingMemory(currentSessionId.value, false)
    }
  }

  const handleCompositionEnd = () => {
    isInputComposing.value = false
    if (!inputText.value.trim()) {
      resetMemorySuggestionState()
      return
    }

    scheduleMemorySuggestionSearch(inputText.value)
  }

  const toPendingAttachment = async (attachment: MessageAttachment): Promise<PendingImageAttachment> => ({
    ...attachment,
    previewUrl: await resolveAttachmentPreviewUrl(attachment)
  })

  const buildAttachmentPreview = (attachments: MessageAttachment[]) => {
    if (attachments.length === 0) {
      return ''
    }

    if (attachments.length === 1) {
      return attachments[0].name.trim()
    }

    return t('message.queueAttachments', { count: attachments.length })
  }

  const uploadAttachments = async (files: File[]) => {
    const sessionId = currentSessionId.value
    if (!sessionId || files.length === 0) {
      return
    }

    try {
      sessionExecutionStore.setIsUploadingImages(sessionId, true)

      const payload: UploadImageInput[] = await Promise.all(files.map(async (file) => ({
        fileName: file.name,
        mimeType: file.type || 'application/octet-stream',
        bytes: Array.from(new Uint8Array(await file.arrayBuffer()))
      })))

      const result = await invoke<UploadSessionImagesResponse>('upload_session_images', {
        sessionId,
        files: payload
      })

      const pendingImages = await Promise.all(result.attachments.map(toPendingAttachment))
      sessionExecutionStore.appendPendingImages(sessionId, pendingImages)
    } catch (error) {
      console.error('Failed to upload attachments:', error)
      notificationStore.smartError('上传附件', error instanceof Error ? error : new Error(String(error)))
    } finally {
      sessionExecutionStore.setIsUploadingImages(sessionId, false)
    }
  }

  const openAttachmentPicker = () => {
    fileInputRef.value?.click()
  }

  const handleAttachmentFileChange = async (event: Event) => {
    const target = event.target as HTMLInputElement
    const files = target.files ? Array.from(target.files) : []
    target.value = ''
    await uploadAttachments(files)
  }

  const handlePaste = async (event: ClipboardEvent) => {
    const items = Array.from(event.clipboardData?.items ?? [])
    const imageFiles = items
      .filter(item => item.type.startsWith('image/'))
      .map(item => item.getAsFile())
      .filter((file): file is File => file !== null)

    if (imageFiles.length === 0) {
      return
    }

    event.preventDefault()
    await uploadAttachments(imageFiles)
  }

  const removeImage = async (imageId: string) => {
    const sessionId = currentSessionId.value
    const image = pendingImages.value.find(item => item.id === imageId)
    if (!sessionId || !image) {
      return
    }

    try {
      await invoke('delete_uploaded_image', {
        sessionId,
        path: image.path
      })
      sessionExecutionStore.removePendingImage(sessionId, imageId)
    } catch (error) {
      console.error('Failed to delete uploaded attachment:', error)
      notificationStore.smartError('删除附件', error instanceof Error ? error : new Error(String(error)))
    }
  }

  const restorePendingImages = async (attachments: MessageAttachment[] = []) => {
    const sessionId = currentSessionId.value
    if (!sessionId) {
      return
    }

    const pendingImages = await Promise.all(attachments.map(toPendingAttachment))
    sessionExecutionStore.setPendingImages(sessionId, pendingImages)
  }

  const buildQueuedMessagePreview = (draft: Pick<QueuedMessageDraft, 'content' | 'displayContent' | 'attachments'>) => {
    const trimmed = removeMemoryReferenceTokens(draft.displayContent ?? draft.content).trim()
    if (trimmed) {
      return trimmed
    }

    return buildAttachmentPreview(draft.attachments)
  }

  const removeQueuedMessage = (draftId: string) => {
    if (!currentSessionId.value) {
      return
    }

    sessionExecutionStore.removeQueuedMessage(currentSessionId.value, draftId)
  }

  const updateQueuedMessage = (
    draftId: string,
    updates: Partial<Pick<QueuedMessageDraft, 'content' | 'displayContent' | 'attachments' | 'expertId' | 'agentId' | 'modelId' | 'memoryReferences' | 'status' | 'errorMessage'>>
  ) => {
    if (!currentSessionId.value) {
      return
    }

    sessionExecutionStore.updateQueuedMessage(currentSessionId.value, draftId, updates)
  }

  const retryQueuedMessage = async (draftId: string) => {
    if (!currentSessionId.value) {
      return
    }

    sessionExecutionStore.retryQueuedMessage(currentSessionId.value, draftId)
    if (!isSending.value) {
      await conversationService.drainQueue(currentSessionId.value)
    }
  }

  const getExecutionAgentConfig = () => {
    if (!currentAgent.value) {
      return null
    }

    const selectedModel = selectedModelId.value.trim()
    if (!selectedModel) {
      return currentAgent.value
    }

    return {
      ...currentAgent.value,
      modelId: selectedModel
    }
  }

  const validateCurrentAgentAvailability = () => {
    const executionAgent = getExecutionAgentConfig()
    if (!executionAgent) {
      notificationStore.smartError('发送失败', new Error('未找到可用专家运行时'))
      return false
    }

    const availability = conversationService.isAgentAvailable(executionAgent)
    if (!availability.available) {
      notificationStore.smartError('发送失败', new Error(availability.reason || '当前专家运行时不可用'))
      return false
    }

    return true
  }

  const clearComposerDraft = (sessionId: string) => {
    inputText.value = ''
    sessionExecutionStore.clearMemoryReferences(sessionId)
    sessionExecutionStore.clearMemorySuggestions(sessionId)
    sessionExecutionStore.clearDismissedMemorySuggestionKeys(sessionId)
    sessionExecutionStore.clearPendingImages(sessionId)
    closeFileMention()
    closeSlashCommand()
    closeCdPathSuggestions()
    clearMemorySuggestionTimer()
    clearMemorySuggestionAutoHideTimer()
    clearMemorySuggestionEmptyTimer()
    hideMemorySuggestionPanel()
  }

  const sendWithCurrentAgent = async (
    userInput: string,
    attachments: MessageAttachment[],
    options?: {
      displayPreviewContent?: string
      memoryReferences?: ComposerMemoryReference[]
      reuseAssistantMessageId?: string
      targetSessionId?: string
    }
  ): Promise<boolean> => {
    const sessionId = options?.targetSessionId ?? currentSessionId.value
    if ((!userInput.trim() && attachments.length === 0) || !sessionId || isSending.value) return false

    const targetSession = sessionStore.sessions.find(session => session.id === sessionId) || null
    const expert = currentExpert.value
    const executionAgent = getExecutionAgentConfig()
    if (!expert || !executionAgent) {
      notificationStore.smartError('发送失败', new Error('未找到可用专家'))
      return false
    }

    const availability = conversationService.isAgentAvailable(executionAgent)
    if (!availability.available) {
      notificationStore.smartError('发送失败', new Error(availability.reason || '当前专家运行时不可用'))
      return false
    }

    try {
      if (targetSession?.expertId !== expert.id || targetSession?.agentId !== executionAgent.id) {
        await sessionStore.updateSession(sessionId, {
          expertId: expert.id,
          agentId: executionAgent.id,
          agentType: executionAgent.provider || executionAgent.type || 'claude',
          cliSessionId: '',
          cliSessionProvider: ''
        })
      }

      await conversationService.sendMessage(
        sessionId,
        userInput,
        executionAgent.id,
        targetSession?.projectId,
        attachments,
        {
          workingDirectory: currentWorkingDirectory.value || undefined,
          modelId: selectedModelId.value.trim() || undefined,
          injectedSystemMessages: [
            buildExpertSystemPrompt(expert.prompt)
          ],
          previewContent: options?.displayPreviewContent,
          memoryReferencesToPersist: options?.memoryReferences ?? [],
          reuseAssistantMessageId: options?.reuseAssistantMessageId
        }
      )
      return true
    } catch (error) {
      console.error('Failed to send message:', error)
      const normalizedError = error instanceof Error
        ? error
        : new Error(getErrorMessage(error, '发送失败'))
      void writeFrontendRuntimeLog(
        'ERROR',
        'conversation-composer',
        `sendWithCurrentAgent failed | sessionId=${sessionId} | projectId=${currentSession.value?.projectId || ''} | agentId=${executionAgent.id} | expertId=${expert.id} | error=${normalizedError.message}`,
        error
      )
      notificationStore.smartError('发送失败', normalizedError)
      sessionExecutionStore.endSending(sessionId)
      return false
    }
  }

  const clearCurrentSession = async () => {
    const sessionId = currentSessionId.value
    if (!sessionId) {
      return
    }

    await messageStore.clearSessionMessages(sessionId)
    sessionExecutionStore.clearExecutionState(sessionId)
    sessionExecutionStore.setInputText(sessionId, '')
    focusInput()
  }

  const runSlashCommand = async (parsedSlashCommand: ParsedSlashCommand) => {
    const sessionId = currentSessionId.value
    if (!sessionId) {
      return false
    }

    const result = await executeSlashCommand(parsedSlashCommand, {
      panelType: options.panelType,
      sessionId,
      isSending: isSending.value,
      hasMessages: messageCount.value > 0,
      currentWorkingDirectory: currentWorkingDirectory.value,
      openCompressionDialog: handleOpenCompress,
      clearSession: clearCurrentSession,
      setWorkingDirectory: options.setWorkingDirectory,
      notifySuccess: message => notificationStore.success(message),
      notifyWarning: message => notificationStore.warning(message),
      notifyError: message => notificationStore.error(t('common.error'), message)
    })

    if (result.handled) {
      closeSlashCommand()
      closeCdPathSuggestions()
      if (result.clearInput) {
        inputText.value = ''
      }
    }

    return result.handled
  }

  const handleSend = async () => {
    const sessionId = currentSessionId.value
    if (!sessionId || isUploadingImages.value) return

    const rawInput = inputText.value
    const expandedInput = expandComposerMentions(rawInput, currentFileMentions.value).trim()
    const orderedMemoryReferences = reconcileMemoryReferences(expandedInput, currentMemoryReferences.value)
    const annotatedMessage = buildMemoryAnnotatedMessage(expandedInput, orderedMemoryReferences)
    const userInput = annotatedMessage.content.trim()
    const displayInput = removeMemoryReferenceTokens(rawInput).trim()
    const hasMemoryReferences = orderedMemoryReferences.length > 0
    const attachments = pendingImages.value.map((image) => {
      const { previewUrl, ...attachment } = image
      void previewUrl
      return attachment
    })

    if (!displayInput && !hasMemoryReferences && attachments.length === 0) return

    const parsedSlashCommand = attachments.length === 0 ? parseSlashCommandInput(userInput) : null
    if (parsedSlashCommand) {
      const handled = await runSlashCommand(parsedSlashCommand)
      if (handled) {
        return
      }
    }

    if (isSending.value || isCurrentSessionDispatching.value) {
      if (!validateCurrentAgentAvailability()) {
        return
      }

      const queuedExpert = currentExpert.value
      const queuedAgent = currentAgent.value
      if (!queuedExpert || !queuedAgent) {
        return
      }

      sessionExecutionStore.queueMessage(sessionId, {
        content: userInput,
        displayContent: annotatedMessage.previewContent || rawInput,
        attachments,
        expertId: queuedExpert.id,
        agentId: queuedAgent.id,
        modelId: selectedModelId.value.trim() || undefined,
        memoryReferences: orderedMemoryReferences
      })
      clearComposerDraft(sessionId)
      focusInput()
      return
    }

    if (!validateCurrentAgentAvailability()) {
      return
    }

    const executionAgent = getExecutionAgentConfig()
    if (!executionAgent) {
      return
    }

    let sendSessionId = sessionId

    try {
      if (compressionService.shouldAutoCompressSession(sessionId)) {
        isCompressing.value = true
        const result = await compressionService.compressSession(
          sessionId,
          executionAgent.id,
          { strategy: settingsStore.settings.compressionStrategy as CompressionStrategy }
        )
        isCompressing.value = false

        if (!result.success || !result.newSessionId) {
          notificationStore.error(t('compression.failed'), result.error)
          focusInput()
          return
        }

        notificationStore.success(t('compression.success'))
        sendSessionId = result.newSessionId
        await nextTick()
      }

      dispatchingSessionId.value = sendSessionId
      clearComposerDraft(sendSessionId)
      await nextTick()

      const success = await sendWithCurrentAgent(userInput, attachments, {
        displayPreviewContent: annotatedMessage.previewContent,
        memoryReferences: orderedMemoryReferences,
        targetSessionId: sendSessionId
      })
      if (success) {
        focusInput()
      } else {
        inputText.value = rawInput
        sessionExecutionStore.setMemoryReferences(sendSessionId, orderedMemoryReferences)
        await restorePendingImages(attachments)
        focusInput()
      }
    } finally {
      isCompressing.value = false
      if (dispatchingSessionId.value === sendSessionId) {
        dispatchingSessionId.value = null
      }
    }
  }

  const retryMessage = async (
    messageId: string,
    content: string,
    attachments: MessageAttachment[] = [],
    replaceMessageId?: string
  ) => {
    const sessionId = currentSessionId.value
    const normalizedContent = content.trim()
    if (!sessionId || isUploadingImages.value || isSending.value || isCurrentSessionDispatching.value) {
      return false
    }

    if (!normalizedContent && attachments.length === 0) {
      return false
    }

    if (!validateCurrentAgentAvailability()) {
      return false
    }

    const executionAgent = getExecutionAgentConfig()
    if (!executionAgent) {
      return false
    }

    dispatchingSessionId.value = sessionId

    try {
      const expert = currentExpert.value
      if (!expert) {
        throw new Error('未找到可用专家')
      }

      if (replaceMessageId && replaceMessageId !== messageId) {
        await messageStore.deleteMessage(replaceMessageId)
      }

      tokenStore.clearRealtimeTokens(sessionId)

      await conversationService.sendMessage(
        sessionId,
        normalizedContent,
        executionAgent.id,
        currentSession.value?.projectId,
        attachments,
        {
          workingDirectory: currentWorkingDirectory.value || undefined,
          modelId: selectedModelId.value.trim() || undefined,
          injectedSystemMessages: [
            buildExpertSystemPrompt(expert.prompt)
          ],
          existingUserMessageId: messageId
        }
      )
      focusInput()
      return true
    } catch (error) {
      console.error('Failed to retry message:', error)
      notificationStore.smartError('重试失败', error instanceof Error ? error : new Error(String(error)))
      sessionExecutionStore.endSending(sessionId)
      return false
    } finally {
      if (dispatchingSessionId.value === sessionId) {
        dispatchingSessionId.value = null
      }
    }
  }

  const handleMessageFormSubmit = async (
    formId: string,
    values: Record<string, unknown>
  ) => {
    if (!currentSessionId.value || !currentAgent.value || isSending.value || isCurrentSessionDispatching.value) {
      return
    }

    const payload = JSON.stringify({
      type: 'form_response',
      formId,
      values
    }, null, 2)

    await sendWithCurrentAgent(payload, [])
  }

  const handleKeyDown = (event: KeyboardEvent) => {
    const normalizedEvent = event as KeyboardEvent & { keyCode?: number; isComposing?: boolean }
    if (normalizedEvent.isComposing || isInputComposing.value || normalizedEvent.keyCode === 229) {
      return
    }

    if (showFileMention.value || showSlashCommand.value || showCdPathSuggestions.value) {
      return
    }

    if (shouldShowMemorySuggestions.value && flatVisibleMemorySuggestions.value.length > 0) {
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        markMemorySuggestionKeyboardInteraction()
        moveActiveMemorySuggestion(1)
        return
      }

      if (event.key === 'ArrowUp') {
        event.preventDefault()
        markMemorySuggestionKeyboardInteraction()
        moveActiveMemorySuggestion(-1)
        return
      }

      if (event.key === 'Enter' && !event.shiftKey && activeMemorySuggestion.value) {
        event.preventDefault()
        insertMemoryReference(activeMemorySuggestion.value)
        return
      }
    }

    if (event.key === 'Escape' && shouldShowMemorySuggestions.value) {
      event.preventDefault()
      hideMemorySuggestionPanel()
      return
    }

    if (event.key === 'Enter') {
      const sendOnEnter = settingsStore.settings.sendOnEnter

      if (sendOnEnter && !event.shiftKey) {
        event.preventDefault()
        void handleSend()
      } else if (!sendOnEnter && (event.metaKey || event.ctrlKey)) {
        event.preventDefault()
        void handleSend()
      }
    }
  }

  const insertFileMentions = (paths: string[]) => {
    if (paths.length === 0) {
      return
    }

    const textarea = textareaRef.value
    const baseMentions = [...reconcileFileMentions(inputText.value, currentFileMentions.value)]
    const globalMentions: ComposerFileMention[] = []
    const insertText = paths.map((path) => {
      if (!isGlobalMentionPath(path)) {
        return formatMentionLiteral(path)
      }

      const mention = createComposerMention(path)
      globalMentions.push(mention)
      return mention.displayText
    }).join(' ') + ' '

    if (!textarea) {
      inputText.value += insertText
      if (currentSessionId.value) {
        sessionExecutionStore.setFileMentions(currentSessionId.value, [...baseMentions, ...globalMentions])
      }
      return
    }

    const start = textarea.selectionStart ?? inputText.value.length
    const end = textarea.selectionEnd ?? inputText.value.length
    const mentionIndex = countMentionsInText(inputText.value.slice(0, start))
    const nextMentions = [...baseMentions]
    nextMentions.splice(mentionIndex, 0, ...globalMentions)
    const newText = sanitizeComposerText(`${inputText.value.slice(0, start)}${insertText}${inputText.value.slice(end)}`)
    const newPosition = start + insertText.length

    textarea.value = newText

    inputText.value = newText
    if (currentSessionId.value) {
      sessionExecutionStore.setFileMentions(currentSessionId.value, nextMentions)
    }

    requestAnimationFrame(() => {
      textarea.focus()
      textarea.setSelectionRange(newPosition, newPosition)
      if (renderLayerRef.value) {
        renderLayerRef.value.scrollTop = textarea.scrollTop
      }
    })
  }

  return {
    agentDropdownRef,
    agentOptions,
    buildQueuedMessagePreview,
    cdPathPosition,
    cdPathQuery,
    closeFileMention,
    closeCdPathSuggestions,
    closeSlashCommand,
    currentAgent,
    currentAgentId,
    currentAgentName,
    currentMemoryReferences,
    currentProjectPath,
    currentSessionId,
    currentWorkingDirectory,
    dismissMemorySuggestion,
    fileInputRef,
    fileMentionPosition,
    focusInput,
    getModelLabel,
    handleCancelCompress,
    handleCdPathSelect,
    handleConfirmCompress,
    handleFileSelect,
    handleAttachmentFileChange,
    handleInput,
    handleCompositionEnd,
    handleCompositionStart,
    handleKeyDown,
    handleMessageFormSubmit,
    handleMemorySuggestionPointerEnter,
    handleMemorySuggestionPointerLeave,
    handleMemoryPreviewPointerEnter,
    handleMemoryPreviewPointerLeave,
    insertMemoryReference,
    handleOpenCompress,
    handlePaste,
    hasVisibleMemorySuggestions,
    isMemorySuggestionLoading,
    isSearchingMemory,
    retryMessage,
    handleSend,
    handleSlashCommandSelect,
    inputPlaceholder,
    inputText,
    insertFileMentions,
    isActiveMemorySuggestion,
    isAgentDropdownOpen,
    isCompressing,
    isModelDropdownOpen,
    isSending,
    isUploadingImages,
    mentionSearchText,
    mentionStart,
    messageCount,
    modelDropdownRef,
    openAttachmentPicker,
    parsedInputText,
    pendingImages,
    previewMemoryReference,
    previewMemorySuggestion,
    presetModelOptions,
    queuedMessages,
    currentMemoryPreview,
    removeImage,
    removeMemoryReferenceFromDraft,
    removeQueuedMessage,
    updateQueuedMessage,
    renderLayerRef,
    restorePendingImages,
    retryQueuedMessage,
    selectedModelId,
    selectAgent,
    selectModel,
    shouldShowCompressButton,
    showCompressionDialog,
    showCdPathSuggestions,
    showFileMention,
    showSlashCommand,
    slashCommandPosition,
    slashCommandQuery,
    slashCommands,
    syncScroll,
    textareaRef,
    shouldShowMemorySuggestionEmptyState,
    shouldShowMemorySuggestionIdleHint,
    shouldShowMemorySuggestions,
    toggleAgentDropdown,
    toggleModelDropdown,
    tokenUsage,
    visibleMemorySuggestions,
    clearMemoryPreview,
    scheduleClearMemoryPreview,
    activeMemorySuggestionKey
  }
}
