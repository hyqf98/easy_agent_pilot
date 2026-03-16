<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import type { FileEditChangeType, FileEditRange } from '@/types/fileTrace'

type DiffOpType = 'equal' | 'remove' | 'add'
type DiffRowVariant = 'neutral' | 'removed' | 'added'

interface DiffOp {
  type: DiffOpType
  text: string
}

interface DiffRow {
  marker: '+' | '-' | '·'
  lineNumber: number | null
  text: string
  variant: DiffRowVariant
}

type DisplayDiffRow =
  | (DiffRow & { type: 'row' })
  | {
    type: 'omitted'
    count: number
  }

interface WindowSlice {
  lines: string[]
  startLine: number
  truncated: boolean
}

const MAX_DIFF_LINES_WITHOUT_RANGE = 240
const DIFF_CONTEXT_LINES = 4
const COLLAPSE_CONTEXT_ROWS = 2

const props = withDefaults(defineProps<{
  beforeContent: string
  afterContent: string
  changeType: FileEditChangeType
  focusRange?: FileEditRange | null
}>(), {
  focusRange: null
})

const rootRef = ref<HTMLElement | null>(null)
const showFullContext = ref(false)

function normalizeLines(content: string): string[] {
  if (!content) {
    return []
  }

  const lines = content.replace(/\r\n/g, '\n').split('\n')
  if (lines[lines.length - 1] === '') {
    lines.pop()
  }
  return lines
}

function sliceWindow(lines: string[], range: FileEditRange | null, context: number): WindowSlice {
  if (range) {
    if (lines.length <= 18) {
      return {
        lines,
        startLine: 1,
        truncated: false
      }
    }

    const start = Math.max(0, range.startLine - 1 - context)
    const end = Math.min(lines.length, range.endLine + context)

    return {
      lines: lines.slice(start, end),
      startLine: start + 1,
      truncated: start > 0 || end < lines.length
    }
  }

  if (lines.length <= MAX_DIFF_LINES_WITHOUT_RANGE) {
    return {
      lines,
      startLine: 1,
      truncated: false
    }
  }

  // 没有 focusRange 时只保留有限窗口，避免对整份大文件执行高成本 diff。
  return {
    lines: lines.slice(0, MAX_DIFF_LINES_WITHOUT_RANGE),
    startLine: 1,
    truncated: true
  }
}

function buildDiffOps(beforeLines: string[], afterLines: string[]): DiffOp[] {
  const dp = Array.from({ length: beforeLines.length + 1 }, () =>
    Array<number>(afterLines.length + 1).fill(0)
  )

  for (let i = beforeLines.length - 1; i >= 0; i -= 1) {
    for (let j = afterLines.length - 1; j >= 0; j -= 1) {
      if (beforeLines[i] === afterLines[j]) {
        dp[i][j] = dp[i + 1][j + 1] + 1
      } else {
        dp[i][j] = Math.max(dp[i + 1][j], dp[i][j + 1])
      }
    }
  }

  const ops: DiffOp[] = []
  let i = 0
  let j = 0

  while (i < beforeLines.length && j < afterLines.length) {
    if (beforeLines[i] === afterLines[j]) {
      ops.push({ type: 'equal', text: beforeLines[i] })
      i += 1
      j += 1
      continue
    }

    if (dp[i + 1][j] >= dp[i][j + 1]) {
      ops.push({ type: 'remove', text: beforeLines[i] })
      i += 1
      continue
    }

    ops.push({ type: 'add', text: afterLines[j] })
    j += 1
  }

  while (i < beforeLines.length) {
    ops.push({ type: 'remove', text: beforeLines[i] })
    i += 1
  }

  while (j < afterLines.length) {
    ops.push({ type: 'add', text: afterLines[j] })
    j += 1
  }

  return ops
}

