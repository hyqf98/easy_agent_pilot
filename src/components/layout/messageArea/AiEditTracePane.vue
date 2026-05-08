<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaButton, EaIcon } from '@/components/common'
import { useNotificationStore } from '@/stores/notification'
import { useAiEditTraceStore, useMessageStore, useProjectStore, useSessionStore } from '@/stores'
import { useTracePreviewStore } from '@/stores/tracePreview'
import { openProjectFileInWorkspace, useFileEditorStore } from '@/modules/fileEditor'
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
const isRollingBack = ref(false)
const rolledBackTraceIds = ref<Set<string>>(new Set())

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
  if (!session) return null
  return projectStore.projects.find(project => project.id === session.projectId) ?? null
})

const isSelectedTraceRolledBack = computed(() =>
  selectedTrace.value ? rolledBackTraceIds.value.has(selectedTrace.value.id) : false
)

const selectedTraceBeforeContent = computed(() => {
  return tracePreviewStore.beforeContent
    || selectedTrace.value?.preview?.beforeContent
    || selectedTrace.value?.preview?.beforeSnippet
    || ''
})

const selectedTraceAfterContent = computed(() => {
  if (selectedTrace.value?.changeType === 'delete') return ''

  return tracePreviewStore.afterContent || selectedTrace.value?.preview?.afterContent || tracePreviewStore.content
})

const selectedTraceRollbackContent = computed(() => {
  if (!selectedTrace.value || selectedTrace.value.changeType === 'create') return null
  return selectedTrace.value.preview?.beforeContent ?? null
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

const handleOpenInEditor = async () => {
  if (!currentProject.value || !selectedTrace.value) return
  await openProjectFileInWorkspace({
    projectId: currentProject.value.id,
    projectPath: currentProject.value.path,
    filePath: selectedTrace.value.filePath
  })
}

const refreshEditorIfNeeded = async (trace: FileEditTrace) => {
  if (!currentProject.value) return

  if (
    fileEditorStore.activeProjectPath !== currentProject.value.path
    || fileEditorStore.activeFilePath !== trace.filePath
  ) return

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

const handleAcceptLeft = async () => {
  if (!currentProject.value || !selectedTrace.value || isRollingBack.value) return
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
    rolledBackTraceIds.value.add(trace.id)

    await tracePreviewStore.openTracePreview({
      projectId: currentProject.value.id,
      projectPath: currentProject.value.path,
      trace
    })
  } catch (error) {
    notificationStore.error(
      t('trace.rollbackFailedTitle'),
      error instanceof Error ? error.message : String(error)
    )
  } finally {
    isRollingBack.value = false
  }
}

const handleAcceptRight = () => {
  // 当前文件就是右侧内容，无需操作
}

const formatChangeType = (changeType: FileEditTrace['changeType']) => {
  switch (changeType) {
    case 'create': return t('trace.changeCreate')
    case 'delete': return t('trace.changeDelete')
    default: return t('trace.changeModify')
  }
}
</script>

<template>
  <section class="trace-pane">
    <header class="trace-pane__header">
      <h3 class="trace-pane__title">
        {{ t('trace.title') }}
      </h3>
      <div class="trace-pane__actions">
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
        </EaButton>
        <EaButton
          type="ghost"
          size="small"
          @click="emit('close')"
        >
          <EaIcon
            name="x"
            :size="14"
          />
        </EaButton>
      </div>
    </header>

    <div
      v-if="groupedFiles.length > 0"
      class="trace-pane__body"
    >
      <aside class="trace-pane__files">
        <button
          v-for="group in groupedFiles"
          :key="group.filePath"
          class="trace-pane__file"
          :class="{ 'trace-pane__file--active': group.filePath === selectedGroup?.filePath }"
          @click="handleSelectTrace(group.latestTrace)"
        >
          <span class="trace-pane__file-name">{{ group.relativePath }}</span>
          <span
            class="trace-pane__file-tag"
            :class="`trace-pane__file-tag--${group.latestTrace.changeType}`"
          >
            {{ formatChangeType(group.latestTrace.changeType) }}
          </span>
        </button>
      </aside>

      <div class="trace-pane__viewer">
        <TraceDiffStack
          :before-content="selectedTraceBeforeContent"
          :after-content="selectedTraceAfterContent"
          :change-type="selectedTrace?.changeType ?? 'modify'"
          :focus-range="tracePreviewStore.highlightedRange"
          :rolled-back="isSelectedTraceRolledBack"
          @accept-left="handleAcceptLeft"
          @accept-right="handleAcceptRight"
        />
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
  background: var(--color-surface);
  border-right: 1px solid rgba(148, 163, 184, 0.18);
}

.trace-pane__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.16);
}

