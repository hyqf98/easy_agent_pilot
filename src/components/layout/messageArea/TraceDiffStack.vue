<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { EaIcon } from '@/components/common'
import type { FileEditChangeType, FileEditRange } from '@/types/fileTrace'

type DiffOpType = 'equal' | 'remove' | 'add'

interface DiffOp {
  type: DiffOpType
  text: string
}

interface SideRow {
  lineNumber: number | null
  text: string
  variant: 'neutral' | 'changed'
}

const props = withDefaults(defineProps<{
  beforeContent: string
  afterContent: string
  changeType: FileEditChangeType
  focusRange?: FileEditRange | null
  rolledBack?: boolean
}>(), {
  focusRange: null,
  rolledBack: false
})

const emit = defineEmits<{
  acceptLeft: []
  acceptRight: []
}>()

const { t } = useI18n()
const beforeScrollRef = ref<HTMLElement | null>(null)
const afterScrollRef = ref<HTMLElement | null>(null)
const gutterScrollRef = ref<HTMLElement | null>(null)
const activeChangeIndex = ref(-1)

function normalizeLines(content: string): string[] {
  if (!content) return []
  const lines = content.replace(/\r\n/g, '\n').split('\n')
  if (lines[lines.length - 1] === '') lines.pop()
  return lines
}

function buildDiffOps(beforeLines: string[], afterLines: string[]): DiffOp[] {
  const dp = Array.from({ length: beforeLines.length + 1 }, () =>
    Array<number>(afterLines.length + 1).fill(0)
  )
  for (let i = beforeLines.length - 1; i >= 0; i--) {
    for (let j = afterLines.length - 1; j >= 0; j--) {
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
      i++
      j++
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      ops.push({ type: 'remove', text: beforeLines[i] })
      i++
    } else {
      ops.push({ type: 'add', text: afterLines[j] })
      j++
    }
  }
  while (i < beforeLines.length) {
    ops.push({ type: 'remove', text: beforeLines[i] })
    i++
  }
  while (j < afterLines.length) {
    ops.push({ type: 'add', text: afterLines[j] })
    j++
  }
  return ops
}

interface PairRow {
  before: SideRow
  after: SideRow
  isChanged: boolean
}

function buildPairRows(ops: DiffOp[]): PairRow[] {
  const rows: PairRow[] = []
  let bLine = 1
  let aLine = 1

  let idx = 0
  while (idx < ops.length) {
    const op = ops[idx]
    if (op.type === 'equal') {
      rows.push({
        before: { lineNumber: bLine++, text: op.text, variant: 'neutral' },
        after: { lineNumber: aLine++, text: op.text, variant: 'neutral' },
        isChanged: false
      })
      idx++
    } else {
      const removes: string[] = []
      const adds: string[] = []
      while (idx < ops.length && ops[idx].type === 'remove') {
        removes.push(ops[idx].text)
        idx++
      }
      while (idx < ops.length && ops[idx].type === 'add') {
        adds.push(ops[idx].text)
        idx++
      }
      const maxLen = Math.max(removes.length, adds.length)
      for (let k = 0; k < maxLen; k++) {
        const bText = k < removes.length ? removes[k] : ''
        const aText = k < adds.length ? adds[k] : ''
        const textsEqual = bText !== '' && aText !== '' && bText === aText
        rows.push({
          before: k < removes.length
            ? { lineNumber: bLine++, text: bText, variant: textsEqual ? 'neutral' : 'changed' }
            : { lineNumber: null, text: '', variant: 'neutral' },
          after: k < adds.length
            ? { lineNumber: aLine++, text: aText, variant: textsEqual ? 'neutral' : 'changed' }
            : { lineNumber: null, text: '', variant: 'neutral' },
          isChanged: !textsEqual
        })
      }
    }
  }
  return rows
}

const beforeLines = computed(() => normalizeLines(props.beforeContent))
const afterLines = computed(() => normalizeLines(props.afterContent))
const diffOps = computed(() => buildDiffOps(beforeLines.value, afterLines.value))
const pairRows = computed(() => buildPairRows(diffOps.value))

const changedRowIndices = computed(() =>
  pairRows.value
    .map((row, index) => row.isChanged ? index : -1)
    .filter(index => index !== -1)
)

const diffStats = computed(() => diffOps.value.reduce((stats, op) => {
  if (op.type === 'add') stats.added++
  else if (op.type === 'remove') stats.removed++
  return stats
}, { added: 0, removed: 0 }))

const hasChanges = computed(() => changedRowIndices.value.length > 0)

const handleAcceptLeft = () => emit('acceptLeft')
const handleAcceptRight = () => emit('acceptRight')

function handleGutterWheel(event: WheelEvent) {
  event.preventDefault()
  if (beforeScrollRef.value) {
    beforeScrollRef.value.scrollTop += event.deltaY
  }
}