function buildBeforeRows(ops: DiffOp[], startLine: number): DiffRow[] {
  const rows: DiffRow[] = []
  let lineNumber = startLine

  for (const op of ops) {
    if (op.type === 'add') {
      continue
    }

    rows.push({
      marker: op.type === 'remove' ? '-' : '·',
      lineNumber,
      text: op.text,
      variant: op.type === 'remove' ? 'removed' : 'neutral'
    })
    lineNumber += 1
  }

  return rows
}

function buildAfterRows(ops: DiffOp[], startLine: number): DiffRow[] {
  const rows: DiffRow[] = []
  let lineNumber = startLine

  for (const op of ops) {
    if (op.type === 'remove') {
      continue
    }

    rows.push({
      marker: op.type === 'add' ? '+' : '·',
      lineNumber,
      text: op.text,
      variant: op.type === 'add' ? 'added' : 'neutral'
    })
    lineNumber += 1
  }

  return rows
}

function collapseNeutralRows(rows: DiffRow[]): DisplayDiffRow[] {
  if (showFullContext.value) {
    return rows.map(row => ({ ...row, type: 'row' as const }))
  }

  const changedIndexes = rows
    .map((row, index) => row.variant !== 'neutral' ? index : -1)
    .filter(index => index !== -1)

  if (changedIndexes.length === 0) {
    return rows.map(row => ({ ...row, type: 'row' as const }))
  }

  const keepIndexes = new Set<number>()
  for (const changedIndex of changedIndexes) {
    for (
      let index = Math.max(0, changedIndex - COLLAPSE_CONTEXT_ROWS);
      index <= Math.min(rows.length - 1, changedIndex + COLLAPSE_CONTEXT_ROWS);
      index += 1
    ) {
      keepIndexes.add(index)
    }
  }

  const collapsed: DisplayDiffRow[] = []
  let index = 0
  while (index < rows.length) {
    if (keepIndexes.has(index)) {
      collapsed.push({ ...rows[index], type: 'row' })
      index += 1
      continue
    }

    let omittedCount = 0
    while (index < rows.length && !keepIndexes.has(index)) {
      omittedCount += 1
      index += 1
    }

    collapsed.push({
      type: 'omitted',
      count: omittedCount
    })
  }

  return collapsed
}

const jumpToFirstChange = async () => {
  await nextTick()
  const firstChangedRow = rootRef.value?.querySelector('.trace-diff-stack__row--changed')
  if (!(firstChangedRow instanceof HTMLElement)) {
    return
  }

  firstChangedRow.scrollIntoView({
    block: 'center',
    behavior: 'smooth'
  })
}

const beforeWindow = computed(() => sliceWindow(normalizeLines(props.beforeContent), props.focusRange, DIFF_CONTEXT_LINES))
const afterWindow = computed(() => sliceWindow(normalizeLines(props.afterContent), props.focusRange, DIFF_CONTEXT_LINES))
const diffOps = computed(() => buildDiffOps(beforeWindow.value.lines, afterWindow.value.lines))
const beforeRows = computed(() => buildBeforeRows(diffOps.value, beforeWindow.value.startLine))
const afterRows = computed(() => buildAfterRows(diffOps.value, afterWindow.value.startLine))
const beforeDisplayRows = computed(() => collapseNeutralRows(beforeRows.value))
const afterDisplayRows = computed(() => collapseNeutralRows(afterRows.value))
const isWindowTruncated = computed(() => beforeWindow.value.truncated || afterWindow.value.truncated)
const diffStats = computed(() => diffOps.value.reduce((stats, op) => {
  if (op.type === 'add') {
    stats.added += 1
  } else if (op.type === 'remove') {
    stats.removed += 1
  }

  return stats
}, {
  added: 0,
  removed: 0
}))
</script>

