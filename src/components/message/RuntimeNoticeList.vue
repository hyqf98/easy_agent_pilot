<script setup lang="ts">
import { computed, ref } from 'vue'
import MarkdownRenderer from './MarkdownRenderer.vue'
import type { RuntimeNotice } from '@/utils/runtimeNotice'
import { getUsageNoticeSummary, summarizeRuntimeNotice } from '@/utils/runtimeNotice'

interface UsageFallback {
  model?: string
  inputTokens?: number
  outputTokens?: number
}

const props = withDefaults(defineProps<{
  notices: RuntimeNotice[]
  defaultExpanded?: boolean
  fallbackUsage?: UsageFallback | null
}>(), {
  defaultExpanded: false,
  fallbackUsage: null
})

const expandedIds = ref<Set<string>>(new Set(
  props.defaultExpanded ? props.notices.map(notice => notice.id) : []
))

const usageNotices = computed(() => props.notices.filter(isUsageNotice))
const regularNotices = computed(() => props.notices.filter(notice => !isUsageNotice(notice)))
const primaryRegularNotice = computed(() => regularNotices.value[0] ?? null)
const extraRegularNotices = computed(() => regularNotices.value.slice(1))
const primaryUsageNotice = computed(() => usageNotices.value[0] ?? null)
const extraUsageNotices = computed(() => usageNotices.value.slice(1))
const shouldUseCombinedSummary = computed(() => Boolean(primaryRegularNotice.value && primaryUsageNotice.value))

function toggleNotice(id: string) {
  const next = new Set(expandedIds.value)
  if (next.has(id)) {
    next.delete(id)
  } else {
    next.add(id)
  }
  expandedIds.value = next
}

function isExpanded(id: string) {
  return expandedIds.value.has(id)
}

function isUsageNotice(notice: RuntimeNotice) {
  return notice.id === 'usage' || notice.title.toLowerCase().includes('model')
}

function isEnvironmentNotice(notice: RuntimeNotice) {
  return notice.id === 'environment'
}

function noticeChips(notice: RuntimeNotice) {
  return summarizeRuntimeNotice(notice).map(chip => formatChipLabel(notice, chip))
}

function usageSummary(notice: RuntimeNotice) {
  const summary = getUsageNoticeSummary(notice)
  const fallback = props.fallbackUsage

  if (!summary) {
    if (!fallback) {
      return null
    }

    const hasFallbackInput = typeof fallback.inputTokens === 'number' && fallback.inputTokens > 0
    const hasFallbackOutput = typeof fallback.outputTokens === 'number' && fallback.outputTokens > 0

    return {
      model: fallback.model || null,
      input: hasFallbackInput ? String(fallback.inputTokens) : null,
      output: hasFallbackOutput ? String(fallback.outputTokens) : null
    }
  }

  const hasRealInput = Boolean(summary.input && summary.input !== '0')
  const hasRealOutput = Boolean(summary.output && summary.output !== '0')
  const fallbackInput = typeof fallback?.inputTokens === 'number' && fallback.inputTokens > 0 ? String(fallback.inputTokens) : null
  const fallbackOutput = typeof fallback?.outputTokens === 'number' && fallback.outputTokens > 0 ? String(fallback.outputTokens) : null

  return {
    model: summary.model || fallback?.model || null,
    input: hasRealInput ? summary.input : (fallbackInput || null),
    output: hasRealOutput ? summary.output : (fallbackOutput || null)
  }
}

function formatChipLabel(notice: RuntimeNotice, chip: string) {
  if (!isEnvironmentNotice(notice)) {
    return chip
  }

  return chip
    .replace(/^Skills\s+/i, 'Sk ')
    .replace(/^Plugins\s+/i, 'Pl ')
    .replace(/^Agents\s+/i, 'Ag ')
    .replace(/^Commands\s+/i, 'Cmd ')
  }
</script>

