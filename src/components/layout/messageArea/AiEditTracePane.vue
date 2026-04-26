<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon } from '@/components/common'
import { useNotificationStore } from '@/stores/notification'
import { useAiEditTraceStore, useMessageStore, useProjectStore, useSessionStore } from '@/stores'
import { useTracePreviewStore } from '@/stores/tracePreview'
import { useFileEditorStore } from '@/modules/fileEditor'
import { deleteProjectFile, readProjectFile, writeProjectFile } from '@/modules/fileEditor/services/fileEditorService'
import TraceDiffStack from './TraceDiffStack.vue'
import type { FileEditTrace } from '@/types/fileTrace'

const { t } = useI18n()

interface FileTraceGroup {
  filePath: string
  relativePath: string
  traces: Array<FileEditTrace & { messageId: string }>
  latestTrace: FileEditTrace & { messageId: string }
}

const props = withDefaults(defineProps<{
  sessionId: string
  mobile?: boolean
}>(), {
  mobile: false
})

const emit = defineEmits<{
  close: []
}>()

const messageStore = useMessageStore()
const projectStore = useProjectStore()
const sessionStore = useSessionStore()
const aiEditTraceStore = useAiEditTraceStore()
const tracePreviewStore = useTracePreviewStore()
const fileEditorStore = useFileEditorStore()
const notificationStore = useNotificationStore()
const isFileDrawerVisible = ref(!props.mobile)
const isRollingBack = ref(false)

const sessionState = computed(() => aiEditTraceStore.getSessionState(props.sessionId))

const flattenedTraces = computed(() => {
  return messageStore.getAssistantEditTraces(props.sessionId)
})

const groupedFiles = computed<FileTraceGroup[]>(() => {
  const groups = new Map<string, FileTraceGroup>()

  for (const trace of flattenedTraces.value) {
    const existing = groups.get(trace.filePath)
    if (existing) {
      existing.traces.push(trace)
      existing.latestTrace = trace
      continue
    }

    groups.set(trace.filePath, {
      filePath: trace.filePath,
      relativePath: trace.relativePath,
      traces: [trace],
      latestTrace: trace
    })
  }

  return Array.from(groups.values())
    .map(group => ({
      ...group,
      traces: [...group.traces].sort((left, right) => right.timestamp.localeCompare(left.timestamp))
    }))
    .sort((left, right) => right.latestTrace.timestamp.localeCompare(left.latestTrace.timestamp))
})

const selectedTrace = computed(() =>
  aiEditTraceStore.findSelectedTrace(props.sessionId, flattenedTraces.value)
)

const selectedGroup = computed(() => {
  if (!selectedTrace.value) {
    return groupedFiles.value[0] ?? null
  }

  return groupedFiles.value.find(group => group.filePath === selectedTrace.value?.filePath) ?? null
})

const currentProject = computed(() => {
  const session = sessionStore.sessions.find(item => item.id === props.sessionId)
  if (!session) {
    return null
  }

  return projectStore.projects.find(project => project.id === session.projectId) ?? null
})

const selectedTraceBeforeContent = computed(() =>
  tracePreviewStore.beforeContent
  || selectedTrace.value?.preview?.beforeContent
  || selectedTrace.value?.preview?.beforeSnippet
  || ''
)

const selectedTraceAfterContent = computed(() => {
  if (selectedTrace.value?.changeType === 'delete') {
    return ''
  }

  return tracePreviewStore.afterContent || selectedTrace.value?.preview?.afterContent || tracePreviewStore.content
})

const selectedTraceRollbackContent = computed(() => {
  if (!selectedTrace.value || selectedTrace.value.changeType === 'create') {
    return null
  }

  return selectedTrace.value.preview?.beforeContent ?? null
})

const canRollbackSelectedTrace = computed(() => {
  if (!selectedTrace.value) {
    return false
  }

  if (selectedTrace.value.changeType === 'create') {
    return true
  }

  return selectedTraceRollbackContent.value !== null
})

const selectedTraceRevisionLabel = computed(() => {
  if (!selectedGroup.value || !selectedTrace.value) {
    return null
  }

  const revisionIndex = selectedGroup.value.traces.findIndex(trace => trace.id === selectedTrace.value?.id)
  if (revisionIndex === -1) {
    return null
  }

  return t('trace.revisionLabel', { version: selectedGroup.value.traces.length - revisionIndex })
})

