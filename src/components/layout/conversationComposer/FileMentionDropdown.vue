<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useSessionStore } from '@/stores/session'
import { useProjectStore } from '@/stores/project'
import { useThemeStore } from '@/stores/theme'
import { resolveFileIcon } from '@/utils/fileIcon'
import { EaIcon } from '@/components/common'
import type { PendingImageAttachment } from '@/stores/sessionExecution'

type FileMentionScope = 'project' | 'global'

interface FileMentionSearchResult {
  name: string
  path: string
  insertPath: string
  displayPath: string
  nodeType: 'file' | 'directory'
  extension: string | null
  scope: FileMentionScope
}

interface AttachmentMentionEntry {
  name: string
  placeholder: string
  mimeType: string
  isImage: boolean
}

type MentionResultItem =
  | { kind: 'file'; data: FileMentionSearchResult }
  | { kind: 'attachment'; data: AttachmentMentionEntry }

const LAST_SCOPE_KEY = 'ea-file-mention-scope'

const props = defineProps<{
  visible: boolean
  position: { x: number; y: number; width: number; height: number }
  searchText: string
  mentionStart: number
  projectPath?: string | null
  defaultScope?: FileMentionScope
  pendingImages?: PendingImageAttachment[]
}>()

const emit = defineEmits<{
  select: [insertPath: string, mentionStart: number]
  close: []
}>()

const { t } = useI18n()
const sessionStore = useSessionStore()
const projectStore = useProjectStore()
const themeStore = useThemeStore()

const isOpen = computed(() => props.visible)
const isDarkTheme = computed(() => themeStore.isDark)
const isLoading = ref(false)
const hasResolvedSearch = ref(false)
const results = ref<FileMentionSearchResult[]>([])
const selectedIndex = ref(0)
const dropdownRef = ref<HTMLElement | null>(null)
const activeScope = ref<FileMentionScope>(
  (localStorage.getItem(LAST_SCOPE_KEY) as FileMentionScope | null) ?? props.defaultScope ?? 'project'
)
let searchTimer: ReturnType<typeof setTimeout> | null = null
let searchToken = 0

const currentProject = computed(() => {
  if (props.projectPath) {
    return {
      id: '__external__',
      path: props.projectPath
    }
  }

  const sessionId = sessionStore.currentSessionId
  if (!sessionId) return null
  return projectStore.projects.find(project => project.id === sessionStore.currentSession?.projectId) || null
})

const trimmedSearchText = computed(() => props.searchText.trim())
const requiresGlobalQuery = computed(() => activeScope.value === 'global' && trimmedSearchText.value.length < 2)

const attachmentEntries = computed<AttachmentMentionEntry[]>(() => {
  const images = props.pendingImages ?? []
  if (images.length === 0) return []

  const query = trimmedSearchText.value.toLowerCase()
  const entries: AttachmentMentionEntry[] = images.map((img, index) => ({
    name: img.name,
    placeholder: `[Image${index + 1}]`,
    mimeType: img.mimeType,
    isImage: img.mimeType.startsWith('image/')
  }))

  if (!query) return entries

  return entries.filter(entry =>
    entry.name.toLowerCase().includes(query)
    || entry.placeholder.toLowerCase().includes(query)
    || (entry.isImage ? 'image' : 'file').includes(query)
  )
})

const scopeOptions = computed(() => ([
  {
    value: 'project' as const,
    label: t('fileMention.scopeProject'),
    icon: 'folder-open'
  },
  {
    value: 'global' as const,
    label: t('fileMention.scopeGlobal'),
    icon: 'globe'
  }
]))

const dropdownStyle = computed(() => {
  if (!props.position.x || !props.position.y) return {}

  const dropdownHeight = 360
  const spaceBelow = window.innerHeight - props.position.y
  const showAbove = spaceBelow < dropdownHeight

  if (showAbove) {
    return {
      left: `${props.position.x}px`,
      bottom: `${window.innerHeight - props.position.y + 24}px`
    }
  }

  return {
    left: `${props.position.x}px`,
    top: `${props.position.y + 4}px`
  }
})

const emptyStateMessage = computed(() => {
  if (requiresGlobalQuery.value) {
    return trimmedSearchText.value
      ? t('fileMention.globalMinChars')
      : t('fileMention.globalHint')
  }

  if (activeScope.value === 'project' && !currentProject.value) {
    return t('fileMention.projectUnavailable')
  }

  if (trimmedSearchText.value) {
    return t('fileMention.noMatches')
  }

  return activeScope.value === 'project'
    ? t('fileMention.projectEmpty')
    : t('fileMention.globalEmpty')
})