<template>
  <div
    ref="rootRef"
    class="trace-diff-stack"
  >
    <div
      v-if="isWindowTruncated"
      class="trace-diff-stack__notice"
    >
      当前仅展示局部窗口以保证大文件对比性能。
    </div>
    <div
      v-if="changeType === 'modify' && (diffStats.added > 0 || diffStats.removed > 0)"
      class="trace-diff-stack__summary"
    >
      <span class="trace-diff-stack__summary-chip trace-diff-stack__summary-chip--removed">
        - {{ diffStats.removed }} 行
      </span>
      <span class="trace-diff-stack__summary-chip trace-diff-stack__summary-chip--added">
        + {{ diffStats.added }} 行
      </span>
      <button
        class="trace-diff-stack__summary-action"
        @click="showFullContext = !showFullContext"
      >
        {{ showFullContext ? '折叠未改动区块' : '展开完整上下文' }}
      </button>
      <button
        class="trace-diff-stack__summary-action"
        @click="jumpToFirstChange"
      >
        跳到首个变更
      </button>
    </div>

    <section class="trace-diff-stack__panel">
      <header class="trace-diff-stack__header trace-diff-stack__header--before">
        <span>修改前</span>
        <span class="trace-diff-stack__header-tag">
          {{ changeType === 'create' ? '文件不存在' : '历史版本' }}
        </span>
      </header>

      <div
        v-if="changeType === 'create'"
        class="trace-diff-stack__empty"
      >
        创建文件前没有内容
      </div>

      <div
        v-else
        class="trace-diff-stack__rows"
      >
        <div
          v-for="(row, index) in beforeDisplayRows"
          :key="`before-${index}-${row.type === 'row' ? row.lineNumber : `omitted-${row.count}`}`"
          class="trace-diff-stack__row-wrap"
        >
          <button
            v-if="row.type === 'omitted'"
            class="trace-diff-stack__omitted"
            @click="showFullContext = true"
          >
            展开未改动 {{ row.count }} 行
          </button>
          <div
            v-else
            class="trace-diff-stack__row"
            :class="{
              'trace-diff-stack__row--removed': row.variant === 'removed',
              'trace-diff-stack__row--neutral': row.variant === 'neutral',
              'trace-diff-stack__row--changed': row.variant !== 'neutral'
            }"
          >
            <span class="trace-diff-stack__marker">{{ row.marker }}</span>
            <span class="trace-diff-stack__line">{{ row.lineNumber }}</span>
            <code class="trace-diff-stack__code">{{ row.text || ' ' }}</code>
          </div>
        </div>
      </div>
    </section>

    <section class="trace-diff-stack__panel">
      <header class="trace-diff-stack__header trace-diff-stack__header--after">
        <span>AI 修改后</span>
        <span class="trace-diff-stack__header-tag">
          {{ changeType === 'delete' ? '文件已删除' : '当前结果' }}
        </span>
      </header>

      <div
        v-if="changeType === 'delete'"
        class="trace-diff-stack__empty trace-diff-stack__empty--danger"
      >
        这次改动会删除整个文件
      </div>

      <div
        v-else
        class="trace-diff-stack__rows"
      >
        <div
          v-for="(row, index) in afterDisplayRows"
          :key="`after-${index}-${row.type === 'row' ? row.lineNumber : `omitted-${row.count}`}`"
          class="trace-diff-stack__row-wrap"
        >
          <button
            v-if="row.type === 'omitted'"
            class="trace-diff-stack__omitted"
            @click="showFullContext = true"
          >
            展开未改动 {{ row.count }} 行
          </button>
          <div
            v-else
            class="trace-diff-stack__row"
            :class="{
              'trace-diff-stack__row--added': row.variant === 'added',
              'trace-diff-stack__row--neutral': row.variant === 'neutral',
              'trace-diff-stack__row--changed': row.variant !== 'neutral'
            }"
          >
            <span class="trace-diff-stack__marker">{{ row.marker }}</span>
            <span class="trace-diff-stack__line">{{ row.lineNumber }}</span>
            <code class="trace-diff-stack__code">{{ row.text || ' ' }}</code>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.trace-diff-stack {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  flex: 1;
  padding: 16px;
  overflow: auto;
  background:
    radial-gradient(circle at top right, rgba(59, 130, 246, 0.08), transparent 28%),
    linear-gradient(180deg, color-mix(in srgb, var(--color-surface-hover) 72%, transparent), var(--color-surface));
}

