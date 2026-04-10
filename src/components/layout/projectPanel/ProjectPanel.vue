<script setup lang="ts">
import { EaButton, EaIcon, EaSkeleton } from '@/components/common'
import { FileTree } from '@/components/fileTree'
import { ProjectCreateModal } from '@/components/project'
import PanelHeader from '../PanelHeader.vue'
import {
  useProjectPanel,
  type ProjectPanelEmits,
  type ProjectPanelProps,
} from './useProjectPanel'

defineProps<ProjectPanelProps>()
defineEmits<ProjectPanelEmits>()

const {
  confirmDelete,
  deletingProject,
  editingProject,
  handleAdd,
  handleCreateProject,
  handleDeleteProject,
  handleEditProject,
  handleFileSelect,
  handleRefresh,
  handleSelectProject,
  handleToggleExpand,
  projectStore,
  showDeleteConfirm,
  t,
  uiStore,
} = useProjectPanel()
</script>

<template>
  <div :class="['project-panel', { 'project-panel--collapsed': collapsed }]">
    <PanelHeader
      :title="t('panel.projects')"
      icon="folder"
      :collapsed="collapsed"
      :show-toggle="showHeaderToggle"
      show-add
      @toggle="$emit('toggle')"
      @add="handleAdd"
    />

    <div
      v-if="!collapsed"
      class="project-panel__content"
    >
      <!-- 加载状态 -->
      <div
        v-if="projectStore.isLoading"
        class="project-loading"
      >
        <div
          v-for="i in 3"
          :key="i"
          class="project-skeleton"
        >
          <EaSkeleton
            variant="circle"
            height="16px"
            width="16px"
            animation="wave"
          />
          <EaSkeleton
            variant="text"
            height="14px"
            :width="`${60 + Math.random() * 30}%`"
            animation="wave"
          />
        </div>
      </div>

      <!-- 错误状态 -->
      <div
        v-else-if="projectStore.loadError"
        class="project-error"
      >
        <EaIcon
          name="alert-circle"
          :size="32"
          class="project-error__icon"
        />
        <p class="project-error__text">
          {{ t('common.loadFailed') }}
        </p>
        <p class="project-error__detail">
          {{ projectStore.loadError }}
        </p>
        <EaButton
          type="primary"
          size="small"
          @click="handleRefresh"
        >
          <EaIcon
            name="refresh-cw"
            :size="14"
          />
          {{ t('common.retry') }}
        </EaButton>
      </div>

      <!-- 空状态 -->
      <div
        v-else-if="projectStore.projects.length === 0"
        class="project-empty"
      >
        <div class="project-empty__illustration">
          <EaIcon
            name="folder-plus"
            :size="48"
            class="project-empty__icon"
          />
        </div>
        <p class="project-empty__title">
          {{ t('project.noProjects') }}
        </p>
        <p class="project-empty__hint">
          {{ t('project.noProjectsHint') }}
        </p>
        <EaButton
          type="primary"
          size="medium"
          class="project-empty__button"
          @click="handleAdd"
        >
          <EaIcon
            name="plus"
            :size="16"
          />
          {{ t('project.createFirstProject') }}
        </EaButton>
      </div>

      <!-- 项目列表 -->
      <div
        v-else-if="projectStore.projects.length > 0"
        class="project-list"
        role="list"
      >
        <template
          v-for="project in projectStore.projects"
          :key="project.id"
        >
          <!-- 项目项 -->
          <div
            :class="[
              'project-item',
              {
                'project-item--active': project.id === projectStore.currentProjectId,
                'project-item--expanded': projectStore.isProjectExpanded(project.id)
              }
            ]"
            tabindex="0"
            role="listitem"
            :aria-selected="project.id === projectStore.currentProjectId"
            :aria-expanded="projectStore.isProjectExpanded(project.id)"
            @click="handleSelectProject(project.id)"
            @keydown.enter="handleSelectProject(project.id)"
            @keydown.space.prevent="handleSelectProject(project.id)"
          >
            <!-- 展开/折叠按钮 -->
            <button
              class="project-item__expand"
              :class="{ 'project-item__expand--expanded': projectStore.isProjectExpanded(project.id) }"
              title="展开项目文件"
              @click="handleToggleExpand(project, $event)"
            >
              <EaIcon
                :name="projectStore.isFileTreeLoading(project.id) ? 'loader' : 'chevron-right'"
                :size="14"
                :class="{ 'animate-spin': projectStore.isFileTreeLoading(project.id) }"
              />
            </button>
            <EaIcon
              name="folder"
              :size="16"
              class="project-item__icon"
            />
            <span class="project-item__name">{{ project.name }}</span>
            <span
              v-if="project.sessionCount && project.sessionCount > 0"
              class="project-item__badge"
            >
              {{ project.sessionCount }}
            </span>
            <button
              class="project-item__edit"
              title="编辑项目"
              @click.stop="handleEditProject(project)"
            >
              <EaIcon
                name="edit-2"
                :size="12"
              />
            </button>
            <button
              class="project-item__delete"
              title="删除项目"
              @click.stop="handleDeleteProject(project)"
            >
              <EaIcon
                name="x"
                :size="12"
              />
            </button>
          </div>

          <!-- 文件树 - 使用新的 FileTree 组件 -->
          <div
            v-if="projectStore.isProjectExpanded(project.id)"
            class="file-tree"
          >
            <div
              v-if="projectStore.isFileTreeLoading(project.id)"
              class="file-tree__loading"
            >
              <EaSkeleton
                variant="text"
                height="14px"
                width="80%"
                animation="wave"
              />
              <EaSkeleton
                variant="text"
                height="14px"
                width="60%"
                animation="wave"
              />
            </div>
            <FileTree
              v-else
              :project-id="project.id"
              :project-path="project.path"
              class="file-tree__content"
              @file-select="(path: string) => handleFileSelect(path, project)"
            />
          </div>
        </template>
      </div>
    </div>

    <!-- 创建项目弹框 -->
    <Teleport to="body">
      <Transition name="modal">
        <div
          v-if="uiStore.projectCreateModalVisible"
          class="modal-overlay"
          @click="uiStore.closeProjectCreateModal()"
        >
          <div
            class="modal-container"
            @click.stop
          >
            <ProjectCreateModal
              :project="editingProject"
              @submit="handleCreateProject"
              @cancel="uiStore.closeProjectCreateModal()"
            />
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- 删除确认弹框 -->
    <Teleport to="body">
      <Transition name="modal">
        <div
          v-if="showDeleteConfirm"
          class="modal-overlay"
          @click="showDeleteConfirm = false"
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
                {{ t('project.confirmDeleteTitle') }}
              </h4>
              <p class="confirm-dialog__message">
                {{ t('project.confirmDeleteMessage', { name: deletingProject?.name }) }}
              </p>
            </div>
            <div class="confirm-dialog__actions">
              <EaButton
                type="secondary"
                @click="showDeleteConfirm = false"
              >
                {{ t('common.cancel') }}
              </EaButton>
              <EaButton
                type="primary"
                @click="confirmDelete"
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

<style scoped src="./styles.css"></style>
