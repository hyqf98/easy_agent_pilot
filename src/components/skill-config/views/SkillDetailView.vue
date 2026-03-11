<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { UnifiedSkillConfig } from '@/stores/skillConfig'
import { EaButton, EaIcon } from '@/components/common'
import ConfigFileWorkspace from '@/components/skill-config/common/ConfigFileWorkspace.vue'

// Reference 文件类型
interface ReferenceFile {
  name: string
  path: string
  file_type: string
  size: number
}

// Reference 文件内容类型
interface ReferenceFileContent {
  name: string
  path: string
  content: string
  file_type: string
}

// Skill 文件内容类型
interface SkillFileContent {
  path: string
  content: string
  file_type: string
}

const props = defineProps<{
  skill: UnifiedSkillConfig
}>()

const emit = defineEmits<{
  (e: 'back'): void
  (e: 'delete', skill: UnifiedSkillConfig): void
}>()

const { t } = useI18n()

// 状态
const isLoading = ref(true)
const skillFileContent = ref<SkillFileContent | null>(null)
const referenceFiles = ref<ReferenceFile[]>([])
const showReferences = ref(false)
const isEditMode = ref(false)
const editContent = ref('')

// 当前显示的文件
const currentFile = computed(() => {
  if (selectedReference.value && referenceContent.value) {
    return {
      name: selectedReference.value.name,
      path: selectedReference.value.path,
      content: referenceContent.value.content,
      file_type: referenceContent.value.file_type,
    }
  }
  if (skillFileContent.value) {
    return {
      name: 'SKILL.md',
      path: skillFileContent.value.path,
      content: skillFileContent.value.content,
      file_type: skillFileContent.value.file_type,
    }
  }
  return null
})

const selectedReference = ref<ReferenceFile | null>(null)
const referenceContent = ref<ReferenceFileContent | null>(null)
const isLoadingFile = ref(false)

// 加载 Skill 详情
async function loadSkillDetail() {
  isLoading.value = true
  selectedReference.value = null
  referenceContent.value = null
  isEditMode.value = false
  try {
    // 并行加载 Skill 文件和 references 列表
    const [skillFile, refs] = await Promise.all([
      invoke<SkillFileContent>('read_skill_file', {
        skillPath: props.skill.skillPath
      }),
      invoke<ReferenceFile[]>('list_skill_references', {
        skillPath: props.skill.skillPath
      })
    ])

    skillFileContent.value = skillFile
    referenceFiles.value = refs
  } catch (error) {
    console.error('Failed to load skill detail:', error)
  } finally {
    isLoading.value = false
  }
}

// 选择 Reference 文件
async function selectReference(file: ReferenceFile) {
  selectedReference.value = file
  isLoadingFile.value = true
  referenceContent.value = null
  isEditMode.value = false
  showReferences.value = false // 收起侧边栏

  try {
    const content = await invoke<ReferenceFileContent>('read_reference_file', {
      filePath: file.path
    })
    referenceContent.value = content
  } catch (error) {
    console.error('Failed to load reference file:', error)
  } finally {
    isLoadingFile.value = false
  }
}

// 返回 SKILL.md
function showSkillMd() {
  selectedReference.value = null
  referenceContent.value = null
  isEditMode.value = false
}

// 切换编辑模式
function toggleEditMode() {
  if (isEditMode.value) {
    // 退出编辑模式
    isEditMode.value = false
  } else {
    // 进入编辑模式
    if (currentFile.value) {
      editContent.value = currentFile.value.content
      isEditMode.value = true
    }
  }
}

// 保存编辑
async function saveEdit() {
  // TODO: 实现保存功能
  if (currentFile.value) {
    currentFile.value.content = editContent.value
  }
  isEditMode.value = false
}

// 返回列表
function handleBack() {
  emit('back')
}

// 删除 Skill
function handleDelete() {
  emit('delete', props.skill)
}

// 格式化文件大小
function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

// 获取文件图标
function getFileIcon(fileType: string): string {
  switch (fileType) {
    case 'markdown':
      return 'lucide:file-text'
    case 'javascript':
    case 'typescript':
    case 'python':
    case 'rust':
      return 'lucide:file-code'
    case 'json':
      return 'lucide:file-json'
    case 'html':
      return 'lucide:file-code'
    case 'css':
      return 'lucide:file-code'
    default:
      return 'lucide:file'
  }
}

// 监听 skill 变化
watch(() => props.skill, () => {
  loadSkillDetail()
}, { immediate: true })