const shouldShowRevisionChips = computed(() => (selectedGroup.value?.traces.length ?? 0) > 1)

const isSelectedTraceDiffReliable = computed(() => {
  if (!selectedTrace.value || selectedTrace.value.changeType !== 'modify') {
    return true
  }

  const preview = selectedTrace.value.preview
  if (!preview) {
    return false
  }

  if (preview.beforeContent || preview.afterContent) {
    return true
  }

  if (preview.beforeSnippet && preview.afterSnippet) {
    return preview.beforeSnippet !== preview.afterSnippet
  }

  return false
})

watch([selectedTrace, currentProject], async ([trace, project]) => {
  if (!trace || !project) {
    tracePreviewStore.clear()
    return
  }

  await tracePreviewStore.openTracePreview({
    projectId: project.id,
    projectPath: project.path,
    trace
  })
}, { immediate: true })

const handleSelectTrace = (trace: FileEditTrace & { messageId: string }) => {
  aiEditTraceStore.selectTrace(props.sessionId, {
    messageId: trace.messageId,
    traceId: trace.id,
    openPane: !props.mobile,
    openMobileDrawer: props.mobile,
    userInitiated: true
  })
}

const handleToggleAutoFollow = () => {
  aiEditTraceStore.toggleAutoFollow(props.sessionId)
}

const handleOpenInEditor = async () => {
  if (!currentProject.value || !selectedTrace.value) {
    return
  }

  await fileEditorStore.openFile({
    projectId: currentProject.value.id,
    projectPath: currentProject.value.path,
    filePath: selectedTrace.value.filePath
  })
}

const toggleFileDrawer = () => {
  isFileDrawerVisible.value = !isFileDrawerVisible.value
}

const refreshEditorIfNeeded = async (trace: FileEditTrace) => {
  if (!currentProject.value) {
    return
  }

  if (
    fileEditorStore.activeProjectPath !== currentProject.value.path
    || fileEditorStore.activeFilePath !== trace.filePath
  ) {
    return
  }

  if (fileEditorStore.isDirty) {
    notificationStore.warning(
      t('trace.editorRefreshWarningTitle'),
      t('trace.editorRefreshWarningMessage')
    )
    return
  }

  if (trace.changeType === 'create') {
    fileEditorStore.closeEditor()
    return
  }

  const nextContent = await readProjectFile(currentProject.value.path, trace.filePath)
  fileEditorStore.replaceContentSnapshot(nextContent.content)
}

const handleRollbackTrace = async () => {
  if (!currentProject.value || !selectedTrace.value || isRollingBack.value) {
    return
  }

  isRollingBack.value = true

  try {
    const trace = selectedTrace.value

    if (trace.changeType === 'create') {
      await deleteProjectFile(trace.filePath)
    } else {
      const rollbackContent = selectedTraceRollbackContent.value
      if (rollbackContent === null) {
        notificationStore.warning(
          t('trace.rollbackWarningTitle'),
          t('trace.rollbackWarningMessage')
        )
        return
      }

      await writeProjectFile({
        projectPath: currentProject.value.path,
        filePath: trace.filePath,
        content: rollbackContent
      })
    }

    await refreshEditorIfNeeded(trace)

    notificationStore.success(
      t('trace.rollbackSuccessTitle'),
      t('trace.rollbackSuccessMessage', { path: trace.relativePath })
    )
  } catch (error) {
    notificationStore.error(
      t('trace.rollbackFailedTitle'),
      error instanceof Error ? error.message : String(error)
    )
  } finally {
    isRollingBack.value = false
  }
}

const formatChangeType = (changeType: FileEditTrace['changeType']) => {
  switch (changeType) {
    case 'create':
      return t('trace.changeCreate')
    case 'delete':
      return t('trace.changeDelete')
    default:
      return t('trace.changeModify')
  }
}
</script>

