<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useSessionStore } from '@/stores/session'
import { useProjectStore } from '@/stores/project'
import { resolveFileIcon } from '@/utils/fileIcon'
import { EaIcon } from '@/components/common'

const props = defineProps<{
  visible: boolean
  position: { x: number; y: number; width: number; height: number }
  searchText: string
  mentionStart: number
}>()

const emit = defineEmits<{
  select: [path: string, relativePath: string, mentionStart: number]
  close: []
}>()

const { t } = useI18n()
const sessionStore = useSessionStore()
const projectStore = useProjectStore()

// 扁平化的文件列表（包含相对路径）
interface FlatFile {
  name: string
  path: string
  relativePath: string
  nodeType: 'file' | 'directory'
  extension: string | null
  depth: number
}

// 状态
const isOpen = computed(() => props.visible)
const isLoading = ref(false)
const allFiles = ref<FlatFile[]>([])
const selectedIndex = ref(0)
const dropdownRef = ref<HTMLElement | null>(null)

const currentProject = computed(() => {
  const sessionId = sessionStore.currentSessionId
  if (!sessionId) return null
  return projectStore.projects.find(p => p.id === sessionStore.currentSession?.projectId) || null
})

// 过滤后的文件列表 - 使用 props.searchText 进行过滤
const filteredFiles = computed(() => {
  const query = props.searchText.toLowerCase().trim()
  if (!query) return allFiles.value

  return allFiles.value.filter(file => {
    // 同时匹配文件名和相对路径
    const relativePath = file.relativePath || ''
    return file.name.toLowerCase().includes(query) ||
      relativePath.toLowerCase().includes(query)
  })
})

// 计算下拉框位置 - 显示在上方
const dropdownStyle = computed(() => {
  if (!props.position.x || !props.position.y) return {}

  const dropdownHeight = 280
  const spaceBelow = window.innerHeight - props.position.y
  const showAbove = spaceBelow < dropdownHeight

  if (showAbove) {
    return {
      left: `${props.position.x}px`,
      bottom: `${window.innerHeight - props.position.y + 24}px`
    }
  } else {
    return {
      left: `${props.position.x}px`,
      top: `${props.position.y + 4}px`
    }
  }
})

// 关闭下拉框
const close = () => {
  emit('close')
}

// 递归加载所有文件
const loadAllFiles = async () => {
  if (!currentProject.value || !isOpen.value) return

  isLoading.value = true
  allFiles.value = []

  try {
    const projectPath = currentProject.value.path
    const result = await invoke<FlatFile[]>('list_all_project_files_flat', {
      projectPath
    })
    allFiles.value = result || []
    selectedIndex.value = 0
  } catch (error) {
    console.error('Failed to load all files:', error)
    allFiles.value = []
  } finally {
    isLoading.value = false
  }
}

// 选择文件或文件夹
const selectFile = (file: FlatFile) => {
  close()
  const relativePath = file.relativePath || file.name
  emit('select', file.path, relativePath, props.mentionStart)
}

// 键盘导航
const handleKeyDown = (e: KeyboardEvent) => {
  if (!isOpen.value) return

  switch (e.key) {
    case 'ArrowUp':
      e.preventDefault()
      e.stopPropagation()
      selectedIndex.value = selectedIndex.value > 0
        ? selectedIndex.value - 1
        : filteredFiles.value.length - 1
      scrollToSelected()
      break
    case 'ArrowDown':
      e.preventDefault()
      e.stopPropagation()
      selectedIndex.value = selectedIndex.value < filteredFiles.value.length - 1
        ? selectedIndex.value + 1
        : 0
      scrollToSelected()
      break
    case 'Enter':
      e.preventDefault()
      e.stopPropagation()
      const selectedFile = filteredFiles.value[selectedIndex.value]
      if (selectedFile) {
        selectFile(selectedFile)
      }
      break
    case 'Escape':
      e.preventDefault()
      e.stopPropagation()
      close()
      break
  }
}

// 滚动到选中项
const scrollToSelected = () => {
  nextTick(() => {
    if (!dropdownRef.value) return
    const selectedEl = dropdownRef.value.querySelector('.file-mention__item--selected')
    if (selectedEl) {
      selectedEl.scrollIntoView({ block: 'nearest' })
    }
  })
}

// 获取文件图标名称
const getFileIconName = (file: FlatFile): string => {
  if (file.nodeType === 'directory') return 'folder'
  const iconMeta = resolveFileIcon(file.nodeType, file.name, file.extension ?? undefined)
  return typeof iconMeta === 'string' ? iconMeta : (iconMeta?.icon || 'file')
}

