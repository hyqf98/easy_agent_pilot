<script setup lang="ts">
/**
 * 文件树核心组件
 * 支持拖拽移动、复选框多选、右键菜单、懒加载、重命名
 */

import { ref, h, watch, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { NTree, TreeOption } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import type { UnwatchFn } from '@tauri-apps/plugin-fs'
import { EaIcon, EaButton } from '@/components/common'
import type { FileTreeNode } from '@/stores/project'
import { resolveFileIcon } from '@/utils/fileIcon'
import { startFsWatcher } from '@/utils/fsWatcher'
import { useFileOperations } from './composables/useFileOperations'
import FileTreeContextMenu from './FileTreeContextMenu.vue'
import FileTreeRenameDialog from './FileTreeRenameDialog.vue'
import type { FileTreeNodeData, ContextMenuContext } from './types'

const { t } = useI18n()
const { renameFile, deleteFile, batchDeleteFiles, moveFile, loading } = useFileOperations()

interface Props {
  projectId: string
  projectPath: string
}

const props = defineProps<Props>()

const emit = defineEmits<{
  fileSelect: [path: string]
}>()

/// 树数据
const treeData = ref<TreeOption[]>([])

/// 展开的节点 keys
const expandedKeys = ref<string[]>([])

/// 选中的节点 keys（复选框）
const checkedKeys = ref<string[]>([])

/// 当前选中的节点（单选）
const selectedKey = ref<string | null>(null)

/// 右键菜单上下文
const contextMenuContext = ref<ContextMenuContext | null>(null)

/// 重命名对话框
const renameDialogVisible = ref(false)
const renameNode = ref<FileTreeNodeData | null>(null)

/// 删除确认对话框
const deleteConfirmVisible = ref(false)
const deleteNode = ref<FileTreeNodeData | null>(null)

/// 批量删除确认对话框
const batchDeleteConfirmVisible = ref(false)

/// 加载状态
const isLoading = ref(false)
const pendingReload = ref(false)
const unwatchFileTree = ref<UnwatchFn | null>(null)
const reloadTimer = ref<ReturnType<typeof setTimeout> | null>(null)

/// 加载根目录文件树
const loadTreeData = async () => {
  if (isLoading.value) {
    pendingReload.value = true
    return
  }

  isLoading.value = true
  try {
    const result = await invoke<FileTreeNode[]>('list_project_files', {
      projectPath: props.projectPath
    })
    treeData.value = convertToTreeOptions(result, props.projectId)
  } catch (error) {
    console.error('Failed to load file tree:', error)
  } finally {
    isLoading.value = false

    if (pendingReload.value) {
      pendingReload.value = false
      await loadTreeData()
    }
  }
}

const scheduleTreeReload = () => {
  if (reloadTimer.value) {
    clearTimeout(reloadTimer.value)
  }

  reloadTimer.value = setTimeout(() => {
    reloadTimer.value = null
    void loadTreeData()
  }, 250)
}

const stopFileWatcher = () => {
  if (reloadTimer.value) {
    clearTimeout(reloadTimer.value)
    reloadTimer.value = null
  }

  if (unwatchFileTree.value) {
    unwatchFileTree.value()
    unwatchFileTree.value = null
  }
}

const startFileWatcher = async (projectPath: string) => {
  stopFileWatcher()

  try {
    unwatchFileTree.value = await startFsWatcher(
      projectPath,
      () => {
        scheduleTreeReload()
      },
      {
        recursive: true,
        delayMs: 300
      }
    )
  } catch (error) {
    console.error('Failed to watch project directory:', error)
  }
}

/// 将 FileTreeNode 转换为 Naive UI TreeOption
const convertToTreeOptions = (nodes: FileTreeNode[], projectId: string): TreeOption[] => {
  return nodes.map(node => {
    const isFile = node.nodeType === 'file'
    const option: TreeOption = {
      key: node.path,
      label: node.name,
      isLeaf: isFile,
      nodeType: node.nodeType,
      extension: node.extension,
      projectId
    }
    if (!isFile) {
      // 目录节点默认给空 children，避免 Tree 组件进入无 on-load 的加载态
      option.children = []
    }
    return option as TreeOption
  })
}

const findTreeNodeByKey = (nodes: TreeOption[], key: string): (TreeOption & { nodeType?: string }) | null => {
  for (const node of nodes) {
    if (String(node.key) === key) {
      return node as (TreeOption & { nodeType?: string })
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

const loadChildrenForNode = async (node: TreeOption): Promise<void> => {
  const nodePath = node.key as string
  const children = await invoke<FileTreeNode[]>('load_directory_children', {
    dirPath: nodePath
  })

  const newChildren = children.map(child => {
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

  node.children = newChildren
}

/// 处理展开状态变化
const handleExpandChange = async (keys: string[]) => {
  const previousKeys = new Set(expandedKeys.value)
  expandedKeys.value = keys

  const justExpandedKeys = keys.filter(key => !previousKeys.has(key))
  if (justExpandedKeys.length === 0) {
    return
  }

  const targetNodes = justExpandedKeys
    .map(key => findTreeNodeByKey(treeData.value, key))
    .filter((node): node is TreeOption & { nodeType?: string } => !!node)
    .filter(node => node.nodeType === 'directory' || node.isLeaf === false)

  await Promise.all(targetNodes.map(async node => {
    try {
      // 每次展开目录都实时查询，避免懒加载结果缓存
      await loadChildrenForNode(node)
    } catch (error) {
      console.error('Failed to refresh node children on expand:', error)
    }
  }))
}

/// 处理复选框变化
const handleCheckChange = (keys: string[]) => {
  checkedKeys.value = keys
}

/// 处理选中变化
const handleSelectChange = (keys: Array<string | number>, options: Array<TreeOption | null>) => {
  if (keys.length === 0) {
    selectedKey.value = null
    return
  }

  const selectedPath = String(keys[0])
  selectedKey.value = selectedPath

  const selectedNode = (options[0] ?? null) as (TreeOption & { nodeType?: string }) | null
  if (selectedNode?.nodeType !== 'file' && selectedNode?.isLeaf === false) {
    return
  }

  emit('fileSelect', selectedPath)
}

/// 处理右键菜单
const handleContextMenu = (e: MouseEvent, node: TreeOption) => {
  e.preventDefault()
  e.stopPropagation()

  const nodeData: FileTreeNodeData = {
    key: node.key as string,
    label: node.label as string,
    nodeType: (node as any).nodeType as 'file' | 'directory',
    extension: (node as any).extension,
    projectId: props.projectId,
    isLeaf: node.isLeaf || false
  }

  contextMenuContext.value = {
    node: nodeData,
    position: { x: e.clientX, y: e.clientY }
  }
}

/// 关闭右键菜单
const closeContextMenu = () => {
  contextMenuContext.value = null
}

/// 处理重命名（从右键菜单触发）
const handleRename = (node: FileTreeNodeData) => {
  renameNode.value = node
  renameDialogVisible.value = true
}

/// 确认重命名
const confirmRename = async (oldPath: string, newName: string) => {
  const result = await renameFile(oldPath, newName)
  if (result?.success) {
    await loadTreeData()
  }
}

/// 处理删除（从右键菜单触发）
const handleDelete = (node: FileTreeNodeData) => {
  deleteNode.value = node
  deleteConfirmVisible.value = true
}

/// 确认删除单个文件
const confirmDelete = async () => {
  if (!deleteNode.value) return

  const result = await deleteFile(deleteNode.value.key)
  if (result?.success) {
    await loadTreeData()
  }
  deleteConfirmVisible.value = false
  deleteNode.value = null
}

/// 处理批量删除
const handleBatchDelete = () => {
  if (checkedKeys.value.length === 0) return
  batchDeleteConfirmVisible.value = true
}

/// 确认批量删除
const confirmBatchDelete = async () => {
  const result = await batchDeleteFiles(checkedKeys.value)
  if (result?.success) {
    checkedKeys.value = []
    await loadTreeData()
  }
  batchDeleteConfirmVisible.value = false
}

/// 清空选择
const clearSelection = () => {
  checkedKeys.value = []
}

/// 拖拽允许放置判断
const allowDrop = (info: { dropPosition: 'inside' | 'before' | 'after'; node: TreeOption; phase: 'drag' | 'drop' }) => {
  // 只允许放入目录内部
  if (info.dropPosition !== 'inside') {
    return false
  }
  // 只允许放入目录节点
  const nodeType = (info.node as any).nodeType as string
  return nodeType === 'directory'
}

/// 处理拖拽放置
const handleDrop = async (info: { node: TreeOption; dragNode: TreeOption; dropPosition: 'inside' | 'before' | 'after' }) => {
  const { node, dragNode, dropPosition } = info

  // 只处理放入目录内部的情况
  if (dropPosition !== 'inside') return

  const sourcePath = dragNode.key as string
  const targetPath = node.key as string

  const result = await moveFile(sourcePath, targetPath)
  if (result?.success) {
    await loadTreeData()
  }
}

/// 自定义节点渲染
const renderLabel = ({ option }: { option: TreeOption }) => {
  const nodeType = (option as any).nodeType as string
  const fileName = option.label as string
  const extension = (option as any).extension as string | undefined
  const iconMeta = resolveFileIcon(nodeType, fileName, extension)

  return h('div', {
    class: 'file-tree-node__content',
    onContextmenu: (e: MouseEvent) => handleContextMenu(e, option)
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

/// 点击空白处关闭右键菜单
const handleClickOutside = () => {
  closeContextMenu()
}

/// 监听点击事件关闭右键菜单
onMounted(() => {
  document.addEventListener('click', handleClickOutside)
  void loadTreeData()
  void startFileWatcher(props.projectPath)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  stopFileWatcher()
})

/// 监听项目路径变化重新加载
watch(() => props.projectPath, async (newPath, oldPath) => {
  if (newPath === oldPath) {
    return
  }

  await loadTreeData()
  await startFileWatcher(newPath)
})
</script>

<template>
  <div class="file-tree">
    <!-- 批量操作工具栏 -->
    <div
      v-if="checkedKeys.length > 0"
      class="file-tree__toolbar"
    >
      <span class="file-tree__toolbar-text">
        {{ t('fileTree.selectedCount', { count: checkedKeys.length }) }}
      </span>
      <EaButton
        type="danger"
        size="small"
        :loading="loading"
        @click="handleBatchDelete"
      >
        <EaIcon
          name="trash-2"
          :size="14"
        />
        {{ t('common.batchDelete') }}
      </EaButton>
      <EaButton
        type="secondary"
        size="small"
        @click="clearSelection"
      >
        {{ t('common.clearSelection') }}
      </EaButton>
    </div>

    <!-- 文件树 -->
    <n-tree
      :data="treeData"
      :expanded-keys="expandedKeys"
      :checked-keys="checkedKeys"
      :selected-keys="selectedKey ? [selectedKey] : []"
      draggable
      checkable
      selectable
      cascade
      block-line
      :allow-drop="allowDrop"
      :render-label="renderLabel"
      class="file-tree__n-tree"
      @update:expanded-keys="handleExpandChange"
      @update:checked-keys="handleCheckChange"
      @update:selected-keys="handleSelectChange"
      @drop="handleDrop"
    />

    <!-- 右键菜单 -->
    <FileTreeContextMenu
      :context="contextMenuContext"
      @rename="handleRename"
      @delete="handleDelete"
      @close="closeContextMenu"
    />

    <!-- 重命名对话框 -->
    <FileTreeRenameDialog
      v-model:visible="renameDialogVisible"
      :node="renameNode"
      @confirm="confirmRename"
      @cancel="renameDialogVisible = false"
    />

    <!-- 删除确认对话框 -->
    <Teleport to="body">
      <Transition name="modal">
        <div
          v-if="deleteConfirmVisible"
          class="modal-overlay"
          @click="deleteConfirmVisible = false"
        >
          <div
            class="confirm-dialog"
            @click.stop
          >
            <div class="confirm-dialog__content">
              <EaIcon
                name="alert-triangle"
                :size="24"
                class="confirm-dialog__icon"
              />
              <h4 class="confirm-dialog__title">
                {{ t('fileTree.confirmDeleteTitle') }}
              </h4>
              <p class="confirm-dialog__message">
                {{ t('fileTree.confirmDeleteMessage', { name: deleteNode?.label }) }}
              </p>
            </div>
            <div class="confirm-dialog__actions">
              <EaButton
                type="secondary"
                @click="deleteConfirmVisible = false"
              >
                {{ t('common.cancel') }}
              </EaButton>
              <EaButton
                type="primary"
                :loading="loading"
                @click="confirmDelete"
              >
                {{ t('common.confirmDelete') }}
              </EaButton>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- 批量删除确认对话框 -->
    <Teleport to="body">
      <Transition name="modal">
        <div
          v-if="batchDeleteConfirmVisible"
          class="modal-overlay"
          @click="batchDeleteConfirmVisible = false"
        >
          <div
            class="confirm-dialog"
            @click.stop
          >
            <div class="confirm-dialog__content">
              <EaIcon
                name="alert-triangle"
                :size="24"
                class="confirm-dialog__icon"
              />
              <h4 class="confirm-dialog__title">
                {{ t('fileTree.confirmBatchDeleteTitle') }}
              </h4>
              <p class="confirm-dialog__message">
                {{ t('fileTree.confirmBatchDeleteMessage', { count: checkedKeys.length }) }}
              </p>
            </div>
            <div class="confirm-dialog__actions">
              <EaButton
                type="secondary"
                @click="batchDeleteConfirmVisible = false"
              >
                {{ t('common.cancel') }}
              </EaButton>
              <EaButton
                type="primary"
                :loading="loading"
                @click="confirmBatchDelete"
              >
                {{ t('common.confirmDelete') }}
              </EaButton>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.file-tree {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.file-tree__toolbar {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  background-color: var(--color-bg-tertiary);
  border-bottom: 1px solid var(--color-border);
}

.file-tree__toolbar-text {
  flex: 1;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

/* Naive UI n-tree 自定义样式 */
.file-tree__n-tree {
  flex: 1;
  overflow: auto;
  --n-font-size: var(--font-size-xs) !important;
  --n-text-color: var(--color-text-secondary) !important;
  --n-node-text-color: var(--color-text-secondary) !important;
  --n-node-text-color-hover: var(--color-text-primary) !important;
  --n-node-text-color-active: var(--color-primary) !important;
  --n-node-text-color-selected: var(--color-primary) !important;
  --n-node-color-hover: var(--color-surface-hover) !important;
  --n-node-color-active: var(--color-primary-light) !important;
  --n-node-color-selected: var(--color-primary-light) !important;
  --n-arrow-color: var(--color-text-tertiary) !important;
  --n-line-color: var(--color-border) !important;
  padding: var(--spacing-1) 0;
}

.file-tree__n-tree :deep(.n-tree-node) {
  padding: 2px 0;
}

.file-tree__n-tree :deep(.n-tree-node-content) {
  padding: 5px 10px !important;
  border-radius: var(--radius-sm);
}

.file-tree__n-tree :deep(.n-tree-node-wrapper) {
  padding: 0 4px;
}

.file-tree__n-tree :deep(.n-tree-switcher) {
  width: 16px !important;
  height: 16px !important;
}

.file-tree__n-tree :deep(.file-tree-node__content) {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  width: 100%;
}

.file-tree__n-tree :deep(.file-tree-node__icon) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.file-tree__n-tree :deep(.file-tree-node__name) {
  display: flex;
  align-items: center;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.35;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
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

.confirm-dialog {
  width: 400px;
  max-width: 90vw;
  background-color: var(--color-surface);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-2xl);
}

.confirm-dialog__content {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--spacing-6);
  text-align: center;
}

.confirm-dialog__icon {
  color: var(--color-warning);
  margin-bottom: var(--spacing-4);
}

.confirm-dialog__title {
  margin: 0 0 var(--spacing-2);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.confirm-dialog__message {
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  line-height: 1.5;
}

.confirm-dialog__actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-3);
  padding: var(--spacing-4) var(--spacing-6);
  border-top: 1px solid var(--color-border);
}

/* 动画 */
.modal-enter-active,
.modal-leave-active {
  transition: opacity var(--transition-normal) var(--easing-default);
}

.modal-enter-active .confirm-dialog,
.modal-leave-active .confirm-dialog {
  transition: transform var(--transition-normal) var(--easing-default),
              opacity var(--transition-normal) var(--easing-default);
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .confirm-dialog,
.modal-leave-to .confirm-dialog {
  transform: scale(0.95);
  opacity: 0;
}
</style>