function handleBeforeScroll() {
  handleScrollSync()
}

function handleAfterScroll() {
  if (!afterScrollRef.value) return
  const top = afterScrollRef.value.scrollTop
  if (beforeScrollRef.value) beforeScrollRef.value.scrollTop = top
  if (gutterScrollRef.value) gutterScrollRef.value.scrollTop = top
}

function findNearestChangeIndex(): number {
  const el = beforeScrollRef.value
  const indices = changedRowIndices.value
  if (!el || indices.length === 0) return -1

  const viewportTop = el.scrollTop + el.clientHeight / 3

  let bestIndex = 0
  let bestDist = Infinity
  for (let i = 0; i < indices.length; i++) {
    const row = el.querySelector(`[data-row-index="${indices[i]}"]`) as HTMLElement | null
    if (!row) continue
    const dist = Math.abs(row.offsetTop - viewportTop)
    if (dist < bestDist) {
      bestDist = dist
      bestIndex = i
    }
  }
  return bestIndex
}

function handleScrollSync() {
  if (changedRowIndices.value.length === 0) return
  activeChangeIndex.value = findNearestChangeIndex()
}

function scrollToRow(rowIndex: number) {
  const row = beforeScrollRef.value?.querySelector(`[data-row-index="${rowIndex}"]`) as HTMLElement | null
  if (!row || !beforeScrollRef.value) return
  const top = row.offsetTop - (beforeScrollRef.value.clientHeight / 2) + (row.offsetHeight / 2)
  beforeScrollRef.value.scrollTop = top
  if (afterScrollRef.value) afterScrollRef.value.scrollTop = top
  if (gutterScrollRef.value) gutterScrollRef.value.scrollTop = top
}

function handlePrevChange() {
  if (!hasChanges.value) return
  const current = findNearestChangeIndex()
  const target = current > 0 ? current - 1 : 0
  activeChangeIndex.value = target
  scrollToRow(changedRowIndices.value[target])
}

function handleNextChange() {
  if (!hasChanges.value) return
  const current = findNearestChangeIndex()
  const target = current < changedRowIndices.value.length - 1 ? current + 1 : changedRowIndices.value.length - 1
  activeChangeIndex.value = target
  scrollToRow(changedRowIndices.value[target])
}

watch(pairRows, () => {
  activeChangeIndex.value = -1
  if (changedRowIndices.value.length === 0) return

  nextTick(() => {
    const firstChangeIndex = changedRowIndices.value[0]
    activeChangeIndex.value = 0
    scrollToRow(firstChangeIndex)
  })
}, { immediate: true })
</script>

<template>
  <div class="diff-view">
    <div class="diff-view__header">
      <span class="diff-view__stats diff-view__stats--remove">-{{ diffStats.removed }}</span>
      <span class="diff-view__stats diff-view__stats--add">+{{ diffStats.added }}</span>

      <span class="diff-view__spacer" />

      <button
        class="diff-view__nav-btn"
        :disabled="!hasChanges"
        :title="t('trace.prevChange')"
        @click="handlePrevChange"
      >
        <EaIcon
          name="chevron-up"
          :size="14"
        />
      </button>
      <button
        class="diff-view__nav-btn"
        :disabled="!hasChanges"
        :title="t('trace.nextChange')"
        @click="handleNextChange"
      >
        <EaIcon
          name="chevron-down"
          :size="14"
        />
      </button>
    </div>

    <div class="diff-view__body">
      <div
        ref="beforeScrollRef"
        class="diff-view__panel diff-view__panel--before"
        @scroll="handleBeforeScroll"
      >
        <div class="diff-view__panel-head">
          {{ t('trace.before') }}
        </div>
        <div
          v-for="(row, index) in pairRows"
          :key="`b-${index}`"
          :data-row-index="index"
          class="diff-view__row"
          :class="{
            'diff-view__row--changed': row.before.variant === 'changed',
            'diff-view__row--empty': row.before.lineNumber === null,
            'diff-view__row--active': activeChangeIndex >= 0 && changedRowIndices[activeChangeIndex] === index
          }"
        >
          <span class="diff-view__num">{{ row.before.lineNumber ?? '' }}</span>
          <pre class="diff-view__text">{{ row.before.text }}</pre>
        </div>
      </div>

      <div
        ref="gutterScrollRef"
        class="diff-view__gutter"
        @wheel="handleGutterWheel"
      >
        <div class="diff-view__panel-head" />
        <template
          v-for="(row, index) in pairRows"
          :key="`g-${index}`"
        >
          <div
            v-if="row.isChanged && !rolledBack"
            class="diff-view__gutter-cell"
          >
            <button
              class="diff-view__arrow diff-view__arrow--left"
              :title="t('trace.acceptLeft')"
              @click="handleAcceptLeft"
            >
              <EaIcon
                name="chevrons-left"
                :size="12"
              />
            </button>
            <button
              class="diff-view__arrow diff-view__arrow--right"
              :title="t('trace.acceptRight')"
              @click="handleAcceptRight"
            >
              <EaIcon
                name="chevrons-right"
                :size="12"
              />
            </button>
          </div>
          <div
            v-else
            class="diff-view__gutter-cell"
          />
        </template>
      </div>

      <div
        ref="afterScrollRef"
        class="diff-view__panel diff-view__panel--after"
        @scroll="handleAfterScroll"
      >
        <div class="diff-view__panel-head">
          {{ t('trace.after') }}
        </div>
        <div
          v-for="(row, index) in pairRows"
          :key="`a-${index}`"
          class="diff-view__row"
          :class="{
            'diff-view__row--changed': row.after.variant === 'changed',
            'diff-view__row--empty': row.after.lineNumber === null,
            'diff-view__row--active': activeChangeIndex >= 0 && changedRowIndices[activeChangeIndex] === index
          }"
        >
          <span class="diff-view__num">{{ row.after.lineNumber ?? '' }}</span>
          <pre class="diff-view__text">{{ row.after.text }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.diff-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: var(--color-surface);
}