const mergedResults = computed<MentionResultItem[]>(() => {
  const items: MentionResultItem[] = []
  for (const att of attachmentEntries.value) {
    items.push({ kind: 'attachment', data: att })
  }
  for (const file of results.value) {
    items.push({ kind: 'file', data: file })
  }
  return items
})

const close = () => {
  emit('close')
}

const setScope = (scope: FileMentionScope) => {
  if (activeScope.value === scope) {
    return
  }

  activeScope.value = scope
  localStorage.setItem(LAST_SCOPE_KEY, scope)
  selectedIndex.value = 0

  if ((scope === 'project' && currentProject.value) || (scope === 'global' && trimmedSearchText.value.length >= 2)) {
    results.value = []
    isLoading.value = true
    hasResolvedSearch.value = false
  }
}

const performSearch = async () => {
  if (!isOpen.value) {
    return
  }

  if (activeScope.value === 'project' && !currentProject.value) {
    results.value = []
    isLoading.value = false
    hasResolvedSearch.value = false
    return
  }

  if (requiresGlobalQuery.value) {
    results.value = []
    isLoading.value = false
    hasResolvedSearch.value = false
    return
  }

  const currentToken = ++searchToken
  isLoading.value = true

  try {
    const payload = {
      input: {
        query: props.searchText,
        scope: activeScope.value,
        projectPath: currentProject.value?.path,
        limit: 80
      }
    }

    const nextResults = await invoke<FileMentionSearchResult[]>('search_file_mentions', payload)
    if (currentToken !== searchToken) {
      return
    }

    results.value = nextResults ?? []
    hasResolvedSearch.value = true
    selectedIndex.value = Math.min(selectedIndex.value, Math.max(mergedResults.value.length - 1, 0))
  } catch (error) {
    console.error('Failed to search file mentions:', error)
    if (currentToken === searchToken) {
      results.value = []
      hasResolvedSearch.value = true
    }
  } finally {
    if (currentToken === searchToken) {
      isLoading.value = false
    }
  }
}

const scheduleSearch = () => {
  if (searchTimer) {
    clearTimeout(searchTimer)
  }

  if (activeScope.value === 'project' && !currentProject.value) {
    isLoading.value = false
    hasResolvedSearch.value = false
    results.value = []
    return
  }

  if (requiresGlobalQuery.value) {
    isLoading.value = false
    hasResolvedSearch.value = false
    results.value = []
    return
  }

  isLoading.value = true
  hasResolvedSearch.value = false

  searchTimer = setTimeout(() => {
    searchTimer = null
    void performSearch()
  }, activeScope.value === 'global' ? 160 : 100)
}

const selectFile = (file: FileMentionSearchResult) => {
  close()
  emit('select', file.insertPath, props.mentionStart)
}

const selectAttachment = (entry: AttachmentMentionEntry) => {
  close()
  emit('select', entry.placeholder, props.mentionStart)
}

const selectItem = (item: MentionResultItem) => {
  if (item.kind === 'attachment') {
    selectAttachment(item.data)
  } else {
    selectFile(item.data)
  }
}

const scrollToSelected = () => {
  nextTick(() => {
    const selectedEl = dropdownRef.value?.querySelector('.file-mention__item--selected')
    selectedEl?.scrollIntoView({ block: 'nearest' })
  })
}

const switchScopeByKeyboard = () => {
  setScope(activeScope.value === 'project' ? 'global' : 'project')
}

const handleKeyDown = (event: KeyboardEvent) => {
  if (!isOpen.value) return

  const totalItems = mergedResults.value.length

  switch (event.key) {
    case 'ArrowUp':
      event.preventDefault()
      event.stopPropagation()
      if (totalItems === 0) return
      selectedIndex.value = selectedIndex.value > 0 ? selectedIndex.value - 1 : totalItems - 1
      scrollToSelected()
      break
    case 'ArrowDown':
      event.preventDefault()
      event.stopPropagation()
      if (totalItems === 0) return
      selectedIndex.value = selectedIndex.value < totalItems - 1 ? selectedIndex.value + 1 : 0
      scrollToSelected()
      break
    case 'Enter': {
      const selectedItem = mergedResults.value[selectedIndex.value]
      if (!selectedItem) return
      event.preventDefault()
      event.stopPropagation()
      selectItem(selectedItem)
      break
    }
    case 'Escape':
      event.preventDefault()
      event.stopPropagation()
      close()
      break
    case 'Tab':
      event.preventDefault()
      event.stopPropagation()
      switchScopeByKeyboard()
      break
  }
}