onMounted(() => {
  loadSkillDetail()
})
</script>

<template>
  <div class="skill-detail">
    <!-- 头部工具栏 -->
    <div class="skill-detail__toolbar">
      <div class="skill-detail__toolbar-left">
        <EaButton
          variant="ghost"
          size="small"
          @click="handleBack"
        >
          <EaIcon name="lucide:arrow-left" />
          {{ t('common.back') }}
        </EaButton>
        <div class="skill-detail__breadcrumb">
          <EaIcon
            name="lucide:book-open"
            class="skill-detail__icon"
          />
          <span class="skill-detail__name">{{ skill.name }}</span>
          <EaIcon
            name="lucide:chevron-right"
            class="skill-detail__chevron"
          />
          <span class="skill-detail__current-file">{{ currentFile?.name }}</span>
        </div>
      </div>
      <div class="skill-detail__toolbar-right">
        <!-- References 切换按钮 -->
        <EaButton
          v-if="referenceFiles.length > 0"
          variant="ghost"
          size="small"
          :class="{ 'skill-detail__ref-btn--active': showReferences }"
          @click="showReferences = !showReferences"
        >
          <EaIcon name="lucide:folder-tree" />
          {{ t('settings.skills.references') }}
          <span class="skill-detail__badge">{{ referenceFiles.length }}</span>
        </EaButton>

        <!-- 编辑按钮 -->
        <EaButton
          v-if="skill.source === 'file' && currentFile"
          :variant="isEditMode ? 'primary' : 'ghost'"
          size="small"
          @click="toggleEditMode"
        >
          <EaIcon :name="isEditMode ? 'lucide:eye' : 'lucide:pencil'" />
          {{ isEditMode ? t('common.view') : t('common.edit') }}
        </EaButton>

        <!-- 保存按钮 -->
        <EaButton
          v-if="isEditMode"
          variant="primary"
          size="small"
          @click="saveEdit"
        >
          <EaIcon name="lucide:save" />
          {{ t('common.save') }}
        </EaButton>

        <!-- 返回 SKILL.md 按钮 -->
        <EaButton
          v-if="selectedReference"
          variant="ghost"
          size="small"
          @click="showSkillMd"
        >
          <EaIcon name="lucide:file-text" />
          SKILL.md
        </EaButton>

        <!-- 删除按钮 -->
        <EaButton
          v-if="skill.source === 'file'"
          variant="ghost"
          size="small"
          danger
          @click="handleDelete"
        >
          <EaIcon name="lucide:trash-2" />
        </EaButton>
      </div>
    </div>

    <!-- 路径信息 -->
    <div class="skill-detail__path-bar">
      <EaIcon name="lucide:folder" />
      <span>{{ skill.skillPath }}</span>
      <span
        v-if="currentFile && currentFile.name !== 'SKILL.md'"
        class="skill-detail__path-separator"
      >/</span>
      <span
        v-if="currentFile && currentFile.name !== 'SKILL.md'"
        class="skill-detail__path-file"
      >
        {{ currentFile.name }}
      </span>
    </div>

    <!-- 加载中 -->
    <div
      v-if="isLoading"
      class="skill-detail__loading"
    >
      <EaIcon
        name="lucide:loader-2"
        class="skill-detail__spinner"
      />
      {{ t('common.loading') }}
    </div>

    <!-- 主内容区域 -->
    <div
      v-else
      class="skill-detail__body"
    >
      <!-- References 侧边栏 -->
      <Transition name="slide">
        <div
          v-if="showReferences && referenceFiles.length > 0"
          class="skill-detail__sidebar"
        >
          <div class="skill-detail__sidebar-header">
            <h3>{{ t('settings.skills.references') }}</h3>
            <EaButton
              variant="ghost"
              size="small"
              @click="showReferences = false"
            >
              <EaIcon name="lucide:x" />
            </EaButton>
          </div>
          <div class="skill-detail__sidebar-content">
            <!-- SKILL.md 选项 -->
            <div
              class="skill-detail__file-item"
              :class="{ 'skill-detail__file-item--active': !selectedReference }"
              @click="showSkillMd"
            >
              <EaIcon
                name="lucide:file-text"
                class="skill-detail__file-icon"
              />
              <div class="skill-detail__file-info">
                <span class="skill-detail__file-name">SKILL.md</span>
                <span class="skill-detail__file-type">Markdown</span>
              </div>
            </div>

            <div class="skill-detail__divider" />

            <!-- Reference 文件列表 -->
            <div
              v-for="file in referenceFiles"
              :key="file.path"
              class="skill-detail__file-item"
              :class="{ 'skill-detail__file-item--active': selectedReference?.path === file.path }"
              @click="selectReference(file)"
            >
              <EaIcon
                :name="getFileIcon(file.file_type)"
                class="skill-detail__file-icon"
              />
              <div class="skill-detail__file-info">
                <span class="skill-detail__file-name">{{ file.name }}</span>
                <span class="skill-detail__file-meta">
                  {{ file.file_type }} · {{ formatFileSize(file.size) }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </Transition>

      <!-- 主面板 -->
      <div class="skill-detail__main">
        <ConfigFileWorkspace
          :loading="isLoadingFile"
          :editing="isEditMode"
          :file="currentFile ? {
            name: currentFile.name,
            path: currentFile.path,
            content: currentFile.content,
            fileType: currentFile.file_type
          } : null"
          :edit-content="editContent"
          :edit-placeholder="t('settings.skills.editPlaceholder')"
          :empty-text="t('settings.skills.noContent')"
          max-width="900px"
          padding="var(--spacing-6)"
          @update:edit-content="editContent = $event"
        >
          <template #loading>
            {{ t('common.loading') }}
          </template>
        </ConfigFileWorkspace>
      </div>
    </div>
  </div>