.diff-view__header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.16);
  font-family: var(--font-family-mono);
  font-size: 12px;
  font-weight: 600;
  flex-shrink: 0;
}

.diff-view__spacer {
  flex: 1;
}

.diff-view__stats {
  padding: 2px 8px;
  border-radius: 4px;
}

.diff-view__stats--remove {
  color: #b91c1c;
  background: rgba(254, 226, 226, 0.7);
}

.diff-view__stats--add {
  color: #15803d;
  background: rgba(220, 252, 231, 0.7);
}

.diff-view__nav-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: 1px solid rgba(148, 163, 184, 0.18);
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.6);
  color: var(--color-text-secondary);
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}

.diff-view__nav-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.9);
  color: var(--color-text-primary);
  border-color: rgba(59, 130, 246, 0.3);
}

.diff-view__nav-btn:disabled {
  opacity: 0.35;
  cursor: default;
}

.diff-view__body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.diff-view__panel {
  flex: 1;
  min-width: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
}

.diff-view__panel-head {
  position: sticky;
  top: 0;
  z-index: 2;
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  line-height: 20px;
  background: var(--color-surface);
  border-bottom: 1px solid rgba(148, 163, 184, 0.14);
  white-space: nowrap;
}

.diff-view__panel--before .diff-view__panel-head {
  color: #991b1b;
}

.diff-view__panel--after .diff-view__panel-head {
  color: #166534;
}

.diff-view__gutter {
  flex-shrink: 0;
  width: 44px;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
  border-left: 1px solid rgba(148, 163, 184, 0.1);
  border-right: 1px solid rgba(148, 163, 184, 0.1);
  scrollbar-width: none;
}

.diff-view__gutter::-webkit-scrollbar {
  display: none;
}

.diff-view__gutter .diff-view__panel-head {
  background: rgba(148, 163, 184, 0.03);
  border-bottom-color: rgba(148, 163, 184, 0.1);
}

.diff-view__gutter .diff-view__panel-head::after {
  content: '\00a0';
}

.diff-view__gutter-cell {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 2px;
  min-height: 20px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.04);
  background: rgba(148, 163, 184, 0.03);
}

.diff-view__row {
  display: flex;
  align-items: stretch;
  min-height: 20px;
  font-family: var(--font-family-mono);
  font-size: 12px;
  line-height: 20px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.04);
}

.diff-view__row--changed {
  background: rgba(254, 226, 226, 0.45);
}

.diff-view__panel--after .diff-view__row--changed {
  background: rgba(220, 252, 231, 0.45);
}

.diff-view__row--empty {
  background: rgba(148, 163, 184, 0.04);
}

.diff-view__row--active {
  outline: 2px solid rgba(59, 130, 246, 0.3);
  outline-offset: -2px;
}