.trace-diff-stack__notice {
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(59, 130, 246, 0.18);
  background: rgba(59, 130, 246, 0.06);
  color: var(--color-text-secondary);
  font-size: 12px;
}

.trace-diff-stack__summary {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.trace-diff-stack__summary-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background: color-mix(in srgb, var(--color-surface) 92%, white 8%);
  color: var(--color-text-secondary);
  font-size: 11px;
  font-weight: 600;
}

.trace-diff-stack__summary-action:hover {
  color: var(--color-text-primary);
  border-color: rgba(59, 130, 246, 0.28);
}

.trace-diff-stack__summary-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.16);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.01em;
}

.trace-diff-stack__summary-chip--removed {
  color: #b91c1c;
  background: rgba(254, 226, 226, 0.72);
}

.trace-diff-stack__summary-chip--added {
  color: #15803d;
  background: rgba(220, 252, 231, 0.76);
}

.trace-diff-stack__panel {
  border: 1px solid rgba(148, 163, 184, 0.16);
  border-radius: 18px;
  overflow: hidden;
  background: color-mix(in srgb, var(--color-surface) 94%, white 6%);
  box-shadow: 0 12px 24px rgba(15, 23, 42, 0.06);
}

.trace-diff-stack__header {
  position: sticky;
  top: 0;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.16);
  font-size: 12px;
  font-weight: 700;
}

.trace-diff-stack__header--before {
  background: linear-gradient(180deg, rgba(254, 242, 242, 0.9), rgba(255, 255, 255, 0.96));
  color: #991b1b;
}

.trace-diff-stack__header--after {
  background: linear-gradient(180deg, rgba(240, 253, 244, 0.94), rgba(255, 255, 255, 0.96));
  color: #166534;
}

.trace-diff-stack__header-tag {
  font-size: 11px;
  font-weight: 600;
  opacity: 0.74;
}

.trace-diff-stack__rows {
  overflow: auto;
}

.trace-diff-stack__row-wrap {
  display: block;
}

.trace-diff-stack__row {
  display: grid;
  grid-template-columns: 24px 56px minmax(0, 1fr);
  align-items: stretch;
  min-height: 28px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.72);
  font-family: var(--font-family-mono);
  font-size: 12px;
}

.trace-diff-stack__row:last-child {
  border-bottom: none;
}

.trace-diff-stack__row--neutral {
  background: color-mix(in srgb, var(--color-surface-hover) 78%, transparent);
}

.trace-diff-stack__row--removed {
  background: rgba(254, 226, 226, 0.65);
}

.trace-diff-stack__row--added {
  background: rgba(220, 252, 231, 0.7);
}

.trace-diff-stack__omitted {
  width: 100%;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.72);
  background: color-mix(in srgb, var(--color-surface-hover) 82%, transparent);
  color: var(--color-text-secondary);
  font-size: 12px;
  text-align: left;
}

.trace-diff-stack__omitted:hover {
  color: var(--color-text-primary);
}

.trace-diff-stack__marker,
.trace-diff-stack__line {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-right: 1px solid rgba(226, 232, 240, 0.8);
  color: var(--color-text-tertiary);
  user-select: none;
}

.trace-diff-stack__marker {
  font-weight: 800;
}

.trace-diff-stack__row--removed .trace-diff-stack__marker {
  color: #dc2626;
}

.trace-diff-stack__row--added .trace-diff-stack__marker {
  color: #16a34a;
}

.trace-diff-stack__code {
  padding: 6px 12px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--color-text-primary);
  background: transparent;
}

.trace-diff-stack__empty {
  padding: 18px 16px;
  font-size: 13px;
  color: var(--color-text-secondary);
  background: rgba(248, 250, 252, 0.84);
}

.trace-diff-stack__empty--danger {
  color: #b91c1c;
  background: rgba(254, 242, 242, 0.86);
}

