<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { EaButton, EaInput } from '@/components/common'
import EaSelect, { type SelectOption } from '@/components/common/EaSelect.vue'
import { useMemoryStore } from '@/stores/memory'
import { useProjectStore } from '@/stores/project'
import type {
  CreateMemoryLibraryInput,
  CreateRawMemoryRecordInput,
  MemoryLibrary,
  RawMemoryRecord,
  UpdateMemoryLibraryInput,
  UpdateRawMemoryRecordInput
} from '@/types/memory'
import MemoryLibraryModal from './MemoryLibraryModal.vue'
import RawMemoryModal from './RawMemoryModal.vue'
import MemoryMergeModal from './MemoryMergeModal.vue'
import MemoryBatchDeleteModal from './MemoryBatchDeleteModal.vue'

const memoryStore = useMemoryStore()
const projectStore = useProjectStore()

const search = ref('')
const projectFilter = ref<string>('all')
const selectedRecordId = ref<string | null>(null)
const libraryModalVisible = ref(false)
const libraryEditing = ref<MemoryLibrary | null>(null)
const rawModalVisible = ref(false)
const rawEditing = ref<RawMemoryRecord | null>(null)
const mergeModalVisible = ref(false)
const batchDeleteModalVisible = ref(false)
const libraryContentDraft = ref('')
const libraryContentDirty = ref(false)

let filterTimer: ReturnType<typeof setTimeout> | null = null

const selectedRecord = computed(() =>
  memoryStore.rawRecords.find((record) => record.id === selectedRecordId.value) ?? null
)

const projectOptions = computed<SelectOption[]>(() => [
  { value: 'all', label: '全部项目' },
  ...projectStore.projects.map((project) => ({
    value: project.id,
    label: project.name
  }))
])
const currentProjectLabel = computed(() =>
  projectOptions.value.find(option => option.value === projectFilter.value)?.label ?? '全部项目'
)

const sortedLibraries = computed(() =>
  [...memoryStore.libraries].sort((left, right) =>
    new Date(right.updatedAt).getTime() - new Date(left.updatedAt).getTime()
  )
)

const canSaveLibrary = computed(() => {
  return Boolean(memoryStore.activeLibrary && libraryContentDirty.value)
})

watch(
  () => memoryStore.activeLibrary,
  (library) => {
    libraryContentDraft.value = library?.contentMd ?? ''
    libraryContentDirty.value = false
  },
  { immediate: true }
)

watch(search, triggerReload)
watch(projectFilter, triggerReload)

function triggerReload() {
  if (filterTimer) {
    clearTimeout(filterTimer)
  }
  filterTimer = setTimeout(() => {
    void reloadRawRecords()
  }, 180)
}

async function reloadRawRecords() {
  await memoryStore.loadRawRecords({
    search: search.value.trim() || undefined,
    projectId: projectFilter.value !== 'all' ? projectFilter.value : undefined
  })
}

function openLibraryCreate() {
  libraryEditing.value = null
  libraryModalVisible.value = true
}

function openLibraryEdit(library: MemoryLibrary) {
  libraryEditing.value = library
  libraryModalVisible.value = true
}

function openRawCreate() {
  rawEditing.value = null
  rawModalVisible.value = true
}

function openRawEdit(record: RawMemoryRecord) {
  rawEditing.value = record
  rawModalVisible.value = true
}

async function handleLibrarySubmit(payload: { name: string; description?: string }) {
  if (libraryEditing.value) {
    await memoryStore.updateLibrary(libraryEditing.value.id, payload as UpdateMemoryLibraryInput)
  } else {
    await memoryStore.createLibrary(payload as CreateMemoryLibraryInput)
  }
  libraryModalVisible.value = false
}

async function handleRawSubmit(payload: { content: string }) {
  if (rawEditing.value) {
    const record = await memoryStore.updateRawRecord(rawEditing.value.id, payload as UpdateRawMemoryRecordInput)
    selectedRecordId.value = record.id
  } else {
    const record = await memoryStore.createRawRecord({
      content: payload.content,
      projectId: projectStore.currentProjectId ?? undefined,
      sourceRole: 'user'
    } as CreateRawMemoryRecordInput)
    selectedRecordId.value = record.id
  }
  rawModalVisible.value = false
}