const getFileIconName = (file: FileMentionSearchResult): string => {
  if (file.nodeType === 'directory') return 'folder'
  const iconMeta = resolveFileIcon(file.nodeType, file.name, file.extension ?? undefined)
  return typeof iconMeta === 'string' ? iconMeta : (iconMeta?.icon || 'file')
}

const getItemIconName = (item: MentionResultItem): string => {
  if (item.kind === 'attachment') {
    return item.data.isImage ? 'image' : 'file-text'
  }
  return getFileIconName(item.data)
}

const getItemName = (item: MentionResultItem): string => {
  if (item.kind === 'attachment') {
    return item.data.name
  }
  return item.data.name
}

const getItemSubtext = (item: MentionResultItem): string => {
  if (item.kind === 'attachment') {
    return item.data.placeholder
  }
  return item.data.displayPath
}

const highlightMatch = (text: string) => {
  const query = trimmedSearchText.value.toLowerCase()
  if (!query) return text

  const index = text.toLowerCase().indexOf(query)
  if (index < 0) return text

  return text.slice(0, index)
    + '<mark>' + text.slice(index, index + query.length) + '</mark>'
    + text.slice(index + query.length)
}

watch(
  () => [props.visible, props.searchText, activeScope.value, currentProject.value?.path] as const,
  ([visible]) => {
    if (!visible) {
      results.value = []
      isLoading.value = false
      hasResolvedSearch.value = false
      return
    }

    selectedIndex.value = 0
    scheduleSearch()
  },
  { immediate: true }
)

watch(attachmentEntries, () => {
  if (isOpen.value) {
    selectedIndex.value = Math.min(selectedIndex.value, Math.max(mergedResults.value.length - 1, 0))
  }
})

watch(results, () => {
  nextTick(scrollToSelected)
})

onMounted(() => {
  document.addEventListener('keydown', handleKeyDown, true)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeyDown, true)
  if (searchTimer) {
    clearTimeout(searchTimer)
    searchTimer = null
  }
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      ref="dropdownRef"
      class="file-mention-dropdown"
      :class="{ 'file-mention-dropdown--dark': isDarkTheme }"
      :style="dropdownStyle"
    >
      <div class="file-mention__header">
        <div class="file-mention__scope-switch">
          <button
            v-for="scope in scopeOptions"
            :key="scope.value"
            class="file-mention__scope"
            :class="{ 'file-mention__scope--active': scope.value === activeScope }"
            @click="setScope(scope.value)"
          >
            <EaIcon
              :name="scope.icon"
              :size="12"
            />
            <span>{{ scope.label }}</span>
          </button>
        </div>
        <div class="file-mention__meta">
          <EaIcon
            :name="activeScope === 'global' ? 'sparkles' : 'search'"
            :size="12"
          />
          <span v-if="trimmedSearchText">{{ t('fileMention.searchingFor', { query: trimmedSearchText }) }}</span>
          <span v-else>{{ t('fileMention.scopeHint', { scope: activeScope === 'project' ? t('fileMention.scopeProject') : t('fileMention.scopeGlobal') }) }}</span>
          <span
            v-if="mergedResults.length > 0"
            class="file-mention__count"
          >
            {{ t('fileMention.resultCount', { count: mergedResults.length }) }}
          </span>
        </div>
      </div>

      <div
        v-if="!isLoading && mergedResults.length === 0 && (hasResolvedSearch || requiresGlobalQuery || (activeScope === 'project' && !currentProject) || !trimmedSearchText)"
        class="file-mention__empty"
      >
        <EaIcon
          :name="requiresGlobalQuery ? 'search' : 'file-x'"
          :size="24"
        />
        <span>{{ emptyStateMessage }}</span>
      </div>

      <div
        v-else
        class="file-mention__list"
      >
        <div
          v-for="(item, index) in mergedResults"
          :key="item.kind === 'attachment' ? `att-${item.data.placeholder}` : `file-${item.data.scope}-${item.data.path}`"
          class="file-mention__item"
          :class="{
            'file-mention__item--selected': index === selectedIndex,
            'file-mention__item--attachment': item.kind === 'attachment'
          }"
          @click="selectItem(item)"
          @mouseenter="selectedIndex = index"
        >
          <div class="file-mention__item-icon">
            <EaIcon
              :name="getItemIconName(item)"
              :size="14"
            />
          </div>
          <div class="file-mention__item-body">
            <span
              class="file-mention__item-name"
              v-html="highlightMatch(getItemName(item))"
            />
            <span
              class="file-mention__item-path"
              :class="{ 'file-mention__item-path--muted': item.kind === 'file' && item.data.displayPath === item.data.name }"
              v-html="highlightMatch(getItemSubtext(item))"
            />
          </div>
          <span
            v-if="item.kind === 'attachment'"
            class="file-mention__item-scope file-mention__item-scope--attachment"
          >
            {{ item.data.isImage ? t('fileMention.scopeImage') : t('fileMention.scopeFile') }}
          </span>
          <span
            v-else
            class="file-mention__item-scope"
          >
            {{ item.data.scope === 'project' ? t('fileMention.scopeProjectShort') : t('fileMention.scopeGlobalShort') }}
          </span>
        </div>

        <div
          v-if="isLoading"
          class="file-mention__loading"
        >
          <EaIcon
            name="loader-circle"
            :size="16"
            spin
          />
          <span>{{ t('fileMention.loading') }}</span>
        </div>
      </div>

      <div class="file-mention__footer">
        <kbd>Tab</kbd>
        <span>{{ t('fileMention.switchScope') }}</span>
        <kbd>↑↓</kbd>
        <span>{{ t('fileMention.navigate') }}</span>
        <kbd>Enter</kbd>
        <span>{{ t('fileMention.select') }}</span>
        <kbd>Esc</kbd>
        <span>{{ t('fileMention.close') }}</span>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.file-mention-dropdown {
  position: fixed;
  min-width: 360px;
  max-width: 480px;
  max-height: 380px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid rgba(148, 163, 184, 0.24);
  border-radius: 18px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.97), rgba(248, 250, 252, 0.96));
  box-shadow: 0 24px 60px rgba(15, 23, 42, 0.18);
  backdrop-filter: blur(18px);
  z-index: 10000;
}

