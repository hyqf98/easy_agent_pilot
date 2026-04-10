<script setup lang="ts">
import { EaButton, EaIcon } from '@/components/common'
import {
  useProjectCreateModal,
  type ProjectCreateModalEmits,
  type ProjectCreateModalProps,
} from './useProjectCreateModal'

const props = defineProps<ProjectCreateModalProps>()
const emit = defineEmits<ProjectCreateModalEmits>()

const {
  errorMessage,
  form,
  handleBrowse,
  handleSubmit,
  isEditMode,
  isFormValid,
  isValidatingPath,
  memoryStore,
  nameInputRef,
  pathError,
} = useProjectCreateModal(props, emit)
</script>

<template>
  <div class="project-form">
    <div class="project-form__header">
      <h3 class="project-form__title">
        {{ isEditMode ? '编辑项目' : '创建新项目' }}
      </h3>
    </div>

    <form
      class="project-form__body"
      @submit.prevent="handleSubmit"
    >
      <div class="form-group">
        <label class="form-label">项目名称</label>
        <input
          ref="nameInputRef"
          v-model="form.name"
          type="text"
          class="form-input"
          :class="{ 'form-input--error': errorMessage }"
          placeholder="留空将使用路径文件夹名"
        >
        <span
          v-if="errorMessage"
          class="form-error"
        >
          {{ errorMessage }}
        </span>
      </div>

      <div class="form-group">
        <label class="form-label">项目路径</label>
        <div class="form-input-group">
          <input
            v-model="form.path"
            type="text"
            class="form-input"
            :class="{ 'form-input--error': pathError }"
            placeholder="~/my-project"
            :disabled="isValidatingPath"
          >
          <EaButton
            type="secondary"
            size="small"
            @click="handleBrowse"
          >
            <EaIcon
              name="folder-open"
              :size="14"
            />
            浏览
          </EaButton>
        </div>
        <span
          v-if="pathError"
          class="form-error"
        >
          {{ pathError }}
        </span>
        <span
          v-else
          class="form-hint"
        >
          留空将使用项目名称作为目录名；若名称为空，将以路径文件夹名作为名称
        </span>
      </div>

      <div class="form-group">
        <label class="form-label">描述</label>
        <textarea
          v-model="form.description"
          class="form-textarea"
          placeholder="项目描述（可选）"
          rows="2"
        />
      </div>

      <div class="form-group">
        <div class="form-label-row">
          <label class="form-label">挂载记忆库</label>
          <span class="form-hint">已选 {{ form.memoryLibraryIds.length }} 个</span>
        </div>

        <div
          v-if="memoryStore.isLoadingLibraries"
          class="memory-library-list memory-library-list--loading"
        >
          正在加载记忆库...
        </div>

        <div
          v-else-if="memoryStore.libraries.length === 0"
          class="memory-library-list memory-library-list--empty"
        >
          暂无可挂载的记忆库，请先在记忆管理中创建。
        </div>

        <div
          v-else
          class="memory-library-list"
        >
          <label
            v-for="library in memoryStore.libraries"
            :key="library.id"
            class="memory-library-item"
          >
            <input
              v-model="form.memoryLibraryIds"
              type="checkbox"
              :value="library.id"
              class="memory-library-item__checkbox"
            >
            <div class="memory-library-item__content">
              <span class="memory-library-item__title">{{ library.name }}</span>
              <span
                v-if="library.description"
                class="memory-library-item__description"
              >
                {{ library.description }}
              </span>
            </div>
          </label>
        </div>
      </div>

      <div class="project-form__actions">
        <EaButton
          type="secondary"
          @click="emit('cancel')"
        >
          取消
        </EaButton>
        <EaButton
          type="primary"
          native-type="submit"
          :disabled="!isFormValid"
        >
          {{ isEditMode ? '保存' : '创建' }}
        </EaButton>
      </div>
    </form>
  </div>
</template>

<style scoped src="./styles.css"></style>