// 获取显示路径（搜索时显示完整相对路径，否则只显示文件名）
const getDisplayPath = (file: FlatFile): string => {
  if (props.searchText.trim()) {
    return file.relativePath
  }
  return file.name
}

// 高亮匹配文本
const highlightMatch = (text: string): string => {
  const query = props.searchText.toLowerCase().trim()
  if (!query) return text

  const index = text.toLowerCase().indexOf(query)
  if (index === -1) return text

  return text.slice(0, index) +
    '<mark>' + text.slice(index, index + query.length) + '</mark>' +
    text.slice(index + query.length)
}

// 监听 visible 变化
watch(() => props.visible, async (visible) => {
  if (visible && currentProject.value) {
    await loadAllFiles()
  } else {
    allFiles.value = []
  }
}, { immediate: true })

// 当搜索文本变化时重置选中索引
watch(() => props.searchText, () => {
  selectedIndex.value = 0
  nextTick(scrollToSelected)
})

// 监听全局键盘事件
onMounted(() => {
  document.addEventListener('keydown', handleKeyDown, true)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeyDown, true)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      ref="dropdownRef"
      class="file-mention-dropdown"
      :style="dropdownStyle"
    >
      <!-- 搜索提示 -->
      <div v-if="searchText" class="file-mention__search-hint">
        <EaIcon name="search" :size="12" />
        <span>搜索: "{{ searchText }}"</span>
          <span class="file-mention__count">{{ t('fileMention.resultCount', { count: filteredFiles.length }) }}</span>
      </div>

      <!-- 搜索结果为空 -->
      <div
        v-if="!isLoading && filteredFiles.length === 0"
        class="file-mention__empty"
      >
        <EaIcon name="file-x" :size="24" />
        <span>{{ searchText ? '未找到匹配的文件' : '项目中暂无文件' }}</span>
      </div>

      <!-- 文件列表 -->
      <div
        v-else
        class="file-mention__list"
      >
        <!-- 文件项 -->
        <div
          v-for="(file, index) in filteredFiles"
          :key="file.path"
          class="file-mention__item"
          :class="{ 'file-mention__item--selected': index === selectedIndex }"
          @click="selectFile(file)"
          @mouseenter="selectedIndex = index"
        >
          <EaIcon :name="getFileIconName(file)" :size="14" />
          <span
            class="file-mention__path"
            v-html="highlightMatch(getDisplayPath(file))"
          />
        </div>

        <!-- 加载中 -->
        <div v-if="isLoading" class="file-mention__loading">
          <EaIcon name="loading" :size="16" class="file-mention__loading-icon" />
          <span>{{ t('fileMention.loading') }}</span>
        </div>
      </div>

      <!-- 底部提示 -->
      <div class="file-mention__footer">
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
  min-width: 300px;
  max-width: 400px;
  max-height: 320px;
  background-color: var(--color-surface-elevated);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  z-index: 10000;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.file-mention__search-hint {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-bottom: 1px solid var(--color-border);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  background-color: var(--color-surface);
}

.file-mention__count {
  margin-left: auto;
  padding: 2px 6px;
  background-color: var(--color-primary-light);
  color: var(--color-primary);
  border-radius: var(--radius-sm);
  font-size: 10px;
}

.file-mention__list {
  flex: 1;
  display: flex;
  flex-direction: column;
  max-height: 240px;
  overflow-y: auto;
}

.file-mention__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  padding: var(--spacing-6);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-sm);
}

.file-mention__item {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: 0;
  cursor: pointer;
  transition: all var(--transition-fast) var(--easing-default);
  border-bottom: 1px solid var(--color-border-light);
}

.file-mention__item:last-child {
  border-bottom: none;
}

.file-mention__item:hover {
  background-color: var(--color-surface-hover);
}

.file-mention__item--selected {
  background-color: var(--color-primary-light);
}

.file-mention__path {
  flex: 1;
  font-size: var(--font-size-xs);
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-family-mono);
}

.file-mention__path :deep(mark) {
  background-color: var(--color-warning-light);
  color: var(--color-warning-dark);
  padding: 0 2px;
  border-radius: 2px;
}

.file-mention__loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  padding: var(--spacing-4);
  color: var(--color-text-tertiary);
  font-size: var(--font-size-sm);
}

.file-mention__loading-icon {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.file-mention__footer {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2);
  border-top: 1px solid var(--color-border);
  font-size: 10px;
  color: var(--color-text-tertiary);
  background-color: var(--color-surface);
}

.file-mention__footer kbd {
  padding: 2px 4px;
  background-color: var(--color-bg-tertiary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: 9px;
  font-family: inherit;
}
</style>