.file-mention__header {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  border-bottom: 1px solid rgba(148, 163, 184, 0.16);
  background: linear-gradient(180deg, rgba(226, 232, 240, 0.24), rgba(255, 255, 255, 0.3));
}

.file-mention__scope-switch {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  width: fit-content;
  padding: 4px;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.1);
}

.file-mention__scope {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 10px;
  border: none;
  border-radius: 999px;
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.file-mention__scope:hover {
  color: var(--color-text-primary);
}

.file-mention__scope--active {
  background: rgba(14, 165, 233, 0.12);
  color: #0369a1;
  box-shadow: inset 0 0 0 1px rgba(14, 165, 233, 0.16);
}

.file-mention__meta {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.file-mention__meta span:first-of-type {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-mention__count {
  margin-left: auto;
  padding: 3px 8px;
  border-radius: 999px;
  background: rgba(14, 165, 233, 0.12);
  color: #0369a1;
  font-weight: 600;
}

.file-mention__list {
  flex: 1;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.34) transparent;
}

.file-mention__list::-webkit-scrollbar {
  width: 6px;
}

.file-mention__list::-webkit-scrollbar-track {
  background: transparent;
}

.file-mention__list::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.34);
  border-radius: 999px;
}

.file-mention__empty,
.file-mention__loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  padding: var(--spacing-6) var(--spacing-4);
  color: var(--color-text-tertiary);
  text-align: center;
}

.file-mention__item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 11px 14px;
  cursor: pointer;
  transition: background-color var(--transition-fast) var(--easing-default);
  border-bottom: 1px solid rgba(148, 163, 184, 0.08);
}

.file-mention__item:last-child {
  border-bottom: none;
}

.file-mention__item:hover,
.file-mention__item--selected {
  background: rgba(14, 165, 233, 0.08);
}

.file-mention__item-icon {
  width: 18px;
  display: inline-flex;
  justify-content: center;
  padding-top: 2px;
  color: var(--color-text-secondary);
}

.file-mention__item-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.file-mention__item-name,
.file-mention__item-path {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-mention__item-name {
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
}

.file-mention__item-path {
  font-size: 11px;
  color: var(--color-text-tertiary);
}

.file-mention__item-path--muted {
  opacity: 0.5;
}

.file-mention__item-scope {
  flex-shrink: 0;
  margin-top: 1px;
  padding: 3px 7px;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.05);
  color: var(--color-text-tertiary);
  font-size: 10px;
  font-weight: 600;
}

.file-mention__item-scope--attachment {
  background: rgba(139, 92, 246, 0.12);
  color: #7c3aed;
}

