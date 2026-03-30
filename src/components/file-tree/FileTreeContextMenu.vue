<script setup lang="ts">
/**
 * 文件树右键菜单组件
 */

import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import type { ContextMenuContext } from './types'

const { t } = useI18n()

interface Props {
  context: ContextMenuContext | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  createFile: [node: ContextMenuContext['node']]
  createFolder: [node: ContextMenuContext['node']]
  rename: [node: ContextMenuContext['node']]
  delete: [node: ContextMenuContext['node']]
  sendToSession: [node: ContextMenuContext['node']]
  close: []
}>()

/// 菜单样式位置
const menuStyle = computed(() => {
  if (!props.context) return {}
  return {
    left: `${props.context.position.x}px`,
    top: `${props.context.position.y}px`
  }
})

/// 是否显示菜单
const visible = computed(() => props.context !== null)

/// 节点类型
const nodeType = computed(() => props.context?.node.nodeType)
const isRoot = computed(() => props.context?.node.isRoot === true)

/// 处理新建文件
const handleCreateFile = () => {
  if (props.context) {
    emit('createFile', props.context.node)
  }
  emit('close')
}

/// 处理新建文件夹
const handleCreateFolder = () => {
  if (props.context) {
    emit('createFolder', props.context.node)
  }
  emit('close')
}

/// 处理重命名
const handleRename = () => {
  if (props.context) {
    emit('rename', props.context.node)
  }
  emit('close')
}

/// 处理删除
const handleDelete = () => {
  if (props.context) {
    emit('delete', props.context.node)
  }
  emit('close')
}

/// 处理发送到会话
const handleSendToSession = () => {
  if (props.context) {
    emit('sendToSession', props.context.node)
  }
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <Transition name="context-menu">
      <div
        v-if="visible"
        class="file-tree-context-menu"
        :style="menuStyle"
        @click.stop
      >
        <div
          v-if="!isRoot"
          class="context-menu__item"
          @click="handleSendToSession"
        >
          <EaIcon
            name="at-sign"
            :size="14"
            class="context-menu__icon"
          />
          <span>{{ t('fileTree.sendToSession') }}</span>
        </div>
        <div
          class="context-menu__item"
          @click="handleCreateFile"
        >
          <EaIcon
            name="file-plus"
            :size="14"
            class="context-menu__icon"
          />
          <span>{{ t('fileTree.createFile') }}</span>
        </div>
        <div
          class="context-menu__item"
          @click="handleCreateFolder"
        >
          <EaIcon
            name="folder-plus"
            :size="14"
            class="context-menu__icon"
          />
          <span>{{ t('fileTree.createFolder') }}</span>
        </div>
        <div class="context-menu__divider" />
        <div
          v-if="!isRoot"
          class="context-menu__item"
          @click="handleRename"
        >
          <EaIcon
            name="edit-2"
            :size="14"
            class="context-menu__icon"
          />
          <span>{{ t('common.rename') }}</span>
        </div>
        <div
          v-if="!isRoot"
          class="context-menu__item context-menu__item--danger"
          @click="handleDelete"
        >
          <EaIcon
            :name="nodeType === 'directory' ? 'folder-minus' : 'trash-2'"
            :size="14"
            class="context-menu__icon"
          />
          <span>{{ nodeType === 'directory' ? t('common.deleteFolder') : t('common.deleteFile') }}</span>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.file-tree-context-menu {
  position: fixed;
  z-index: var(--z-dropdown);
  min-width: 160px;
  padding: var(--spacing-1) 0;
  background-color: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg);
}

.context-menu__item {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
}

.context-menu__item:hover {
  background-color: var(--color-surface-hover);
}

.context-menu__item--danger {
  color: var(--color-error);
}

.context-menu__item--danger:hover {
  background-color: var(--color-error-light);
}

.context-menu__icon {
  flex-shrink: 0;
  color: var(--color-text-tertiary);
}

.context-menu__item--danger .context-menu__icon {
  color: var(--color-error);
}

.context-menu__divider {
  height: 1px;
  margin: var(--spacing-1) 0;
  background-color: var(--color-border);
}

/* 动画 */
.context-menu-enter-active,
.context-menu-leave-active {
  transition: opacity var(--transition-fast) var(--easing-default),
              transform var(--transition-fast) var(--easing-default);
}

.context-menu-enter-from,
.context-menu-leave-to {
  opacity: 0;
  transform: scale(0.95);
}
</style>