<template>
  <section class="trace-pane">
    <header class="trace-pane__toolbar">
      <div class="trace-pane__toolbar-copy">
        <span class="trace-pane__eyebrow">{{ t('trace.eyebrow') }}</span>
        <h3 class="trace-pane__title">
          {{ t('trace.title') }}
        </h3>
      </div>
      <div class="trace-pane__toolbar-actions">
        <EaButton
          type="ghost"
          size="small"
          @click="toggleFileDrawer"
        >
          <EaIcon
            :name="isFileDrawerVisible ? 'panel-left-close' : 'panel-left-open'"
            :size="14"
          />
          {{ isFileDrawerVisible ? t('trace.hideFileList') : t('trace.fileListCount', { count: groupedFiles.length }) }}
        </EaButton>
        <EaButton
          type="ghost"
          size="small"
          @click="handleToggleAutoFollow"
        >
          <EaIcon
            :name="sessionState.isAutoFollow ? 'locate-fixed' : 'locate-off'"
            :size="14"
          />
          {{ sessionState.isAutoFollow ? t('trace.autoFollow') : t('trace.manualBrowse') }}
        </EaButton>
        <EaButton
          v-if="selectedTrace && currentProject"
          type="ghost"
          size="small"
          @click="handleOpenInEditor"
        >
          <EaIcon
            name="square-pen"
            :size="14"
          />
          {{ t('trace.fullEditor') }}
        </EaButton>
        <EaButton
          type="ghost"
          size="small"
          @click="emit('close')"
        >
          <EaIcon
            name="panel-left-close"
            :size="14"
          />
          {{ t('trace.hide') }}
        </EaButton>
      </div>
    </header>

    <div
      v-if="groupedFiles.length > 0"
      class="trace-pane__body"
    >
      <aside
        v-if="isFileDrawerVisible"
        class="trace-pane__file-drawer"
      >
        <div class="trace-pane__drawer-header">
          <div>
            <div class="trace-pane__drawer-eyebrow">
              {{ t('trace.editedFiles') }}
            </div>
            <div class="trace-pane__drawer-title">
              {{ t('trace.editsInRound', { count: groupedFiles.length }) }}
            </div>
          </div>
          <button
            class="trace-pane__drawer-close"
            @click="isFileDrawerVisible = false"
          >
            <EaIcon
              name="x"
              :size="14"
            />
          </button>
        </div>

        <div class="trace-pane__drawer-list">
          <button
            v-for="group in groupedFiles"
            :key="group.filePath"
            class="trace-pane__file-item"
            :class="{ 'trace-pane__file-item--active': group.filePath === selectedGroup?.filePath }"
            @click="handleSelectTrace(group.latestTrace)"
          >
            <div class="trace-pane__file-name-row">
              <span class="trace-pane__file-name">{{ group.relativePath }}</span>
              <span
                class="trace-pane__file-tag"
                :class="`trace-pane__file-tag--${group.latestTrace.changeType}`"
              >
                {{ formatChangeType(group.latestTrace.changeType) }}
              </span>
            </div>
            <div class="trace-pane__file-meta">
              <span>{{ t('trace.changeCount', { count: group.traces.length }) }}</span>
              <span>{{ group.latestTrace.range.startLine }}-{{ group.latestTrace.range.endLine }}</span>
            </div>
          </button>
        </div>
      </aside>

      <div class="trace-pane__viewer">
        <div class="trace-pane__viewer-hero">
          <div class="trace-pane__viewer-copy">
            <span class="trace-pane__viewer-kicker">{{ t('trace.tracePreview') }}</span>
            <div class="trace-pane__viewer-path">
              {{ tracePreviewStore.activeRelativePath || t('trace.noFileSelected') }}
            </div>
            <div class="trace-pane__viewer-summary">
              <span
                v-if="selectedTrace"
                class="trace-pane__file-tag"
                :class="`trace-pane__file-tag--${selectedTrace.changeType}`"
              >
                {{ formatChangeType(selectedTrace.changeType) }}
              </span>
              <span v-if="selectedTrace">
                {{ t('trace.lineRange', { start: selectedTrace.range.startLine, end: selectedTrace.range.endLine }) }}
              </span>
              <span v-if="selectedTraceRevisionLabel">{{ selectedTraceRevisionLabel }}</span>
            </div>
          </div>
        </div>

        <div
          v-if="selectedGroup && shouldShowRevisionChips"
          class="trace-pane__hunks"
        >
          <button
            v-for="trace in selectedGroup.traces"
            :key="trace.id"
            class="trace-pane__hunk-chip"
            :class="{ 'trace-pane__hunk-chip--active': trace.id === selectedTrace?.id }"
            @click="handleSelectTrace(trace)"
          >
            {{ trace.hunkHeader || `${trace.relativePath}:${trace.range.startLine}` }}
          </button>
        </div>

        <div class="trace-pane__editor">
          <div class="trace-pane__editor-meta">
            <span class="trace-pane__editor-path">{{ selectedTrace?.hunkHeader || t('trace.codePreview') }}</span>
            <div class="trace-pane__editor-actions">
              <span
                v-if="tracePreviewStore.loadError"
                class="trace-pane__editor-error"
              >
                {{ tracePreviewStore.loadError }}
              </span>
              <EaButton
                v-if="selectedTrace"
                type="danger"
                size="small"
                :loading="isRollingBack"
                :disabled="!canRollbackSelectedTrace"
                @click="handleRollbackTrace"
              >
                <EaIcon
                  name="rotate-ccw"
                  :size="14"
                />
                {{ canRollbackSelectedTrace ? t('trace.rollbackToBefore') : t('trace.oldRecordNoRollback') }}
              </EaButton>
            </div>
          </div>

          <div
            v-if="selectedTrace && !isSelectedTraceDiffReliable"
            class="trace-pane__history-note"
          >
            <EaIcon
              name="triangle-alert"
              :size="14"
            />
            <span>{{ t('trace.historyNote') }}</span>
          </div>

          <TraceDiffStack
            :before-content="selectedTraceBeforeContent"
            :after-content="selectedTraceAfterContent"
            :change-type="selectedTrace?.changeType ?? 'modify'"
            :focus-range="tracePreviewStore.highlightedRange"
          />
        </div>
      </div>
    </div>

    <div
      v-else
      class="trace-pane__empty"
    >
      <EaIcon
        name="file-search"
        :size="18"
      />
      <span>{{ t('trace.emptyMessage') }}</span>
    </div>
  </section>