async function handleDeleteLibrary(library: MemoryLibrary) {
  if (!window.confirm(`确定删除记忆库「${library.name}」吗？`)) return
  await memoryStore.deleteLibrary(library.id)
}

async function handleDeleteRecord(record: RawMemoryRecord) {
  if (!window.confirm('确定删除这条原始记忆吗？')) return
  await memoryStore.deleteRawRecord(record.id)
  if (selectedRecordId.value === record.id) {
    selectedRecordId.value = null
  }
}

async function handleBatchDeleteConfirm(payload: {
  startAt?: string
  endAt?: string
  limit?: number
  deleteOrder?: 'oldest' | 'latest'
}) {
  const scopeText = [
    currentProjectLabel.value !== '全部项目' ? `项目：${currentProjectLabel.value}` : '',
    search.value.trim() ? `搜索：${search.value.trim()}` : ''
  ].filter(Boolean).join('，')
  const actionText = payload.limit
    ? `按条件批量删除 ${payload.limit} 条原始记忆`
    : '按时间范围批量删除原始记忆'

  if (!window.confirm(`${actionText}${scopeText ? `（${scopeText}）` : ''}，确认继续吗？`)) {
    return
  }

  const result = await memoryStore.batchDeleteRawRecords(payload)
  batchDeleteModalVisible.value = false
  if (selectedRecordId.value && result.deletedIds.includes(selectedRecordId.value)) {
    selectedRecordId.value = null
  }
}

async function handleSaveLibrary() {
  const library = memoryStore.activeLibrary
  if (!library) return

  const updated = await memoryStore.updateLibrary(library.id, {
    contentMd: libraryContentDraft.value
  })
  libraryContentDraft.value = updated.contentMd
  libraryContentDirty.value = false
}

async function handleMergeConfirm(payload: { libraryId: string; agentId?: string }) {
  mergeModalVisible.value = false
  const result = await memoryStore.mergeIntoLibrary({
    libraryId: payload.libraryId,
    agentId: payload.agentId,
    recordIds: memoryStore.selectedRecordIds
  })
  libraryContentDraft.value = result.library.contentMd
  libraryContentDirty.value = false
}

onMounted(async () => {
  await memoryStore.initialize()
  if (projectStore.projects.length === 0) {
    await projectStore.loadProjects()
  }
})

onUnmounted(() => {
  if (filterTimer) {
    clearTimeout(filterTimer)
  }
})
</script>