<template>
  <div class="runtime-notice-list">
    <article
      v-if="shouldUseCombinedSummary && primaryRegularNotice && primaryUsageNotice"
      class="runtime-notice runtime-notice--summary"
      :class="`runtime-notice--${primaryRegularNotice.tone || 'info'}`"
    >
      <div class="runtime-notice__summary">
        <button
          type="button"
          class="runtime-notice__summary-runtime"
          @click="toggleNotice(primaryRegularNotice.id)"
        >
          <span class="runtime-notice__eyebrow">Runtime</span>
          <span class="runtime-notice__title">{{ primaryRegularNotice.title }}</span>
          <div
            v-if="noticeChips(primaryRegularNotice).length > 0"
            class="runtime-notice__chips"
          >
            <span
              v-for="chip in noticeChips(primaryRegularNotice)"
              :key="chip"
              class="runtime-notice__chip"
            >
              {{ chip }}
            </span>
          </div>
          <span
            class="runtime-notice__chevron"
            :class="{ 'runtime-notice__chevron--expanded': isExpanded(primaryRegularNotice.id) }"
          >▼</span>
        </button>

        <div class="runtime-notice__summary-divider" />

        <div class="runtime-notice__summary-usage">
          <div class="runtime-notice__usage-main">
            <span class="runtime-notice__usage-label">Model</span>
            <span class="runtime-notice__usage-model">
              {{ usageSummary(primaryUsageNotice)?.model || 'Unknown' }}
            </span>
          </div>
          <div class="runtime-notice__usage-stats">
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--input">
              In {{ usageSummary(primaryUsageNotice)?.input || '—' }}
            </span>
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--output">
              Out {{ usageSummary(primaryUsageNotice)?.output || '—' }}
            </span>
          </div>
        </div>
      </div>

      <div
        v-show="isExpanded(primaryRegularNotice.id)"
        class="runtime-notice__content"
      >
        <MarkdownRenderer :content="primaryRegularNotice.content" />
      </div>
    </article>

    <template v-if="shouldUseCombinedSummary">
      <article
        v-for="notice in extraRegularNotices"
        :key="notice.id"
        class="runtime-notice"
        :class="`runtime-notice--${notice.tone || 'info'}`"
      >
        <button
          type="button"
          class="runtime-notice__header"
          @click="toggleNotice(notice.id)"
        >
          <div class="runtime-notice__header-main">
            <span class="runtime-notice__eyebrow">Runtime</span>
            <span class="runtime-notice__title">{{ notice.title }}</span>
          </div>
          <div class="runtime-notice__header-side">
            <div
              v-if="noticeChips(notice).length > 0"
              class="runtime-notice__chips"
            >
              <span
                v-for="chip in noticeChips(notice)"
                :key="chip"
                class="runtime-notice__chip"
              >
                {{ chip }}
              </span>
            </div>
            <span
              class="runtime-notice__chevron"
              :class="{ 'runtime-notice__chevron--expanded': isExpanded(notice.id) }"
            >▼</span>
          </div>
        </button>

        <div
          v-show="isExpanded(notice.id)"
          class="runtime-notice__content"
        >
          <MarkdownRenderer :content="notice.content" />
        </div>
      </article>

      <article
        v-for="notice in extraUsageNotices"
        :key="notice.id"
        class="runtime-notice runtime-notice--usage"
        :class="`runtime-notice--${notice.tone || 'info'}`"
      >
        <div class="runtime-notice__usage">
          <div class="runtime-notice__usage-main">
            <span class="runtime-notice__usage-label">Model</span>
            <span class="runtime-notice__usage-model">
              {{ usageSummary(notice)?.model || 'Unknown' }}
            </span>
          </div>
          <div class="runtime-notice__usage-stats">
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--input">
              In {{ usageSummary(notice)?.input || '—' }}
            </span>
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--output">
              Out {{ usageSummary(notice)?.output || '—' }}
            </span>
          </div>
        </div>
      </article>
    </template>

    <template v-else>
      <article
        v-for="notice in notices"
        :key="notice.id"
        class="runtime-notice"
        :class="[
          `runtime-notice--${notice.tone || 'info'}`,
          { 'runtime-notice--usage': isUsageNotice(notice) }
        ]"
      >
        <div
          v-if="isUsageNotice(notice)"
          class="runtime-notice__usage"
        >
          <div class="runtime-notice__usage-main">
            <span class="runtime-notice__usage-label">Model</span>
            <span class="runtime-notice__usage-model">
              {{ usageSummary(notice)?.model || 'Unknown' }}
            </span>
          </div>
          <div class="runtime-notice__usage-stats">
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--input">
              In {{ usageSummary(notice)?.input || '—' }}
            </span>
            <span class="runtime-notice__usage-chip runtime-notice__usage-chip--output">
              Out {{ usageSummary(notice)?.output || '—' }}
            </span>
          </div>
        </div>

        <template v-else>
          <button
            type="button"
            class="runtime-notice__header"
            @click="toggleNotice(notice.id)"
          >
            <div class="runtime-notice__header-main">
              <span class="runtime-notice__eyebrow">Runtime</span>
              <span class="runtime-notice__title">{{ notice.title }}</span>
            </div>
            <div class="runtime-notice__header-side">
              <div
                v-if="noticeChips(notice).length > 0"
                class="runtime-notice__chips"
              >
                <span
                  v-for="chip in noticeChips(notice)"
                  :key="chip"
                  class="runtime-notice__chip"
                >
                  {{ chip }}
                </span>
              </div>
              <span
                class="runtime-notice__chevron"
                :class="{ 'runtime-notice__chevron--expanded': isExpanded(notice.id) }"
              >▼</span>
            </div>
          </button>

          <div
            v-show="isExpanded(notice.id)"
            class="runtime-notice__content"
          >
            <MarkdownRenderer :content="notice.content" />
          </div>
        </template>
      </article>
    </template>
  </div>