.diff-view__arrow {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: none;
  border-radius: 3px;
  background: transparent;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.diff-view__arrow--left {
  color: #b91c1c;
}

.diff-view__arrow--left:hover {
  background: rgba(254, 226, 226, 0.6);
  color: #991b1b;
}

.diff-view__arrow--right {
  color: #15803d;
}

.diff-view__arrow--right:hover {
  background: rgba(220, 252, 231, 0.6);
  color: #166534;
}

.diff-view__num {
  flex-shrink: 0;
  width: 40px;
  padding: 0 6px;
  text-align: right;
  color: var(--color-text-tertiary);
  user-select: none;
  border-right: 1px solid rgba(148, 163, 184, 0.08);
  font-size: 11px;
}

.diff-view__text {
  flex: 1;
  min-width: 0;
  margin: 0;
  padding: 0 8px;
  color: var(--color-text-primary);
  font-family: inherit;
  font-size: inherit;
  line-height: inherit;
  white-space: pre;
}

:global([data-theme='dark']) .diff-view,
:global(.dark) .diff-view {
  background: rgba(15, 23, 42, 0.92);
}

:global([data-theme='dark']) .diff-view__header,
:global(.dark) .diff-view__header {
  border-bottom-color: rgba(71, 85, 105, 0.5);
}

:global([data-theme='dark']) .diff-view__stats--remove,
:global(.dark) .diff-view__stats--remove {
  color: #fecaca;
  background: rgba(127, 29, 29, 0.46);
}

:global([data-theme='dark']) .diff-view__stats--add,
:global(.dark) .diff-view__stats--add {
  color: #bbf7d0;
  background: rgba(20, 83, 45, 0.5);
}

:global([data-theme='dark']) .diff-view__nav-btn,
:global(.dark) .diff-view__nav-btn {
  border-color: rgba(71, 85, 105, 0.5);
  background: rgba(30, 41, 59, 0.8);
  color: #cbd5e1;
}

:global([data-theme='dark']) .diff-view__nav-btn:hover:not(:disabled),
:global(.dark) .diff-view__nav-btn:hover:not(:disabled) {
  background: rgba(30, 41, 59, 0.95);
  color: #f1f5f9;
  border-color: rgba(96, 165, 250, 0.4);
}

:global([data-theme='dark']) .diff-view__panel-head,
:global(.dark) .diff-view__panel-head {
  background: rgba(15, 23, 42, 0.98);
  border-bottom-color: rgba(71, 85, 105, 0.5);
}

:global([data-theme='dark']) .diff-view__panel--before .diff-view__panel-head,
:global(.dark) .diff-view__panel--before .diff-view__panel-head {
  color: #fca5a5;
}

:global([data-theme='dark']) .diff-view__panel--after .diff-view__panel-head,
:global(.dark) .diff-view__panel--after .diff-view__panel-head {
  color: #86efac;
}

:global([data-theme='dark']) .diff-view__gutter,
:global(.dark) .diff-view__gutter {
  border-left-color: rgba(71, 85, 105, 0.4);
  border-right-color: rgba(71, 85, 105, 0.4);
}

:global([data-theme='dark']) .diff-view__gutter .diff-view__panel-head,
:global(.dark) .diff-view__gutter .diff-view__panel-head {
  background: rgba(15, 23, 42, 0.5);
}

:global([data-theme='dark']) .diff-view__row,
:global(.dark) .diff-view__row {
  border-bottom-color: rgba(51, 65, 85, 0.3);
}

:global([data-theme='dark']) .diff-view__row--changed,
:global(.dark) .diff-view__row--changed {
  background: rgba(127, 29, 29, 0.22);
}

:global([data-theme='dark']) .diff-view__panel--after .diff-view__row--changed,
:global(.dark) .diff-view__panel--after .diff-view__row--changed {
  background: rgba(20, 83, 45, 0.22);
}

:global([data-theme='dark']) .diff-view__row--empty,
:global(.dark) .diff-view__row--empty {
  background: rgba(30, 41, 59, 0.2);
}

:global([data-theme='dark']) .diff-view__row--active,
:global(.dark) .diff-view__row--active {
  outline-color: rgba(96, 165, 250, 0.35);
}

:global([data-theme='dark']) .diff-view__num,
:global(.dark) .diff-view__num {
  border-right-color: rgba(51, 65, 85, 0.5);
  color: #64748b;
}

:global([data-theme='dark']) .diff-view__text,
:global(.dark) .diff-view__text {
  color: #e2e8f0;
}

:global([data-theme='dark']) .diff-view__arrow--left,
:global(.dark) .diff-view__arrow--left {
  color: #fca5a5;
}

:global([data-theme='dark']) .diff-view__arrow--left:hover,
:global(.dark) .diff-view__arrow--left:hover {
  background: rgba(127, 29, 29, 0.3);
  color: #fecaca;
}

:global([data-theme='dark']) .diff-view__arrow--right,
:global(.dark) .diff-view__arrow--right {
  color: #86efac;
}

:global([data-theme='dark']) .diff-view__arrow--right:hover,
:global(.dark) .diff-view__arrow--right:hover {
  background: rgba(20, 83, 45, 0.3);
  color: #bbf7d0;
}

@media (max-width: 768px) {
  .diff-view__row {
    font-size: 11px;
    line-height: 18px;
    min-height: 18px;
  }

  .diff-view__num {
    width: 30px;
    font-size: 10px;
  }

  .diff-view__gutter {
    width: 36px;
  }
}
</style>