.trace-pane__title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.trace-pane__actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.trace-pane__body {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.trace-pane__files {
  width: 200px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  border-right: 1px solid rgba(148, 163, 184, 0.12);
  background: rgba(248, 250, 252, 0.5);
}

.trace-pane__file {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 8px 10px;
  border: none;
  border-bottom: 1px solid rgba(148, 163, 184, 0.08);
  background: transparent;
  text-align: left;
  cursor: pointer;
  transition: background 0.15s;
}

.trace-pane__file:hover {
  background: rgba(59, 130, 246, 0.04);
}

.trace-pane__file--active {
  background: rgba(59, 130, 246, 0.08);
  border-left: 2px solid #3b82f6;
}

.trace-pane__file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-family-mono);
  font-size: 12px;
  color: var(--color-text-primary);
}

.trace-pane__file-tag {
  flex-shrink: 0;
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 600;
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

.trace-pane__viewer {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

.trace-pane__empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--color-text-secondary);
  font-size: 13px;
}

:global([data-theme='dark']) .trace-pane,
:global(.dark) .trace-pane {
  background: rgba(15, 23, 42, 0.96);
  border-right-color: rgba(71, 85, 105, 0.4);
}

:global([data-theme='dark']) .trace-pane__header,
:global(.dark) .trace-pane__header {
  border-bottom-color: rgba(71, 85, 105, 0.5);
}

:global([data-theme='dark']) .trace-pane__title,
:global(.dark) .trace-pane__title {
  color: #f1f5f9;
}

:global([data-theme='dark']) .trace-pane__files,
:global(.dark) .trace-pane__files {
  border-right-color: rgba(71, 85, 105, 0.4);
  background: rgba(15, 23, 42, 0.5);
}

:global([data-theme='dark']) .trace-pane__file,
:global(.dark) .trace-pane__file {
  border-bottom-color: rgba(51, 65, 85, 0.4);
}

:global([data-theme='dark']) .trace-pane__file:hover,
:global(.dark) .trace-pane__file:hover {
  background: rgba(59, 130, 246, 0.08);
}

:global([data-theme='dark']) .trace-pane__file--active,
:global(.dark) .trace-pane__file--active {
  background: rgba(59, 130, 246, 0.14);
  border-left-color: #60a5fa;
}

:global([data-theme='dark']) .trace-pane__file-name,
:global(.dark) .trace-pane__file-name {
  color: #e2e8f0;
}

:global([data-theme='dark']) .trace-pane__file-tag--create,
:global(.dark) .trace-pane__file-tag--create {
  color: #6ee7b7;
  background: rgba(20, 83, 45, 0.4);
}

:global([data-theme='dark']) .trace-pane__file-tag--modify,
:global(.dark) .trace-pane__file-tag--modify {
  color: #93c5fd;
  background: rgba(30, 64, 175, 0.3);
}

:global([data-theme='dark']) .trace-pane__file-tag--delete,
:global(.dark) .trace-pane__file-tag--delete {
  color: #fca5a5;
  background: rgba(127, 29, 29, 0.4);
}

:global([data-theme='dark']) .trace-pane__empty,
:global(.dark) .trace-pane__empty {
  color: #94a3b8;
}

@media (max-width: 768px) {
  .trace-pane__files {
    width: 160px;
  }
}
</style>
