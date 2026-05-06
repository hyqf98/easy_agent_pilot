import { computed, h, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { UnwatchFn } from '@tauri-apps/plugin-fs'
import type { TreeOption } from 'naive-ui'
import { EaIcon } from '@/components/common'
import { useSessionFileReference } from '@/composables'
import type { FileTreeNode } from '@/stores/project'
import { createComposerFileMention } from '@/utils/composerFileMention'
import { resolveFileIcon } from '@/utils/fileIcon'
import { startFsWatcher } from '@/utils/fsWatcher'
import { useFileOperations } from './composables/useFileOperations'
import type { ContextMenuContext, CreateEntryType, FileTreeNodeData } from './types'

export interface FileTreeProps {
  projectId: string
  projectPath: string
}

export type FileTreeEmits = {
  (event: 'fileSelect', path: string): void
}

interface LoadTreeDataOptions {
  resetChildCache?: boolean
  force?: boolean
}

const DRAG_AUTO_EXPAND_DELAY_MS = 420
const TREE_RELOAD_DEBOUNCE_MS = 180

interface FileTreeRuntimeCache {
  rootNodes: FileTreeNode[] | null
  rootLoadedAt: number
  childrenByDir: Map<string, FileTreeNode[]>
  expandedKeys: string[]
  staleDirs: Set<string>
  pendingLoads: Map<string, Promise<FileTreeNode[]>>
  rootLoadPromise: Promise<FileTreeNode[]> | null
  scrollTop: number
}

interface ProjectFileSearchResult {
  name: string
  path: string
  insertPath: string
  displayPath: string
  nodeType: 'file' | 'directory'
  extension?: string
  scope: 'project' | 'global'
}

const projectFileTreeCaches = new Map<string, FileTreeRuntimeCache>()
const projectFileTreeInstances = new Map<string, Set<() => Promise<void>>>()

function buildCacheKey(projectId: string, projectPath: string): string {
  return `${projectId}:${projectPath}`
}

function createRuntimeCache(): FileTreeRuntimeCache {
  return {
    rootNodes: null,
    rootLoadedAt: 0,
    childrenByDir: new Map(),
    expandedKeys: [],
    staleDirs: new Set(),
    pendingLoads: new Map(),
    rootLoadPromise: null,
    scrollTop: 0
  }
}

function getRuntimeCache(projectId: string, projectPath: string): FileTreeRuntimeCache {
  const key = buildCacheKey(projectId, projectPath)
  const cached = projectFileTreeCaches.get(key)
  if (cached) {
    return cached
  }

  const nextCache = createRuntimeCache()
  projectFileTreeCaches.set(key, nextCache)
  return nextCache
}

function registerProjectFileTreeInstance(projectId: string, projectPath: string, handler: () => Promise<void>): () => void {
  const key = buildCacheKey(projectId, projectPath)
  const instances = projectFileTreeInstances.get(key) ?? new Set<() => Promise<void>>()
  instances.add(handler)
  projectFileTreeInstances.set(key, instances)

  return () => {
    const current = projectFileTreeInstances.get(key)
    if (!current) {
      return
    }

    current.delete(handler)
    if (current.size === 0) {
      projectFileTreeInstances.delete(key)
    }
  }
}

function clearProjectFileTreeCache(projectId: string, projectPath: string): void {
  const cache = getRuntimeCache(projectId, projectPath)
  cache.rootNodes = null
  cache.rootLoadedAt = 0
  cache.childrenByDir.clear()
  cache.staleDirs.clear()
  cache.pendingLoads.clear()
  cache.rootLoadPromise = null
}

export async function refreshProjectFileTreeView(projectId: string, projectPath: string): Promise<void> {
  clearProjectFileTreeCache(projectId, projectPath)

  try {
    await invoke<number>('warm_project_file_index', { projectPath })
  } catch (error) {
    console.error('Failed to refresh project file index:', error)
  }

  const handlers = Array.from(projectFileTreeInstances.get(buildCacheKey(projectId, projectPath)) ?? [])
  if (handlers.length === 0) {
    return
  }

  await Promise.all(handlers.map(handler => handler().catch((error) => {
    console.error('Failed to refresh file tree instance:', error)
  })))
}

function cloneTreeNodes(nodes: FileTreeNode[]): FileTreeNode[] {
  return nodes.map(node => ({
    ...node,
    children: node.children ? cloneTreeNodes(node.children) : undefined
  }))
}

async function loadProjectRootNodes(
  projectId: string,
  projectPath: string,
  force = false
): Promise<FileTreeNode[]> {
  const cache = getRuntimeCache(projectId, projectPath)

  if (!force && cache.rootNodes) {
    return cloneTreeNodes(cache.rootNodes)
  }

  if (!force && cache.rootLoadPromise) {
    return cloneTreeNodes(await cache.rootLoadPromise)
  }

  const loadPromise = invoke<FileTreeNode[]>('list_project_files', { projectPath })
  cache.rootLoadPromise = loadPromise

  try {
    const nodes = await loadPromise
    cache.rootNodes = cloneTreeNodes(nodes)
    cache.rootLoadedAt = Date.now()
    cache.staleDirs.delete(projectPath)
    return cloneTreeNodes(nodes)
  } finally {
    if (cache.rootLoadPromise === loadPromise) {
      cache.rootLoadPromise = null
    }
  }
}

export async function prewarmProjectFileTreeCache(projectId: string, projectPath: string): Promise<void> {
  const cache = getRuntimeCache(projectId, projectPath)
  if (cache.rootNodes || cache.rootLoadPromise) {
    return
  }

  try {
    await loadProjectRootNodes(projectId, projectPath)
  } catch (error) {
    console.error('Failed to prewarm project file tree:', error)
  }
}

export async function prewarmProjectFileSearchIndex(projectPath: string): Promise<void> {
  try {
    await invoke<number>('warm_project_file_index', { projectPath })
  } catch (error) {
    console.error('Failed to warm project file index:', error)
  }
}

/**
 * 文件树视图状态。
 * 负责目录懒加载、选择态、右键操作、文件监听和拖拽移动。
 */
export function useFileTree(props: FileTreeProps, emit: FileTreeEmits) {
  const { t } = useI18n()
  const { createEntry, renameFile, deleteFile, batchDeleteFiles, moveFile, loading } = useFileOperations()
  const { sendFileReferencesToSession } = useSessionFileReference()

  const rootRef = ref<HTMLElement | null>(null)
  const searchInputRef = ref<HTMLInputElement | null>(null)
  const treeData = ref<TreeOption[]>([])
  const expandedKeys = ref<string[]>([])
  const selectedPaths = ref<string[]>([])
  const activePath = ref<string | null>(null)
  const anchorPath = ref<string | null>(null)
  const contextMenuContext = ref<ContextMenuContext | null>(null)
  const renameDialogVisible = ref(false)
  const renameNode = ref<FileTreeNodeData | null>(null)
  const createDialogVisible = ref(false)
  const createTargetNode = ref<FileTreeNodeData | null>(null)
  const createEntryType = ref<CreateEntryType>('file')
  const deleteConfirmVisible = ref(false)
  const deleteNode = ref<FileTreeNodeData | null>(null)
  const batchDeleteConfirmVisible = ref(false)
  const isLoading = ref(false)
  const pendingReload = ref(false)
  const unwatchFileTree = ref<UnwatchFn | null>(null)
  const reloadTimer = ref<ReturnType<typeof setTimeout> | null>(null)
  const watcherStartTimer = ref<ReturnType<typeof setTimeout> | null>(null)
  const watcherIdleCallbackId = ref<number | null>(null)
  const activeWatchTargets = ref<string[]>([])
  const pendingRefreshDirs = ref<Set<string>>(new Set())
  const dragPaths = ref<string[]>([])
  const dragOverKey = ref<string | null>(null)
  const dragExpandTargetKey = ref<string | null>(null)
  const dragExpandTimer = ref<ReturnType<typeof setTimeout> | null>(null)
  const searchQuery = ref('')
  const searchResults = ref<ProjectFileSearchResult[]>([])
  const searchActiveIndex = ref(0)
  const isSearching = ref(false)
  const searchTimer = ref<ReturnType<typeof setTimeout> | null>(null)
  const searchRequestId = ref(0)
  let unregisterInstance: (() => void) | null = null

  const rootContextNode = computed<FileTreeNodeData>(() => ({
    key: props.projectPath,
    label: extractProjectLabel(props.projectPath),
    nodeType: 'directory',
    projectId: props.projectId,
    isLeaf: false,
    isRoot: true
  }))

  const selectedActionPaths = computed(() => dedupePaths(selectedPaths.value))
  const selectedPathSet = computed(() => new Set(selectedPaths.value))
  const normalizedSearchQuery = computed(() => searchQuery.value.trim())
  const isSearchActive = computed(() => normalizedSearchQuery.value.length > 0)

  function extractProjectLabel(projectPath: string): string {
    const normalized = projectPath.replace(/[\\/]+$/, '').replace(/\\/g, '/')
    const segments = normalized.split('/').filter(Boolean)
    return segments[segments.length - 1] || normalized || projectPath
  }

  function normalizeComparablePath(path: string): string {
    return path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()
  }

  function isAncestorPath(ancestor: string, candidate: string): boolean {
    const normalizedAncestor = normalizeComparablePath(ancestor)
    const normalizedCandidate = normalizeComparablePath(candidate)
    return normalizedCandidate.startsWith(`${normalizedAncestor}/`)
  }

  function dedupePaths(paths: string[]): string[] {
    const uniquePaths = Array.from(new Set(paths))
      .filter(Boolean)
      .sort((left, right) => normalizeComparablePath(left).length - normalizeComparablePath(right).length)

    return uniquePaths.filter((path, index) =>
      !uniquePaths.slice(0, index).some(existing => isAncestorPath(existing, path))
    )
  }

  function buildNodeData(node: TreeOption, extra: Partial<FileTreeNodeData> = {}): FileTreeNodeData {
    return {
      key: node.key as string,
      label: node.label as string,
      nodeType: (node as { nodeType?: 'file' | 'directory' }).nodeType ?? 'file',
      extension: (node as { extension?: string }).extension,
      projectId: props.projectId,
      isLeaf: Boolean(node.isLeaf),
      ...extra
    }
  }

  function resolveParentPath(node: FileTreeNodeData): string {
    if (node.isRoot || node.nodeType === 'directory') {
      return node.key
    }

    const normalized = node.key.replace(/[\\/]+$/, '')
    const separatorIndex = Math.max(normalized.lastIndexOf('/'), normalized.lastIndexOf('\\'))
    return separatorIndex >= 0 ? normalized.slice(0, separatorIndex) : props.projectPath
  }

  function resolveActionPaths(node: FileTreeNodeData): string[] {
    if (selectedActionPaths.value.length > 1 && selectedPathSet.value.has(node.key)) {
      return selectedActionPaths.value
    }

    return dedupePaths([node.key])
  }

  function currentCache(): FileTreeRuntimeCache {
    return getRuntimeCache(props.projectId, props.projectPath)
  }

  function buildWatchTargets(): string[] {
    const targets = new Set<string>([props.projectPath])

    for (const path of expandedKeys.value) {
      const node = findTreeNodeByKey(treeData.value, path)
      if (!node || (node.nodeType !== 'directory' && node.isLeaf !== false)) {
        continue
      }

      targets.add(path)
    }

    return Array.from(targets)
  }

  function hasSameWatchTargets(left: string[], right: string[]): boolean {
    if (left.length !== right.length) {
      return false
    }

    return left.every((value, index) => value === right[index])
  }

  function syncExpandedKeysToCache(): void {
    currentCache().expandedKeys = [...expandedKeys.value]
  }

  function getPathDepth(path: string): number {
    return path.split(/[\\/]/).filter(Boolean).length
  }

  function resolveDirectoryPath(path: string): string {
    const normalized = path.replace(/[\\/]+$/, '')
    if (!normalized || normalizeComparablePath(normalized) === normalizeComparablePath(props.projectPath)) {
      return props.projectPath
    }

    const targetNode = findTreeNodeByKey(treeData.value, normalized)
    if (targetNode && ((targetNode as { nodeType?: string }).nodeType === 'directory' || targetNode.isLeaf === false)) {
      return normalized
    }

    const separatorIndex = Math.max(normalized.lastIndexOf('/'), normalized.lastIndexOf('\\'))
    return separatorIndex >= 0 ? normalized.slice(0, separatorIndex) : props.projectPath
  }

  function resolveParentDirectoryPath(path: string): string {
    const normalized = path.replace(/[\\/]+$/, '')
    const separatorIndex = Math.max(normalized.lastIndexOf('/'), normalized.lastIndexOf('\\'))
    return separatorIndex >= 0 ? normalized.slice(0, separatorIndex) : props.projectPath
  }

  function buildAncestorDirectories(targetPath: string): string[] {
    const ancestors: string[] = []
    let current = resolveParentDirectoryPath(targetPath)

    while (
      current
      && normalizeComparablePath(current) !== normalizeComparablePath(props.projectPath)
      && !ancestors.some(item => normalizeComparablePath(item) === normalizeComparablePath(current))
    ) {
      ancestors.unshift(current)
      current = resolveParentDirectoryPath(current)
    }

    return ancestors
  }

  function resolveScrollElement(): HTMLElement | null {
    return rootRef.value?.querySelector('.file-tree__n-tree') as HTMLElement | null
  }

  function saveScrollPosition(): void {
    const element = resolveScrollElement()
    if (!element) {
      return
    }

    currentCache().scrollTop = element.scrollTop
  }

  function restoreScrollPosition(): void {
    nextTick(() => {
      const element = resolveScrollElement()
      if (!element) {
        return
      }

      element.scrollTop = currentCache().scrollTop
    })
  }

  function clearSearch(): void {
    searchQuery.value = ''
    searchResults.value = []
    searchActiveIndex.value = 0
    if (searchTimer.value) {
      clearTimeout(searchTimer.value)
      searchTimer.value = null
    }
  }

  function clearDragState(): void {
    dragPaths.value = []
    dragOverKey.value = null
    dragExpandTargetKey.value = null

    if (dragExpandTimer.value) {
      clearTimeout(dragExpandTimer.value)
      dragExpandTimer.value = null
    }
  }

  function clearSelectionState(): void {
    selectedPaths.value = []
    activePath.value = null
    anchorPath.value = null
  }

  function setSingleSelection(path: string): void {
    selectedPaths.value = [path]
    activePath.value = path
    anchorPath.value = path
  }

  function flattenVisiblePaths(nodes: TreeOption[]): string[] {
    const expandedSet = new Set(expandedKeys.value)
    const visiblePaths: string[] = []

    const walk = (items: TreeOption[]) => {
      items.forEach(item => {
        const path = String(item.key)
        visiblePaths.push(path)

        if (item.children?.length && expandedSet.has(path)) {
          walk(item.children as TreeOption[])
        }
      })
    }

    walk(nodes)
    return visiblePaths
  }

  function applyRangeSelection(targetPath: string): void {
    const currentAnchor = anchorPath.value
    if (!currentAnchor) {
      setSingleSelection(targetPath)
      return
    }

    const visiblePaths = flattenVisiblePaths(treeData.value)
    const anchorIndex = visiblePaths.indexOf(currentAnchor)
    const targetIndex = visiblePaths.indexOf(targetPath)

    if (anchorIndex === -1 || targetIndex === -1) {
      setSingleSelection(targetPath)
      return
    }

    const [start, end] = anchorIndex <= targetIndex
      ? [anchorIndex, targetIndex]
      : [targetIndex, anchorIndex]

    selectedPaths.value = visiblePaths.slice(start, end + 1)
    activePath.value = targetPath
  }

  function openFileIfNeeded(node: FileTreeNodeData): void {
    if (node.nodeType !== 'file') {
      return
    }

    emit('fileSelect', node.key)
  }

  async function loadTreeData(options: LoadTreeDataOptions = {}) {
    if (isLoading.value) {
      pendingReload.value = true
      return
    }

    isLoading.value = true
    try {
      const cache = currentCache()
      if (options.resetChildCache) {
        cache.childrenByDir.clear()
        cache.staleDirs.clear()
      }

      const shouldForce = Boolean(options.force || options.resetChildCache)
      const result = await loadProjectRootNodes(props.projectId, props.projectPath, shouldForce)
      cache.rootNodes = cloneTreeNodes(result)
      treeData.value = convertToTreeOptions(result)
      if (cache.expandedKeys.length > 0 && expandedKeys.value.length === 0) {
        expandedKeys.value = [...cache.expandedKeys]
      }
      await restoreExpandedDirectories()
      syncExpandedKeysToCache()
      restoreScrollPosition()

      selectedPaths.value = selectedPaths.value.filter(path => !!findTreeNodeByKey(treeData.value, path))

      if (activePath.value && !findTreeNodeByKey(treeData.value, activePath.value)) {
        activePath.value = null
      }

      if (anchorPath.value && !findTreeNodeByKey(treeData.value, anchorPath.value)) {
        anchorPath.value = null
      }
    } catch (error) {
      console.error('Failed to load file tree:', error)
    } finally {
      isLoading.value = false

      if (pendingReload.value) {
        pendingReload.value = false
        await loadTreeData({ force: true })
      }
    }
  }

  async function restoreExpandedDirectories() {
    if (expandedKeys.value.length === 0) {
      return
    }

    const sortedExpandedKeys = [...expandedKeys.value].sort((left, right) => getPathDepth(left) - getPathDepth(right))

    for (const key of sortedExpandedKeys) {
      const node = findTreeNodeByKey(treeData.value, key)
      if (!node || (node.nodeType !== 'directory' && node.isLeaf !== false)) {
        continue
      }

      try {
        await loadChildrenForNode(node)
      } catch (error) {
        console.error('Failed to restore expanded directory:', error)
      }
    }
  }

  function queueDirectoryRefresh(dirPath: string): void {
    pendingRefreshDirs.value.add(dirPath)

    if (reloadTimer.value) {
      clearTimeout(reloadTimer.value)
    }

    reloadTimer.value = setTimeout(() => {
      reloadTimer.value = null
      const targetDirs = Array.from(pendingRefreshDirs.value)
      pendingRefreshDirs.value.clear()
      void refreshDirectories(targetDirs)
    }, TREE_RELOAD_DEBOUNCE_MS)
  }

  function stopFileWatcher() {
    if (watcherIdleCallbackId.value !== null && typeof window !== 'undefined' && typeof window.cancelIdleCallback === 'function') {
      window.cancelIdleCallback(watcherIdleCallbackId.value)
      watcherIdleCallbackId.value = null
    }

    if (watcherStartTimer.value) {
      clearTimeout(watcherStartTimer.value)
      watcherStartTimer.value = null
    }

    if (reloadTimer.value) {
      clearTimeout(reloadTimer.value)
      reloadTimer.value = null
    }

    if (unwatchFileTree.value) {
      unwatchFileTree.value()
      unwatchFileTree.value = null
    }

    activeWatchTargets.value = []
  }

  async function startFileWatcher(paths: string[]) {
    const normalizedTargets = Array.from(new Set(paths.filter(Boolean))).sort()
    if (normalizedTargets.length === 0) {
      return
    }

    if (hasSameWatchTargets(activeWatchTargets.value, normalizedTargets) && unwatchFileTree.value) {
      return
    }

    if (unwatchFileTree.value) {
      unwatchFileTree.value()
      unwatchFileTree.value = null
    }

    try {
      unwatchFileTree.value = await startFsWatcher(
        normalizedTargets,
        (event) => {
          const eventType = event.type
          if (typeof eventType === 'object' && 'access' in eventType) {
            return
          }

          const affectedDirs = event.paths
            .flatMap(path => [resolveDirectoryPath(path), resolveParentDirectoryPath(path)])
            .filter(Boolean)

          if (affectedDirs.length === 0) {
            queueDirectoryRefresh(props.projectPath)
            return
          }

          affectedDirs.forEach(queueDirectoryRefresh)
        },
        {
          recursive: false,
          delayMs: 300
        }
      )
      activeWatchTargets.value = normalizedTargets
    } catch (error) {
      console.error('Failed to watch project directory:', error)
    }
  }

  function scheduleFileWatcherStart(): void {
    if (watcherIdleCallbackId.value !== null && typeof window !== 'undefined' && typeof window.cancelIdleCallback === 'function') {
      window.cancelIdleCallback(watcherIdleCallbackId.value)
      watcherIdleCallbackId.value = null
    }

    if (watcherStartTimer.value) {
      clearTimeout(watcherStartTimer.value)
      watcherStartTimer.value = null
    }

    const targets = buildWatchTargets()

    const run = () => {
      watcherStartTimer.value = null
      watcherIdleCallbackId.value = null
      void startFileWatcher(targets)
    }

    if (typeof window !== 'undefined' && typeof window.requestIdleCallback === 'function') {
      watcherIdleCallbackId.value = window.requestIdleCallback(() => run(), { timeout: 1200 })
      return
    }

    watcherStartTimer.value = setTimeout(run, 420)
  }

  function convertToTreeOptions(nodes: FileTreeNode[]): TreeOption[] {
    const cache = currentCache()

    return nodes.map(node => {
      const isFile = node.nodeType === 'file'
      const option: TreeOption = {
        key: node.path,
        label: node.name,
        isLeaf: isFile,
        nodeType: node.nodeType,
        extension: node.extension,
        projectId: props.projectId
      }

      if (!isFile) {
        const cachedChildren = cache.childrenByDir.get(node.path)
        option.children = cachedChildren ? convertToTreeOptions(cachedChildren) : []
      }

      return option
    })
  }

  function findTreeNodeByKey(nodes: TreeOption[], key: string): (TreeOption & { nodeType?: string }) | null {
    for (const node of nodes) {
      if (String(node.key) === key) {
        return node as TreeOption & { nodeType?: string }
      }
      if (node.children?.length) {
        const matched = findTreeNodeByKey(node.children as TreeOption[], key)
        if (matched) {
          return matched
        }
      }
    }

    return null
  }

  async function loadChildrenForNode(node: TreeOption, forceRefresh = false): Promise<void> {
    const nodePath = node.key as string
    const cache = currentCache()
    const isStale = cache.staleDirs.has(nodePath)
    const cachedChildren = cache.childrenByDir.get(nodePath)

    let children = cachedChildren

    if (forceRefresh || isStale || !cachedChildren) {
      let pendingLoad = cache.pendingLoads.get(nodePath)
      if (!pendingLoad) {
        pendingLoad = invoke<FileTreeNode[]>('load_directory_children', { dirPath: nodePath })
        cache.pendingLoads.set(nodePath, pendingLoad)
      }

      try {
        children = await pendingLoad
        cache.childrenByDir.set(nodePath, cloneTreeNodes(children))
        cache.staleDirs.delete(nodePath)
      } finally {
        if (cache.pendingLoads.get(nodePath) === pendingLoad) {
          cache.pendingLoads.delete(nodePath)
        }
      }
    }

    node.children = (children ?? []).map(child => {
      const isFile = child.nodeType === 'file'
      const option: TreeOption = {
        key: child.path,
        label: child.name,
        isLeaf: isFile,
        nodeType: child.nodeType,
        extension: child.extension,
        projectId: props.projectId
      }
      if (!isFile) {
        option.children = []
      }
      return option
    })
  }

  async function ensureExpanded(path: string): Promise<void> {
    if (expandedKeys.value.includes(path)) {
      return
    }

    expandedKeys.value = [...expandedKeys.value, path]
    syncExpandedKeysToCache()
    const node = findTreeNodeByKey(treeData.value, path)
    if (!node || node.nodeType !== 'directory') {
      return
    }

    try {
      await loadChildrenForNode(node)
      scheduleFileWatcherStart()
    } catch (error) {
      console.error('Failed to load node children on auto expand:', error)
    }
  }

  async function handleExpandChange(keys: string[]) {
    const previousKeys = new Set(expandedKeys.value)
    expandedKeys.value = keys
    syncExpandedKeysToCache()

    const justExpandedKeys = keys.filter(key => !previousKeys.has(key))
    if (justExpandedKeys.length === 0) {
      scheduleFileWatcherStart()
      return
    }

    const targetNodes = justExpandedKeys
      .map(key => findTreeNodeByKey(treeData.value, key))
      .filter((node): node is TreeOption & { nodeType?: string } => !!node)
      .filter(node => node.nodeType === 'directory' || node.isLeaf === false)

    await Promise.all(targetNodes.map(async node => {
      try {
        await loadChildrenForNode(node)
      } catch (error) {
        console.error('Failed to refresh node children on expand:', error)
      }
    }))

    scheduleFileWatcherStart()
  }

  async function refreshDirectories(dirPaths: string[]): Promise<void> {
    const cache = currentCache()
    const uniqueDirs = Array.from(new Set(dirPaths.map(resolveDirectoryPath))).filter(Boolean)

    if (uniqueDirs.length === 0) {
      return
    }

    const shouldRefreshRoot = uniqueDirs.some(dir => normalizeComparablePath(dir) === normalizeComparablePath(props.projectPath))
    uniqueDirs.forEach(dir => cache.staleDirs.add(dir))

    if (shouldRefreshRoot) {
      await loadTreeData({ force: true })
      return
    }

    await Promise.all(uniqueDirs.map(async dirPath => {
      const node = findTreeNodeByKey(treeData.value, dirPath)
      if (!node || !expandedKeys.value.includes(dirPath)) {
        return
      }

      try {
        await loadChildrenForNode(node, true)
      } catch (error) {
        console.error('Failed to refresh directory children:', error)
      }
    }))

    treeData.value = convertToTreeOptions(cache.rootNodes ?? [])
    restoreScrollPosition()
  }

  async function hardRefreshTree(): Promise<void> {
    clearSearch()
    await loadTreeData({ resetChildCache: true, force: true })
  }

  function handleNodeClick(event: MouseEvent, node: TreeOption) {
    rootRef.value?.focus()
    const nodeData = buildNodeData(node)

    if (event.shiftKey) {
      applyRangeSelection(nodeData.key)
    } else {
      if (selectedPaths.value.length === 1 && activePath.value === nodeData.key) {
        clearSelectionState()
        return
      }

      setSingleSelection(nodeData.key)
    }

    openFileIfNeeded(nodeData)
  }

  function handleContextMenu(event: MouseEvent, node: TreeOption) {
    event.preventDefault()
    event.stopPropagation()
    rootRef.value?.focus()

    const nodePath = String(node.key)
    if (!selectedPathSet.value.has(nodePath)) {
      setSingleSelection(nodePath)
    } else {
      activePath.value = nodePath
    }

    contextMenuContext.value = {
      node: buildNodeData(node),
      position: { x: event.clientX, y: event.clientY }
    }
  }

  function handleTreeRootContextMenu(event: MouseEvent) {
    const target = event.target
    if (target instanceof Element && target.closest('.n-tree-node')) {
      return
    }

    event.preventDefault()
    event.stopPropagation()
    rootRef.value?.focus()
    contextMenuContext.value = {
      node: rootContextNode.value,
      position: { x: event.clientX, y: event.clientY }
    }
  }

  function handleRootClick(event: MouseEvent) {
    const target = event.target
    if (target instanceof Element && target.closest('.file-tree__search')) {
      closeContextMenu()
      return
    }

    rootRef.value?.focus()

    if (target instanceof Element && target.closest('.n-tree-node')) {
      return
    }

    clearSelectionState()
    closeContextMenu()
  }

  function closeContextMenu() {
    contextMenuContext.value = null
  }

  function handleRename(node: FileTreeNodeData) {
    renameNode.value = node
    renameDialogVisible.value = true
  }

  async function confirmRename(oldPath: string, newName: string) {
    const result = await renameFile(oldPath, newName)
    if (result?.success) {
      await refreshDirectories([resolveParentDirectoryPath(oldPath), resolveParentDirectoryPath(result.newPath ?? oldPath)])
    }
  }

  function handleCreateFile(node: FileTreeNodeData) {
    createTargetNode.value = node
    createEntryType.value = 'file'
    createDialogVisible.value = true
  }

  function handleCreateFolder(node: FileTreeNodeData) {
    createTargetNode.value = node
    createEntryType.value = 'directory'
    createDialogVisible.value = true
  }

  async function confirmCreate(node: FileTreeNodeData, name: string, entryType: CreateEntryType) {
    const parentPath = resolveParentPath(node)
    const result = await createEntry({
      parentPath,
      name,
      entryType
    })

    if (result?.success) {
      if (!node.isRoot && node.nodeType === 'directory') {
        await ensureExpanded(node.key)
      }
      await refreshDirectories([parentPath])
    }
  }

  function handleDelete(node: FileTreeNodeData) {
    const actionPaths = resolveActionPaths(node)
    if (actionPaths.length > 1) {
      batchDeleteConfirmVisible.value = true
      return
    }

    deleteNode.value = node
    deleteConfirmVisible.value = true
  }

  function handleDeleteSelection() {
    const actionPaths = selectedActionPaths.value
    if (actionPaths.length === 0) {
      return
    }

    if (actionPaths.length > 1) {
      batchDeleteConfirmVisible.value = true
      return
    }

    const targetNode = findTreeNodeByKey(treeData.value, actionPaths[0])
    if (!targetNode) {
      return
    }

    deleteNode.value = buildNodeData(targetNode)
    deleteConfirmVisible.value = true
  }

  async function confirmDelete() {
    if (!deleteNode.value) {
      return
    }

    const targetPath = deleteNode.value.key
    const result = await deleteFile(targetPath)
    if (result?.success) {
      selectedPaths.value = selectedPaths.value.filter(path => path !== targetPath)
      if (activePath.value === targetPath) {
        activePath.value = null
      }
      if (anchorPath.value === targetPath) {
        anchorPath.value = null
      }
      await refreshDirectories([resolveParentDirectoryPath(targetPath)])
    }

    deleteConfirmVisible.value = false
    deleteNode.value = null
  }

  async function confirmBatchDelete() {
    const paths = selectedActionPaths.value
    const result = await batchDeleteFiles(paths)
    if (result?.success) {
      clearSelectionState()
      await refreshDirectories(paths.map(resolveParentDirectoryPath))
    }

    batchDeleteConfirmVisible.value = false
  }

  function canDropIntoTarget(targetPath: string): boolean {
    const sourcePaths = dragPaths.value.length > 0 ? dragPaths.value : selectedActionPaths.value
    if (sourcePaths.length === 0) {
      return true
    }

    return sourcePaths.every(path => path !== targetPath && !isAncestorPath(path, targetPath))
  }

  function allowDrop(info: { dropPosition: 'inside' | 'before' | 'after'; node: TreeOption }) {
    if (info.dropPosition !== 'inside') {
      return false
    }

    const nodeType = (info.node as { nodeType?: string }).nodeType
    if (nodeType !== 'directory') {
      return false
    }

    return canDropIntoTarget(String(info.node.key))
  }

  function scheduleDragAutoExpand(node: TreeOption): void {
    const nodePath = String(node.key)
    if (
      dragExpandTargetKey.value === nodePath
      || expandedKeys.value.includes(nodePath)
      || (node as { nodeType?: string }).nodeType !== 'directory'
    ) {
      return
    }

    if (dragExpandTimer.value) {
      clearTimeout(dragExpandTimer.value)
    }

    dragExpandTargetKey.value = nodePath
    dragExpandTimer.value = setTimeout(() => {
      dragExpandTimer.value = null
      void ensureExpanded(nodePath)
    }, DRAG_AUTO_EXPAND_DELAY_MS)
  }

  function handleTreeDragStart(info: { event: DragEvent; node: TreeOption }) {
    const draggedPath = String(info.node.key)
    if (selectedPathSet.value.has(draggedPath)) {
      dragPaths.value = selectedActionPaths.value
    } else {
      setSingleSelection(draggedPath)
      dragPaths.value = [draggedPath]
    }

    if (info.event.dataTransfer) {
      info.event.dataTransfer.effectAllowed = 'move'
      info.event.dataTransfer.setData('text/plain', draggedPath)
    }
  }

  function handleTreeDragOver(info: { event: DragEvent; node: TreeOption }) {
    const nodeType = (info.node as { nodeType?: string }).nodeType
    const targetPath = String(info.node.key)
    if (nodeType !== 'directory' || !canDropIntoTarget(targetPath)) {
      dragOverKey.value = null
      return
    }

    if (info.event.dataTransfer) {
      info.event.dataTransfer.dropEffect = 'move'
    }

    dragOverKey.value = targetPath
    scheduleDragAutoExpand(info.node)
  }

  function handleTreeDragLeave(info: { node: TreeOption }) {
    const targetPath = String(info.node.key)
    if (dragOverKey.value === targetPath) {
      dragOverKey.value = null
    }

    if (dragExpandTargetKey.value === targetPath && dragExpandTimer.value) {
      clearTimeout(dragExpandTimer.value)
      dragExpandTimer.value = null
      dragExpandTargetKey.value = null
    }
  }

  function handleTreeDragEnd() {
    clearDragState()
  }

  async function handleDrop(info: { node: TreeOption; dragNode: TreeOption; dropPosition: 'inside' | 'before' | 'after' }) {
    const { node, dragNode, dropPosition } = info
    if (dropPosition !== 'inside') {
      clearDragState()
      return
    }

    const draggedPath = String(dragNode.key)
    const targetPath = String(node.key)
    const sourcePaths = dragPaths.value.length > 0
      ? dragPaths.value
      : (selectedActionPaths.value.includes(draggedPath) ? selectedActionPaths.value : [draggedPath])
    const movablePaths = dedupePaths(sourcePaths).filter(path =>
      path !== targetPath && !isAncestorPath(path, targetPath)
    )

    if (movablePaths.length === 0) {
      clearDragState()
      return
    }

    if (!expandedKeys.value.includes(targetPath)) {
      expandedKeys.value = [...expandedKeys.value, targetPath]
    }

    const results = []
    for (const sourcePath of movablePaths) {
      results.push(await moveFile(sourcePath, targetPath))
    }

    clearDragState()

    if (results.every(result => result?.success)) {
      clearSelectionState()
      await refreshDirectories([...movablePaths.map(resolveParentDirectoryPath), targetPath])
    }
  }

  async function handleSendToSession(node: FileTreeNodeData) {
    const mentions = resolveActionPaths(node).map(path => createComposerFileMention({ fullPath: path }))
    await sendFileReferencesToSession({
      sourceProjectId: props.projectId,
      mentions
    })
  }

  async function revealPath(targetPath: string): Promise<void> {
    const ancestorDirs = buildAncestorDirectories(targetPath)
    for (const dirPath of ancestorDirs) {
      await ensureExpanded(dirPath)
    }

    setSingleSelection(targetPath)
    saveScrollPosition()
  }

  async function selectSearchResult(result: ProjectFileSearchResult): Promise<void> {
    clearSearch()
    await revealPath(result.path)

    if (result.nodeType === 'directory') {
      await ensureExpanded(result.path)
      return
    }

    emit('fileSelect', result.path)
  }

  async function runFileSearch(query: string, requestId: number): Promise<void> {
    if (!query) {
      searchResults.value = []
      searchActiveIndex.value = 0
      isSearching.value = false
      return
    }

    isSearching.value = true
    try {
      const results = await invoke<ProjectFileSearchResult[]>('search_file_mentions', {
        input: {
          query,
          scope: 'project',
          projectPath: props.projectPath,
          limit: 60
        }
      })

      if (requestId !== searchRequestId.value) {
        return
      }

      searchResults.value = results
      searchActiveIndex.value = 0
    } catch (error) {
      if (requestId !== searchRequestId.value) {
        return
      }

      console.error('Failed to search project files:', error)
      searchResults.value = []
    } finally {
      if (requestId === searchRequestId.value) {
        isSearching.value = false
      }
    }
  }

  function handleSearchInput(value: string): void {
    searchQuery.value = value

    if (searchTimer.value) {
      clearTimeout(searchTimer.value)
      searchTimer.value = null
    }

    const nextQuery = value.trim()
    const requestId = searchRequestId.value + 1
    searchRequestId.value = requestId

    if (!nextQuery) {
      searchResults.value = []
      searchActiveIndex.value = 0
      isSearching.value = false
      return
    }

    searchTimer.value = setTimeout(() => {
      searchTimer.value = null
      void runFileSearch(nextQuery, requestId)
    }, 120)
  }

  function handleSearchInputEvent(event: Event): void {
    const target = event.target
    handleSearchInput(target instanceof HTMLInputElement ? target.value : '')
  }

  async function handleSearchKeydown(event: KeyboardEvent): Promise<void> {
    if (!isSearchActive.value || searchResults.value.length === 0) {
      if (event.key === 'Escape') {
        clearSearch()
        rootRef.value?.focus()
      }
      return
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault()
      searchActiveIndex.value = (searchActiveIndex.value + 1) % searchResults.value.length
      return
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault()
      searchActiveIndex.value = (searchActiveIndex.value - 1 + searchResults.value.length) % searchResults.value.length
      return
    }

    if (event.key === 'Enter') {
      event.preventDefault()
      const activeResult = searchResults.value[searchActiveIndex.value] ?? searchResults.value[0]
      if (activeResult) {
        await selectSearchResult(activeResult)
      }
      return
    }

    if (event.key === 'Escape') {
      event.preventDefault()
      clearSearch()
      rootRef.value?.focus()
    }
  }

  function renderLabel({ option }: { option: TreeOption }) {
    const nodeType = (option as { nodeType?: string }).nodeType ?? 'file'
    const fileName = option.label as string
    const extension = (option as { extension?: string }).extension
    const iconMeta = resolveFileIcon(nodeType, fileName, extension)

    return h('div', {
      class: 'file-tree-node__content',
      onClick: (event: MouseEvent) => handleNodeClick(event, option),
      onContextmenu: (event: MouseEvent) => handleContextMenu(event, option)
    }, [
      h(EaIcon, {
        name: iconMeta.icon,
        size: 14,
        class: 'file-tree-node__icon',
        style: { color: iconMeta.color }
      }),
      h('span', { class: 'file-tree-node__name' }, fileName)
    ])
  }

  function resolveNodeProps({ option }: { option: TreeOption }) {
    const nodePath = String(option.key)

    return {
      class: [
        'file-tree-node',
        selectedPathSet.value.has(nodePath) && 'file-tree-node--selected',
        activePath.value === nodePath && 'file-tree-node--active',
        dragOverKey.value === nodePath && 'file-tree-node--drop-target'
      ]
    }
  }

  function handleClickOutside() {
    closeContextMenu()
  }

  function handleDocumentKeydown(event: KeyboardEvent) {
    const root = rootRef.value
    const target = event.target
    if (!root || !(target instanceof Node)) {
      return
    }

    if (!root.contains(target) && document.activeElement !== root) {
      return
    }

    if (
      target instanceof HTMLInputElement
      || target instanceof HTMLTextAreaElement
      || (target instanceof HTMLElement && target.isContentEditable)
    ) {
      return
    }

    if (event.key === 'Escape') {
      closeContextMenu()
      return
    }

    if (event.key !== 'Delete' && event.key !== 'Backspace') {
      return
    }

    if (selectedActionPaths.value.length > 0) {
      event.preventDefault()
      handleDeleteSelection()
      return
    }

    if (!activePath.value) {
      return
    }

    const selectedNode = findTreeNodeByKey(treeData.value, activePath.value)
    if (!selectedNode) {
      return
    }

    event.preventDefault()
    handleDelete(buildNodeData(selectedNode))
  }

  function hydrateFromCache(): void {
    const cache = currentCache()
    expandedKeys.value = [...cache.expandedKeys]
    treeData.value = cache.rootNodes ? convertToTreeOptions(cache.rootNodes) : []
  }

  function handleTreeScroll(): void {
    saveScrollPosition()
  }

  onMounted(() => {
    unregisterInstance = registerProjectFileTreeInstance(props.projectId, props.projectPath, hardRefreshTree)
    document.addEventListener('click', handleClickOutside)
    document.addEventListener('keydown', handleDocumentKeydown)
    hydrateFromCache()
    nextTick(() => {
      resolveScrollElement()?.addEventListener('scroll', handleTreeScroll, { passive: true })
      restoreScrollPosition()
    })
    void loadTreeData({ force: treeData.value.length === 0 })
    scheduleFileWatcherStart()
  })

  onUnmounted(() => {
    unregisterInstance?.()
    unregisterInstance = null
    document.removeEventListener('click', handleClickOutside)
    document.removeEventListener('keydown', handleDocumentKeydown)
    resolveScrollElement()?.removeEventListener('scroll', handleTreeScroll)
    if (searchTimer.value) {
      clearTimeout(searchTimer.value)
      searchTimer.value = null
    }
    saveScrollPosition()
    stopFileWatcher()
    clearDragState()
  })

  watch(
    () => [props.projectId, props.projectPath] as const,
    async ([nextProjectId, nextProjectPath], [previousProjectId, previousProjectPath]) => {
      if (nextProjectId === previousProjectId && nextProjectPath === previousProjectPath) {
        return
      }

      const scrollElement = resolveScrollElement()
      if (scrollElement) {
        getRuntimeCache(previousProjectId, previousProjectPath).scrollTop = scrollElement.scrollTop
      }
      clearSelectionState()
      closeContextMenu()
      clearDragState()
      clearSearch()
      unregisterInstance?.()
      unregisterInstance = registerProjectFileTreeInstance(nextProjectId, nextProjectPath, hardRefreshTree)
      hydrateFromCache()
      await loadTreeData({ force: treeData.value.length === 0 })
      scheduleFileWatcherStart()
    }
  )

  return {
    t,
    loading,
    rootRef,
    treeData,
    expandedKeys,
    selectedPaths,
    contextMenuContext,
    renameDialogVisible,
    renameNode,
    createDialogVisible,
    createTargetNode,
    createEntryType,
    deleteConfirmVisible,
    deleteNode,
    batchDeleteConfirmVisible,
    isLoading,
    selectedActionPaths,
    handleExpandChange,
    handleTreeRootContextMenu,
    handleRootClick,
    closeContextMenu,
    handleRename,
    confirmRename,
    handleCreateFile,
    handleCreateFolder,
    confirmCreate,
    handleDelete,
    handleDeleteSelection,
    confirmDelete,
    confirmBatchDelete,
    allowDrop,
    handleTreeDragStart,
    handleTreeDragOver,
    handleTreeDragLeave,
    handleTreeDragEnd,
    handleDrop,
    handleSendToSession,
    searchInputRef,
    searchQuery,
    searchResults,
    searchActiveIndex,
    isSearchActive,
    isSearching,
    handleSearchInput,
    handleSearchInputEvent,
    handleSearchKeydown,
    selectSearchResult,
    clearSearch,
    renderLabel,
    resolveNodeProps
  }
}