<template>
  <div class="memory-mode">
    <aside class="memory-mode__libraries">
      <div class="memory-panel-heading">
        <div>
          <p class="memory-panel-heading__eyebrow">
            Memory Libraries
          </p>
          <h2>记忆库</h2>
        </div>
        <EaButton
          type="secondary"
          size="small"
          @click="openLibraryCreate"
        >
          新建
        </EaButton>
      </div>

      <div
        v-if="memoryStore.isLoadingLibraries && !memoryStore.libraries.length"
        class="memory-empty"
      >
        正在加载记忆库...
      </div>

      <div
        v-else-if="!sortedLibraries.length"
        class="memory-empty"
      >
        还没有记忆库，先创建一个 Markdown 记忆库。
      </div>

      <div
        v-else
        class="memory-library-list"
      >
        <button
          v-for="library in sortedLibraries"
          :key="library.id"
          type="button"
          class="memory-library-card"
          :class="{ 'memory-library-card--active': library.id === memoryStore.activeLibraryId }"
          @click="memoryStore.setActiveLibrary(library.id)"
        >
          <div class="memory-library-card__head">
            <div>
              <strong>{{ library.name }}</strong>
              <p>{{ library.description || '未填写说明' }}</p>
            </div>
            <span>{{ new Date(library.updatedAt).toLocaleDateString() }}</span>
          </div>
          <div class="memory-library-card__foot">
            <button
              type="button"
              class="memory-inline-action"
              @click.stop="openLibraryEdit(library)"
            >
              编辑
            </button>
            <button
              type="button"
              class="memory-inline-action memory-inline-action--danger"
              @click.stop="handleDeleteLibrary(library)"
            >
              删除
            </button>
          </div>
        </button>
      </div>
    </aside>

    <section class="memory-mode__records">
      <div class="memory-panel-heading">
        <div>
          <p class="memory-panel-heading__eyebrow">
            Raw Memory Pool
          </p>
          <h2>原始记忆数据</h2>
        </div>
        <div class="memory-toolbar__actions">
          <EaButton
            type="secondary"
            size="small"
            @click="openRawCreate"
          >
            手动添加
          </EaButton>
          <EaButton
            type="secondary"
            size="small"
            @click="reloadRawRecords"
          >
            刷新
          </EaButton>
          <EaButton
            type="danger"
            size="small"
            @click="batchDeleteModalVisible = true"
          >
            批量删除
          </EaButton>
        </div>
      </div>

      <div class="memory-filters">
        <EaInput
          v-model="search"
          placeholder="搜索原始记忆内容"
        />
        <EaSelect
          v-model="projectFilter"
          :options="projectOptions"
        />
      </div>

      <div class="memory-records-toolbar">
        <div class="memory-records-toolbar__summary">
          <span>共 {{ memoryStore.rawRecords.length }} 条</span>
          <span v-if="memoryStore.selectedRecordIds.length">已勾选 {{ memoryStore.selectedRecordIds.length }} 条</span>
        </div>
        <div class="memory-toolbar__actions">
          <EaButton
            type="ghost"
            size="small"
            :disabled="memoryStore.selectedRecordIds.length === 0"
            @click="memoryStore.clearSelectedRecords()"
          >
            清空勾选
          </EaButton>
          <EaButton
            size="small"
            :disabled="memoryStore.selectedRecordIds.length === 0 || !memoryStore.libraries.length"
            :loading="memoryStore.isMerging"
            @click="mergeModalVisible = true"
          >
            AI 压缩到记忆库
          </EaButton>
        </div>
      </div>

      <div
        v-if="selectedRecord"
        class="memory-record-preview"
      >
        <div class="memory-record-preview__head">
          <div>
            <strong>当前查看</strong>
            <span>{{ selectedRecord.projectName || '未关联项目' }} / {{ selectedRecord.sessionName || '未关联会话' }}</span>
          </div>
          <div class="memory-toolbar__actions">
            <EaButton
              type="secondary"
              size="small"
              @click="openRawEdit(selectedRecord)"
            >
              编辑
            </EaButton>
            <EaButton
              type="danger"
              size="small"
              @click="handleDeleteRecord(selectedRecord)"
            >
              删除
            </EaButton>
          </div>
        </div>
        <pre>{{ selectedRecord.content }}</pre>
      </div>

      <div
        v-if="memoryStore.isLoadingRecords && !memoryStore.rawRecords.length"
        class="memory-empty"
      >
        正在加载原始记忆...
      </div>

      <div
        v-else-if="!memoryStore.rawRecords.length"
        class="memory-empty"
      >
        还没有原始记忆数据。你在会话面板发送的用户消息会自动进入这里。
      </div>

      <div
        v-else
        class="memory-record-list"
      >
        <article
          v-for="record in memoryStore.rawRecords"
          :key="record.id"
          class="memory-record-card"
          :class="{ 'memory-record-card--active': record.id === selectedRecordId }"
          @click="selectedRecordId = record.id"
        >
          <label
            class="memory-record-card__check"
            @click.stop
          >
            <input
              type="checkbox"
              :checked="memoryStore.selectedRecordIds.includes(record.id)"
              @change="memoryStore.toggleRecordSelection(record.id)"
            >
            <span>{{ new Date(record.createdAt).toLocaleString() }}</span>
          </label>

          <div class="memory-record-card__content">
            <p>{{ record.content }}</p>
          </div>

          <div class="memory-record-card__meta">
            <span>{{ record.projectName || '未关联项目' }}</span>
            <span>{{ record.sessionName || '未关联会话' }}</span>
          </div>
        </article>
      </div>
    </section>

    <aside class="memory-mode__library-editor">
      <div class="memory-panel-heading">
        <div>
          <p class="memory-panel-heading__eyebrow">
            Markdown Library
          </p>
          <h2>{{ memoryStore.activeLibrary?.name || '选择记忆库' }}</h2>
          <p class="memory-library-editor__desc">
            {{ memoryStore.activeLibrary?.description || '记忆库是一篇持续维护的 Markdown 文档。' }}
          </p>
        </div>
      </div>

      <div
        v-if="!memoryStore.activeLibrary"
        class="memory-empty"
      >
        从左侧选择一个记忆库，或先创建新的记忆库。
      </div>

      <template v-else>
        <div class="memory-library-editor__toolbar">
          <span>最后更新 {{ new Date(memoryStore.activeLibrary.updatedAt).toLocaleString() }}</span>
          <EaButton
            size="small"
            :disabled="!canSaveLibrary"
            :loading="memoryStore.isSavingLibrary"
            @click="handleSaveLibrary"
          >
            保存 Markdown
          </EaButton>
        </div>

        <textarea
          v-model="libraryContentDraft"
          class="memory-library-editor__textarea"
          placeholder="这里是记忆库的完整 Markdown 内容"
          @input="libraryContentDirty = true"
        />

        <section class="memory-merge-history">
          <div class="memory-merge-history__head">
            <strong>最近压缩记录</strong>
            <span v-if="memoryStore.isLoadingMergeRuns">加载中...</span>
          </div>

          <div
            v-if="!memoryStore.mergeRuns.length"
            class="memory-merge-history__empty"
          >
            还没有压缩记录。
          </div>

          <div
            v-else
            class="memory-merge-history__list"
          >
            <article
              v-for="run in memoryStore.mergeRuns.slice(0, 6)"
              :key="run.id"
              class="memory-merge-history__item"
            >
              <div class="memory-merge-history__meta">
                <span>{{ new Date(run.createdAt).toLocaleString() }}</span>
                <span>{{ run.sourceRecordCount }} 条原始记忆</span>
              </div>
              <p>{{ run.agentId || '未记录智能体' }} / {{ run.modelId || '默认模型' }}</p>
            </article>
          </div>
        </section>
      </template>
    </aside>

    <MemoryLibraryModal
      v-model:visible="libraryModalVisible"
      :library="libraryEditing"
      :loading="memoryStore.isSavingLibrary"
      @submit="handleLibrarySubmit"
    />

    <RawMemoryModal
      v-model:visible="rawModalVisible"
      :record="rawEditing"
      @submit="handleRawSubmit"
    />

    <MemoryMergeModal
      v-model:visible="mergeModalVisible"
      :libraries="memoryStore.libraries"
      :selected-count="memoryStore.selectedRecordIds.length"
      :current-library-id="memoryStore.activeLibraryId"
      :loading="memoryStore.isMerging"
      @confirm="handleMergeConfirm"
    />

    <MemoryBatchDeleteModal
      v-model:visible="batchDeleteModalVisible"
      :visible-count="memoryStore.rawRecords.length"
      :project-label="currentProjectLabel"
      :search-keyword="search.trim()"
      :loading="memoryStore.isDeletingRecords"
      @confirm="handleBatchDeleteConfirm"
    />
  </div>