</template>

<style scoped>
.skill-detail {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--color-surface);
}

/* 工具栏 */
.skill-detail__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3) var(--spacing-4);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-background-secondary);
}

.skill-detail__toolbar-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-3);
}

.skill-detail__toolbar-right {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.skill-detail__breadcrumb {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
}

.skill-detail__icon {
  width: 20px;
  height: 20px;
  color: var(--color-success);
}

.skill-detail__name {
  font-weight: var(--font-weight-semibold);
  font-size: var(--font-size-base);
}

.skill-detail__chevron {
  width: 14px;
  height: 14px;
  color: var(--color-text-tertiary);
}

.skill-detail__current-file {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.skill-detail__badge {
  padding: 2px 6px;
  background: var(--color-primary-bg);
  color: var(--color-primary);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
}

.skill-detail__ref-btn--active {
  background: var(--color-primary-bg);
  color: var(--color-primary);
}

/* 路径栏 */
.skill-detail__path-bar {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-4);
  background: var(--color-background-tertiary);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
  border-bottom: 1px solid var(--color-border);
}

.skill-detail__path-bar svg {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.skill-detail__path-separator {
  color: var(--color-text-tertiary);
}

.skill-detail__path-file {
  color: var(--color-text-secondary);
}

/* 主体 */
.skill-detail__body {
  display: flex;
  flex: 1;
  overflow: hidden;
  position: relative;
}

/* 侧边栏 */
.skill-detail__sidebar {
  width: 280px;
  background: var(--color-background-secondary);
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  z-index: 10;
}

.skill-detail__sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-3) var(--spacing-4);
  border-bottom: 1px solid var(--color-border);
}

.skill-detail__sidebar-header h3 {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
  margin: 0;
}

.skill-detail__sidebar-content {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-2);
}

.skill-detail__file-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  padding: var(--spacing-2) var(--spacing-3);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.15s;
}

.skill-detail__file-item:hover {
  background: var(--color-background-tertiary);
}

.skill-detail__file-item--active {
  background: var(--color-primary-bg);
}

.skill-detail__file-icon {
  width: 18px;
  height: 18px;
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.skill-detail__file-item--active .skill-detail__file-icon {
  color: var(--color-primary);
}

.skill-detail__file-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.skill-detail__file-name {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.skill-detail__file-type,
.skill-detail__file-meta {
  font-size: var(--font-size-xs);
  color: var(--color-text-tertiary);
}

.skill-detail__divider {
  height: 1px;
  background: var(--color-border);
  margin: var(--spacing-2) var(--spacing-3);
}

/* 主面板 */
.skill-detail__main {
  flex: 1;
  overflow: auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

/* 加载状态 */
.skill-detail__loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-2);
  padding: var(--spacing-8);
  color: var(--color-text-tertiary);
}

.skill-detail__spinner {
  width: 20px;
  height: 20px;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* 侧边栏动画 */
.slide-enter-active,
.slide-leave-active {
  transition: all 0.2s ease;
}

.slide-enter-from,
.slide-leave-to {
  transform: translateX(-100%);
  opacity: 0;
}
</style>