</template>

<style scoped>
.trace-pane {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background:
    radial-gradient(circle at top right, rgba(59, 130, 246, 0.08), transparent 42%),
    linear-gradient(180deg, rgba(15, 23, 42, 0.02), transparent 32%),
    var(--color-surface);
  border-right: 1px solid rgba(148, 163, 184, 0.18);
}

.trace-pane__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  padding: var(--spacing-3) var(--spacing-4);
  border-bottom: 1px solid rgba(148, 163, 184, 0.16);
  backdrop-filter: blur(10px);
}

.trace-pane__toolbar-copy {
  min-width: 0;
}

.trace-pane__eyebrow {
  display: block;
  font-size: 11px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
}

.trace-pane__title {
  margin: 4px 0 0;
  font-size: var(--font-size-lg);
}

.trace-pane__toolbar-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-2);
  flex-wrap: wrap;
  justify-content: flex-end;
}

.trace-pane__body {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.trace-pane__file-drawer {
  position: absolute;
  left: 16px;
  top: 16px;
  bottom: 16px;
  z-index: 6;
  width: min(320px, calc(100% - 32px));
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 14px;
  border: 1px solid rgba(148, 163, 184, 0.18);
  border-radius: 24px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.95), rgba(248, 250, 252, 0.92)),
    rgba(255, 255, 255, 0.84);
  box-shadow: 0 24px 48px rgba(15, 23, 42, 0.16);
  backdrop-filter: blur(20px);
}

.trace-pane__drawer-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-3);
  padding-bottom: 12px;
}

.trace-pane__drawer-eyebrow {
  font-size: 10px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
}

.trace-pane__drawer-title {
  margin-top: 4px;
  font-size: var(--font-size-base);
  font-weight: 700;
  color: var(--color-text-primary);
}

.trace-pane__drawer-close,
.trace-pane__drawer-trigger {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 9px 12px;
  border: 1px solid rgba(148, 163, 184, 0.18);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.8);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: transform var(--transition-fast) var(--easing-default), box-shadow var(--transition-fast) var(--easing-default);
}

.trace-pane__drawer-close:hover,
.trace-pane__drawer-trigger:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-sm);
}

.trace-pane__drawer-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  min-height: 0;
  overflow: auto;
}

.trace-pane__viewer {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  padding: 16px 18px 18px;
  gap: 14px;
}

.trace-pane__viewer-hero {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-3);
  padding: 16px 18px;
  border: 1px solid rgba(148, 163, 184, 0.14);
  border-radius: 22px;
  background:
    radial-gradient(circle at top right, rgba(59, 130, 246, 0.12), transparent 36%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(248, 250, 252, 0.9));
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.66);
}