</template>

<style scoped>
.memory-mode {
  --memory-bg-top: rgba(191, 219, 254, 0.18);
  --memory-bg-bottom: rgba(251, 191, 36, 0.12);
  --memory-bg-surface-start: rgba(255, 255, 255, 0.94);
  --memory-bg-surface-end: rgba(248, 250, 252, 0.92);
  --memory-panel-border: rgba(148, 163, 184, 0.18);
  --memory-panel-border-strong: rgba(148, 163, 184, 0.45);
  --memory-card-bg: rgba(255, 255, 255, 0.9);
  --memory-card-hover-border: rgba(14, 116, 144, 0.28);
  --memory-card-hover-shadow: rgba(14, 116, 144, 0.08);
  --memory-card-active-border: rgba(15, 118, 110, 0.38);
  --memory-card-active-shadow: rgba(15, 118, 110, 0.12);
  --memory-card-shadow: rgba(15, 23, 42, 0.04);
  --memory-heading-color: var(--color-text-primary);
  --memory-muted-color: var(--color-text-secondary);
  --memory-accent-color: #0f766e;
  --memory-danger-color: #b91c1c;
  --memory-textarea-bg-start: rgba(255, 255, 255, 0.94);
  --memory-textarea-bg-end: rgba(248, 250, 252, 0.96);
  --memory-textarea-focus-border: rgba(15, 118, 110, 0.42);
  --memory-textarea-focus-shadow: rgba(15, 118, 110, 0.12);
  --memory-textarea-inner-shadow: rgba(255, 255, 255, 0.6);
  display: grid;
  grid-template-columns: 300px minmax(0, 1.1fr) minmax(380px, 0.9fr);
  height: 100%;
  min-width: 0;
  background:
    radial-gradient(circle at top left, var(--memory-bg-top), transparent 28%),
    radial-gradient(circle at bottom right, var(--memory-bg-bottom), transparent 24%),
    linear-gradient(180deg, var(--memory-bg-surface-start), var(--memory-bg-surface-end));
}