:global([data-theme='dark']) .trace-diff-stack,
:global(.dark) .trace-diff-stack {
  background:
    radial-gradient(circle at top right, rgba(96, 165, 250, 0.1), transparent 26%),
    linear-gradient(180deg, rgba(15, 23, 42, 0.64), rgba(2, 6, 23, 0.84));
}

:global([data-theme='dark']) .trace-diff-stack__summary-chip,
:global(.dark) .trace-diff-stack__summary-chip {
  border-color: rgba(71, 85, 105, 0.55);
}

:global([data-theme='dark']) .trace-diff-stack__summary-chip--removed,
:global(.dark) .trace-diff-stack__summary-chip--removed {
  color: #fecaca;
  background: rgba(127, 29, 29, 0.46);
}

:global([data-theme='dark']) .trace-diff-stack__summary-chip--added,
:global(.dark) .trace-diff-stack__summary-chip--added {
  color: #bbf7d0;
  background: rgba(20, 83, 45, 0.5);
}

:global([data-theme='dark']) .trace-diff-stack__panel,
:global(.dark) .trace-diff-stack__panel {
  border-color: rgba(71, 85, 105, 0.5);
  background: rgba(15, 23, 42, 0.84);
  box-shadow: 0 12px 28px rgba(2, 6, 23, 0.34);
}

:global([data-theme='dark']) .trace-diff-stack__header,
:global(.dark) .trace-diff-stack__header {
  border-bottom-color: rgba(71, 85, 105, 0.5);
}

:global([data-theme='dark']) .trace-diff-stack__header--before,
:global(.dark) .trace-diff-stack__header--before {
  background: linear-gradient(180deg, rgba(127, 29, 29, 0.54), rgba(15, 23, 42, 0.92));
  color: #fecaca;
}

:global([data-theme='dark']) .trace-diff-stack__header--after,
:global(.dark) .trace-diff-stack__header--after {
  background: linear-gradient(180deg, rgba(20, 83, 45, 0.58), rgba(15, 23, 42, 0.92));
  color: #bbf7d0;
}

:global([data-theme='dark']) .trace-diff-stack__row,
:global(.dark) .trace-diff-stack__row {
  border-bottom-color: rgba(51, 65, 85, 0.72);
}

:global([data-theme='dark']) .trace-diff-stack__row--neutral,
:global(.dark) .trace-diff-stack__row--neutral {
  background: rgba(15, 23, 42, 0.46);
}

:global([data-theme='dark']) .trace-diff-stack__row--removed,
:global(.dark) .trace-diff-stack__row--removed {
  background: rgba(127, 29, 29, 0.34);
}

:global([data-theme='dark']) .trace-diff-stack__row--added,
:global(.dark) .trace-diff-stack__row--added {
  background: rgba(20, 83, 45, 0.32);
}

:global([data-theme='dark']) .trace-diff-stack__marker,
:global([data-theme='dark']) .trace-diff-stack__line,
:global(.dark) .trace-diff-stack__marker,
:global(.dark) .trace-diff-stack__line {
  border-right-color: rgba(51, 65, 85, 0.76);
}

:global([data-theme='dark']) .trace-diff-stack__code,
:global(.dark) .trace-diff-stack__code {
  color: #e2e8f0;
}

:global([data-theme='dark']) .trace-diff-stack__empty,
:global(.dark) .trace-diff-stack__empty {
  color: #cbd5e1;
  background: rgba(15, 23, 42, 0.72);
}

:global([data-theme='dark']) .trace-diff-stack__empty--danger,
:global(.dark) .trace-diff-stack__empty--danger {
  color: #fecaca;
  background: rgba(127, 29, 29, 0.36);
}

@media (max-width: 768px) {
  .trace-diff-stack {
    padding: 10px;
  }

  .trace-diff-stack__row {
    grid-template-columns: 20px 44px minmax(0, 1fr);
    font-size: 11px;
  }

  .trace-diff-stack__header {
    padding: 10px 12px;
  }
}
</style>
