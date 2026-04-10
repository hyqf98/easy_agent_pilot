<script setup lang="ts">
import { EaButton, EaInput } from '@/components/common'
import EaSelect from '@/components/common/EaSelect.vue'
import MemoryLibraryModal from '../MemoryLibraryModal.vue'
import RawMemoryModal from '../RawMemoryModal.vue'
import MemoryMergeModal from '../MemoryMergeModal.vue'
import MemoryBatchDeleteModal from '../MemoryBatchDeleteModal.vue'
import { useMemoryModePanel } from './useMemoryModePanel'

const {
  memoryStore,
  search,
  projectFilter,
  selectedRecordId,
  selectedRecord,
  projectOptions,
  currentProjectLabel,
  sortedLibraries,
  canSaveLibrary,
  libraryModalVisible,
  libraryEditing,
  rawModalVisible,
  rawEditing,
  mergeModalVisible,
  batchDeleteModalVisible,
  libraryContentDraft,
  libraryContentDirty,
  reloadRawRecords,
  openLibraryCreate,
  openLibraryEdit,
  openRawCreate,
  openRawEdit,
  handleLibrarySubmit,
  handleRawSubmit,
  handleDeleteLibrary,
  handleDeleteRecord,
  handleBatchDeleteConfirm,
  handleSaveLibrary,
  handleMergeConfirm
} = useMemoryModePanel()
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
<style scoped src="./styles.css"></style>