[data-theme='dark'] .memory-mode,
.dark .memory-mode {
  --memory-bg-top: rgba(59, 130, 246, 0.14);
  --memory-bg-bottom: rgba(245, 158, 11, 0.08);
  --memory-bg-surface-start: rgba(15, 23, 42, 0.98);
  --memory-bg-surface-end: rgba(17, 24, 39, 0.96);
  --memory-panel-border: rgba(71, 85, 105, 0.5);
  --memory-panel-border-strong: rgba(71, 85, 105, 0.72);
  --memory-card-bg: rgba(15, 23, 42, 0.82);
  --memory-card-hover-border: rgba(56, 189, 248, 0.4);
  --memory-card-hover-shadow: rgba(8, 145, 178, 0.18);
  --memory-card-active-border: rgba(45, 212, 191, 0.42);
  --memory-card-active-shadow: rgba(20, 184, 166, 0.22);
  --memory-card-shadow: rgba(2, 6, 23, 0.3);
  --memory-accent-color: #5eead4;
  --memory-danger-color: #fca5a5;
  --memory-textarea-bg-start: rgba(15, 23, 42, 0.96);
  --memory-textarea-bg-end: rgba(30, 41, 59, 0.98);
  --memory-textarea-focus-border: rgba(45, 212, 191, 0.46);
  --memory-textarea-focus-shadow: rgba(45, 212, 191, 0.18);
  --memory-textarea-inner-shadow: rgba(255, 255, 255, 0.04);
}

.memory-mode__libraries,
.memory-mode__records,
.memory-mode__library-editor {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 24px 20px;
}

.memory-mode__libraries,
.memory-mode__records {
  border-right: 1px solid var(--memory-panel-border);
}

.memory-panel-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.memory-panel-heading h2 {
  margin: 2px 0 0;
  color: var(--memory-heading-color);
  line-height: 1;
  font-size: 30px;
  font-family: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Georgia, serif;
}

.memory-panel-heading__eyebrow {
  margin: 0;
  color: var(--memory-muted-color);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.14em;
}

.memory-empty {
  padding: 18px;
  border: 1px dashed var(--memory-panel-border-strong);
  border-radius: 20px;
  background: color-mix(in srgb, var(--memory-card-bg) 92%, transparent);
  color: var(--memory-muted-color);
  line-height: 1.7;
}

.memory-library-list,
.memory-record-list,
.memory-merge-history__list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  overflow: auto;
}

.memory-library-card,
.memory-record-card,
.memory-record-preview,
.memory-merge-history__item {
  border: 1px solid var(--memory-panel-border);
  border-radius: 22px;
  background: var(--memory-card-bg);
  box-shadow: 0 20px 50px var(--memory-card-shadow);
}

.memory-library-card {
  padding: 16px;
  text-align: left;
  cursor: pointer;
  transition: transform 160ms ease, border-color 160ms ease, box-shadow 160ms ease;
}