.trace-pane__viewer-copy {
  min-width: 0;
}

.trace-pane__viewer-kicker {
  display: inline-block;
  font-size: 10px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
}

.trace-pane__viewer-path {
  margin-top: 6px;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.trace-pane__viewer-summary {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 10px;
  font-size: 12px;
  color: var(--color-text-secondary);
}

.trace-pane__file-item {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-2);
  padding: var(--spacing-3);
  border: 1px solid rgba(148, 163, 184, 0.18);
  border-radius: calc(var(--radius-lg) + 2px);
  background: rgba(255, 255, 255, 0.72);
  text-align: left;
  cursor: pointer;
  transition: border-color var(--transition-fast) var(--easing-default), transform var(--transition-fast) var(--easing-default), box-shadow var(--transition-fast) var(--easing-default);
}

.trace-pane__file-item:hover,
.trace-pane__file-item--active {
  border-color: rgba(59, 130, 246, 0.32);
  transform: translateY(-1px);
  box-shadow: var(--shadow-sm);
}

.trace-pane__file-name-row,
.trace-pane__file-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-2);
}

.trace-pane__file-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  color: var(--color-text-primary);
}

.trace-pane__file-meta {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  font-variant-numeric: tabular-nums;
  color: var(--color-text-tertiary);
}

.trace-pane__file-tag {
  flex-shrink: 0;
  padding: 3px 8px;
  border-radius: var(--radius-full);
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
}

.trace-pane__file-tag--create {
  color: #047857;
  background: rgba(16, 185, 129, 0.12);
}

.trace-pane__file-tag--modify {
  color: #1d4ed8;
  background: rgba(59, 130, 246, 0.12);
}

.trace-pane__file-tag--delete {
  color: #b91c1c;
  background: rgba(239, 68, 68, 0.12);
}

.trace-pane__hunks {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-2);
  padding: 0 2px;
}

.trace-pane__hunk-chip {
  padding: 6px 10px;
  border-radius: var(--radius-full);
  border: 1px solid rgba(148, 163, 184, 0.2);
  background: rgba(255, 255, 255, 0.6);
  color: var(--color-text-secondary);
  font-size: 11px;
  font-family: var(--font-family-mono);
  cursor: pointer;
}

.trace-pane__hunk-chip--active {
  color: #1d4ed8;
  border-color: rgba(59, 130, 246, 0.34);
  background: rgba(59, 130, 246, 0.08);
}

.trace-pane__editor {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  border: 1px solid rgba(148, 163, 184, 0.16);
  border-radius: 22px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.9);
  box-shadow: 0 20px 40px rgba(15, 23, 42, 0.08);
}

.trace-pane__editor-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-3);
  padding: var(--spacing-2) var(--spacing-3);
  border-bottom: 1px solid rgba(148, 163, 184, 0.14);
  font-size: 12px;
  color: var(--color-text-secondary);
}

.trace-pane__editor-actions {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.trace-pane__editor-path {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-family-mono);
}

.trace-pane__editor-error {
  color: var(--color-warning);
}

.trace-pane__history-note {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.14);
  background: rgba(245, 158, 11, 0.08);
  color: var(--color-warning-dark);
  font-size: 12px;
}

:global([data-theme='dark']) .trace-pane__file-drawer,
:global(.dark) .trace-pane__file-drawer {
  border-color: rgba(71, 85, 105, 0.56);
  background:
    linear-gradient(180deg, rgba(15, 23, 42, 0.96), rgba(15, 23, 42, 0.92)),
    rgba(2, 6, 23, 0.76);
  box-shadow: 0 24px 48px rgba(2, 6, 23, 0.38);
}

:global([data-theme='dark']) .trace-pane__drawer-close,
:global([data-theme='dark']) .trace-pane__drawer-trigger,
:global(.dark) .trace-pane__drawer-close,
:global(.dark) .trace-pane__drawer-trigger {
  border-color: rgba(71, 85, 105, 0.5);
  background: rgba(30, 41, 59, 0.94);
  color: #cbd5e1;
}

:global([data-theme='dark']) .trace-pane__viewer-hero,
:global(.dark) .trace-pane__viewer-hero {
  border-color: rgba(71, 85, 105, 0.44);
  background:
    radial-gradient(circle at top right, rgba(96, 165, 250, 0.14), transparent 36%),
    linear-gradient(180deg, rgba(30, 41, 59, 0.96), rgba(15, 23, 42, 0.94));
  box-shadow: inset 0 1px 0 rgba(148, 163, 184, 0.08);
}