.file-mention__item--attachment {
  background: rgba(139, 92, 246, 0.04);
}

.file-mention__item--attachment:hover,
.file-mention__item--attachment.file-mention__item--selected {
  background: rgba(139, 92, 246, 0.1);
}

.file-mention__item-name :deep(mark),
.file-mention__item-path :deep(mark) {
  background: rgba(251, 191, 36, 0.28);
  color: inherit;
  padding: 0 2px;
  border-radius: 4px;
}

.file-mention__footer {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  padding: var(--spacing-2) var(--spacing-3);
  border-top: 1px solid rgba(148, 163, 184, 0.16);
  font-size: 11px;
  color: var(--color-text-tertiary);
  background: rgba(255, 255, 255, 0.72);
}

.file-mention__footer kbd {
  padding: 2px 6px;
  border: 1px solid rgba(148, 163, 184, 0.36);
  border-radius: 6px;
  background: rgba(248, 250, 252, 0.92);
  color: var(--color-text-secondary);
  font-family: inherit;
  font-size: 10px;
}

.file-mention-dropdown--dark {
  border-color: rgba(71, 85, 105, 0.72);
  background:
    linear-gradient(180deg, rgba(15, 23, 42, 0.97), rgba(2, 6, 23, 0.96));
  box-shadow: 0 24px 60px rgba(2, 6, 23, 0.46);
}

.file-mention-dropdown--dark .file-mention__header {
  border-bottom-color: rgba(71, 85, 105, 0.42);
  background: linear-gradient(180deg, rgba(30, 41, 59, 0.72), rgba(15, 23, 42, 0.38));
}

.file-mention-dropdown--dark .file-mention__scope-switch {
  background: rgba(51, 65, 85, 0.72);
}

.file-mention-dropdown--dark .file-mention__scope {
  color: #94a3b8;
}

.file-mention-dropdown--dark .file-mention__scope:hover {
  color: #e2e8f0;
}

.file-mention-dropdown--dark .file-mention__scope--active {
  background: rgba(14, 165, 233, 0.18);
  color: #bae6fd;
  box-shadow: inset 0 0 0 1px rgba(56, 189, 248, 0.22);
}

.file-mention-dropdown--dark .file-mention__meta,
.file-mention-dropdown--dark .file-mention__empty,
.file-mention-dropdown--dark .file-mention__loading,
.file-mention-dropdown--dark .file-mention__footer {
  color: #94a3b8;
}

.file-mention-dropdown--dark .file-mention__count {
  background: rgba(14, 165, 233, 0.18);
  color: #bae6fd;
}

.file-mention-dropdown--dark .file-mention__item {
  border-bottom-color: rgba(71, 85, 105, 0.28);
}

.file-mention-dropdown--dark .file-mention__item:hover,
.file-mention-dropdown--dark .file-mention__item--selected {
  background: rgba(14, 165, 233, 0.14);
}

.file-mention-dropdown--dark .file-mention__item-icon,
.file-mention-dropdown--dark .file-mention__item-path,
.file-mention-dropdown--dark .file-mention__item-scope {
  color: #94a3b8;
}

.file-mention-dropdown--dark .file-mention__item-name {
  color: #e2e8f0;
}

.file-mention-dropdown--dark .file-mention__item-scope {
  background: rgba(51, 65, 85, 0.72);
}

.file-mention-dropdown--dark .file-mention__item-scope--attachment {
  background: rgba(139, 92, 246, 0.22);
  color: #c4b5fd;
}

.file-mention-dropdown--dark .file-mention__item--attachment {
  background: rgba(139, 92, 246, 0.08);
}

.file-mention-dropdown--dark .file-mention__item--attachment:hover,
.file-mention-dropdown--dark .file-mention__item--attachment.file-mention__item--selected {
  background: rgba(139, 92, 246, 0.16);
}

.file-mention-dropdown--dark .file-mention__footer {
  border-top-color: rgba(71, 85, 105, 0.42);
  background: rgba(15, 23, 42, 0.78);
}

.file-mention-dropdown--dark .file-mention__footer kbd {
  border-color: rgba(71, 85, 105, 0.76);
  background: rgba(30, 41, 59, 0.9);
  color: #cbd5e1;
}

.file-mention-dropdown--dark .file-mention__list {
  scrollbar-color: rgba(100, 116, 139, 0.48) transparent;
}

.file-mention-dropdown--dark .file-mention__list::-webkit-scrollbar-thumb {
  background: rgba(100, 116, 139, 0.48);
}
</style>