</template>

<style scoped>
.runtime-notice-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: 100%;
}

.runtime-notice {
  border-radius: 0.85rem;
  border: 1px solid var(--runtime-notice-border);
  background: var(--runtime-notice-bg);
  box-shadow: 0 10px 22px rgba(15, 23, 42, 0.04);
  overflow: hidden;
}

.runtime-notice--summary {
  overflow: visible;
}

.runtime-notice--usage {
  border-color: var(--runtime-notice-usage-border);
  background: var(--runtime-notice-usage-bg);
}

.runtime-notice--info {
  border-color: rgba(14, 165, 233, 0.12);
}

.runtime-notice--success {
  border-color: rgba(34, 197, 94, 0.14);
}

.runtime-notice--warning {
  border-color: rgba(245, 158, 11, 0.16);
}

.runtime-notice__summary {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(10.5rem, auto);
  align-items: center;
  gap: 0.55rem;
  padding: 0.55rem 0.7rem;
}

.runtime-notice__summary-runtime,
.runtime-notice__header {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem;
  padding: 0.55rem 0.7rem;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.runtime-notice__summary-runtime {
  padding: 0;
  min-width: 0;
}

.runtime-notice__summary-runtime:hover,
.runtime-notice__header:hover {
  background: var(--runtime-notice-hover);
}

.runtime-notice__header-main {
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 0.45rem;
}

.runtime-notice__eyebrow,
.runtime-notice__usage-label {
  font-size: 0.58rem;
  line-height: 1;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.runtime-notice__title {
  min-width: 0;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.runtime-notice__header-side {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.45rem;
  min-width: 0;
  flex: 1;
}

.runtime-notice__chips {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.28rem;
  min-width: 0;
}

.runtime-notice__chip,
.runtime-notice__usage-chip {
  max-width: 100%;
  padding: 0.18rem 0.42rem;
  border-radius: 999px;
  background: var(--runtime-notice-chip-bg);
  color: var(--runtime-notice-chip-text);
  font-size: 0.64rem;
  line-height: 1.1;
  white-space: nowrap;
}

.runtime-notice__chevron {
  flex-shrink: 0;
  font-size: 0.66rem;
  color: var(--color-text-tertiary);
  transition: transform 0.18s ease;
}

.runtime-notice__chevron--expanded {
  transform: rotate(180deg);
}

.runtime-notice__summary-divider {
  width: 1px;
  height: 1.65rem;
  background: var(--runtime-notice-content-border);
}

.runtime-notice__content {
  padding: 0 0.9rem 0.85rem;
  border-top: 1px solid var(--runtime-notice-content-border);
}

.runtime-notice__usage,
.runtime-notice__summary-usage {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.55rem;
  min-width: 0;
}

.runtime-notice__usage {
  padding: 0.58rem 0.72rem;
}

.runtime-notice__usage-main {
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 0.42rem;
}

.runtime-notice__usage-model {
  min-width: 0;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--runtime-notice-usage-model);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.runtime-notice__usage-stats {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.28rem;
  min-width: 0;
}

.runtime-notice__usage-chip {
  background: rgba(37, 99, 235, 0.08);
}

.runtime-notice__usage-chip--output {
  background: rgba(14, 165, 233, 0.1);
}

@media (max-width: 768px) {
  .runtime-notice__summary {
    grid-template-columns: minmax(0, 1fr);
  }

  .runtime-notice__summary-divider {
    display: none;
  }

  .runtime-notice__summary-usage,
  .runtime-notice__usage,
  .runtime-notice__header,
  .runtime-notice__summary-runtime {
    flex-wrap: wrap;
  }

  .runtime-notice__header-side,
  .runtime-notice__usage-stats {
    width: 100%;
    justify-content: flex-start;
  }
}
</style>
