<script setup lang="ts">
import EaModal from '@/components/common/EaModal.vue'
import {
  useTaskEditModal,
  type TaskEditModalEmits,
  type TaskEditModalProps
} from './useTaskEditModal'

const props = defineProps<TaskEditModalProps>()
const emit = defineEmits<TaskEditModalEmits>()

const {
  t,
  form,
  isCreateMode,
  isSaving,
  expandedSections,
  isDepDropdownOpen,
  depDropdownRef,
  formScrollRef,
  depDropdownDirection,
  depDropdownMaxHeight,
  priorityOptions,
  expertOptions,
  modelOptions,
  executionConfigHint,
  dependencyOptions,
  selectedDependencies,
  toggleSection,
  toggleDepDropdown,
  handleDependencyToggle,
  isDependencySelected,
  removeDependency,
  addStep,
  removeStep,
  handleSave,
  close
} = useTaskEditModal(props, emit)
</script>

<template>
  <EaModal
    :visible="visible"
    content-class="task-edit-modal-dialog"
    overlay-class="task-edit-modal-overlay"
    @update:visible="emit('update:visible', $event)"
  >
    <template #header>
      <div class="modal-header">
        <div class="header-title">
          <div class="header-icon">
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
            </svg>
          </div>
          <h3>{{ isCreateMode ? '创建新任务' : '编辑任务' }}</h3>
        </div>
        <button
          class="btn-close"
          @click="close"
        >
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    </template>

    <div
      class="task-edit-modal"
      :class="{ 'task-edit-modal--dropdown-active': isDepDropdownOpen }"
    >
      <div
        ref="formScrollRef"
        class="modal-body"
      >
        <!-- 基本信息 -->
        <div class="form-section">
          <div class="section-title">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <circle
                cx="12"
                cy="12"
                r="10"
              />
              <path d="M12 16v-4M12 8h.01" />
            </svg>
            基本信息
          </div>

          <div class="form-grid">
            <div class="form-field full-width">
              <label>任务标题 <span class="required">*</span></label>
              <input
                v-model="form.title"
                type="text"
                placeholder="输入任务标题..."
                class="input-title"
              >
            </div>

            <div class="form-field full-width">
              <label>任务描述</label>
              <textarea
                v-model="form.description"
                rows="3"
                placeholder="描述任务的目标和要求..."
              />
            </div>

            <div class="form-field">
              <label>优先级</label>
              <div class="priority-buttons">
                <button
                  v-for="opt in priorityOptions"
                  :key="opt.value"
                  type="button"
                  class="priority-btn"
                  :class="[opt.value, { active: form.priority === opt.value }]"
                  @click="form.priority = opt.value"
                >
                  <span class="priority-dot" />
                  {{ opt.label }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 专家配置 -->
        <div class="form-section">
          <div class="section-title">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M12 2a3 3 0 00-3 3v4a3 3 0 006 0V5a3 3 0 00-3-3z" />
              <path d="M19 10v2a7 7 0 01-14 0v-2" />
              <line
                x1="12"
                y1="19"
                x2="12"
                y2="22"
              />
            </svg>
            执行配置
          </div>

          <div class="form-grid">
            <div class="form-field full-width">
              <label>执行专家</label>
              <select
                v-model="form.expertId"
                class="execution-select"
              >
                <option value="">
                  请选择执行专家
                </option>
                <option
                  v-for="expert in expertOptions"
                  :key="expert.id"
                  :value="expert.id"
                >
                  {{ expert.name }}
                </option>
              </select>
              <span class="field-hint">{{ executionConfigHint }}</span>
            </div>

            <div class="form-field full-width">
              <label>{{ t('task.selectModel') }}</label>
              <select
                v-model="form.modelId"
                class="execution-select"
                :disabled="modelOptions.length === 0"
              >
                <option
                  v-if="modelOptions.length === 0"
                  value=""
                >
                  当前专家绑定运行时暂无可用模型
                </option>
                <option
                  v-for="opt in modelOptions"
                  :key="opt.value"
                  :value="opt.value"
                >
                  {{ opt.label }}
                </option>
              </select>
            </div>
          </div>
        </div>

        <!-- 任务详情 -->
        <div class="form-section collapsible">
          <div
            class="section-header"
            @click="toggleSection('details')"
          >
            <div class="section-title">
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
                <polyline points="14,2 14,8 20,8" />
                <line
                  x1="16"
                  y1="13"
                  x2="8"
                  y2="13"
                />
                <line
                  x1="16"
                  y1="17"
                  x2="8"
                  y2="17"
                />
              </svg>
              任务详情
            </div>
            <svg
              class="section-chevron"
              :class="{ expanded: expandedSections.details }"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M6 9l6 6 6-6" />
            </svg>
          </div>
          <div
            v-show="expandedSections.details"
            class="section-content"
          >
            <!-- 实现步骤 -->
            <div class="form-field">
              <label>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="M12 20V10M18 20V4M6 20v-4" />
                </svg>
                实现步骤
              </label>
              <div class="steps-container">
                <div
                  v-for="(_, i) in form.implementationSteps"
                  :key="i"
                  class="step-card"
                >
                  <span class="step-number">{{ i + 1 }}</span>
                  <textarea
                    v-model="form.implementationSteps[i]"
                    class="step-textarea"
                    rows="3"
                    spellcheck="false"
                    :placeholder="`描述第 ${i + 1} 步操作...`"
                  />
                  <button
                    class="step-remove"
                    @click="removeStep('implementationSteps', i)"
                  >
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="M18 6L6 18M6 6l12 12" />
                    </svg>
                  </button>
                </div>
                <button
                  class="add-step-btn"
                  @click="addStep('implementationSteps')"
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <path d="M12 5v14M5 12h14" />
                  </svg>
                  添加步骤
                </button>
              </div>
            </div>

            <!-- 测试步骤 -->
            <div class="form-field">
              <label>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="M9 11l3 3L22 4" />
                  <path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11" />
                </svg>
                测试步骤
              </label>
              <div class="steps-container">
                <div
                  v-for="(_, i) in form.testSteps"
                  :key="i"
                  class="step-card test"
                >
                  <span class="step-number">{{ i + 1 }}</span>
                  <textarea
                    v-model="form.testSteps[i]"
                    class="step-textarea"
                    rows="3"
                    spellcheck="false"
                    :placeholder="`描述第 ${i + 1} 步测试...`"
                  />
                  <button
                    class="step-remove"
                    @click="removeStep('testSteps', i)"
                  >
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="M18 6L6 18M6 6l12 12" />
                    </svg>
                  </button>
                </div>
                <button
                  class="add-step-btn test"
                  @click="addStep('testSteps')"
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <path d="M12 5v14M5 12h14" />
                  </svg>
                  添加测试
                </button>
              </div>
            </div>

            <!-- 验收标准 -->
            <div class="form-field">
              <label>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="M22 11.08V12a10 10 0 11-5.93-9.14" />
                  <polyline points="22,4 12,14.01 9,11.01" />
                </svg>
                验收标准
              </label>
              <div class="criteria-container">
                <div
                  v-for="(_, i) in form.acceptanceCriteria"
                  :key="i"
                  class="criteria-card"
                >
                  <span class="criteria-icon">
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <polyline points="20,6 9,17 4,12" />
                    </svg>
                  </span>
                  <textarea
                    v-model="form.acceptanceCriteria[i]"
                    class="criteria-textarea"
                    rows="3"
                    spellcheck="false"
                    :placeholder="`验收标准 ${i + 1}...`"
                  />
                  <button
                    class="criteria-remove"
                    @click="removeStep('acceptanceCriteria', i)"
                  >
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="M18 6L6 18M6 6l12 12" />
                    </svg>
                  </button>
                </div>
                <button
                  class="add-criteria-btn"
                  @click="addStep('acceptanceCriteria')"
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <path d="M12 5v14M5 12h14" />
                  </svg>
                  添加标准
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 依赖关系 -->
        <div class="form-section">
          <div class="section-title">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <circle
                cx="18"
                cy="5"
                r="3"
              />
              <circle
                cx="6"
                cy="12"
                r="3"
              />
              <circle
                cx="18"
                cy="19"
                r="3"
              />
              <line
                x1="8.59"
                y1="13.51"
                x2="15.42"
                y2="17.49"
              />
              <line
                x1="15.41"
                y1="6.51"
                x2="8.59"
                y2="10.49"
              />
            </svg>
            依赖关系
          </div>

          <div
            v-if="dependencyOptions.length > 0"
            ref="depDropdownRef"
            class="dependency-selector"
            :class="[
              { open: isDepDropdownOpen },
              depDropdownDirection === 'up' ? 'dependency-selector--up' : 'dependency-selector--down'
            ]"
          >
            <div
              class="dependency-trigger"
              @click.stop="toggleDepDropdown"
            >
              <div
                v-if="selectedDependencies.length === 0"
                class="dep-placeholder"
              >
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
                  <line
                    x1="12"
                    y1="9"
                    x2="12"
                    y2="13"
                  />
                  <line
                    x1="12"
                    y1="17"
                    x2="12.01"
                    y2="17"
                  />
                </svg>
                选择依赖的任务（可选）
              </div>
              <div
                v-else
                class="dep-tags"
              >
                <span
                  v-for="dependency in selectedDependencies"
                  :key="dependency.value"
                  class="dep-tag"
                >
                  {{ dependency.label }}
                  <button
                    type="button"
                    class="tag-remove"
                    @click.stop="removeDependency(dependency.value)"
                  >
                    <svg
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="M18 6L6 18M6 6l12 12" />
                    </svg>
                  </button>
                </span>
              </div>
              <svg
                class="dep-chevron"
                :class="{ rotated: isDepDropdownOpen }"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path d="M6 9l6 6 6-6" />
              </svg>
            </div>
            <div
              v-show="isDepDropdownOpen"
              class="dependency-dropdown"
              :style="{ maxHeight: `${depDropdownMaxHeight}px` }"
            >
              <label
                v-for="option in dependencyOptions"
                :key="option.value"
                class="dep-option"
                :class="{ selected: isDependencySelected(option.value as string) }"
              >
                <input
                  type="checkbox"
                  :checked="isDependencySelected(option.value as string)"
                  @change="handleDependencyToggle(option.value as string)"
                >
                <span class="checkbox-visual">
                  <svg
                    v-if="isDependencySelected(option.value as string)"
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="3"
                  >
                    <polyline points="20,6 9,17 4,12" />
                  </svg>
                </span>
                <span class="option-label">{{ option.label }}</span>
              </label>
            </div>
          </div>
          <div
            v-else
            class="empty-dependencies"
          >
            <svg
              width="32"
              height="32"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            >
              <rect
                x="3"
                y="3"
                width="18"
                height="18"
                rx="2"
                ry="2"
              />
              <line
                x1="9"
                y1="9"
                x2="15"
                y2="15"
              />
              <line
                x1="15"
                y1="9"
                x2="9"
                y2="15"
              />
            </svg>
            <span>{{ t('task.noTasksAvailable') }}</span>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="modal-footer">
        <button
          class="btn-cancel"
          @click="close"
        >
          取消
        </button>
        <button
          class="btn-save"
          :disabled="isSaving || !form.title.trim()"
          @click="handleSave"
        >
          <svg
            v-if="isSaving"
            class="spin"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M21 12a9 9 0 11-6.219-8.56" />
          </svg>
          {{ isSaving ? '保存中...' : (isCreateMode ? '创建任务' : '保存修改') }}
        </button>
      </div>
    </template>
  </EaModal>
</template>

<style scoped src="./styles.css"></style>