.memory-library-card:hover,
.memory-record-card:hover {
  transform: translateY(-1px);
  border-color: var(--memory-card-hover-border);
  box-shadow: 0 24px 50px var(--memory-card-hover-shadow);
}

.memory-library-card--active,
.memory-record-card--active {
  border-color: var(--memory-card-active-border);
  box-shadow: 0 24px 50px var(--memory-card-active-shadow);
}

.memory-library-card__head {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

.memory-library-card__head strong {
  display: block;
  color: var(--memory-heading-color);
  font-size: 16px;
}

.memory-library-card__head p,
.memory-merge-history__item p {
  margin: 6px 0 0;
  color: var(--memory-muted-color);
  line-height: 1.6;
}

.memory-library-card__head span,
.memory-record-card__check span,
.memory-record-card__meta,
.memory-library-editor__toolbar,
.memory-merge-history__meta {
  color: var(--memory-muted-color);
  font-size: 12px;
}

.memory-library-card__foot,
.memory-toolbar__actions,
.memory-library-editor__toolbar,
.memory-record-preview__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.memory-inline-action {
  border: none;
  background: transparent;
  color: var(--memory-accent-color);
  cursor: pointer;
  padding: 0;
  font-size: 13px;
}

.memory-inline-action--danger {
  color: var(--memory-danger-color);
}

.memory-filters,
.memory-records-toolbar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 180px;
  gap: 12px;
  align-items: center;
}

.memory-records-toolbar {
  grid-template-columns: minmax(0, 1fr) auto;
}

.memory-records-toolbar__summary {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  color: var(--memory-muted-color);
  font-size: 13px;
}

.memory-record-card {
  padding: 16px;
  cursor: pointer;
}

.memory-record-card__check {
  display: flex;
  align-items: center;
  gap: 10px;
}

.memory-record-card__check input {
  margin: 0;
}

.memory-record-card__content p {
  margin: 12px 0;
  color: var(--memory-heading-color);
  line-height: 1.7;
  display: -webkit-box;
  -webkit-line-clamp: 4;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.memory-record-card__meta {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.memory-record-preview {
  padding: 16px;
}

.memory-record-preview__head strong {
  display: block;
  color: var(--memory-heading-color);
}

.memory-record-preview__head span {
  font-size: 12px;
  color: var(--memory-muted-color);
}

.memory-record-preview pre {
  margin: 14px 0 0;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--memory-heading-color);
  line-height: 1.75;
}

.memory-library-editor__desc {
  margin: 10px 0 0;
  color: var(--memory-muted-color);
  line-height: 1.7;
}

.memory-library-editor__textarea {
  flex: 1;
  min-height: 320px;
  resize: none;
  padding: 18px;
  border: 1px solid var(--memory-panel-border);
  border-radius: 24px;
  background:
    linear-gradient(180deg, var(--memory-textarea-bg-start), var(--memory-textarea-bg-end));
  color: var(--memory-heading-color);
  font: 500 14px/1.8 var(--font-family-mono, "SFMono-Regular", Consolas, monospace);
  outline: none;
  box-shadow: inset 0 1px 0 var(--memory-textarea-inner-shadow);
}

.memory-library-editor__textarea:focus {
  border-color: var(--memory-textarea-focus-border);
  box-shadow: 0 0 0 2px var(--memory-textarea-focus-shadow);
}

.memory-merge-history {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

.memory-merge-history__head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  color: var(--memory-heading-color);
}

.memory-merge-history__empty {
  color: var(--memory-muted-color);
  font-size: 13px;
}

.memory-merge-history__item {
  padding: 14px 16px;
}

.memory-merge-history__meta {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

@media (max-width: 1440px) {
  .memory-mode {
    grid-template-columns: 280px minmax(0, 1fr) minmax(320px, 0.9fr);
  }
}

@media (max-width: 1180px) {
  .memory-mode {
    grid-template-columns: 1fr;
    overflow: auto;
  }

  .memory-mode__libraries,
  .memory-mode__records {
    border-right: none;
    border-bottom: 1px solid var(--memory-panel-border);
  }

  .memory-library-editor__textarea {
    min-height: 420px;
  }
}
</style>