:global([data-theme='dark']) .trace-pane__viewer-kicker,
:global(.dark) .trace-pane__viewer-kicker {
  color: #94a3b8;
}

:global([data-theme='dark']) .trace-pane__viewer-path,
:global(.dark) .trace-pane__viewer-path {
  color: #f8fafc;
}

:global([data-theme='dark']) .trace-pane__viewer-summary,
:global(.dark) .trace-pane__viewer-summary {
  color: #cbd5e1;
}

:global([data-theme='dark']) .trace-pane__file-item,
:global(.dark) .trace-pane__file-item {
  border-color: rgba(71, 85, 105, 0.44);
  background: rgba(15, 23, 42, 0.78);
}

:global([data-theme='dark']) .trace-pane__file-item:hover,
:global([data-theme='dark']) .trace-pane__file-item--active,
:global(.dark) .trace-pane__file-item:hover,
:global(.dark) .trace-pane__file-item--active {
  border-color: rgba(96, 165, 250, 0.42);
  box-shadow: 0 10px 20px rgba(2, 6, 23, 0.22);
}

:global([data-theme='dark']) .trace-pane__file-name,
:global(.dark) .trace-pane__file-name {
  color: #e2e8f0;
}

:global([data-theme='dark']) .trace-pane__file-meta,
:global(.dark) .trace-pane__file-meta {
  color: #94a3b8;
}

:global([data-theme='dark']) .trace-pane__hunk-chip,
:global(.dark) .trace-pane__hunk-chip {
  border-color: rgba(71, 85, 105, 0.48);
  background: rgba(30, 41, 59, 0.88);
  color: #cbd5e1;
}

:global([data-theme='dark']) .trace-pane__hunk-chip--active,
:global(.dark) .trace-pane__hunk-chip--active {
  color: #bfdbfe;
  border-color: rgba(96, 165, 250, 0.46);
  background: rgba(30, 64, 175, 0.24);
}

:global([data-theme='dark']) .trace-pane__editor,
:global(.dark) .trace-pane__editor {
  border-color: rgba(71, 85, 105, 0.5);
  background: rgba(15, 23, 42, 0.92);
  box-shadow: 0 20px 40px rgba(2, 6, 23, 0.3);
}

:global([data-theme='dark']) .trace-pane__editor-meta,
:global(.dark) .trace-pane__editor-meta {
  border-bottom-color: rgba(71, 85, 105, 0.48);
  color: #94a3b8;
}

:global([data-theme='dark']) .trace-pane__deleted,
:global([data-theme='dark']) .trace-pane__empty,
:global(.dark) .trace-pane__deleted,
:global(.dark) .trace-pane__empty {
  color: #cbd5e1;
}

:global([data-theme='dark']) .trace-pane__deleted-title,
:global(.dark) .trace-pane__deleted-title {
  color: #f8fafc;
}

:global([data-theme='dark']) .trace-pane__history-note,
:global(.dark) .trace-pane__history-note {
  border-bottom-color: rgba(71, 85, 105, 0.5);
  background: rgba(120, 53, 15, 0.38);
  color: #fcd34d;
}

.trace-pane__deleted,
.trace-pane__empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-3);
  color: var(--color-text-secondary);
  padding: var(--spacing-6);
}

.trace-pane__deleted-title {
  font-weight: 600;
  color: var(--color-text-primary);
}

.trace-pane__deleted-text {
  margin-top: 4px;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
}

@media (max-width: 768px) {
  .trace-pane__toolbar {
    align-items: flex-start;
    flex-direction: column;
  }

  .trace-pane__viewer {
    padding: 12px;
  }

  .trace-pane__viewer-hero {
    flex-direction: column;
    align-items: stretch;
  }

  .trace-pane__editor-meta,
  .trace-pane__toolbar-actions {
    align-items: flex-start;
    flex-direction: column;
  }

  .trace-pane__editor-actions {
    width: 100%;
    justify-content: space-between;
  }

  .trace-pane__file-drawer {
    left: 12px;
    right: 12px;
    width: auto;
    top: 12px;
    bottom: 12px;
  }
}
</style>
